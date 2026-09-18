use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process;

use recallgate_core::Store;
use recallgate_daemon::rpc::{DaemonState, LockCapability};

fn main() {
    if let Err(err) = run() {
        eprintln!("recallgate-daemon: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let db_path = data_db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let store = Store::open(&db_path).map_err(|err| err.to_string())?;
    let mut state = DaemonState::new(store, LockCapability::None);

    let socket_path = runtime_socket_path()?;
    if socket_path.exists() {
        return Err(format!(
            "socket already exists at {} (another daemon running?)",
            socket_path.display()
        ));
    }
    let listener = UnixListener::bind(&socket_path).map_err(|err| err.to_string())?;
    let _socket_guard = SocketGuard(socket_path.clone());

    for stream in listener.incoming() {
        let stream = stream.map_err(|err| err.to_string())?;
        if let Err(err) = serve_client(stream, &mut state) {
            eprintln!("recallgate-daemon: client error: {err}");
        }
    }
    Ok(())
}

fn serve_client(stream: UnixStream, state: &mut DaemonState) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if bytes == 0 {
            break;
        }
        let response = state.handle_line(&line);
        writer.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(b"\n").map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn data_db_path() -> Result<PathBuf, String> {
    let base = env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map_err(|_| "HOME is not set".to_string())?;
    Ok(base.join("recallgate").join("recallgate.db"))
}

fn runtime_socket_path() -> Result<PathBuf, String> {
    let dir = env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set".to_string())?;
    Ok(Path::new(&dir).join("recallgate.sock"))
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
