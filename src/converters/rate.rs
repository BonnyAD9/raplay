use cpal::Sample;
use num::{Float, NumCast, One, ToPrimitive, Zero};

/// Iterator that converts sample rates
#[derive(Debug)]
pub struct Rate<S, I>
where
    S: Sample + std::ops::Add<Output = S>,
    I: Iterator<Item = S>,
    S::Float: Float + NumCast,
{
    source: I,
    ratio: S::Float,
    index: S::Float,
    a: Option<S>,
    b: Option<S>,
}

impl<S, I> Rate<S, I>
where
    S: Sample + std::ops::Add<Output = S>,
    I: Iterator<Item = S>,
    S::Float: Float + NumCast,
{
    /// Craetes new iterator that converts the source iterator from the source
    /// sample rate to the target sample rate
    pub fn new<R: ToPrimitive>(
        source: I,
        source_rate: R,
        target_rate: R,
    ) -> Self {
        Rate {
            source,
            ratio: <S::Float as NumCast>::from(source_rate).unwrap()
                / <S::Float as NumCast>::from(target_rate).unwrap(),
            index: S::Float::zero(),
            a: None,
            b: None,
        }
    }
}

impl<S, I> Iterator for Rate<S, I>
where
    S: Sample + std::ops::Add<Output = S>,
    I: Iterator<Item = S>,
    S::Float: Float + NumCast,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: low pass filter
        if self.ratio.is_one() {
            return self.source.next();
        }

        if self.a.is_none() {
            self.a = self.source.next();
            self.b = self.source.next();
            self.a?;
            if self.b.is_none() {
                return self.a;
            }
        } else if self.b.is_none() {
            return None;
        }

        // a and b are Some
        let a = self.a.unwrap();
        let b = self.b.unwrap();

        let res = a.mul_amp(S::Float::one() - self.index)
            + b.mul_amp(S::Float::from_sample(self.index));

        self.index = self.index + self.ratio;

        while self.index >= S::Float::one() {
            self.a = self.b;
            self.index = self.index - S::Float::one();
            self.b = self.source.next();
        }

        Some(res)
    }
}
