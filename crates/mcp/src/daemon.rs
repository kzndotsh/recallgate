use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

const JSONRPC_VERSION: &str = "2.0";
const READ_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonError {
    Connect(String),
    Timeout,
    Parse(String),
    Rpc { code: i64, message: String },
}

impl DaemonError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Connect(msg) => format!("daemon unavailable: {msg}"),
            Self::Timeout => "daemon read timed out".to_string(),
            Self::Parse(msg) => format!("daemon response invalid: {msg}"),
            Self::Rpc { message, .. } => message.clone(),
        }
    }
}

pub fn call(method: &str, params: Value) -> Result<Value, DaemonError> {
    let path = socket_path()?;
    let mut stream =
        UnixStream::connect(&path).map_err(|err| DaemonError::Connect(err.to_string()))?;
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|err| DaemonError::Connect(err.to_string()))?;
    stream
        .set_write_timeout(Some(READ_TIMEOUT))
        .map_err(|err| DaemonError::Connect(err.to_string()))?;

    let request = json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": method,
        "params": params,
        "id": 1,
    });
    let line = format!("{request}\n");
    stream.write_all(line.as_bytes()).map_err(|err| DaemonError::Connect(err.to_string()))?;
    stream.flush().map_err(|err| DaemonError::Connect(err.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    let bytes = reader.read_line(&mut response).map_err(|err| {
        if err.kind() == std::io::ErrorKind::WouldBlock
            || err.kind() == std::io::ErrorKind::TimedOut
        {
            DaemonError::Timeout
        } else {
            DaemonError::Connect(err.to_string())
        }
    })?;
    if bytes == 0 {
        return Err(DaemonError::Connect("daemon closed connection".into()));
    }

    let value: Value =
        serde_json::from_str(response.trim()).map_err(|err| DaemonError::Parse(err.to_string()))?;
    if let Some(error) = value.get("error") {
        return Err(DaemonError::Rpc {
            code: error["code"].as_i64().unwrap_or(-32000),
            message: error["message"].as_str().unwrap_or("rpc error").to_string(),
        });
    }
    value.get("result").cloned().ok_or_else(|| DaemonError::Parse("missing result".into()))
}

fn socket_path() -> Result<PathBuf, DaemonError> {
    let dir = env::var("XDG_RUNTIME_DIR")
        .map_err(|_| DaemonError::Connect("XDG_RUNTIME_DIR is not set".into()))?;
    Ok(Path::new(&dir).join("recallgate.sock"))
}
