use std::f32::consts::PI;

use cpal::FromSample;

use crate::{operate_samples, sample_buffer::SampleBufferMut};

use super::Source;

/// Source of sine waves
pub struct SineSource {
    frequency: f32,
    channels: u32,
    iter_step: f32,
    iter: f32,
}

impl Source for SineSource {
    fn init(&mut self, info: &super::DeviceInfo) {
        self.channels = info.channel_count;
        self.iter_step = 2. * PI * self.frequency / info.sample_rate as f32;
    }

    fn read(&mut self, buffer: &mut SampleBufferMut) -> usize {
        operate_samples!(buffer, b, {
            self.generate(b);
            b.len()
        })
    }
}

impl SineSource {
    /// Creates source that generates infinite sine wave with the given
    /// frequency
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            channels: 0,
            iter_step: 0.,
            iter: 0.,
        }
    }

    fn generate<T: FromSample<f32> + Clone>(&mut self, mut data: &mut [T]) {
        while data.len() >= self.channels as usize {
            let val = T::from_sample_(self.iter.sin());
            data[..self.channels as usize].fill(val);
            data = &mut data[self.channels as usize..];
            self.iter += self.iter_step;
            if self.iter > 2. * PI {
                self.iter -= 2. * PI
            }
        }
    }
}
