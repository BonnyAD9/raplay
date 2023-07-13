use std::sync::{Arc, Mutex};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream
};
use eyre::{Report, Result};

use crate::{operate_samples, sample_buffer::SampleBufferMut, source::{Source, DeviceInfo}};

/// A player that can play `Source`
pub struct Sink {
    controls: Arc<Mutex<Controls>>,
    #[allow(dead_code)]
    stream: Stream,
    info: DeviceInfo,
}

struct Controls {
    source: Option<Box<dyn Source>>,
}

impl Sink {
    /// Creates the player from the default audio output device with the
    /// default configuration
    pub fn default_out() -> Result<Sink> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(Report::msg("No available output device"))?;
        let config = device.default_output_config()?;
        let sample_format = config.sample_format();

        let info = DeviceInfo {
            channel_count: config.channels() as u32,
            sample_rate: config.sample_rate().0,
            sample_format: config.sample_format(),
        };

        let config = config.into();

        let controls = Arc::new(Mutex::new(Controls { source: None }));
        let cap_controls = controls.clone();

        let err = |e| { println!("{e}") };

        macro_rules! arm {
            ($t:ident, $e:ident) => {
                device.build_output_stream(
                    &config,
                    move |d: &mut [$t], _| {
                        if cap_controls
                            .as_ref()
                            .lock()
                            .as_mut()
                            .and_then(|c| {
                                c.write_source(&mut SampleBufferMut::$e(d));
                                Ok(())
                            })
                            .is_err()
                        {
                            write_silence(d);
                        }
                    },
                    err,
                    //Some(Duration::from_millis(5)),
                    None,
                )
            };
        }

        let stream = match sample_format {
            SampleFormat::I8 => arm!(i8, I8),
            SampleFormat::I16 => arm!(i16, I16),
            SampleFormat::I32 => arm!(i32, I32),
            SampleFormat::I64 => arm!(i64, I64),
            SampleFormat::U8 => arm!(u8, U8),
            SampleFormat::U16 => arm!(u16, U16),
            SampleFormat::U32 => arm!(u32, U32),
            SampleFormat::U64 => arm!(u64, U64),
            SampleFormat::F32 => arm!(f32, F32),
            SampleFormat::F64 => arm!(f64, F64),
            _ => {
                return Err(Report::msg(
                    "Unsupported sample format '{sample_format}'",
                ))
            }
        }?;

        stream.play()?;

        let sink = Sink { controls, stream, info };

        Ok(sink)
    }

    /// Discards the old source and starts playing the given source
    pub fn play(&self, mut src: impl Source + 'static) -> Result<()> {
        src.init(&self.info);
        match self.controls.lock().and_then(|mut c| {
            c.source = Some(Box::new(src));
            Ok(())
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(Report::msg(e.to_string())),
        }
    }
}

impl Controls {
    fn write_source(&mut self, data: &mut SampleBufferMut) {
        if self.source.is_some() {
            let mut src = self.source.take().unwrap();
            let i = src.read(data);
            operate_samples!(data, d, write_silence(&mut d[i..]));
            self.source = Some(src)
        } else {
            operate_samples!(data, d, write_silence(*d))
        }
    }
}

fn write_silence<T: cpal::Sample>(data: &mut [T]) {
    data.fill(T::EQUILIBRIUM);
}
