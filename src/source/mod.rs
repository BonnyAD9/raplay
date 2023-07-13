use cpal::SampleFormat;

use crate::sample_buffer::SampleBufferMut;

pub mod symph;
pub mod sine;

/// Information needed to properly play sound
pub struct DeviceInfo {
    pub channel_count: u32,
    pub sample_rate: u32,
    pub sample_format: SampleFormat,
}

/// Source of audio samples
pub trait Source: Send {
    /// Delivers info to the source, read is not called before init
    ///
    /// Init may be called multiple times to update the info
    fn init(&mut self, info: &DeviceInfo);

    /// Reads data from the source into the buffer, returns number of written
    /// samples
    fn read(&mut self, buffer: &mut SampleBufferMut) -> usize;
}
