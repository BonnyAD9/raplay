use std::time::Duration;

use anyhow::Result;
use cpal::SampleFormat;

use crate::{sample_buffer::SampleBufferMut, Error};

pub mod sine;
pub mod symph;

pub use sine::SineSource;
pub use symph::Symph;

// TODO: better selecting algorithm (if not sample rate at least channel count)
// TODO: fallback sample format when unsupported sample rate
// TODO: go back to reasonable settings when no prefered config
/// Information needed to properly play sound
#[derive(PartialEq, Debug, Clone)]
pub struct DeviceConfig {
    pub channel_count: u32,
    pub sample_rate: u32,
    pub sample_format: SampleFormat,
}

/// Source of audio samples
pub trait Source: Send {
    /// Delivers configuration to the source, read is not called before init
    ///
    /// Init may be called multiple times to update the info
    fn init(&mut self, info: &DeviceConfig) -> Result<()>;

    /// Reads data from the source into the buffer, returns number of written
    /// samples
    fn read(&mut self, buffer: &mut SampleBufferMut) -> (usize, Result<()>);

    /// Gets the preffered configuration.
    fn preffered_config(&mut self) -> Option<DeviceConfig> {
        None
    }

    /// Sets the volume iterator
    ///
    /// The volume iterator is used to modify the volume of the source
    ///
    /// # Returns
    /// false if the volume iterator is not supported by the source,
    /// otherwise true
    fn volume(&mut self, volume: VolumeIterator) -> bool {
        // just to ignore the warning but don't have to change the name
        _ = volume;
        false
    }

    /// Seeks to the given timestamp in the source.
    fn seek(&mut self, time: Duration) -> Result<()> {
        // just to ignore the warning but don't have to change the name
        _ = time;
        Err(Error::Unsupported {
            component: "Source",
            feature: "seeking",
        }
        .into())
    }

    /// Gets the current time and whole length
    ///
    /// # Returns
    /// (current timestamp, total duration)
    fn get_time(&self) -> Option<(Duration, Duration)> {
        None
    }
}

/// Iterates over volume of sequence of samples
/// A sample should be multiplied by the value returned by the iterator.
///
/// The multiplication must be done in the target sample rate.
///
/// Calling [`Iterator::next`] never returns [`None`], if you don't
/// want to get the [`Option`] you can use [`VolumeIterator::next_vol`].
#[derive(Clone, Copy, Debug)]
pub enum VolumeIterator {
    /// Constant volume
    Constant(f32),
    /// Changes the volume in linear time, than transitions to the constant
    Linear {
        /// The starting volume
        base: f32,
        /// How much volume change is single tick
        step: f32,
        /// Current tick
        cur_count: i32,
        /// The target tick, must be larger or equal to cur_count
        target_count: i32,
        /// Multiplier for the resulting volume, used when the volume changes
        /// during the transition
        multiplier: f32,
        /// The channel count of the result, each volume will be repeated
        /// this amount of times
        channel_count: usize,
        /// The current channel index
        cur_channel: usize,
    },
}

impl VolumeIterator {
    /// Creates constant volume
    pub fn constant(volume: f32) -> Self {
        Self::Constant(volume)
    }

    /// Creates volume iterator that changes lineary with time.
    ///
    /// The volume will start at the `start` volume and it will end at the
    /// `target` volume in `tick_count` samples
    pub fn linear(
        start: f32,
        target: f32,
        tick_count: i32,
        channels: usize,
    ) -> Self {
        Self::Linear {
            base: start,
            step: (target - start) / tick_count as f32,
            cur_count: 0,
            target_count: tick_count.abs(),
            multiplier: 1.,
            channel_count: channels,
            cur_channel: 0,
        }
    }

