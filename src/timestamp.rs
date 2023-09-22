use std::time::Duration;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Timestamp {
    pub current: Duration,
    pub total: Duration,
}

impl Timestamp {
    pub fn new(current: Duration, total: Duration) -> Self {
        Self { current, total }
    }
}
