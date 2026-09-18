use chrono::Utc;
use recallgate_core::{
    CooldownState, GateError, GateId, GatePhase, GateState, ItemId, LockedState,
};

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
