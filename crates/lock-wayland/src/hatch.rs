use std::time::{Duration, Instant};

const HOLD: Duration = Duration::from_secs(2);
const CONFIRM: &str = "ABORT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HatchOutcome {
    Ignored,
    Armed,
    Confirming,
    Completed,
}

#[derive(Debug, Default)]
pub struct HatchTracker {
    hold_from: Option<Instant>,
    confirming: bool,
    buffer: String,
}

impl HatchTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_confirming(&self) -> bool {
        self.confirming
    }

    pub fn on_chord(
        &mut self,
        ctrl: bool,
        shift: bool,
        escape: bool,
        now: Instant,
    ) -> HatchOutcome {
        if self.confirming {
            return HatchOutcome::Confirming;
        }
        if ctrl && shift && escape {
            let start = *self.hold_from.get_or_insert(now);
            if now.duration_since(start) >= HOLD {
                self.confirming = true;
                self.buffer.clear();
                return HatchOutcome::Confirming;
            }
            return HatchOutcome::Armed;
        }
        self.hold_from = None;
        HatchOutcome::Ignored
    }

    pub fn on_text(&mut self, ch: char) -> HatchOutcome {
        if !self.confirming {
            return HatchOutcome::Ignored;
        }
        if ch == '\n' {
            let done = self.buffer == CONFIRM;
            self.buffer.clear();
            if done {
                self.confirming = false;
                self.hold_from = None;
                return HatchOutcome::Completed;
            }
            return HatchOutcome::Confirming;
        }
        if ch.is_ascii_alphabetic() {
            self.buffer.push(ch.to_ascii_uppercase());
            if self.buffer.len() > CONFIRM.len() {
                self.buffer.clear();
            }
        }
        HatchOutcome::Confirming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hatch_completes_after_hold_and_abort() {
        let mut hatch = HatchTracker::new();
        let t0 = Instant::now();
        assert_eq!(hatch.on_chord(true, true, true, t0), HatchOutcome::Armed);
        assert_eq!(
            hatch.on_chord(true, true, true, t0 + Duration::from_secs(2)),
            HatchOutcome::Confirming
        );
        for ch in CONFIRM.chars() {
            assert_eq!(hatch.on_text(ch), HatchOutcome::Confirming);
        }
        assert_eq!(hatch.on_text('\n'), HatchOutcome::Completed);
    }

    #[test]
    fn hatch_ignores_short_hold() {
        let mut hatch = HatchTracker::new();
        let t0 = Instant::now();
        hatch.on_chord(true, true, true, t0);
        assert_eq!(
            hatch.on_chord(true, true, true, t0 + Duration::from_millis(500)),
            HatchOutcome::Armed
        );
        hatch.on_chord(false, false, false, t0 + Duration::from_millis(501));
        assert_eq!(hatch.on_text('A'), HatchOutcome::Ignored);
    }
}
