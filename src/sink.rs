use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Sample, SampleFormat, SampleRate, Stream, SupportedOutputConfigs,
    SupportedStreamConfig,
};

use crate::{
    err::{Error, Result},
    operate_samples,
    sample_buffer::{write_silence, SampleBufferMut},
    silence_sbuf, slice_sbuf,
    source::{DeviceConfig, Source, VolumeIterator},
};

/// A player that can play `Source`
pub struct Sink {
    /// Data shared with the playback loop ([`Mixer`])
    shared: Arc<SharedData>,
    // The stream is never read, it just stays alive so that the audio plays
    /// The stream, if you drop this the playbakc loop will stop
    #[allow(dead_code)]
    stream: Option<Stream>,
    /// Info about the current device configuration
    info: DeviceConfig,
}

/// Data shared between sink and the playback loop
struct SharedData {
    /// Used to control the playback loop from the [`Sink`]
    controls: Mutex<Controls>,
    /// The source for the audio
    source: Mutex<Option<Box<dyn Source>>>,
    /// Function used as callback from the playback loop on events
    callback: Mutex<Option<Box<dyn FnMut(CallbackInfo) + Send>>>,
    /// Function used as callback when errors occur on the playback loop
    err_callback: Mutex<Option<Box<dyn FnMut(Error) + Send>>>,
}

/// Callback type and asociated information
#[non_exhaustive]
pub enum CallbackInfo {
    /// Invoked when the current source has reached end
    SourceEnded,
}

/// Used to control the playback loop from the sink
#[derive(Clone)]
struct Controls {
    /// Fade duration when play/pause
    fade_duration: Duration,
    /// When true, playback plays, when false playback is paused
    play: bool,
    /// Sets the volume of the playback
    volume: f32,
}

/// Struct that handles the playback loop
struct Mixer {
    /// Data shared with [`Sink`]
    shared: Arc<SharedData>,
    /// Volume iterator presented to the source
    volume: VolumeIterator,
    /// The last status of play
    last_play: Option<bool>,
    /// Info about the device that is playing
    info: DeviceConfig,
}

impl Sink {
    /// Creates the output stream and starts the playback loop.
    /// `config` is preffered device configuration, [`None`] = choose
    /// default.
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
        let mut mixer = Mixer::new(shared.clone(), self.info.clone());

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

    /// Seeks to the given position
    ///
    /// # Errors
    /// - no source is playing
    /// - the source doesn't support this
    /// - failed to seek
    pub fn seek_to(&mut self, timestamp: Duration) -> Result<()> {
        self.shared
            .source()?
            .as_mut()
            .ok_or(Error::NoSourceIsPlaying)?
            .seek(timestamp)?;
        Ok(())
    }

    /// Gets the current timestamp and the total length of the currently
    /// playing source.
    ///
    /// # Errors
    /// - no source is playing
    /// - the source doesn't support this
    pub fn get_timestamp(&self) -> Result<(Duration, Duration)> {
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
    /// Creates new [`Mixer`]
    fn new(shared: Arc<SharedData>, info: DeviceConfig) -> Self {
        Self {
            shared,
            volume: VolumeIterator::default(),
            last_play: None,
            info,
        }
    }

    /// Writes the data from the source to the buffer `data`
    fn mix<'a, 'b: 'a>(&mut self, data: &'a mut SampleBufferMut<'b>) {
        if let Err(e) = self.try_mix(data) {
            silence_sbuf!(data);
            _ = self.shared.invoke_err_callback(e);
        }
    }

    /// Tries to write the data from the source to the buffer `data`
    fn try_mix<'a, 'b: 'a>(
        &mut self,
        data: &'a mut SampleBufferMut<'b>,
    ) -> Result<()> {
        let controls = { self.shared.controls()?.clone() };

        let lp = self.last_play.unwrap_or(controls.play);
        self.last_play = Some(controls.play);

        self.volume.set_volume(controls.volume, lp);

        if controls.play {
            // Change the volume transition if the transition is to pause or
            // if it was previously paused
            if !lp {
                if self.volume.until_target().is_none() {
                    self.volume.set_volume(0., lp);
                }

                self.volume.to_linear_time_rate(
                    controls.volume,
                    self.info.sample_rate,
                    controls.fade_duration,
                    self.info.channel_count as usize,
                );
            }

            self.play_source(data, controls)?;
        } else {
            // Change the volume transition if the transition is to play or
            // if it was previously played
            if lp {
                self.volume.to_linear_time_rate(
                    0.,
                    self.info.sample_rate,
                    controls.fade_duration,
                    self.info.channel_count as usize,
                );
            }

            let len = (self.volume.until_target().unwrap_or(0)
                * self.info.channel_count as usize)
                .min(data.len());

            if len != 0 {
                // play the silencing
                self.play_source(&mut slice_sbuf!(data, 0..len), controls)?;
            }

            // than pause
            let data_len = data.len();
            silence_sbuf!(slice_sbuf!(data, len..data_len));
        }

        Ok(())
    }

    /// Writes the data from the source to the buffer `data`
    fn play_source(
        &mut self,
        data: &mut SampleBufferMut,
        controls: Controls,
    ) -> Result<()> {
        let mut src = self.shared.source()?;

        match src.as_mut() {
            Some(s) => {
                let supports_volume = s.volume(self.volume);

                let (cnt, e) = s.read(data);

                if let Err(e) = e {
                    _ = self.shared.invoke_err_callback(e.into());
                }

                if supports_volume {
                    self.volume.skip_vol(cnt);
                }

                operate_samples!(data, d, {
                    // manually change the volume of each sample if the
                    // source doesn't support volume
                    if !supports_volume {
                        if controls.volume != 1. {
                            for s in d.iter_mut() {
                                *s = (*s)
                                    .mul_amp(self.volume.next_vol().into());
                            }
                        } else if controls.volume == 0. {
                            write_silence(&mut d[..cnt]);
                        }
                    }

                    write_silence(&mut d[cnt..]);
                    if cnt < d.len() {
                        *src = None;
                        self.shared.invoke_callback(CallbackInfo::SourceEnded)
                    } else {
                        Ok(())
                    }
                })
            }
            None => {
                silence_sbuf!(data);
                Ok(())
            }
        }
    }
}

impl SharedData {
    /// Creates new shared data
    fn new() -> Self {
        Self {
            controls: Mutex::new(Controls::new()),
            source: Mutex::new(None),
            callback: Mutex::new(None),
            err_callback: Mutex::new(None),
        }
    }

    /// Aquires lock on controls
    fn controls(&self) -> Result<MutexGuard<'_, Controls>> {
        Ok(self.controls.lock()?)
    }

    /// Aquires lock on source
    fn source(&self) -> Result<MutexGuard<'_, Option<Box<dyn Source>>>> {
        Ok(self.source.lock()?)
    }

    /// Aquires lock on callback function
    fn callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(CallbackInfo) + Send>>>>
    {
        Ok(self.callback.lock()?)
    }

    /// Aquires lock on error callback function
    fn err_callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(Error) + Send>>>> {
        Ok(self.err_callback.lock()?)
    }

    /// Invokes callback function
    fn invoke_callback(&self, args: CallbackInfo) -> Result<()> {
        if let Some(cb) = self.callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }

    /// Invokes error callback function
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
    /// Creates new controls
    pub fn new() -> Self {
        Self {
            fade_duration: Duration::ZERO,
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
