#![cfg(feature = "ui")]

use std::fs;
use std::process;

use gtk4::prelude::*;
use gtk4::Application;

use recallgate_lock_wayland::ipc;
use recallgate_lock_wayland::lock;
use recallgate_lock_wayland::paths;

const APP_ID: &str = "dev.recallgate.lock-wayland";

fn main() {
    if gtk4::init().is_err() {
        eprintln!("unsupported");
        process::exit(1);
    }
    if !gtk4_session_lock::is_supported() {
        eprintln!("unsupported");
        process::exit(1);
    }

    let ready_path = paths::wayland_ready_path().unwrap_or_else(|err| {
        eprintln!("{err}");
        process::exit(1);
    });
    if let Some(parent) = ready_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&ready_path, b"ready").unwrap_or_else(|err| {
        eprintln!("{err}");
        process::exit(1);
    });
    let _ready_guard = ReadyGuard(ready_path);

    let (sender, receiver) = std::sync::mpsc::channel();
    if let Err(err) = ipc::spawn_listener(sender) {
        eprintln!("{err}");
        process::exit(1);
    }

    let app = Application::builder().application_id(APP_ID).build();
    lock::run_app(app, receiver);
}

struct ReadyGuard(std::path::PathBuf);

impl Drop for ReadyGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
