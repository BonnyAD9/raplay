mod channels;
mod interleave;
mod rate;

pub fn interleave<T>(
    i: impl Iterator<Item = impl Iterator<Item = T>>,
) -> impl Iterator<Item = T> {
    interleave::Interleave::new(i)
}

pub fn channels(
    source: impl Iterator<Item = f32>,
    source_channels: u32,
    target_channels: u32,
) -> impl Iterator<Item = f32> {
    channels::ChannelConverter::new(source, source_channels, target_channels)
}

pub fn rate(
    source: impl Iterator<Item = f32>,
    source_rate: u32,
    target_rate: u32,
) -> impl Iterator<Item = f32> {
    rate::RateConverter::new(source, source_rate, target_rate)
}

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
