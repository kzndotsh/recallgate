use std::sync::{Arc, Mutex};

use recallgate_core::Store;
use recallgate_daemon::rpc::{DaemonState, LockCapability, RpcError};
use serde_json::json;
use tempfile::NamedTempFile;

struct TestHarness {
    _db: NamedTempFile,
    state: DaemonState,
    shows: Arc<Mutex<Vec<String>>>,
}

impl TestHarness {
    fn new() -> Self {
        Self::with_capability(LockCapability::None, Ok(()))
    }

    fn with_backend() -> Self {
        Self::with_capability(LockCapability::SessionLock, Ok(()))
    }

    fn with_capability(capability: LockCapability, result: Result<(), RpcError>) -> Self {
        let db = NamedTempFile::new().expect("temp db");
        let store = Store::open(db.path()).expect("store");
        let shows = Arc::new(Mutex::new(Vec::new()));
        let shows_cb = Arc::clone(&shows);
        let notify = Arc::new(move |_stem: &str, _c: &[String; 4], _i: u8, session: &str| {
            shows_cb.lock().expect("shows").push(session.to_string());
            result.clone()
        });
        let state = DaemonState::with_notify(store, capability, notify);
        Self { _db: db, state, shows }
    }

    fn handle(&mut self, line: &str) -> String {
        self.state.handle_line(line)
    }

    fn store_mut(&mut self) -> &mut Store {
        self.state.store_mut()
    }

    fn push_card(&mut self) {
        let params = json!({
            "stem": "Q",
            "choices": ["a", "b", "c", "d"],
            "correct_index": 0
        });
        let line =
            format!(r#"{{"jsonrpc":"2.0","method":"gate_push_prompt","params":{params},"id":1}}"#);
        self.handle(&line);
    }
}

fn parse_response(line: &str) -> serde_json::Value {
    serde_json::from_str(line.trim()).expect("json response")
}

#[test]
fn rpc_rejects_unknown_method() {
    let mut harness = TestHarness::new();
    let raw = harness.handle(r#"{"jsonrpc":"2.0","method":"nope","id":1}"#);
    let body = parse_response(&raw);
    assert_eq!(body["error"]["code"], -32601);
}

#[test]
fn gate_unlock_is_method_not_found() {
    let mut harness = TestHarness::new();
    let raw = harness.handle(r#"{"jsonrpc":"2.0","method":"gate_unlock","id":2}"#);
    let body = parse_response(&raw);
    assert_eq!(body["error"]["code"], -32601);
}

#[test]
fn gate_status_idle_by_default() {
    let mut harness = TestHarness::new();
    let raw = harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":3}"#);
    let body = parse_response(&raw);
    assert_eq!(body["result"]["phase"], "idle");
    assert_eq!(body["result"]["capability"], "none");
    assert!(body["result"]["session_id"].is_null());
}

#[test]
fn gate_lock_without_backend_errors() {
    let mut harness = TestHarness::new();
    let push = json!({
        "stem": "Q",
        "choices": ["a", "b", "c", "d"],
        "correct_index": 0
    });
    harness.store_mut().push_item_from_json(&push.to_string()).expect("item");
    let raw = harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":4}"#);
    let body = parse_response(&raw);
    assert_eq!(body["error"]["message"], "no_lock_backend");
    let status =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":5}"#));
    assert_eq!(status["result"]["phase"], "idle");
}

#[test]
fn gate_push_prompt_then_gate_due() {
    let mut harness = TestHarness::new();
    let params = json!({
        "stem": "Capital?",
        "choices": ["a", "b", "c", "d"],
        "correct_index": 1
    });
    let push_line =
        format!(r#"{{"jsonrpc":"2.0","method":"gate_push_prompt","params":{},"id":6}}"#, params);
    harness.handle(&push_line);
    let due = parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_due","id":7}"#));
    assert_eq!(due["result"]["prompt_ids"].as_array().unwrap().len(), 1);
}

#[test]
fn malformed_json_then_next_call_works() {
    let mut harness = TestHarness::new();
    let bad = harness.handle("{not json");
    let bad_body = parse_response(&bad);
    assert_eq!(bad_body["error"]["code"], -32700);
    let ok = parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":8}"#));
    assert_eq!(ok["result"]["phase"], "idle");
}

#[test]
fn gate_lock_with_backend_then_submit() {
    let mut harness = TestHarness::with_backend();
    harness.push_card();
    let lock = parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":10}"#));
    assert!(lock["result"]["session_id"].is_string());
    assert_eq!(harness.shows.lock().expect("shows").len(), 1);
    let submit = parse_response(&harness.handle(
        r#"{"jsonrpc":"2.0","method":"gate_submit_choice","params":{"chosen_index":0},"id":11}"#,
    ));
    assert_eq!(submit["result"]["correct"], true);
    let status =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":12}"#));
    assert_eq!(status["result"]["phase"], "idle");
}

#[test]
fn gate_lock_while_locked_reshows() {
    let mut harness = TestHarness::with_backend();
    harness.push_card();
    let first =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":20}"#));
    let sid = first["result"]["session_id"].as_str().expect("sid").to_string();
    let second =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":21}"#));
    assert_eq!(second["result"]["session_id"], sid);
    assert_eq!(harness.shows.lock().expect("shows").len(), 2);
}

#[test]
fn gate_abort_enters_cooldown_without_answer() {
    let mut harness = TestHarness::with_backend();
    harness.push_card();
    harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":30}"#);
    let abort =
        parse_response(&harness.handle(
            r#"{"jsonrpc":"2.0","method":"gate_abort","params":{"method":"hatch"},"id":31}"#,
        ));
    assert!(abort["result"]["cooldown_until"].is_string());
    let status =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":32}"#));
    assert_eq!(status["result"]["phase"], "cooldown");
    assert_eq!(harness.store_mut().answer_count().expect("answers"), 0);
    assert_eq!(harness.store_mut().abort_count().expect("aborts"), 1);
}

#[test]
fn gate_lock_grab_busy() {
    let mut harness =
        TestHarness::with_capability(LockCapability::X11Grab, Err(RpcError::GrabBusy));
    harness.push_card();
    let body = parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":40}"#));
    assert_eq!(body["error"]["message"], "grab_busy");
    let status =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":41}"#));
    assert_eq!(status["result"]["phase"], "idle");
}

#[test]
fn cooldown_tick_relocks_without_gate_lock() {
    let mut harness = TestHarness::with_backend();
    harness.push_card();
    harness.handle(r#"{"jsonrpc":"2.0","method":"gate_lock","id":50}"#);
    harness.store_mut().set_cooldown_duration(std::time::Duration::from_millis(0)).expect("cd");
    harness.handle(r#"{"jsonrpc":"2.0","method":"gate_abort","id":51}"#);
    harness.state.tick_cooldown().expect("tick");
    let status =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":52}"#));
    assert_eq!(status["result"]["phase"], "locked");
}

#[test]
fn gate_abort_unknown_to_mcp_shape_is_still_daemon_method() {
    let mut harness = TestHarness::new();
    let raw = harness.handle(r#"{"jsonrpc":"2.0","method":"gate_abort","id":60}"#);
    let body = parse_response(&raw);
    assert_eq!(body["error"]["message"], "not_locked");
}
