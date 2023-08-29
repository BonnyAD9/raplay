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
#[derive(PartialEq, Debug)]
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
/// A sample should be multiplied by the value returned by the iterator
///
/// Calling [`Iterator::next`] never returns [`None`], if you don't
/// want to get the [`Option`] you can use [`VolumeIterator::next_vol`]
#[derive(Clone, Copy, Debug)]
pub struct VolumeIterator {
    volume: f32,
}

impl VolumeIterator {
    /// Creates volume iterator with constant volume
    pub fn constant(volume: f32) -> Self {
        Self { volume }
    }

    /// This is the same as next on the iterator
    ///
    /// # Returns
    /// Volume for the next sample
    pub fn next_vol(&self) -> f32 {
        self.volume
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
