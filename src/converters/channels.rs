use cpal::Sample;

// TODO: smarter conversion
/// Iterator that converts number of channels
#[derive(Debug)]
pub struct ChannelConverter<S: Sample, I: Iterator<Item = S>> {
    /// Original iterator
    source: I,
    /// Number of channels in the original iterator
    source_channels: u32,
    /// The target number of channels, how many channels should there be
    /// after conversion
    target_channels: u32,
    /// The index of the next channel that will be generated
    index: usize,
}

impl<S: Sample, I: Iterator<Item = S>> ChannelConverter<S, I> {
    /// Creates new channel converter iterator from iterator source and the
    /// source and target channel counts.
    pub fn new(source: I, source_channels: u32, target_channels: u32) -> Self {
        ChannelConverter {
            source,
            source_channels,
            target_channels,
            index: 0,
        }
    }
}

impl<S: Sample, I: Iterator<Item = S>> Iterator for ChannelConverter<S, I> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        match self.source_channels.cmp(&self.target_channels) {
            std::cmp::Ordering::Less => {
                let res = if self.index >= self.source_channels as usize {
                    Some(S::EQUILIBRIUM)
                } else {
                    self.source.next()
                };
                self.index = (self.index + 1) % self.target_channels as usize;
                res
            }
            std::cmp::Ordering::Equal => self.source.next(),
            std::cmp::Ordering::Greater => {
                let res = self.source.next();
                self.index += 1;
                if self.index >= self.target_channels as usize {
                    for _ in 0..(self.source_channels - self.target_channels) {
                        _ = self.source.next();
                    }
                    self.index = 0;
                }
                res
            }
        }
    }
}
