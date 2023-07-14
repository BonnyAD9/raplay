use std::sync::{Arc, Mutex, MutexGuard};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream, SupportedOutputConfigs,
    SupportedStreamConfig,
};
use eyre::{Report, Result};

use crate::{
    operate_samples,
    sample_buffer::SampleBufferMut,
    source::{DeviceConfig, Source},
};

/// A player that can play `Source`
pub struct Sink {
    shared: Arc<SharedData>,
    #[allow(dead_code)]
    stream: Stream,
    info: DeviceConfig,
}

struct SharedData {
    controls: Mutex<Controls>,
    source: Mutex<Option<Box<dyn Source>>>,
    callback: Mutex<Option<Box<dyn FnMut(CallbackInfo) + Send>>>,
    err_callback: Mutex<Option<Box<dyn FnMut(ErrCallbackInfo) + Send>>>,
}

/// Callback type and asociated information
#[non_exhaustive]
pub enum CallbackInfo {
    /// Invoked when the current source has reached end
    SourceEnded,
}

#[non_exhaustive]
pub enum ErrSource {
    /// Errors from `cpal`
    Playback,
    /// Errors from the current source
    Source,
    /// Errors from the sink
    Sink,
}

/// Error source and the original error as `eyre::Report`
pub struct ErrCallbackInfo {
    /// The part which failed
    pub source: ErrSource,
    /// The original error as Report
    pub err: Report,
}

#[derive(Clone)]
struct Controls {
    play: bool,
}

struct Mixer {
    shared: Arc<SharedData>,
}

impl Sink {
    /// Creates the player from the default audio output device with the
    /// default configuration
    ///
    /// # Errors
    /// - no default device found
    /// - device became unavailable
    /// - device uses unsupported sample format
    pub fn default_out() -> Result<Sink> {
        // TODO: select device when the default device was not found
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(Report::msg("No available output device"))?;
        let config = device.default_output_config()?;
        let sample_format = config.sample_format();

        let info = DeviceConfig {
            channel_count: config.channels() as u32,
            sample_rate: config.sample_rate().0,
            sample_format: config.sample_format(),
        };

        let config = config.into();

        let shared = Arc::new(SharedData {
            controls: Mutex::new(Controls { play: false }),
            source: Mutex::new(None),
            callback: Mutex::new(None),
            err_callback: Mutex::new(None),
        });

        let mut mixer = Mixer {
            shared: shared.clone(),
        };

        let shared_clone = shared.clone();

        macro_rules! arm {
            ($t:ident, $e:ident) => {
                device.build_output_stream(
                    &config,
                    move |d: &mut [$t], _| {
                        mixer.mix(&mut SampleBufferMut::$e(d))
                    },
                    move |e| {
                        _ = shared_clone.invoke_err_callback(
                            ErrCallbackInfo::playback(Report::new(e)),
                        );
                    },
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
                // TODO: select other format when this is not supported
                return Err(Report::msg(
                    "Unsupported sample format '{sample_format}'",
                ));
            }
        }?;

        stream.play()?;

        let sink = Sink {
            shared,
            stream,
            info,
        };

        Ok(sink)
    }

    fn build_out_stream(
        &mut self,
        config: Option<DeviceConfig>,
    ) -> Result<()> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or(Report::msg("No available output device"))?;
        let config = match config {
            Some(c) => select_config(c, device.supported_output_configs()?)
                .unwrap_or(device.default_output_config()?),
            None => device.default_output_config()?,
        };

        self.info = DeviceConfig {
            channel_count: config.channels() as u32,
            sample_rate: config.sample_rate().0,
            sample_format: config.sample_format(),
        };

        let shared = self.shared.clone();
        let mut mixer = Mixer {
            shared: shared.clone(),
        };

        let config = config.into();
        macro_rules! arm {
            ($t:ident, $e:ident) => {
                device.build_output_stream(
                    &config,
                    move |d: &mut [$t], _| {
                        mixer.mix(&mut SampleBufferMut::$e(d))
                    },
                    move |e| {
                        _ = shared.invoke_err_callback(
                            ErrCallbackInfo::playback(Report::new(e)),
                        );
                    },
                    //Some(Duration::from_millis(5)),
                    None,
                )
            };
        }

        self.stream = match self.info.sample_format {
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
                // TODO: select other format when this is not supported
                return Err(Report::msg(
                    "Unsupported sample format '{sample_format}'",
                ));
            }
        }?;

