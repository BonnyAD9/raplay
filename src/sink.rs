use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use cpal::{
    Device, Devices, OutputCallbackInfo, SampleFormat, SampleRate, Stream,
    SupportedOutputConfigs, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::{
    BufferSize, Timestamp,
    err::{Error, Result},
    mixer::Mixer,
    sample_buffer::SampleBufferMut,
    shared::{CallbackInfo, SharedData},
    source::{DeviceConfig, Source},
};

/// A player that can play `Source`
pub struct Sink {
    /// Data shared with the playback loop ([`Mixer`])
    shared: Arc<SharedData>,
    // The stream is never read, it just stays alive so that the audio plays
    /// The stream, if you drop this the playbakc loop will stop
    stream: Option<Stream>,
    /// Info about the current device configuration
    info: DeviceConfig,
    /// Prefered device set by the user
    device: Option<Device>,
    /// Sink will try to get the buffer size to be this
    preferred_buffer_size: BufferSize,
}

impl Sink {
    /// Creates the output stream and starts the playback loop.
    /// `config` is preffered device configuration, [`None`] = choose
    /// default.
    fn build_out_stream(
        &mut self,
        config: Option<DeviceConfig>,
    ) -> Result<()> {
        let mut device =
            self.device.take().map(Ok).unwrap_or_else(|| -> Result<_> {
                cpal::default_host()
                    .default_output_device()
                    .ok_or(Error::NoOutDevice)
            })?;

        let sup = if let Ok(c) = device.supported_output_configs() {
            c
        } else {
            device = cpal::default_host()
                .default_output_device()
                .ok_or(Error::NoOutDevice)?;
            device.supported_output_configs()?
        };

        let supported_config = match config {
            Some(c) => select_config(c, sup)
                .unwrap_or(device.default_output_config()?),
            None => device.default_output_config()?,
        };

        self.info = DeviceConfig {
            channel_count: supported_config.channels() as u32,
            sample_rate: supported_config.sample_rate().0,
            sample_format: supported_config.sample_format(),
        };

        let shared = self.shared.clone();
        let mut mixer = Mixer::new(shared.clone(), self.info.clone());

        let mut config = supported_config.config();
        config.buffer_size = self
            .preferred_buffer_size
            .to_cpal(supported_config.buffer_size(), config.sample_rate.0);

        macro_rules! arm {
            ($t:ident, $e:ident) => {
                device.build_output_stream(
                    &config,
                    move |d: &mut [$t], info| {
                        mixer.mix(
                            &mut SampleBufferMut::$e(d),
                            get_play_time(info),
                        )
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

        self.device = Some(device);

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
        self.shared.callback().set(
            callback.map(|c| -> Box<dyn FnMut(CallbackInfo) + Send> {
                Box::new(c)
            }),
        )
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
        self.shared.err_callback().set(
            callback.map(|c| -> Box<dyn FnMut(Error) + Send> { Box::new(c) }),
        )
    }

    /// Discards the old source and sets the new source. Starts playing if
    /// `play` is set to true.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails topreferred_config
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn load(
        &mut self,
        mut src: impl Source + 'static,
        play: bool,
    ) -> Result<()> {
        src.set_err_callback(self.shared.err_callback());

        let config = src.preferred_config();
        if self.device.is_none()
            || config.as_ref().map(|c| *c != self.info).unwrap_or_default()
        {
            self.build_out_stream(config)?;
        }

        let mut controls = self.shared.controls()?;
        let mut source = self.shared.source()?;

        src.init(&self.info)?;

        controls.play = play;
        *source = Some(Box::new(src));

        if let Some(s) = &self.stream {
            if play {
                s.play()?;
            }
        }

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
        if let Some(s) = &self.stream {
            s.play()?;
        }
        Ok(())
    }

    /// Pauses the loop that is feeding new samples. This can be used to reduce
    /// cpu usage, but it is very different from the normal pause.
    ///
    /// It doesn't ignores fade play/pause.
    pub fn hard_pause(&self) -> Result<()> {
        if let Some(s) = &self.stream {
            s.pause()?;
        }
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

    /// Seeks to the given position
    ///
    /// # Errors
    /// - no source is playing
    /// - the source doesn't support this
    /// - failed to seek
    pub fn seek_to(&mut self, timestamp: Duration) -> Result<Timestamp> {
        Ok(self
            .shared
            .source()?
            .as_mut()
            .ok_or(Error::NoSourceIsPlaying)?
            .seek(timestamp)?)
    }

    /// Seeks by the given amount. If `forward` is true, seeks forward,
    /// otherwise seeks backward
    pub fn seek_by(
        &mut self,
        time: Duration,
        forward: bool,
    ) -> Result<Timestamp> {
        Ok(self
            .shared
            .source()?
            .as_mut()
            .ok_or(Error::NoSourceIsPlaying)?
            .seek_by(time, forward)?)
    }

    /// Gets the current timestamp and the total length of the currently
    /// playing source.
    ///
    /// # Errors
    /// - no source is playing
    /// - the source doesn't support this
    pub fn get_timestamp(&self) -> Result<Timestamp> {
        self.shared
            .source()?
            .as_ref()
            .ok_or(Error::NoSourceIsPlaying)?
            .get_time()
            .ok_or(Error::Unsupported {
                component: "Source",
                feature: "getting current timestamp",
            })
    }

    /// Sets the fade-in/fade-out time for play/pause
    pub fn set_fade_len(&mut self, fade: Duration) -> Result<()> {
        self.shared.controls()?.fade_duration = fade;
        Ok(())
    }

    /// Sets the preferred buffer size. None means, use default size.
    ///
    /// Set to small values (such as 1024 or even less) for low latency.
    /// Set to large values (such as 16384) for better performace efficiency.
    pub fn set_buffer_size(&mut self, size: BufferSize) {
        self.preferred_buffer_size = size;
    }

    /// Gets the preferred buffer size set by you
    pub fn get_preferred_buffer_size(&self) -> BufferSize {
        self.preferred_buffer_size
    }

    /// Gets info about the configuration of the output device that is
    /// currently playing
    pub fn get_info(&self) -> &DeviceConfig {
        &self.info
    }

    /// Gets iterator over all available devices
    pub fn list_devices() -> Result<Devices> {
        Ok(cpal::default_host().devices()?)
    }

    /// Sets the device to be used
    pub fn set_device(&mut self, device: Option<Device>) {
        self.device = device;
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
            device: None,
            preferred_buffer_size: BufferSize::Auto,
        }
    }
}

/// Selects config based on the prefered configuration
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

impl std::fmt::Debug for Sink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sink").field("info", &self.info).finish()
    }
}

fn get_play_time(info: &OutputCallbackInfo) -> Instant {
    let now = Instant::now();
    now + info
        .timestamp()
        .playback
        .duration_since(&info.timestamp().callback)
        .unwrap_or_default()
}
