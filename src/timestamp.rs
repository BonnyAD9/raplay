use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    pub current: Duration,
    pub total: Duration,
}

impl Timestamp {
    pub fn new(current: Duration, total: Duration) -> Self {
        Self { current, total }
    }
}
