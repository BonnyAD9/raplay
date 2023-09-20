use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    current: Duration,
    total: Duration,
}

impl Timestamp {
    pub fn new(current: Duration, total: Duration) -> Self {
        Self {
            current,
            total
        }
    }
}
