use std::ops::Index;

use cpal::Sample;
use dasp_sample::ToSample;

use crate::{converters::lin_resample::LinResample, source::VolumeIterator};

/// Converter for converting channel counts, sample rate, sample type and for
/// interleaving.
#[derive(Debug, Copy, Clone)]
pub struct Convert {
    src_channels: usize,
    dst_channels: usize,
    src_rate: u32,
    dst_rate: u32,
    resample: Option<LinResample>,
    volume: VolumeIterator,
}

impl Default for Convert {
    fn default() -> Self {
        Self::new(1, 1, 44100, 44100, VolumeIterator::Constant(1.))
    }
}

impl Convert {
    /// Create new converter.
    ///
    /// If you don't know the parameters all at once, use [`Convert::default`]
    /// and set them later.
    pub fn new(
        src_channels: usize,
        dst_channels: usize,
        src_rate: u32,
        dst_rate: u32,
        volume: VolumeIterator,
    ) -> Self {
        Self {
            src_channels,
            dst_channels,
            src_rate,
            dst_rate,
            volume,
            resample: (src_rate != dst_rate)
                .then(|| LinResample::new(src_rate as f32, dst_rate as f32)),
        }
    }

    /// Change the source channels.
    pub fn set_src_channels(&mut self, cnt: usize) {
        self.src_channels = cnt;
    }

    /// Change the destination channels.
    pub fn set_dst_channels(&mut self, cnt: usize) {
        self.dst_channels = cnt;
    }

    /// Change the source sample rate.
    pub fn set_src_rate(&mut self, rate: u32) {
        if self.src_rate != rate {
            self.src_rate = rate;
            self.update_rate();
        }
    }

    /// Change the destination sample rate.
    pub fn set_dst_rate(&mut self, rate: u32) {
        if self.dst_rate != rate {
            self.dst_rate = rate;
            self.update_rate();
        }
    }

    /// Change the sample rates.
    pub fn set_rates(&mut self, src_rate: u32, dst_rate: u32) {
        if self.src_rate != src_rate || self.dst_rate != dst_rate {
            self.src_rate = src_rate;
            self.dst_rate = dst_rate;
            self.update_rate();
        }
    }

    /// Set the volume.
    pub fn set_volume(&mut self, volume: VolumeIterator) {
        self.volume = volume;
    }

    fn update_rate(&mut self) {
        self.resample = (self.src_rate != self.dst_rate).then(|| {
            LinResample::new(self.src_rate as f32, self.dst_rate as f32)
        });
    }

