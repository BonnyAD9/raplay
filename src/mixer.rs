use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use cpal::Sample;

use crate::{
    CallbackInfo, Controls, PrefetchState, SharedData, Source,
    err::Result,
    operate_samples,
    sample_buffer::{SampleBufferMut, write_silence},
    silence_sbuf, slice_sbuf,
    source::{DeviceConfig, VolumeIterator},
};

/// Struct that handles the playback loop
#[derive(Debug)]
pub(super) struct Mixer {
    /// Data shared with [`Sink`]
    shared: Arc<SharedData>,
    /// Volume iterator presented to the source
    volume: VolumeIterator,
    /// The last status of play
    last_play: Option<bool>,
    last_sound: bool,
    /// Info about the device that is playing
    info: DeviceConfig,
}

impl Mixer {
    /// Creates new [`Mixer`]
    pub(super) fn new(shared: Arc<SharedData>, info: DeviceConfig) -> Self {
        Self {
            shared,
            volume: VolumeIterator::default(),
            last_play: None,
            last_sound: false,
            info,
        }
    }

    /// Writes the data from the source to the buffer `data`
    pub(super) fn mix<'a, 'b: 'a>(
        &mut self,
        data: &'a mut SampleBufferMut<'b>,
        play_time: Instant,
    ) {
        if let Err(e) = self.try_mix(data, play_time) {
            silence_sbuf!(data);
            _ = self.shared.invoke_err_callback(e);
        }
    }

    /// Tries to write the data from the source to the buffer `data`
    fn try_mix<'a, 'b: 'a>(
        &mut self,
        data: &'a mut SampleBufferMut<'b>,
        play_time: Instant,
    ) -> Result<()> {
        let controls = { self.shared.controls()?.clone() };

        let lp = self.last_play.unwrap_or(controls.play);
        self.last_play = Some(controls.play);

        self.volume.set_volume(controls.volume, lp);

        if controls.play {
            self.last_sound = true;

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

            self.play(data, controls)?;
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
                self.play(&mut slice_sbuf!(data, 0..len), controls)?;
                self.last_sound = true;
            }

            // than pause
            let data_len = data.len();
            silence_sbuf!(slice_sbuf!(data, len..data_len));

            if len == 0 && self.last_sound {
                if let Err(e) = self
                    .shared
                    .invoke_callback(CallbackInfo::PauseEnds(play_time))
                {
                    _ = self.shared.invoke_err_callback(e);
                };
                self.last_sound = false;
            }
        }

        Ok(())
    }

    /// Writes the data from the source to the buffer `data`. Also handles
    /// prefetching.
    fn play(
        &mut self,
        data: &mut SampleBufferMut,
        controls: Controls,
    ) -> Result<()> {
        let mut src = self.shared.source()?.take();

        let cnt = self.play_source(&mut src, data, &controls)?;

        let mut data = slice_sbuf!(data, cnt..);

        if data.is_empty() {
            return self.check_prefetch_callback(src, &controls, None);
        }

        {
            let mut psrc = self.shared.prefech_notify()?;

            let Some(src) = psrc.as_mut() else {
                silence_sbuf!(data);
                return if src.is_none() {
                    self.shared.invoke_callback(CallbackInfo::NoSource)
                } else {
                    self.shared.invoke_callback(CallbackInfo::SourceEnded(
                        PrefetchState::NoPrefetch,
                    ))
                };
            };

            let cfg = src.preferred_config();

            if cfg.is_some() && cfg.as_ref() != Some(&self.info) {
                return self.shared.invoke_callback(
                    CallbackInfo::SourceEnded(PrefetchState::PrefetchFailed),
                );
            }

            src.init(&self.info)?;
        }

        self.shared.do_prefetch_notify.store(true, Ordering::Relaxed);

        let mut src = self.shared.prefech_notify()?.take();

        let cnt = self.play_source(&mut src, &mut data, &controls)?;

        let data = slice_sbuf!(data, cnt..);

        if !data.is_empty() {
            silence_sbuf!(data);
            self.shared.invoke_callback(CallbackInfo::SourceEnded(
                PrefetchState::PrefetchSuccessful,
            ))?;
            self.shared.invoke_callback(CallbackInfo::SourceEnded(
                PrefetchState::NoPrefetch,
            ))
        } else {
            self.check_prefetch_callback(
                src,
                &controls,
                Some(CallbackInfo::SourceEnded(
                    PrefetchState::PrefetchSuccessful,
                )),
            )
        }
    }

    fn play_source(
        &mut self,
        src: &mut Option<Box<dyn Source>>,
        data: &mut SampleBufferMut,
        controls: &Controls,
    ) -> Result<usize> {
        match src.as_mut() {
            Some(s) => self.play_source_inner(s, data, controls),
            None => Ok(0),
        }
    }

    fn play_source_inner(
        &mut self,
        src: &mut Box<dyn Source>,
        data: &mut SampleBufferMut,
        controls: &Controls,
    ) -> Result<usize> {
        let supports_volume = src.volume(self.volume);

        let (cnt, e) = src.read(data);

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
                    #[allow(clippy::useless_conversion)]
                    for s in d.iter_mut() {
                        *s = (*s).mul_amp(self.volume.next_vol().into());
                    }
                } else if controls.volume == 0. {
                    write_silence(&mut d[..cnt]);
                }
            }

            Ok(cnt)
        })
    }

    /// Check if prefetch notification should be sent. Set current source to
    /// `src`.
    fn check_prefetch_callback(
        &mut self,
        src: Option<Box<dyn Source>>,
        controls: &Controls,
        qcb: Option<CallbackInfo>,
    ) -> Result<()> {
        let cb = (controls.prefetch != Duration::ZERO && self.shared.do_prefetch_notify.load(Ordering::Relaxed))
            .then(|| {
                src.as_ref()
                    .and_then(|t| t.get_time())
                    .map(|ts| ts.total - ts.current)
                    .and_then(|t| (t <= controls.prefetch).then_some(t))
            })
            .flatten();
        *(self.shared.source()?) = src;
        if let Some(cb) = qcb {
            self.shared.invoke_callback(cb)?;
        }
        if let Some(t) = cb {
            self.shared.do_prefetch_notify.store(false, Ordering::Relaxed);
            self.shared.invoke_callback(CallbackInfo::PrefetchTime(t))
        } else {
            Ok(())
        }
    }
}
