use std::{mem, ops::Index};

use cpal::{I24, U24};
use symphonia::core::audio::{
    AudioBuffer,
    sample::{Sample, i24, u24},
};

pub struct PlanesTransmute<'a, S: Sample>(pub &'a AudioBuffer<S>);

impl<'a> Index<usize> for PlanesTransmute<'a, u24> {
    type Output = [U24];

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { mem::transmute(&self.0[index]) }
    }
}

impl<'a> Index<usize> for PlanesTransmute<'a, i24> {
    type Output = [I24];

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { mem::transmute(&self.0[index]) }
    }
}
