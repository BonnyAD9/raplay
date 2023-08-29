use std::f32::consts::PI;

use anyhow::Result;
use cpal::FromSample;

use crate::{operate_samples, sample_buffer::SampleBufferMut};

use super::{Source, VolumeIterator};

/// Source of sine waves
pub struct SineSource {
    frequency: f32,
    channels: u32,
    iter_step: f32,
    iter: f32,
    volume: VolumeIterator,
}

impl Source for SineSource {
    fn init(&mut self, info: &super::DeviceConfig) -> Result<()> {
        self.channels = info.channel_count;
        self.iter_step = 2. * PI * self.frequency / info.sample_rate as f32;
        Ok(())
    }

    fn read(&mut self, buffer: &mut SampleBufferMut) -> (usize, Result<()>) {
        operate_samples!(buffer, b, {
            self.generate(b);
            (b.len(), Ok(()))
        })
    }

    fn volume(&mut self, volume: super::VolumeIterator) -> bool {
        self.volume = volume;
        true
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
            volume: VolumeIterator::constant(1.),
        }
    }

    fn generate<T: FromSample<f32> + Clone>(&mut self, mut data: &mut [T]) {
        while data.len() >= self.channels as usize {
            let val =
                T::from_sample_(self.iter.sin() * self.volume.next_vol());
            data[..self.channels as usize].fill(val);
            data = &mut data[self.channels as usize..];
            self.iter += self.iter_step;
            if self.iter > 2. * PI {
                self.iter -= 2. * PI
            }
        }
    }
}
