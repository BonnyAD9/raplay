use cpal::{FromSample, Sample, I24, U24};
use num::{Float, NumCast, ToPrimitive};

use self::{
    channels::ChannelConverter, interleave::Interleave, rate::RateConverter,
};

/// Contains iterator that converts between channel counts
pub mod channels;
/// Contais iterator that interleaves channels
pub mod interleave;
/// Contains iterator that converts rate
pub mod rate;

/// Craetes iterator that interleaves the channels of `i`
pub fn interleave<S, I: Iterator<Item = S>, II: Iterator<Item = I>>(
    i: II,
) -> Interleave<I, S> {
    Interleave::new(i)
}

/// Creates iterator that converts the interleaved audio channel count of
/// `source` from `source_channels` to `target_channels`
pub fn channels<S: Sample, I: Iterator<Item = S>>(
    source: I,
    source_channels: u32,
    target_channels: u32,
) -> ChannelConverter<S, I> {
    ChannelConverter::new(source, source_channels, target_channels)
}

/// Creates iterator that converts the sample rate of `source` from
/// `source_rate` to `target_rate` by lineary interpolating the values
pub fn rate<S, I, R>(
    source: I,
    source_rate: R,
    target_rate: R,
) -> RateConverter<S, I>
where
    S: Sample + std::ops::Add<Output = S>,
    I: Iterator<Item = S>,
    S::Float: Float + NumCast,
    R: ToPrimitive,
{
    RateConverter::new(source, source_rate, target_rate)
}

/// Creates iterator that interleaves the channels of `source`, than
/// converts the interleaved audio channel count of from `source_channels` to
/// `target_channels` and than converts the sample rate of from `source_rate`
/// to `target_rate` by lineary interpolating the values.
///
/// This is equivalent to chaining the functions `rate(channels(interleave()))`
pub fn do_interleave_channels_rate<S, I, R, II>(
    source: II,
    source_channels: u32,
    target_channels: u32,
    source_rate: R,
    target_rate: R,
) -> RateConverter<S, ChannelConverter<S, Interleave<I, S>>>
where
    S: Sample + std::ops::Add<Output = S>,
    I: Iterator<Item = S>,
    S::Float: Float + NumCast,
    R: ToPrimitive,
    II: Iterator<Item = I>,
{
    rate(
        channels(interleave(source), source_channels, target_channels),
        source_rate,
        target_rate,
    )
}

/// Creates iterator that converts the interleaved audio channel count of
/// `source` from `source_channels` to `target_channels`, and than converts
/// the sample rate from `source_rate` to `target_rate` by lineary
/// interpolating the values
///
/// This is equivalent to chaining functions `rate(channels())`
pub fn do_channels_rate<S, I, R>(
    source: I,
    source_channels: u32,
    target_channels: u32,
    source_rate: R,
    target_rate: R,
) -> RateConverter<S, ChannelConverter<S, impl Iterator<Item = S>>>
where
    S: Sample + std::ops::Add<Output = S>,
    I: Iterator<Item = S>,
    S::Float: Float + NumCast,
    R: ToPrimitive,
{
    rate(
        channels(source, source_channels, target_channels),
        source_rate,
        target_rate,
    )
}

#[inline]
pub fn convert_sample<S1, S2: FromSample<S1>>(sample: S1) -> S2 {
    S2::from_sample_(sample)
}

pub trait UniSample:
    Sample
    + FromSample<i8>
    + FromSample<i16>
    + FromSample<I24>
    + FromSample<i32>
    + FromSample<i64>
    + FromSample<u8>
    + FromSample<u16>
    + FromSample<U24>
    + FromSample<u32>
    + FromSample<u64>
    + FromSample<f32>
    + FromSample<f64>
{
}

impl<T> UniSample for T where
    T: Sample
        + FromSample<i8>
        + FromSample<i16>
        + FromSample<I24>
        + FromSample<i32>
        + FromSample<i64>
        + FromSample<u8>
        + FromSample<u16>
        + FromSample<U24>
        + FromSample<u32>
        + FromSample<u64>
        + FromSample<f32>
        + FromSample<f64>
{
}
