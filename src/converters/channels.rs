// TODO: smarter conversion
pub struct ChannelConverter<I: Iterator<Item = f32>> {
    source: I,
    source_channels: u32,
    target_channels: u32,
    index: usize,
}

impl<I: Iterator<Item = f32>> ChannelConverter<I> {
    pub fn new(source: I, source_channels: u32, target_channels: u32) -> Self {
        ChannelConverter {
            source,
            source_channels,
            target_channels,
            index: 0,
        }
    }
}

impl<I: Iterator<Item = f32>> Iterator for ChannelConverter<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.source_channels.cmp(&self.target_channels) {
            std::cmp::Ordering::Less => {
                let res = if self.index >= self.source_channels as usize {
                    Some(0.)
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
