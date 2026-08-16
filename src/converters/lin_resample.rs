use cpal::Sample;
use dasp_sample::ToSample;

/// Linear resampler. Samples are lineary interpolated to get the samples in
/// between them. This is very cheap, but it is not the correct way of doing
/// the interpolation and thus it may result in artifacts.
///
/// This doesn't do any low pass filtering, so you may expect aliasing when
/// downsampling.
#[derive(Debug, Copy, Clone)]
pub struct LinResample {
    rate: f32,
    pos: f32,
    a: f32,
    b: f32,
}

impl LinResample {
    /// Create new linear resampler.
    pub fn new(source_rate: f32, target_rate: f32) -> Self {
        Self {
            rate: source_rate / target_rate,
            pos: 2.,
            a: 0.,
            b: 0.,
        }
    }

    /// Get the next sample.
    pub fn next_sample<S: Sample + ToSample<f32> + 'static>(
        &mut self,
        src: &mut impl Iterator<Item = S>,
    ) -> Option<f32> {
        while self.pos >= 1. {
            self.a = self.b;
            self.b = src.next()?.to_sample();
            self.pos -= 1.;
        }

        let res = self.a * (1. - self.pos) + self.b * self.pos;
        self.pos += self.rate;
        Some(res)
    }

    /// Fill every nth value of `dst` with result of resampling.
    ///
    /// This basically does interleaving of the one channel.
    ///
    /// Returns the number of written samples.
    pub fn fill_every<S: Sample + ToSample<f32> + 'static, D: 'static>(
        &mut self,
        n: usize,
        dst: &mut [D],
        src: &mut impl Iterator<Item = S>,
    ) -> usize
    where
        f32: ToSample<D>,
    {
        let mut i = 0;
        let mut dst = dst.iter_mut().step_by(n);
        while let Some(d) = dst.next()
            && let Some(s) = self.next_sample(src)
        {
            *d = s.to_sample();
            i += 1;
        }
        i
    }
}
