use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

pub fn daemon_socket_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate.sock"))
}

pub fn wayland_ready_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate-wayland.ready"))
}

pub fn wayland_ipc_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate-wayland.sock"))
}

pub fn rpc_call(method: &str, params: Value, id: u64) -> Result<Value, String> {
    let path = daemon_socket_path()?;
    let mut stream = UnixStream::connect(path).map_err(|err| err.to_string())?;
    let request = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });
    let line = format!("{request}\n");
    stream.write_all(line.as_bytes()).map_err(|err| err.to_string())?;
    stream.flush().map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).map_err(|err| err.to_string())?;
    let value: Value = serde_json::from_str(response.trim()).map_err(|err| err.to_string())?;
    if let Some(error) = value.get("error") {
        return Err(error["message"].as_str().unwrap_or("rpc error").to_string());
    }
    value.get("result").cloned().ok_or_else(|| "missing result".to_string())
}

pub fn submit_choice(chosen_index: u8) -> Result<bool, String> {
    let result = rpc_call("gate_submit_choice", json!({ "chosen_index": chosen_index }), 1)?;
    Ok(result["correct"].as_bool().unwrap_or(false))
}
