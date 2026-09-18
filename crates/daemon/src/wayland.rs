use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::rpc::LockCapability;

pub fn detect_capability() -> LockCapability {
    if env::var("WAYLAND_DISPLAY").is_err() {
        return LockCapability::None;
    }
    let ready = wayland_ready_path().ok().is_some_and(|path| path.exists());
    let socket = wayland_ipc_path().ok().is_some_and(|path| path.exists());
    if ready && socket {
        LockCapability::SessionLock
    } else {
        LockCapability::None
    }
}

pub fn notify_show(stem: &str, choices: &[String; 4], correct_index: u8) -> Result<(), String> {
    let path = wayland_ipc_path()?;
    let mut stream = UnixStream::connect(&path).map_err(|err| err.to_string())?;
    let message = json!({
        "stem": stem,
        "choices": choices,
        "correct_index": correct_index,
    });
    writeln!(stream, "{message}").map_err(|err| err.to_string())
}

fn wayland_ready_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate-wayland.ready"))
}

fn wayland_ipc_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate-wayland.sock"))
}
