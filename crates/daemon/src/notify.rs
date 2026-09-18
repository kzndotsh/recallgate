use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::rpc::{LockCapability, RpcError};

const ACK_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Deserialize)]
struct ShowAck {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

pub fn notify_show(
    socket_path: &Path,
    stem: &str,
    choices: &[String; 4],
    correct_index: u8,
    session_id: &str,
) -> Result<(), RpcError> {
    let mut stream = UnixStream::connect(socket_path).map_err(|_| RpcError::NoLockBackend)?;
    stream.set_read_timeout(Some(ACK_TIMEOUT)).map_err(|_| RpcError::NoLockBackend)?;
    stream.set_write_timeout(Some(ACK_TIMEOUT)).map_err(|_| RpcError::NoLockBackend)?;
    let message = json!({
        "stem": stem,
        "choices": choices,
        "correct_index": correct_index,
        "session_id": session_id,
    });
    writeln!(stream, "{message}").map_err(|_| RpcError::NoLockBackend)?;
    stream.flush().map_err(|_| RpcError::NoLockBackend)?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).map_err(|_| RpcError::NoLockBackend)?;
    if bytes == 0 {
        return Err(RpcError::NoLockBackend);
    }
    let ack: ShowAck = serde_json::from_str(line.trim()).map_err(|_| RpcError::NoLockBackend)?;
    if ack.ok {
        return Ok(());
    }
    match ack.error.as_deref() {
        Some("grab_busy") => Err(RpcError::GrabBusy),
        Some("unsupported") => Err(RpcError::NoLockBackend),
        _ => Err(RpcError::NoLockBackend),
    }
}

pub fn live_capability() -> LockCapability {
    let wayland = crate::wayland::detect_capability();
    if wayland != LockCapability::None {
        return wayland;
    }
    crate::x11::detect_capability()
}

pub fn live_notify(
    stem: &str,
    choices: &[String; 4],
    correct_index: u8,
    session_id: &str,
) -> Result<(), RpcError> {
    match live_capability() {
        LockCapability::SessionLock => {
            crate::wayland::notify_show(stem, choices, correct_index, session_id)
        }
        LockCapability::X11Grab => {
            crate::x11::notify_show(stem, choices, correct_index, session_id)
        }
        LockCapability::None => Err(RpcError::NoLockBackend),
    }
}
