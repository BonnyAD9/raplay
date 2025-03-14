use std::f32::consts::PI;

use anyhow::Result;
use cpal::FromSample;

use crate::{operate_samples, sample_buffer::SampleBufferMut};

use super::{Source, VolumeIterator};

/// Source of sine waves
#[derive(Debug)]
pub struct Sine {
    /// Frequency of the sine wave
    frequency: f32,
    /// Number of channels of the result
    channels: u32,
    /// How much to step on the x axis for each sample
    iter_step: f32,
    /// The x axis of the sine function
    iter: f32,
    /// Creates multiplier for each sample
    volume: VolumeIterator,
}

impl Source for Sine {
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

impl Sine {
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

    /// Generates sine wave
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
