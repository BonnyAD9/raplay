use cpal::SampleFormat;

use crate::sample_buffer::SampleBufferMut;

use eyre::Result;

pub mod sine;
pub mod symph;

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
    fn preffered_config(&self) -> Option<DeviceConfig> {
        None
    }
}
