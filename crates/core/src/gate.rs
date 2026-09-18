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
