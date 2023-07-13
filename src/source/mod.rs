use cpal::SampleFormat;

use crate::sample_buffer::SampleBufferMut;

pub mod symph;
pub mod sine;

pub struct DeviceInfo {
    pub channel_count: u32,
    pub sample_rate: u32,
    pub sample_format: SampleFormat,
}

pub trait Source: Send {
    // delivers info to the source, read is not called before init
    // init may be called multiple times to update the info
    fn init(&mut self, info: &DeviceInfo);

    // reads data from the source into the buffer, returns number of written
    // items
    fn read(&mut self, buffer: &mut SampleBufferMut) -> usize;
}
