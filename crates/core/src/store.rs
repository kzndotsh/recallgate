use std::fmt;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::abort::Abort;
use crate::cadence::GateCadence;
use crate::gate::{CooldownState, GateError, GatePhase, GateState, LockedState};
use crate::ids::{GateId, ItemId};
use crate::item::{Answer, ChoiceIndex, Item, McqChoices};

const SCHEMA_VERSION: i32 = 1;

const GATE_ROW_ID: i32 = 1;
const SETTINGS_ROW_ID: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    CorruptDatabase,
    Gate(GateError),
    InvalidPrompt(String),
    Item(crate::item::ItemError),
    ItemMissing,
    ItemSuspended,
    Sqlite(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptDatabase => write!(f, "database is corrupt or not a recallgate store"),
            Self::Gate(err) => write!(f, "{err:?}"),
            Self::InvalidPrompt(msg) => write!(f, "{msg}"),
            Self::Item(err) => write!(f, "{err:?}"),
            Self::ItemMissing => write!(f, "item not found in deck"),
            Self::ItemSuspended => write!(f, "item is suspended and cannot be locked"),
            Self::Sqlite(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<GateError> for StoreError {
    fn from(value: GateError) -> Self {
        Self::Gate(value)
    }
}

impl From<crate::item::ItemError> for StoreError {
    fn from(value: crate::item::ItemError) -> Self {
        Self::Item(value)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value.to_string())
    }
}

/// SQLite persistence for deck items, gate phase, answers, aborts, and cadence settings.
/// The daemon is the only writer; lock clients use RPC only.
pub struct Store {
    conn: Connection,
}

#[derive(Debug, Deserialize)]
struct PushPromptJson {
    stem: String,
    choices: Vec<String>,
    correct_index: u8,
    #[serde(default)]
    external_ref: Option<String>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path.as_ref()).map_err(map_open_error)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn push_item(&mut self, item: Item) -> Result<(), StoreError> {
        let choices_json = serde_json::to_string(item.choices.as_slice())
            .map_err(|err| StoreError::Sqlite(err.to_string()))?;
        self.conn.execute(
            "INSERT INTO items (id, stem, choices_json, correct_index, suspended, external_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               stem = excluded.stem,
               choices_json = excluded.choices_json,
               correct_index = excluded.correct_index,
               suspended = excluded.suspended,
               external_ref = excluded.external_ref",
            params![
                item.id.as_uuid().as_bytes(),
                item.stem,
                choices_json,
                item.correct_index.index(),
                item.suspended as i32,
                item.external_ref,
            ],
        )?;
        Ok(())
    }

    /// Import JSON in the shape of RPC `gate_push_prompt` params.
    pub fn push_item_from_json(&mut self, json: &str) -> Result<ItemId, StoreError> {
        let parsed: PushPromptJson =
            serde_json::from_str(json).map_err(|err| StoreError::InvalidPrompt(err.to_string()))?;
        if parsed.choices.len() != 4 {
            return Err(StoreError::InvalidPrompt(
                "choices must contain exactly four strings".into(),
            ));
        }
        let correct_index = ChoiceIndex::try_new(parsed.correct_index)?;
        let mut choices_arr = [String::new(), String::new(), String::new(), String::new()];
        for (slot, choice) in parsed.choices.into_iter().enumerate() {
            choices_arr[slot] = choice;
        }
        let id = ItemId::new();
        let item = Item::new(
            id,
            parsed.stem,
            McqChoices::new(choices_arr),
            correct_index,
            false,
            parsed.external_ref,
        );
        self.push_item(item)?;
        Ok(id)
    }

    pub fn item(&self, id: ItemId) -> Result<Option<Item>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT stem, choices_json, correct_index, suspended, external_ref
             FROM items WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_uuid().as_bytes()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(read_item_row(id, row)?))
    }

    /// Items that may appear on the lock surface (non-suspended deck rows).
    pub fn gate_due(&self) -> Result<Vec<ItemId>, StoreError> {
        let mut stmt = self.conn.prepare("SELECT id FROM items WHERE suspended = 0 ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            let uuid = uuid_from_bytes(&bytes).map_err(|_| {
                rusqlite::Error::InvalidColumnType(0, "id".into(), rusqlite::types::Type::Blob)
            })?;
            ItemId::from_uuid(uuid).map_err(|_| {
                rusqlite::Error::InvalidColumnType(0, "id".into(), rusqlite::types::Type::Blob)
            })
        })?;
        let mut due = Vec::new();
        for row in rows {
            due.push(row.map_err(|err| StoreError::Sqlite(err.to_string()))?);
        }
        Ok(due)
    }

    pub fn gate(&self) -> Result<GateState, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT phase, gate_id, item_id, started_at, until FROM gate_singleton WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![GATE_ROW_ID])?;
        let Some(row) = rows.next()? else {
            return Err(StoreError::CorruptDatabase);
        };
        let phase: String = row.get(0)?;
        let phase = match phase.as_str() {
            "idle" => GatePhase::Idle,
            "locked" => {
                let gate_id = read_gate_id(row.get(1)?)?;
                let item_id = read_item_id(row.get(2)?)?;
                let started_at = read_timestamp(row.get(3)?)?;
                GatePhase::Locked(LockedState::new(gate_id, item_id, started_at))
            }
            "cooldown" => {
                let gate_id = read_gate_id(row.get(1)?)?;
                let until = read_timestamp(row.get(4)?)?;
                GatePhase::Cooldown(CooldownState::new(gate_id, until))
            }
            _ => return Err(StoreError::CorruptDatabase),
        };
        Ok(GateState::from_phase(phase))
    }

    pub fn set_gate(&mut self, state: &GateState) -> Result<(), StoreError> {
        let (phase, gate_id, item_id, started_at, until) = encode_gate(state.phase())?;
        self.conn.execute(
            "UPDATE gate_singleton
             SET phase = ?2, gate_id = ?3, item_id = ?4, started_at = ?5, until = ?6
             WHERE id = ?1",
            params![GATE_ROW_ID, phase, gate_id, item_id, started_at, until],
        )?;
        Ok(())
    }

    pub fn begin_lock(&mut self, locked: LockedState) -> Result<(), StoreError> {
        self.ensure_lock_item(locked.item_id)?;
        let mut state = self.gate()?;
        state.begin_lock(locked)?;
        self.set_gate(&state)
    }

    pub fn append_answer(&mut self, answer: &Answer) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO answers (item_id, chosen_index, correct, answered_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                answer.item_id().as_uuid().as_bytes(),
                answer.chosen_index().index(),
                answer.correct() as i32,
                answer.answered_at().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn append_abort(&mut self, abort: &Abort) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO aborts (gate_id, method, at, cost_paid)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                abort.gate_id.as_uuid().as_bytes(),
                abort.method,
                abort.at.to_rfc3339(),
                abort.cost_paid,
            ],
        )?;
        Ok(())
    }

    pub fn answer_count(&self) -> Result<u64, StoreError> {
        let count: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM answers", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    pub fn cadence(&self) -> Result<GateCadence, StoreError> {
        let ms: i64 = self.conn.query_row(
            "SELECT lock_interval_ms FROM settings WHERE id = ?1",
            params![SETTINGS_ROW_ID],
            |row| row.get(0),
        )?;
        if ms < 0 {
            return Err(StoreError::CorruptDatabase);
        }
        Ok(GateCadence::new(Duration::from_millis(ms as u64)))
    }

    pub fn set_cadence(&mut self, cadence: GateCadence) -> Result<(), StoreError> {
        let ms = cadence.lock_interval.as_millis();
        let ms = i64::try_from(ms).map_err(|_| StoreError::CorruptDatabase)?;
        self.conn.execute(
            "UPDATE settings SET lock_interval_ms = ?2 WHERE id = ?1",
            params![SETTINGS_ROW_ID, ms],
        )?;
        Ok(())
    }

    fn ensure_lock_item(&self, item_id: ItemId) -> Result<(), StoreError> {
        let Some(item) = self.item(item_id)? else {
            return Err(StoreError::ItemMissing);
        };
        if item.suspended {
            return Err(StoreError::ItemSuspended);
        }
        Ok(())
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let version: i32 =
            self.conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap_or(0);
        if version >= SCHEMA_VERSION {
            return ensure_schema(&self.conn);
        }
        self.conn.execute_batch(
            "
            CREATE TABLE items (
              id BLOB PRIMARY KEY NOT NULL,
              stem TEXT NOT NULL,
              choices_json TEXT NOT NULL,
              correct_index INTEGER NOT NULL CHECK (correct_index BETWEEN 0 AND 3),
              suspended INTEGER NOT NULL CHECK (suspended IN (0, 1)),
              external_ref TEXT
            );

            CREATE TABLE gate_singleton (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              phase TEXT NOT NULL CHECK (phase IN ('idle', 'locked', 'cooldown')),
              gate_id BLOB,
              item_id BLOB,
              started_at TEXT,
              until TEXT
            );

            INSERT INTO gate_singleton (id, phase, gate_id, item_id, started_at, until)
            VALUES (1, 'idle', NULL, NULL, NULL, NULL);

            CREATE TABLE answers (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              item_id BLOB NOT NULL,
              chosen_index INTEGER NOT NULL,
              correct INTEGER NOT NULL,
              answered_at TEXT NOT NULL
            );

            CREATE TABLE aborts (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              gate_id BLOB NOT NULL,
              method TEXT NOT NULL,
              at TEXT NOT NULL,
              cost_paid INTEGER NOT NULL
            );

            CREATE TABLE settings (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              lock_interval_ms INTEGER NOT NULL
            );

            INSERT INTO settings (id, lock_interval_ms) VALUES (1, 0);

            PRAGMA user_version = 1;
            ",
        )?;
        ensure_schema(&self.conn)
    }
}

