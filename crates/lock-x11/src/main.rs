#![cfg(feature = "x11")]

use std::fs;
use std::path::PathBuf;
use std::process;

use clap::Parser;
use recallgate_lock_x11::error::GrabError;
use recallgate_lock_x11::grab;
use recallgate_lock_x11::ipc;
use recallgate_lock_x11::paths;

const APP_NAME: &str = "recallgate-lock-x11";

/// Recall Gate X11 session grab lock client.
#[derive(Debug, Parser)]
#[command(name = APP_NAME, version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    let code = match run() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            if err == GrabError::GrabBusy {
                2
            } else {
                1
            }
        }
    };
    process::exit(code);
}

fn run() -> Result<(), GrabError> {
    if std::env::var("DISPLAY").is_err() {
        return Err(GrabError::Display("DISPLAY is not set".into()));
    }

    let ready_path = paths::x11_ready_path().map_err(GrabError::Display)?;
    let socket_path = paths::x11_ipc_path().map_err(GrabError::Display)?;
    let (sender, receiver) = std::sync::mpsc::channel();
    ipc::spawn_listener(sender).map_err(GrabError::Display)?;

    let _socket_guard = SocketGuard(socket_path);
    if let Some(parent) = ready_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&ready_path, b"ready").map_err(|err| GrabError::Display(err.to_string()))?;
    let _ready_guard = ReadyGuard(ready_path);

    grab::run(receiver)
}

struct ReadyGuard(PathBuf);

impl Drop for ReadyGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
