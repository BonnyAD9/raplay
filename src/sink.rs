use std::sync::{Arc, Mutex, MutexGuard};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Sample, SampleFormat, SampleRate, Stream, SupportedOutputConfigs,
    SupportedStreamConfig,
};

use crate::{
    err::{Error, Result},
    operate_samples,
    sample_buffer::SampleBufferMut,
    source::{DeviceConfig, Source, VolumeIterator},
};

/// A player that can play `Source`
pub struct Sink {
    shared: Arc<SharedData>,
    // The stream is never read, it just stays alive so that the audio plays
    #[allow(dead_code)]
    stream: Option<Stream>,
    info: DeviceConfig,
}

struct SharedData {
    controls: Mutex<Controls>,
    source: Mutex<Option<Box<dyn Source>>>,
    callback: Mutex<Option<Box<dyn FnMut(CallbackInfo) + Send>>>,
    err_callback: Mutex<Option<Box<dyn FnMut(Error) + Send>>>,
}

/// Callback type and asociated information
#[non_exhaustive]
pub enum CallbackInfo {
    /// Invoked when the current source has reached end
    SourceEnded,
}

#[derive(Clone)]
struct Controls {
    play: bool,
    volume: f32,
}

struct Mixer {
    shared: Arc<SharedData>,
}

impl Sink {
    fn build_out_stream(
        &mut self,
        config: Option<DeviceConfig>,
    ) -> Result<()> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or(Error::NoOutDevice)?;
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
        let mut mixer = Mixer::new(shared.clone());

        let config = config.into();
        macro_rules! arm {
            ($t:ident, $e:ident) => {
                device.build_output_stream(
                    &config,
                    move |d: &mut [$t], _| {
                        mixer.mix(&mut SampleBufferMut::$e(d))
                    },
                    move |e| {
                        _ = shared.invoke_err_callback(e.into());
                    },
                    //Some(Duration::from_millis(5)),
                    None,
                )
            };
        }

        let stream = match self.info.sample_format {
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
                return Err(Error::UnsupportedSampleFormat);
            }
        }?;

        stream.play()?;

        self.stream = Some(stream);

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
        callback: Option<impl FnMut(Error) + Send + 'static>,
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

    /// Sets the volume of the playback, 0 = mute, 1 = full volume.
    ///
    /// The value is not clipped so the caller should make sure that the volume
    /// is in the bounds or the audio may have clipping.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn volume(&self, volume: f32) -> Result<()> {
        self.shared.controls()?.volume = volume;
        Ok(())
    }

    /// Gets the volume of the playback, 0 = mute, 1 = full volume.
    ///
    /// The value may not be in the range.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn get_volume(&self) -> Result<f32> {
        Ok(self.shared.controls()?.volume)
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

impl Default for Sink {
    fn default() -> Self {
        Self {
            shared: Arc::new(SharedData::new()),
            stream: None,
            info: DeviceConfig {
                channel_count: 0,
                sample_rate: 0,
                sample_format: SampleFormat::F32,
            },
        }
    }
}

impl Mixer {
    fn new(shared: Arc<SharedData>) -> Self {
        Self { shared }
    }

    fn mix(&mut self, data: &mut SampleBufferMut) {
        if let Err(e) = self.try_mix(data) {
            self.silence(data);
            _ = self.shared.invoke_err_callback(e);
        }
    }

    fn try_mix(&mut self, data: &mut SampleBufferMut) -> Result<()> {
        let controls = { self.shared.controls()?.clone() };

        if controls.play {
            self.play_source(data, controls)?;
        } else {
            self.silence(data)
        }

        Ok(())
    }

    fn play_source(
        &mut self,
        data: &mut SampleBufferMut,
        controls: Controls,
    ) -> Result<()> {
        let mut src = self.shared.source()?;

        match src.as_mut() {
            Some(s) => {
                let supports_volume =
                    s.volume(VolumeIterator::constant(controls.volume));

                let (cnt, e) = s.read(data);

                if let Err(e) = e {
                    _ = self.shared.invoke_err_callback(e.into());
                }

                operate_samples!(data, d, {
                    // manually change the volume of each sample if the
                    // source doesn't support volume
                    if !supports_volume {
                        if controls.volume != 1. {
                            for s in d.iter_mut() {
                                *s = (*s).mul_amp(controls.volume.into());
                            }
                        } else if controls.volume == 0. {
                            Self::write_silence(&mut d[..cnt]);
                        }
                    }

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
    fn new() -> Self {
        Self {
            controls: Mutex::new(Controls::new()),
            source: Mutex::new(None),
            callback: Mutex::new(None),
            err_callback: Mutex::new(None),
        }
    }

    fn controls(&self) -> Result<MutexGuard<'_, Controls>> {
        Ok(self.controls.lock()?)
    }

    fn source(&self) -> Result<MutexGuard<'_, Option<Box<dyn Source>>>> {
        Ok(self.source.lock()?)
    }

    fn callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(CallbackInfo) + Send>>>>
    {
        Ok(self.callback.lock()?)
    }

    fn err_callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(Error) + Send>>>> {
        Ok(self.err_callback.lock()?)
    }

    fn invoke_callback(&self, args: CallbackInfo) -> Result<()> {
        if let Some(cb) = self.callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }

    fn invoke_err_callback(&self, args: Error) -> Result<()> {
        if let Some(cb) = self.err_callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }
}

impl Default for SharedData {
    fn default() -> Self {
        Self::new()
    }
}

impl Controls {
    pub fn new() -> Self {
        Self {
            play: false,
            volume: 1.,
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self::new()
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
