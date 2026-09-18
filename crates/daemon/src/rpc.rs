use std::time::Instant;

use chrono::Utc;
use rand::seq::SliceRandom;
use recallgate_core::{GateError, GateId, GatePhase, ItemId, LockedState, Store, StoreError};
use serde::Deserialize;
use serde_json::{json, Value};

const JSONRPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockCapability {
    None,
    SessionLock,
    X11Grab,
}

impl LockCapability {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SessionLock => "session_lock",
            Self::X11Grab => "x11_grab",
        }
    }
}

pub struct DaemonState {
    store: Store,
    last_unlock: Option<Instant>,
    capability: LockCapability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcError {
    Parse,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    NoLockBackend,
    AlreadyLocked,
    EmptyDeck,
    CadenceActive,
    Store(String),
}

impl DaemonState {
    pub fn new(store: Store, capability: LockCapability) -> Self {
        Self { store, last_unlock: None, capability }
    }

    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    pub fn note_unlock(&mut self) {
        self.last_unlock = Some(Instant::now());
    }

    pub fn handle_line(&mut self, line: &str) -> String {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return error_response(Value::Null, RpcError::InvalidRequest);
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => return error_response(Value::Null, RpcError::Parse),
        };
        let request: JsonRpcRequest = match serde_json::from_value(value) {
            Ok(req) => req,
            Err(_) => return error_response(Value::Null, RpcError::InvalidRequest),
        };
        if request.jsonrpc != JSONRPC_VERSION {
            return error_response(request.id, RpcError::InvalidRequest);
        }
        let result = self.dispatch(&request.method, request.params.as_ref());
        match result {
            Ok(value) => success_response(request.id, value),
            Err(err) => error_response(request.id, err),
        }
    }

    fn dispatch(&mut self, method: &str, params: Option<&Value>) -> Result<Value, RpcError> {
        match method {
            "gate_status" => self.gate_status(),
            "gate_due" => self.gate_due(),
            "gate_push_prompt" => {
                let params = params.ok_or(RpcError::InvalidParams)?;
                self.gate_push_prompt(params)
            }
            "gate_lock" => {
                let params = params.cloned().unwrap_or(Value::Null);
                self.gate_lock(&params)
            }
            "gate_unlock" => Err(RpcError::MethodNotFound),
            _ => Err(RpcError::MethodNotFound),
        }
    }

    fn gate_status(&self) -> Result<Value, RpcError> {
        let state = self.store.gate().map_err(map_store)?;
        let (phase, session_id, cooldown_until) = match state.phase() {
            GatePhase::Idle => ("idle", None, None),
            GatePhase::Locked(locked) => ("locked", Some(locked.gate_id.to_string()), None),
            GatePhase::Cooldown(cooldown) => {
                ("cooldown", Some(cooldown.gate_id.to_string()), Some(cooldown.until.to_rfc3339()))
            }
        };
        Ok(json!({
            "phase": phase,
            "capability": self.capability.as_str(),
            "session_id": session_id,
            "cooldown_until": cooldown_until,
        }))
    }

    fn gate_due(&self) -> Result<Value, RpcError> {
        let due = self.store.gate_due().map_err(map_store)?;
        let prompt_ids: Vec<String> = due.into_iter().map(|id| id.to_string()).collect();
        Ok(json!({ "prompt_ids": prompt_ids }))
    }

    fn gate_push_prompt(&mut self, params: &Value) -> Result<Value, RpcError> {
        let json = serde_json::to_string(params).map_err(|_| RpcError::InvalidParams)?;
        let id = self.store.push_item_from_json(&json).map_err(map_store)?;
        let id_str = id.to_string();
        Ok(json!({ "prompt_id": id_str, "card_id": id_str }))
    }

    fn gate_lock(&mut self, params: &Value) -> Result<Value, RpcError> {
        if self.capability == LockCapability::None {
            return Err(RpcError::NoLockBackend);
        }

        let cadence = self.store.cadence().map_err(map_store)?;
        if !cadence.allows_lock_at(self.last_unlock, Instant::now()) {
            return Err(RpcError::CadenceActive);
        }

        match self.store.gate().map_err(map_store)?.phase() {
            GatePhase::Locked(_) => return Err(RpcError::AlreadyLocked),
            GatePhase::Idle | GatePhase::Cooldown(_) => {}
        }

        let lock_params = if params.is_null() {
            GateLockParams { prompt_id: None }
        } else {
            serde_json::from_value(params.clone()).map_err(|_| RpcError::InvalidParams)?
        };

        let item_id = self.resolve_lock_item(lock_params.prompt_id.as_deref())?;
        let item = self.store.item(item_id).map_err(map_store)?.ok_or(RpcError::EmptyDeck)?;

        let gate_id = GateId::new();
        let locked = LockedState::new(gate_id, item_id, Utc::now());
        self.store.begin_lock(locked).map_err(map_store)?;

        let choices: Vec<&str> = item.choices.as_slice().iter().map(String::as_str).collect();
        Ok(json!({
            "session_id": gate_id.to_string(),
            "prompt": {
                "stem": item.stem,
                "choices": choices,
            }
        }))
    }

    fn resolve_lock_item(&self, prompt_id: Option<&str>) -> Result<ItemId, RpcError> {
        if let Some(raw) = prompt_id {
            let id: ItemId = raw.parse().map_err(|_| RpcError::InvalidParams)?;
            self.store.item(id).map_err(map_store)?.ok_or(RpcError::InvalidParams)?;
            if !self.store.gate_due().map_err(map_store)?.contains(&id) {
                return Err(RpcError::InvalidParams);
            }
            return Ok(id);
        }
        let due = self.store.gate_due().map_err(map_store)?;
        if due.is_empty() {
            return Err(RpcError::EmptyDeck);
        }
        let mut rng = rand::thread_rng();
        due.choose(&mut rng).copied().ok_or(RpcError::EmptyDeck)
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    id: Value,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct GateLockParams {
    prompt_id: Option<String>,
}

fn success_response(id: Value, result: Value) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result,
    }))
    .expect("serialize response")
}

fn error_response(id: Value, err: RpcError) -> String {
    let (code, message) = match &err {
        RpcError::Parse => (-32700, "parse error".to_string()),
        RpcError::InvalidRequest => (-32600, "invalid request".to_string()),
        RpcError::MethodNotFound => (-32601, "method not found".to_string()),
        RpcError::InvalidParams => (-32602, "invalid params".to_string()),
        RpcError::NoLockBackend => (-32000, "no_lock_backend".to_string()),
        RpcError::AlreadyLocked => (-32000, "already_locked".to_string()),
        RpcError::EmptyDeck => (-32000, "empty_deck".to_string()),
        RpcError::CadenceActive => (-32000, "cadence_active".to_string()),
        RpcError::Store(msg) => (-32000, msg.clone()),
    };
    serde_json::to_string(&json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": { "code": code, "message": message },
    }))
    .expect("serialize error")
}

fn map_store(err: StoreError) -> RpcError {
    match err {
        StoreError::Gate(GateError::AlreadyLocked) => RpcError::AlreadyLocked,
        StoreError::InvalidPrompt(_) | StoreError::Item(_) => RpcError::InvalidParams,
        StoreError::ItemMissing | StoreError::ItemSuspended => RpcError::InvalidParams,
        other => RpcError::Store(other.to_string()),
    }
}
