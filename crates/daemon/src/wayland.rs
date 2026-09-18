use std::env;
use std::path::{Path, PathBuf};

use crate::rpc::{LockCapability, RpcError};

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

pub fn notify_show(
    stem: &str,
    choices: &[String; 4],
    correct_index: u8,
    session_id: &str,
) -> Result<(), RpcError> {
    crate::notify::notify_show(
        &wayland_ipc_path().map_err(|_| RpcError::NoLockBackend)?,
        stem,
        choices,
        correct_index,
        session_id,
    )
}

fn wayland_ready_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate-wayland.ready"))
}

fn wayland_ipc_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate-wayland.sock"))
}
