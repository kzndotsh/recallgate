use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateCadence {
    pub lock_interval: Duration,
}

impl GateCadence {
    pub fn new(lock_interval: Duration) -> Self {
        Self { lock_interval }
    }

    /// Returns whether a new lock may start at `now` given the last unlock instant.
    pub fn allows_lock_at(
        &self,
        last_unlock: Option<std::time::Instant>,
        now: std::time::Instant,
    ) -> bool {
        match last_unlock {
            None => true,
            Some(t) => now.duration_since(t) >= self.lock_interval,
        }
    }
}
