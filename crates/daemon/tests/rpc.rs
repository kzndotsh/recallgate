use recallgate_core::Store;
use recallgate_daemon::rpc::{DaemonState, LockCapability};
use serde_json::json;
use tempfile::NamedTempFile;

struct TestHarness {
    _db: NamedTempFile,
    state: DaemonState,
}

impl TestHarness {
    fn new() -> Self {
        let db = NamedTempFile::new().expect("temp db");
        let store = Store::open(db.path()).expect("store");
        let state = DaemonState::new(store, LockCapability::None);
        Self { _db: db, state }
    }

    fn handle(&mut self, line: &str) -> String {
        self.state.handle_line(line)
    }

    fn store_mut(&mut self) -> &mut Store {
        self.state.store_mut()
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
    let ok =
        parse_response(&harness.handle(r#"{"jsonrpc":"2.0","method":"gate_status","id":8}"#));
    assert_eq!(ok["result"]["phase"], "idle");
}
