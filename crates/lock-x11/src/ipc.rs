use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::sync::mpsc::Sender;
use std::thread;

use serde::Deserialize;

use crate::paths;

#[derive(Debug, Clone)]
pub struct ShowPrompt {
    pub stem: String,
    pub choices: [String; 4],
    pub correct_index: u8,
}

#[derive(Debug, Deserialize)]
struct ShowMessage {
    stem: String,
    choices: Vec<String>,
    correct_index: u8,
}

pub fn spawn_listener(sender: Sender<ShowPrompt>) -> Result<(), String> {
    let socket_path = paths::x11_ipc_path()?;
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).map_err(|err| err.to_string())?;
    }
    let listener = UnixListener::bind(&socket_path).map_err(|err| err.to_string())?;
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<ShowMessage>(line.trim()) else {
                continue;
            };
            if msg.choices.len() != 4 || msg.correct_index >= 4 {
                continue;
            }
            let mut choices = [String::new(), String::new(), String::new(), String::new()];
            for (slot, choice) in msg.choices.into_iter().enumerate() {
                choices[slot] = choice;
            }
            let _ = sender.send(ShowPrompt {
                stem: msg.stem,
                choices,
                correct_index: msg.correct_index,
            });
        }
    });
    Ok(())
}
