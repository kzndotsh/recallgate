use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::paths;

#[derive(Debug, Clone)]
pub struct ShowPrompt {
    pub stem: String,
    pub choices: [String; 4],
    pub correct_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowAck {
    Ok,
    GrabBusy,
    Unsupported,
}

pub struct IncomingShow {
    pub prompt: ShowPrompt,
    pub ack: Sender<ShowAck>,
}

#[derive(Debug, Deserialize)]
struct ShowMessage {
    stem: String,
    choices: Vec<String>,
    correct_index: u8,
}

pub fn spawn_listener(sender: Sender<IncomingShow>) -> Result<(), String> {
    let socket_path = paths::wayland_ipc_path()?;
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).map_err(|err| err.to_string())?;
    }
    let listener = UnixListener::bind(&socket_path).map_err(|err| err.to_string())?;
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            handle_client(stream, &sender);
        }
    });
    Ok(())
}

fn handle_client(stream: UnixStream, sender: &Sender<IncomingShow>) {
    let Ok(clone) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(clone);
    let mut writer = stream;
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let Ok(msg) = serde_json::from_str::<ShowMessage>(line.trim()) else {
        let _ = write_ack(&mut writer, ShowAck::Unsupported);
        return;
    };
    if msg.choices.len() != 4 || msg.correct_index >= 4 {
        let _ = write_ack(&mut writer, ShowAck::Unsupported);
        return;
    }
    let mut choices = [String::new(), String::new(), String::new(), String::new()];
    for (slot, choice) in msg.choices.into_iter().enumerate() {
        choices[slot] = choice;
    }
    let (ack_tx, ack_rx) = mpsc::channel();
    if sender
        .send(IncomingShow {
            prompt: ShowPrompt { stem: msg.stem, choices, correct_index: msg.correct_index },
            ack: ack_tx,
        })
        .is_err()
    {
        let _ = write_ack(&mut writer, ShowAck::Unsupported);
        return;
    }
    let ack = ack_rx.recv_timeout(Duration::from_secs(5)).unwrap_or(ShowAck::Unsupported);
    let _ = write_ack(&mut writer, ack);
}

fn write_ack(stream: &mut UnixStream, ack: ShowAck) -> std::io::Result<()> {
    let body = match ack {
        ShowAck::Ok => json!({ "ok": true }),
        ShowAck::GrabBusy => json!({ "ok": false, "error": "grab_busy" }),
        ShowAck::Unsupported => json!({ "ok": false, "error": "unsupported" }),
    };
    writeln!(stream, "{body}")?;
    stream.flush()
}