const REQUIRED_TABLES: &[&str] = &["items", "gate_singleton", "answers", "aborts", "settings"];

fn ensure_schema(conn: &Connection) -> Result<(), StoreError> {
    for table in REQUIRED_TABLES {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )?;
        if count == 0 {
            return Err(StoreError::CorruptDatabase);
        }
    }
    let gate_rows: i64 =
        conn.query_row("SELECT COUNT(*) FROM gate_singleton WHERE id = 1", [], |row| row.get(0))?;
    if gate_rows != 1 {
        return Err(StoreError::CorruptDatabase);
    }
    let settings_rows: i64 =
        conn.query_row("SELECT COUNT(*) FROM settings WHERE id = 1", [], |row| row.get(0))?;
    if settings_rows != 1 {
        return Err(StoreError::CorruptDatabase);
    }
    Ok(())
}

type EncodedGateRow = (String, Option<Vec<u8>>, Option<Vec<u8>>, Option<String>, Option<String>);

fn encode_gate(phase: &GatePhase) -> Result<EncodedGateRow, StoreError> {
    match phase {
        GatePhase::Idle => Ok(("idle".into(), None, None, None, None)),
        GatePhase::Locked(locked) => Ok((
            "locked".into(),
            Some(locked.gate_id.as_uuid().as_bytes().to_vec()),
            Some(locked.item_id.as_uuid().as_bytes().to_vec()),
            Some(locked.started_at.to_rfc3339()),
            None,
        )),
        GatePhase::Cooldown(cooldown) => Ok((
            "cooldown".into(),
            Some(cooldown.gate_id.as_uuid().as_bytes().to_vec()),
            None,
            None,
            Some(cooldown.until.to_rfc3339()),
        )),
    }
}

