#![cfg(feature = "x11")]

use std::fs;
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
    if let Err(err) = run() {
        eprintln!("{err}");
        let code = if err == GrabError::GrabBusy { 2 } else { 1 };
        process::exit(code);
    }
}

fn run() -> Result<(), GrabError> {
    if std::env::var("DISPLAY").is_err() {
        return Err(GrabError::Display("DISPLAY is not set".into()));
    }

    let ready_path = paths::x11_ready_path().map_err(|err| GrabError::Display(err))?;
    let (sender, receiver) = std::sync::mpsc::channel();
    ipc::spawn_listener(sender).map_err(|err| GrabError::Display(err))?;

    if let Some(parent) = ready_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&ready_path, b"ready").map_err(|err| GrabError::Display(err.to_string()))?;
    let _ready_guard = ReadyGuard(ready_path);

    grab::run(receiver)
}

struct ReadyGuard(std::path::PathBuf);

impl Drop for ReadyGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
