pub struct RateConverter<I: Iterator<Item = f32>> {
    source: I,
    ratio: f32,
    index: f32,
    a: Option<f32>,
    b: Option<f32>,
}

impl<I: Iterator<Item = f32>> RateConverter<I> {
    pub fn new(source: I, source_rate: u32, target_rate: u32) -> Self {
        RateConverter {
            source,
            ratio: target_rate as f32 / source_rate as f32,
            index: 0.,
            a: None,
            b: None,
        }
    }
}

impl<I: Iterator<Item = f32>> Iterator for RateConverter<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ratio == 1. {
            return self.source.next();
        }

        if self.a.is_none() {
            self.a = self.source.next();
            self.b = self.source.next();
            if self.a.is_none() {
                return None;
            }
            if self.b.is_none() {
                return self.a;
            }
        } else if self.b.is_none() {
            return None;
        }

        // a and b are Some
        let a = self.a.unwrap();
        let b = self.b.unwrap();

        let res = a * (1. - self.index) + b * self.index;

        self.index += self.ratio;

        while self.index >= 1. {
            self.a = self.b;
            self.b = self.source.next();
        }

        Some(res)
    }
}