fn read_item_row(id: ItemId, row: &rusqlite::Row<'_>) -> Result<Item, StoreError> {
    let stem: String = row.get(0)?;
    let choices_json: String = row.get(1)?;
    let correct_index: i32 = row.get(2)?;
    let suspended: i32 = row.get(3)?;
    let external_ref: Option<String> = row.get(4)?;
    let choices_vec: Vec<String> =
        serde_json::from_str(&choices_json).map_err(|_| StoreError::CorruptDatabase)?;
    if choices_vec.len() != 4 {
        return Err(StoreError::CorruptDatabase);
    }
    let mut choices = [String::new(), String::new(), String::new(), String::new()];
    for (slot, choice) in choices_vec.into_iter().enumerate() {
        choices[slot] = choice;
    }
    let correct = ChoiceIndex::try_new(correct_index as u8)?;
    Ok(Item::new(id, stem, McqChoices::new(choices), correct, suspended != 0, external_ref))
}

fn read_gate_id(value: Option<Vec<u8>>) -> Result<GateId, StoreError> {
    let bytes = value.ok_or(StoreError::CorruptDatabase)?;
    let uuid = uuid_from_bytes(&bytes)?;
    GateId::from_uuid(uuid).map_err(|_| StoreError::CorruptDatabase)
}

fn read_item_id(value: Option<Vec<u8>>) -> Result<ItemId, StoreError> {
    let bytes = value.ok_or(StoreError::CorruptDatabase)?;
    let uuid = uuid_from_bytes(&bytes)?;
    ItemId::from_uuid(uuid).map_err(|_| StoreError::CorruptDatabase)
}

fn read_timestamp(value: Option<String>) -> Result<DateTime<Utc>, StoreError> {
    let text = value.ok_or(StoreError::CorruptDatabase)?;
    DateTime::parse_from_rfc3339(&text)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| StoreError::CorruptDatabase)
}

fn uuid_from_bytes(bytes: &[u8]) -> Result<uuid::Uuid, StoreError> {
    let arr: [u8; 16] = bytes.try_into().map_err(|_| StoreError::CorruptDatabase)?;
    Ok(uuid::Uuid::from_bytes(arr))
}

fn map_open_error(err: rusqlite::Error) -> StoreError {
    match err {
        rusqlite::Error::SqliteFailure(code, _)
            if code.code == rusqlite::ErrorCode::NotADatabase =>
        {
            StoreError::CorruptDatabase
        }
        other => StoreError::Sqlite(other.to_string()),
    }
}