    /// Interleave samples from `src` at offset given by `src_offset` and do
    /// the proper conversion while writing to `dst`.
    ///
    /// # Returns
    /// Number of read samples (per channel) and number of written samples
    /// (all channels together).
    pub fn from_planes<
        S: Sample + ToSample<D> + ToSample<f32> + 'static,
        D: Sample + 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
    ) -> (usize, usize)
    where
        S::Float: ToSample<D>,
        f32: ToSample<S::Float> + ToSample<D>,
    {
        if self.resample.is_some() {
            self.from_planes_resample(src, src_offset, dst)
        } else {
            let res = self.from_planes_no_resample(src, src_offset, dst);
            (res, res * self.dst_channels)
        }
    }

    pub fn from_planes_no_resample<
        S: Sample + ToSample<D> + 'static,
        D: Sample + 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
    ) -> usize
    where
        S::Float: ToSample<D>,
        f32: ToSample<S::Float>,
    {
        let len = src[0][src_offset..]
            .len()
            .min(dst.len() / self.dst_channels);
        match &self.volume {
            VolumeIterator::Constant(1.) => {
                self.from_planes_direct(
                    src,
                    src_offset,
                    dst,
                    len + src_offset,
                );
            }
            VolumeIterator::Constant(c) => {
                self.from_planes_const_volume(
                    src,
                    src_offset,
                    dst,
                    len + src_offset,
                    (*c).to_sample(),
                );
            }
            _ => {
                self.from_planes_volume(
                    src,
                    src_offset,
                    dst,
                    len + src_offset,
                );
            }
        }

        len
    }

    pub fn from_planes_resample<
        S: Sample + ToSample<f32> + 'static,
        D: 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
    ) -> (usize, usize)
    where
        f32: ToSample<D>,
    {
        match &self.volume {
            VolumeIterator::Constant(1.) => {
                self.from_planes_direct_resample(src, src_offset, dst)
            }
            VolumeIterator::Constant(c) => self
                .from_planes_const_volume_resample(src, src_offset, dst, *c),
            _ => self.from_planes_volume_resample(src, src_offset, dst),
        }
    }

    pub fn from_planes_direct<
        S: Sample + ToSample<D> + 'static,
        D: Sample + 'static,
    >(
        &self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
        len: usize,
    ) {
        let mut sc = 0;
        for dc in 0..self.dst_channels {
            let s = &src[sc][src_offset..len];
            let mut di = dc;
            for s in s {
                dst[di] = (*s).to_sample();
                di += self.dst_channels;
            }
            sc += 1;
            if sc >= self.src_channels {
                sc = 0;
            }
        }
    }

    pub fn from_planes_direct_resample<
        S: Sample + ToSample<f32> + 'static,
        D: 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
    ) -> (usize, usize)
    where
        f32: ToSample<D>,
    {
        let sam = self.resample.as_mut().unwrap();
        let mut si = src[0][src_offset..].iter();
        let wrt =
            sam.fill_every(self.dst_channels, dst, &mut (&mut si).copied());
        let rd = src[0][src_offset..].len() - si.as_slice().len();
        let mut sc = 0;
        for dc in 1..self.dst_channels {
            sc += 1;
            if sc >= self.src_channels {
                sc = 0;
            }
            sam.fill_every(
                self.dst_channels,
                &mut dst[dc..],
                &mut src[sc][src_offset..].iter().copied(),
            );
        }
        (rd, wrt * self.dst_channels)
    }

    pub fn from_planes_const_volume<
        S: Sample + ToSample<D> + 'static,
        D: Sample + 'static,
    >(
        &self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
        len: usize,
        vol: S::Float,
    ) where
        S::Float: ToSample<D>,
    {
        let mut sc = 0;
        for dc in 0..self.dst_channels {
            let s = &src[sc][src_offset..len];
            let mut di = dc;
            for s in s {
                dst[di] = ((*s).to_float_sample() * vol).to_sample();
                di += self.dst_channels;
            }
            sc += 1;
            if sc >= self.src_channels {
                sc = 0;
            }
        }
    }

    pub fn from_planes_const_volume_resample<
        S: Sample + ToSample<f32> + 'static,
        D: 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
        vol: f32,
    ) -> (usize, usize)
    where
        f32: ToSample<D>,
    {
        let sam = self.resample.as_mut().unwrap();
        let mut si = src[0][src_offset..].iter();
        let wrt = sam.fill_every(
            self.dst_channels,
            dst,
            &mut (&mut si).map(|s| (*s).to_sample() * vol),
        );
        let rd = src[0][src_offset..].len() - si.as_slice().len();
        let mut sc = 0;
        for dc in 1..self.dst_channels {
            sc += 1;
            if sc >= self.src_channels {
                sc = 0;
            }
            sam.fill_every(
                self.dst_channels,
                &mut dst[dc..],
                &mut src[sc][src_offset..]
                    .iter()
                    .map(|s| (*s).to_sample() * vol),
            );
        }
        (rd, wrt * self.dst_channels)
    }

    pub fn from_planes_volume<
        S: Sample + ToSample<D> + 'static,
        D: Sample + 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
        len: usize,
    ) where
        S::Float: ToSample<D>,
        f32: ToSample<S::Float>,
    {
        let mut sc = 0;
        for dc in 0..self.dst_channels {
            let s = &src[sc][src_offset..len];
            let mut di = dc;
            for s in s {
                dst[di] = ((*s).to_float_sample()
                    * self.volume.next_vol().to_sample())
                .to_sample();
                di += self.dst_channels;
            }
            sc += 1;
            if sc >= self.src_channels {
                sc = 0;
            }
        }
    }

    pub fn from_planes_volume_resample<
        S: Sample + ToSample<f32> + 'static,
        D: 'static,
    >(
        &mut self,
        src: &impl Index<usize, Output = [S]>,
        src_offset: usize,
        dst: &mut [D],
    ) -> (usize, usize)
    where
        f32: ToSample<D>,
    {
        let sam = self.resample.as_mut().unwrap();
        let mut si = src[0][src_offset..].iter();
        let wrt = sam.fill_every(
            self.dst_channels,
            dst,
            &mut (&mut si)
                .zip(&mut self.volume)
                .map(|(&s, v)| s.to_sample() * v),
        );
        let rd = src[0][src_offset..].len() - si.as_slice().len();
        let mut sc = 0;
        for dc in 1..self.dst_channels {
            sc += 1;
            if sc >= self.src_channels {
                sc = 0;
            }
            sam.fill_every(
                self.dst_channels,
                &mut dst[dc..],
                &mut src[sc][src_offset..]
                    .iter()
                    .zip(&mut self.volume)
                    .map(|(&s, v)| s.to_sample() * v),
            );
        }
        (rd, wrt * self.dst_channels)
    }
}
