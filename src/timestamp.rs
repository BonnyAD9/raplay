use std::time::Duration;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Timestamp {
    pub current: Duration,
    pub total: Duration,
}

impl Timestamp {
    pub fn new(current: Duration, total: Duration) -> Self {
        Self { current, total }
    }
}