    /// Creates volume iterator that changes lineary with time.
    ///
    /// The volume will start at the `start` volume and it will end at the
    /// `target` volume in the given `duration` if the rate is the given `rate`
    pub fn linear_time_rate(
        start: f32,
        target: f32,
        rate: u32,
        duration: Duration,
        channels: usize,
    ) -> Self {
        if duration.is_zero() {
            Self::constant(target)
        } else {
            Self::linear(
                start,
                target,
                (rate as f32 * duration.as_secs_f32()) as i32,
                channels,
            )
        }
    }

    /// Transforms this volume iterator to a linear iterator starting at
    /// the current volume and ending at the `target` volume in `tick_count`
    /// samples
    pub fn to_linear(
        &mut self,
        target: f32,
        tick_count: i32,
        channels: usize,
    ) {
        match self {
            Self::Constant(c) => {
                *self = Self::linear(*c, target, tick_count, channels)
            }
            Self::Linear {
                base,
                step,
                cur_count,
                multiplier,
                ..
            } => {
                *self = Self::linear(
                    *base + *step * *cur_count as f32 * *multiplier,
                    target,
                    tick_count,
                    channels,
                );
            }
        }
    }

    /// Transforms this volume iterator to a linear iterator starting at
    /// the current volume and ending at the `target` volume in `tick_count`
    /// samples
    pub fn to_linear_time_rate(
        &mut self,
        target: f32,
        rate: u32,
        duration: Duration,
        channels: usize,
    ) {
        if duration.is_zero() {
            *self = Self::constant(target)
        } else {
            self.to_linear(
                target,
                (rate as f32 * duration.as_secs_f32()) as i32,
                channels,
            )
        }
    }

    /// Returns the number of ticks remaining to get to the target volume
    /// Returns none if the type is constant.
    pub fn until_target(&self) -> Option<usize> {
        match self {
            Self::Constant(_) => None,
            Self::Linear {
                cur_count,
                target_count,
                ..
            } => Some((target_count - cur_count).abs() as usize),
        }
    }

    /// Changes the volume of the iterator
    ///
    /// The `target` wheter in case of transition the source or target
    /// volume is changed. When `target` is true tha target volume is changed,
    /// when target is false the start volume is changed.
    pub fn set_volume(&mut self, volume: f32, target: bool) {
        match self {
            Self::Constant(_) => *self = Self::Constant(volume),
            Self::Linear {
                base,
                multiplier,
                target_count,
                step,
                ..
            } => {
                *multiplier = volume
                    / if target {
                        *base + *step * *target_count as f32
                    } else {
                        *base
                    };
            }
        }
    }

    /// behave as if the next_vol function was called n times
    pub fn skip_vol(&mut self, n: usize) {
        match self {
            Self::Constant(_) => {}
            Self::Linear {
                base,
                step,
                cur_count,
                target_count,
                multiplier,
                channel_count,
                cur_channel,
            } => {
                *cur_count += (n / *channel_count) as i32;
                *cur_channel += n % *channel_count;
                if cur_channel > channel_count {
                    *cur_count += 1;
                    *cur_channel -= *channel_count;
                }

                if cur_count >= target_count {
                    *self = Self::constant(
                        (*base + *step * *target_count as f32) * *multiplier,
                    );
                }
            }
        }
    }

    /// This is the same as next on the iterator
    ///
    /// # Returns
    /// Volume for the next sample
    pub fn next_vol(&mut self) -> f32 {
        match self {
            Self::Constant(vol) => *vol,
            Self::Linear {
                base,
                step,
                cur_count,
                target_count,
                multiplier,
                channel_count,
                cur_channel,
            } => {
                let ret = (*base + *step * *cur_count as f32) * *multiplier;
                *cur_channel += 1;
                if cur_channel == channel_count {
                    *cur_channel = 0;
                    *cur_count += 1;
                    if cur_count >= target_count {
                        *self = Self::Constant(ret)
                    }
                }
                ret
            }
        }
    }
}

impl Iterator for VolumeIterator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_vol())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

impl Default for VolumeIterator {
    fn default() -> Self {
        VolumeIterator::Constant(1.)
    }
}
