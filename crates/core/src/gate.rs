use chrono::{DateTime, Utc};

use crate::ids::{GateId, ItemId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedState {
    pub gate_id: GateId,
    pub item_id: ItemId,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownState {
    pub gate_id: GateId,
    pub until: DateTime<Utc>,
}

/// Persisted gate state. There is no `Unlocked` variant; unlock is a transition to `Idle` or `Cooldown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatePhase {
    Idle,
    Locked(LockedState),
    Cooldown(CooldownState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateError {
    AlreadyLocked,
    NotLocked,
    IllegalTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateState {
    phase: GatePhase,
}

impl LockedState {
    pub fn new(gate_id: GateId, item_id: ItemId, started_at: DateTime<Utc>) -> Self {
        Self { gate_id, item_id, started_at }
    }
}

impl CooldownState {
    pub fn new(gate_id: GateId, until: DateTime<Utc>) -> Self {
        Self { gate_id, until }
    }
}

impl GateState {
    pub fn idle() -> Self {
        Self { phase: GatePhase::Idle }
    }

    pub fn phase(&self) -> &GatePhase {
        &self.phase
    }

    pub fn begin_lock(&mut self, locked: LockedState) -> Result<(), GateError> {
        match self.phase {
            GatePhase::Idle => {
                self.phase = GatePhase::Locked(locked);
                Ok(())
            }
            GatePhase::Locked(_) => Err(GateError::AlreadyLocked),
            GatePhase::Cooldown(_) => Err(GateError::IllegalTransition),
        }
    }

    pub fn unlock_to_idle(&mut self) -> Result<(), GateError> {
        match self.phase {
            GatePhase::Locked(_) => {
                self.phase = GatePhase::Idle;
                Ok(())
            }
            GatePhase::Idle | GatePhase::Cooldown(_) => Err(GateError::NotLocked),
        }
    }

    pub fn begin_cooldown(&mut self, cooldown: CooldownState) -> Result<(), GateError> {
        match self.phase {
            GatePhase::Locked(_) => {
                self.phase = GatePhase::Cooldown(cooldown);
                Ok(())
            }
            GatePhase::Idle | GatePhase::Cooldown(_) => Err(GateError::IllegalTransition),
        }
    }

    pub fn finish_cooldown_begin_lock(&mut self, locked: LockedState) -> Result<(), GateError> {
        match self.phase {
            GatePhase::Cooldown(_) => {
                self.phase = GatePhase::Locked(locked);
                Ok(())
            }
            GatePhase::Idle | GatePhase::Locked(_) => Err(GateError::IllegalTransition),
        }
    }

    /// Skip cooldown and return to idle. Illegal while locked (use unlock or abort paths).
    /// Normal cooldown completion is `finish_cooldown_begin_lock`, not this.
    pub fn force_idle(&mut self) -> Result<(), GateError> {
        match self.phase {
            GatePhase::Locked(_) => Err(GateError::IllegalTransition),
            GatePhase::Idle => Ok(()),
            GatePhase::Cooldown(_) => {
                self.phase = GatePhase::Idle;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::ids::{GateId, ItemId};

    #[test]
    fn gate_rejects_idle_while_locked() {
        let mut state = GateState::idle();
        let locked = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        state.begin_lock(locked).expect("lock");
        assert_eq!(state.force_idle(), Err(GateError::IllegalTransition));
    }

    #[test]
    fn unlock_to_idle_after_lock() {
        let mut state = GateState::idle();
        let locked = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        state.begin_lock(locked).expect("lock");
        state.unlock_to_idle().expect("unlock");
        assert!(matches!(state.phase(), GatePhase::Idle));
    }

    #[test]
    fn cannot_lock_while_already_locked_or_in_cooldown() {
        let mut state = GateState::idle();
        let locked = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        state.begin_lock(locked).expect("lock");
        let again = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        assert_eq!(state.begin_lock(again), Err(GateError::AlreadyLocked));

        let cooldown = CooldownState::new(GateId::new(), Utc::now());
        state.begin_cooldown(cooldown).expect("cooldown");
        let from_cooldown = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        assert_eq!(state.begin_lock(from_cooldown), Err(GateError::IllegalTransition));
    }

    #[test]
    fn abort_path_cooldown_then_relock() {
        let gate = GateId::new();
        let mut state = GateState::idle();
        let locked = LockedState::new(gate, ItemId::new(), Utc::now());
        state.begin_lock(locked).expect("lock");
        state.begin_cooldown(CooldownState::new(gate, Utc::now())).expect("cooldown");
        let relock = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        state.finish_cooldown_begin_lock(relock).expect("relock");
        assert!(matches!(state.phase(), GatePhase::Locked(_)));
    }

    #[test]
    fn force_idle_from_cooldown_only() {
        let mut state = GateState::idle();
        let locked = LockedState::new(GateId::new(), ItemId::new(), Utc::now());
        state.begin_lock(locked).expect("lock");
        state.begin_cooldown(CooldownState::new(GateId::new(), Utc::now())).expect("cooldown");
        state.force_idle().expect("cancel cooldown");
        assert!(matches!(state.phase(), GatePhase::Idle));
    }
}
