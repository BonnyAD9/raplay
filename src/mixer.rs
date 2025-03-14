use std::{sync::Arc, time::Instant};

use cpal::Sample;

use crate::{
    err::Result,
    operate_samples,
    sample_buffer::{SampleBufferMut, write_silence},
    shared::{CallbackInfo, Controls, SharedData},
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
                            #[allow(clippy::useless_conversion)]
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
