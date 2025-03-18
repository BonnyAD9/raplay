use std::{
    mem,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use cpal::{
    Device, Devices, OutputCallbackInfo, SampleFormat, SampleRate, Stream,
    SupportedOutputConfigs, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::{
    BufferSize, CallbackInfo, OptionBox, SharedData, Timestamp,
    err::{Error, Result},
    mixer::Mixer,
    sample_buffer::SampleBufferMut,
    source::{DeviceConfig, Source},
};

/// A player that can play `Source`
pub struct Sink {
    /// Data shared with the playback loop ([`Mixer`])
    shared: Arc<SharedData>,
    // The stream is never read, it just stays alive so that the audio plays
    /// The stream, if you drop this the playbakck loop will stop
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

    /// Sets the callback function. Returns previous callback function.
    ///
    /// The function is called when playback event occurs. For example when
    /// source ends.
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
        callback: Box<dyn FnMut(CallbackInfo) + Send>,
    ) -> Result<OptionBox<dyn FnMut(CallbackInfo) + Send>> {
        self.shared.callback().set(callback)
    }

    /// Sets the error callback method. Returns previous error callback
    /// function.
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
        callback: Box<dyn FnMut(Error) + Send>,
    ) -> Result<OptionBox<dyn FnMut(Error) + Send>> {
        self.shared.err_callback().set(callback)
    }

    /// Discards the old source and sets the new source. Starts playing if
    /// `play` is set to true.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to select preferred configuration.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn load(
        &mut self,
        src: Box<dyn Source>,
        play: bool,
    ) -> Result<()> {
        self.try_load(&mut Some(src), play)
    }

    /// Tries to load the given source. If loading of the source fails, it is
    /// not taken. If it it succeeds, it will be removed from the option.
    ///
    /// `src` MUST NOT BE [`None`].
    ///
    /// There is option where this will return error, but the source will be
    /// taken. In that case the source is not dropped, but already loaded
    /// internaly and the operation can be retried by calling [`Self::play`].
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to select preferred configuration.
    ///
    /// # Panics
    /// - `src` was [`None`].
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn try_load(&mut self, src: &mut Option<Box<dyn Source>>, play: bool) -> Result<()> {
        let srcr = src.as_mut().expect("Sink::try_load() called with None");

        srcr.set_err_callback(self.shared.err_callback());

        let config = srcr.preferred_config();
        let new_stream = if self.device.is_none()
            || config.as_ref().map(|c| *c != self.info).unwrap_or_default()
        {
            self.build_out_stream(config)?;
            true
        } else {
            false
        };

        let mut controls = self.shared.controls()?;
        let mut source = self.shared.source()?;

        srcr.init(&self.info)?;

        controls.play = play;
        *source = src.take();

        if !new_stream {
            self.do_prefetch_notify(true);
        }

        if let Some(s) = &self.stream {
            if play {
                s.play()?;
            }
        }

        Ok(())
    }

    /// Loads the prefetched source.
    ///
    /// # Errors
    /// - There is no prefetched source.
    /// - Another user of one of the used mutexes panicked while using it
    /// - Source fails to select preferred configuration.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn load_prefetched(&mut self, play: bool) -> Result<()> {
        let src = self.shared.prefech_notify()?.take();
        if let Some(src) = src {
            let mut src = Some(src);
            let res = self.try_load(&mut src, play);
            if src.is_some() {
                *self.shared.prefech_notify()? = src;
            }
            res
        } else {
            Err(Error::NoPrefetchedSource)
        }
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

    /// Sets the fade-in/fade-out time for play/pause. Returns the previous
    /// fade length.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to select preferred configuration.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn set_fade_len(&mut self, fade: Duration) -> Result<Duration> {
        Ok(mem::replace(
            &mut self.shared.controls()?.fade_duration,
            fade,
        ))
    }

    /// Gets the current fade-in/fade-out time for play/pause.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to select preferred configuration.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn get_fade_len(&self) -> Result<Duration> {
        Ok(self.shared.controls()?.fade_duration)
    }

    /// Sets the preferred buffer size. None means, use default size.
    ///
    /// Set to small values (such as 1024 or even less) for low latency.
    /// Set to large values (such as 16384) for better performace efficiency.
    pub fn set_buffer_size(&mut self, size: BufferSize) {
        self.preferred_buffer_size = size;
    }

    /// Gets the preferred buffer size set by you.
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

    /// Sets the device to be used. If `device` is [`None`], default device
    /// will be selected. Returns the current device.
    ///
    /// This change will be applied the next time that stream will need to
    /// rebuild or by calling [`Self::restart_stream`].
    pub fn set_device(&mut self, device: Option<Device>) -> Option<Device> {
        mem::replace(&mut self.device, device)
    }

    /// Gets the currently selected playback device.
    pub fn get_device(&self) -> &Option<Device> {
        &self.device
    }

    /// Resets the device and restarts the stream. If device is [`None`],
    /// default device will be selected.
    ///
    /// You may want to call this if [`Self::load`] returns with
    /// `Error::Cpal(CpalError::BuildStream(BuildStreamError::DeviceNotAvailable))`.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to select preferred configuration.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn restart_device(&mut self, device: Option<Device>) -> Result<()> {
        self.set_device(device);
        self.restart_stream()
    }

    /// Rebuilds the stream. Playback is resumed right after restarting the
    /// stream if it was playing.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it
    /// - source fails to select preferred configuration.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn restart_stream(&mut self) -> Result<()> {
        self.stream = None;

        let src = self.shared.source()?.take();

        if let Some(src) = src {
            let play = self.is_playing()?;
            self.load(src, play)?;
        }
        Ok(())
    }

    /// Removes the callback function and returns it.
    ///
    /// # Panics
    /// - If locking mutex returns error.
    pub fn take_callback(
        &self,
    ) -> Option<Box<dyn FnMut(CallbackInfo) + Send>> {
        self.shared.callback().take()
    }

    /// Removes the error callback function and returns it.
    ///
    /// # Panics
    /// - If locking mutex returns error.
    pub fn take_err_callback(&self) -> Option<Box<dyn FnMut(Error) + Send>> {
        self.shared.err_callback().take()
    }

    /// Prefetch the next song. Return the previous value of prefetch if any.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn prefetch(
        &self,
        mut src: Option<Box<dyn Source>>,
    ) -> Result<Option<Box<dyn Source>>> {
        if let Some(src) = &mut src {
            src.set_err_callback(self.shared.err_callback());
        }
        Ok(mem::replace(&mut *self.shared.prefech_notify()?, src))
    }

    /// Sets how long before source ends should notification about the source
    /// ending be sent. Setting this to [`Duration::ZERO`] will disable this
    /// feature.
    ///
    /// If the remaining length of source is less than `rem`, notification
    /// will be sent using the callback function with
    /// [`CallbackInfo::PrefetchTime`] with the remaining time.
    ///
    /// # Errors
    /// - another user of one of the used mutexes panicked while using it.
    ///
    /// # Panics
    /// - the current thread already locked one of the used mutexes and didn't
    ///   release them
    pub fn prefetch_notify(&self, rem: Duration) -> Result<()> {
        self.shared.controls()?.prefetch = rem;
        Ok(())
    }

    /// true - Makes the source notify of prefetch even if that notification
    ///        has already been sent.
    ///
    /// false - Don't sent notify for the current source.
    pub fn do_prefetch_notify(&self, val: bool) {
        self.shared.do_prefetch_notify.store(val, Ordering::Relaxed);
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
        f.debug_struct("Sink")
            .field("shared", &self.shared)
            .field("info", &self.info)
            .finish()
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