        Ok(())
    }

    /// Sets the callback method.
    ///
    /// The function is called when the source ends.
    ///
    /// The function is called from another thread.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to init
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn on_callback(
        &self,
        callback: Option<impl FnMut(CallbackInfo) + Send + 'static>,
    ) -> Result<()> {
        (*self.shared.callback()?) = match callback {
            Some(c) => Some(Box::new(c)),
            None => None,
        };
        Ok(())
    }

    /// Sets the error callback method.
    ///
    /// The funciton is called when an error occures on another thread.
    ///
    /// The function is called from another thread.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to init
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn on_err_callback(
        &self,
        callback: Option<impl FnMut(ErrCallbackInfo) + Send + 'static>,
    ) -> Result<()> {
        (*self.shared.err_callback()?) = match callback {
            Some(c) => Some(Box::new(c)),
            None => None,
        };
        Ok(())
    }

    /// Discards the old source and sets the new source. Starts playing if
    /// `play` is set to true.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to init
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn load(
        &mut self,
        mut src: impl Source + 'static,
        play: bool,
    ) -> Result<()> {
        let config = src.preffered_config();
        if config.is_some() && *config.as_ref().unwrap() != self.info {
            _ = self.build_out_stream(config);
        }

        println!("{:?}", self.info);

        let mut controls = self.shared.controls()?;
        let mut source = self.shared.source()?;

        src.init(&self.info)?;

        controls.play = play;
        *source = Some(Box::new(src));

        Ok(())
    }

    /// Resumes the playback of the current source if `play` is true, otherwise
    /// pauses the playback.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn play(&self, play: bool) -> Result<()> {
        self.shared.controls()?.play = play;
        Ok(())
    }

    /// Pauses the playback of the current source
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn pause(&self) -> Result<()> {
        self.play(false)
    }

    /// Resumes the playback of the current source
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn resume(&self) -> Result<()> {
        self.play(true)
    }

    /// Returns true if the source is playing, otherwise returns false
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn is_playing(&self) -> Result<bool> {
        Ok(self.shared.controls()?.play)
    }
}

impl Mixer {
    fn mix(&mut self, data: &mut SampleBufferMut) {
        if let Err(e) = self.try_mix(data) {
            self.silence(data);
            _ = self.shared.invoke_err_callback(ErrCallbackInfo::sink(e));
        }
    }

    fn try_mix(&mut self, data: &mut SampleBufferMut) -> Result<()> {
        let controls = { self.shared.controls()?.clone() };

        if controls.play {
            self.play_source(data)?;
        } else {
            self.silence(data)
        }

        Ok(())
    }

    fn play_source(&mut self, data: &mut SampleBufferMut) -> Result<()> {
        let mut src = self.shared.source()?;

        match src.as_mut() {
            Some(s) => {
                let (cnt, e) = s.read(data);

                if let Err(e) = e {
                    _ = self
                        .shared
                        .invoke_err_callback(ErrCallbackInfo::source(e));
                }

                operate_samples!(data, d, {
                    Self::write_silence(&mut d[cnt..]);
                    if cnt < d.len() {
                        *src = None;
                        self.shared.invoke_callback(CallbackInfo::SourceEnded)
                    } else {
                        Ok(())
                    }
                })
            }
            None => {
                self.silence(data);
                Ok(())
            }
        }
    }

    fn silence(&self, data: &mut SampleBufferMut) {
        operate_samples!(data, d, Self::write_silence(d));
    }

    fn write_silence<T: cpal::Sample>(data: &mut [T]) {
        data.fill(T::EQUILIBRIUM);
    }
}

impl SharedData {
    fn controls(&self) -> Result<MutexGuard<'_, Controls>> {
        self.controls
            .lock()
            .or_else(|e| Err(Report::msg(e.to_string())))
    }

    fn source(&self) -> Result<MutexGuard<'_, Option<Box<dyn Source>>>> {
        self.source
            .lock()
            .or_else(|e| Err(Report::msg(e.to_string())))
    }

    fn callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(CallbackInfo) + Send>>>>
    {
        self.callback
            .lock()
            .or_else(|e| Err(Report::msg(e.to_string())))
    }

    fn err_callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(ErrCallbackInfo) + Send>>>>
    {
        self.err_callback
            .lock()
            .or_else(|e| Err(Report::msg(e.to_string())))
    }

    fn invoke_callback(&self, args: CallbackInfo) -> Result<()> {
        if let Some(cb) = self.callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }

    fn invoke_err_callback(&self, args: ErrCallbackInfo) -> Result<()> {
        if let Some(cb) = self.err_callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }
}

impl ErrCallbackInfo {
    pub fn playback(err: Report) -> Self {
        ErrCallbackInfo {
            source: ErrSource::Playback,
            err,
        }
    }

    pub fn source(err: Report) -> Self {
        ErrCallbackInfo {
            source: ErrSource::Source,
            err,
        }
    }

    pub fn sink(err: Report) -> Self {
        ErrCallbackInfo {
            source: ErrSource::Sink,
            err,
        }
    }
}

fn select_config(
    prefered: DeviceConfig,
    configs: SupportedOutputConfigs,
) -> Option<SupportedStreamConfig> {
    let mut selected = None;

    for c in configs {
        if c.min_sample_rate().0 <= prefered.sample_rate
            && c.max_sample_rate().0 >= prefered.sample_rate
        {
            if c.channels() as u32 == prefered.channel_count {
                if c.sample_format() == prefered.sample_format {
                    selected = Some(c);
                    break;
                } else if selected.is_none()
                    || selected.as_ref().unwrap().channels() as u32
                        != prefered.channel_count
                {
                    selected = Some(c)
                }
            } else if selected.is_none() {
                selected = Some(c)
            }
        }
    }

    selected.map(|s| s.with_sample_rate(SampleRate(prefered.sample_rate)))
}
