use std::time::Duration;

use chrono::Utc;
use recallgate_core::{
    Abort, GateCadence, GateError, GateId, GatePhase, Item, ItemError, LockedState, McqChoices,
    Store, StoreError,
};
use tempfile::NamedTempFile;

fn sample_item() -> Item {
    Item::new(
        recallgate_core::ItemId::new(),
        "2 + 2",
        McqChoices::new(["3".into(), "4".into(), "5".into(), "6".into()]),
        recallgate_core::ChoiceIndex::try_new(1).expect("index"),
        false,
        None,
    )
}

#[test]
fn store_roundtrip_reopens_locked_gate() {
    let file = NamedTempFile::new().expect("temp db");
    let path = file.path().to_path_buf();
    let gate_id = GateId::new();
    let item = sample_item();
    let item_id = item.id;
    let started_at = Utc::now();

    {
        let mut store = Store::open(&path).expect("open");
        store.push_item(item).expect("item");
        store.begin_lock(LockedState::new(gate_id, item_id, started_at)).expect("lock");
    }

    let store = Store::open(&path).expect("reopen");
    match store.gate().expect("gate").phase() {
        GatePhase::Locked(locked) => {
            assert_eq!(locked.gate_id, gate_id);
            assert_eq!(locked.item_id, item_id);
        }
        other => panic!("expected locked, got {other:?}"),
    }
}

#[test]
fn store_roundtrip_locked_survives_drop_without_unlock() {
    let file = NamedTempFile::new().expect("temp db");
    let path = file.path().to_path_buf();
    let item = sample_item();
    let item_id = item.id;
    {
        let mut store = Store::open(&path).expect("open");
        store.push_item(item).expect("item");
        store.begin_lock(LockedState::new(GateId::new(), item_id, Utc::now())).expect("lock");
    }
    let store = Store::open(&path).expect("reopen");
    assert!(matches!(store.gate().expect("gate").phase(), GatePhase::Locked(_)));
}

#[test]
fn abort_does_not_increment_answer_count() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let gate_id = GateId::new();
    store.append_abort(&Abort::new(gate_id, "hatch", Utc::now(), 1)).expect("abort");
    assert_eq!(store.answer_count().expect("count"), 0);
}

#[test]
fn second_begin_lock_fails() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let item = sample_item();
    let item_id = item.id;
    store.push_item(item).expect("item");
    store.begin_lock(LockedState::new(GateId::new(), item_id, Utc::now())).expect("first lock");
    let err = store
        .begin_lock(LockedState::new(GateId::new(), item_id, Utc::now()))
        .expect_err("second lock");
    assert!(matches!(err, StoreError::Gate(GateError::AlreadyLocked)));
}

#[test]
fn import_json_prompt_lists_as_due() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let json = r#"{
        "stem": "Capital of France?",
        "choices": ["London", "Paris", "Berlin", "Madrid"],
        "correct_index": 1
    }"#;
    store.push_item_from_json(json).expect("import");
    assert_eq!(store.gate_due().expect("due").len(), 1);
}

#[test]
fn suspended_item_absent_from_due() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let mut item = sample_item();
    item.suspended = true;
    store.push_item(item).expect("item");
    assert_eq!(store.gate_due().expect("due").len(), 0);
}

#[test]
fn cadence_lock_interval_roundtrips() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let cadence = GateCadence::new(Duration::from_secs(120));
    store.set_cadence(cadence).expect("set");
    assert_eq!(store.cadence().expect("get"), cadence);
}

#[test]
fn corrupt_header_returns_typed_error() {
    let file = NamedTempFile::new().expect("temp db");
    std::fs::write(file.path(), b"not a sqlite file").expect("write");
    let open_result = Store::open(file.path());
    assert!(open_result.is_err());
    if let Err(err) = open_result {
        assert!(matches!(err, StoreError::CorruptDatabase | StoreError::Sqlite(_)));
    }
}

#[test]
fn store_is_single_writer() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    store.push_item(sample_item()).expect("item");
    let _second = Store::open(file.path()).expect("second connection is allowed for read");
    assert_eq!(store.gate_due().expect("due").len(), 1);
}

#[test]
fn persist_1000() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    for n in 0..1000 {
        let json = format!(r#"{{"stem":"q{n}","choices":["a","b","c","d"],"correct_index":0}}"#);
        store.push_item_from_json(&json).expect("push");
    }
    assert_eq!(store.gate_due().expect("due").len(), 1000);
}

#[test]
fn answer_roundtrip_through_store() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let item = sample_item();
    let answer = item
        .answer_for_choice(recallgate_core::ChoiceIndex::try_new(1).expect("index"), Utc::now());
    store.push_item(item).expect("item");
    store.append_answer(&answer).expect("answer");
    assert_eq!(store.answer_count().expect("count"), 1);
}

#[test]
fn invalid_json_prompt_rejected() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let err = store
        .push_item_from_json(r#"{"stem":"x","choices":["a"],"correct_index":0}"#)
        .expect_err("bad choices");
    assert!(matches!(err, StoreError::InvalidPrompt(_)));
}

#[test]
fn invalid_choice_index_rejected() {
    let file = NamedTempFile::new().expect("temp db");
    let mut store = Store::open(file.path()).expect("open");
    let err = store
        .push_item_from_json(r#"{"stem":"x","choices":["a","b","c","d"],"correct_index":9}"#)
        .expect_err("bad index");
    assert!(matches!(err, StoreError::Item(ItemError::InvalidChosenIndex)));
}
