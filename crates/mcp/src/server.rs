use serde_json::{json, Value};

use crate::daemon::{self, DaemonError};
use crate::tools::{self, SESSION_RESOURCE_URI};

const MCP_PROTOCOL: &str = "2024-11-05";

#[derive(Default)]
pub struct McpServer {
    initialized: bool,
}

impl McpServer {
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => {
                return Some(error_response(Value::Null, -32700, "parse error"));
            }
        };
        let id = value.get("id").cloned().unwrap_or(Value::Null);
        let method = match value.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None => return Some(error_response(id, -32600, "invalid request")),
        };
        if value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(error_response(id, -32600, "invalid request"));
        }

        if method.starts_with("notifications/") {
            if method == "notifications/initialized" {
                return None;
            }
            return None;
        }

        let result = self.dispatch(method, value.get("params"));
        match result {
            Ok(result) => Some(success_response(id, result)),
            Err(err) => Some(error_response(id, err.code, &err.message)),
        }
    }

    fn dispatch(&mut self, method: &str, params: Option<&Value>) -> Result<Value, ServerError> {
        match method {
            "initialize" => self.initialize(params),
            "ping" => Ok(json!({})),
            "tools/list" => {
                self.require_initialized()?;
                Ok(tools::tools_list())
            }
            "tools/call" => {
                self.require_initialized()?;
                self.tools_call(params)
            }
            "resources/list" => {
                self.require_initialized()?;
                Ok(tools::resources_list())
            }
            "resources/read" => {
                self.require_initialized()?;
                self.resources_read(params)
            }
            _ => Err(ServerError { code: -32601, message: "method not found".into() }),
        }
    }

    fn initialize(&mut self, params: Option<&Value>) -> Result<Value, ServerError> {
        let _ = params;
        self.initialized = true;
        Ok(json!({
            "protocolVersion": MCP_PROTOCOL,
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "recallgate-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }

    fn require_initialized(&self) -> Result<(), ServerError> {
        if self.initialized {
            Ok(())
        } else {
            Err(ServerError { code: -32002, message: "server not initialized".into() })
        }
    }

    fn tools_call(&self, params: Option<&Value>) -> Result<Value, ServerError> {
        let params = params.ok_or_else(ServerError::invalid_params)?;
        let name =
            params.get("name").and_then(Value::as_str).ok_or_else(ServerError::invalid_params)?;
        let arguments =
            params.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));

        if name == "gate_unlock" {
            return Ok(tool_error("unknown tool: gate_unlock"));
        }

        if !tools::TOOL_NAMES.iter().any(|tool| *tool == name) {
            return Ok(tool_error(&format!("unknown tool: {name}")));
        }

        let daemon_params =
            if name == "gate_status" || name == "gate_due" { Value::Null } else { arguments };

        match daemon::call(name, daemon_params) {
            Ok(result) => Ok(tool_success(&result)),
            Err(err) => Ok(tool_error(&err.user_message())),
        }
    }

    fn resources_read(&self, params: Option<&Value>) -> Result<Value, ServerError> {
        let params = params.ok_or_else(ServerError::invalid_params)?;
        let uri =
            params.get("uri").and_then(Value::as_str).ok_or_else(ServerError::invalid_params)?;
        if uri != SESSION_RESOURCE_URI {
            return Err(ServerError { code: -32602, message: "unknown resource".into() });
        }
        let result = daemon::call("gate_status", Value::Null).map_err(daemon_error_to_server)?;
        let text = serde_json::to_string(&result)
            .map_err(|err| ServerError { code: -32000, message: err.to_string() })?;
        Ok(json!({
            "contents": [{
                "uri": SESSION_RESOURCE_URI,
                "mimeType": "application/json",
                "text": text
            }]
        }))
    }
}

struct ServerError {
    code: i64,
    message: String,
}

impl ServerError {
    fn invalid_params() -> Self {
        Self { code: -32602, message: "invalid params".into() }
    }
}

fn daemon_error_to_server(err: DaemonError) -> ServerError {
    ServerError { code: -32000, message: err.user_message() }
}

fn tool_success(result: &Value) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(result).unwrap_or_else(|_| "{}".into())
        }],
        "isError": false
    })
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": message
        }],
        "isError": true
    })
}

fn success_response(id: Value, result: Value) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
    .expect("serialize response")
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    }))
    .expect("serialize error")
}
