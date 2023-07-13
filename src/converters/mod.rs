///! Useful conversions on samples

mod channels;
mod interleave;
mod rate;

/// Craetes iterator that interleaves the channels of `i`
pub fn interleave<T>(
    i: impl Iterator<Item = impl Iterator<Item = T>>,
) -> impl Iterator<Item = T> {
    interleave::Interleave::new(i)
}

/// Creates iterator that converts the interleaved audio channel count of
/// `source` from `source_channels` to `target_channels`
pub fn channels(
    source: impl Iterator<Item = f32>,
    source_channels: u32,
    target_channels: u32,
) -> impl Iterator<Item = f32> {
    channels::ChannelConverter::new(source, source_channels, target_channels)
}

/// Creates iterator that converts the sample rate of `source` from
/// `source_rate` to `target_rate` by lineary interpolating the values
pub fn rate(
    source: impl Iterator<Item = f32>,
    source_rate: u32,
    target_rate: u32,
) -> impl Iterator<Item = f32> {
    rate::RateConverter::new(source, source_rate, target_rate)
}

/// Creates iterator that interleaves the channels of `source`, than
/// converts the interleaved audio channel count of from `source_channels` to
/// `target_channels` and than converts the sample rate of from `source_rate`
/// to `target_rate` by lineary interpolating the values.
///
/// This is equivalent to chaining the functions `rate(channels(interleave()))`
pub fn do_interleave_channels_rate(
    source: impl Iterator<Item = impl Iterator<Item = f32>>,
    source_channels: u32,
    target_channels: u32,
    source_rate: u32,
    target_rate: u32,
) -> impl Iterator<Item = f32> {
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
pub fn do_channels_rate(
    source: impl Iterator<Item = f32>,
    source_channels: u32,
    target_channels: u32,
    source_rate: u32,
    target_rate: u32,
) -> impl Iterator<Item = f32> {
    rate(
        channels(source, source_channels, target_channels),
        source_rate,
        target_rate,
    )
}
