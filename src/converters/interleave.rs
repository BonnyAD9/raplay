/// Iterator that interleaves channels
pub struct Interleave<I: Iterator<Item = T>, T> {
    /// Channels to interleave
    iterators: Vec<I>,
    /// The channel that should be used next
    index: usize,
}

impl<I: Iterator<Item = T>, T> Interleave<I, T> {
    /// Creates new interleave channel iterator
    pub fn new<II: Iterator<Item = I>>(iterators: II) -> Self {
        Interleave {
            iterators: iterators.collect(),
            index: 0,
        }
    }
}

impl<I: Iterator<Item = T>, T> Iterator for Interleave<I, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.iterators[self.index].next();
        self.index += 1;
        if self.index >= self.iterators.len() {
            self.index = 0;
        }
        r
    }
}
