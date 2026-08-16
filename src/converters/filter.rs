/// Generic FIR filter.
#[derive(Debug)]
pub struct Filter {
    pos: usize,
    samples: Vec<f32>,
    filter: Vec<f32>,
}

impl Filter {
    /// Generate the filter from its IR.
    pub fn new(filter: Vec<f32>) -> Self {
        Self {
            pos: 0,
            samples: vec![0.; filter.len()],
            filter,
        }
    }

    /// Get the next sample when given current sample.
    pub fn next_sample(&mut self, i: f32) -> f32 {
        self.samples[self.pos] = i;
        self.pos += 1;
        if self.pos >= self.samples.len() {
            self.pos = 0;
        }

        let (end, start) = self.samples.split_at(self.pos);
        let mut sum = 0.;
        for (s, f) in start.iter().zip(&self.filter) {
            sum += *s * *f;
        }
        for (s, f) in end.iter().zip(&self.filter[start.len()..]) {
            sum += *s * *f;
        }
        sum
    }
}
