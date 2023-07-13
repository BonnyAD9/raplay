pub struct Interleave<I: Iterator<Item = T>, T> {
    iterators: Vec<I>,
    index: usize,
}

impl<I: Iterator<Item = T>, T> Interleave<I, T> {
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
        return r;
    }
}