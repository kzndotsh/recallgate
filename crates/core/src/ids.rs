use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

const UUID_VERSION_RANDOM: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    NotVersion4,
    Parse(uuid::Error),
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotVersion4 => write!(f, "identifier must be a UUID version 4"),
            Self::Parse(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for IdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotVersion4 => None,
            Self::Parse(err) => Some(err),
        }
    }
}

fn validate_v4(uuid: Uuid) -> Result<Uuid, IdError> {
    if uuid.get_version_num() == UUID_VERSION_RANDOM {
        Ok(uuid)
    } else {
        Err(IdError::NotVersion4)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GateId(Uuid);

impl Default for ItemId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for GateId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl ItemId {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_uuid(uuid: Uuid) -> Result<Self, IdError> {
        Ok(Self(validate_v4(uuid)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl GateId {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_uuid(uuid: Uuid) -> Result<Self, IdError> {
        Ok(Self(validate_v4(uuid)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ItemId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s).map_err(IdError::Parse)?;
        Self::from_uuid(uuid)
    }
}

impl FromStr for GateId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s).map_err(IdError::Parse)?;
        Self::from_uuid(uuid)
    }
}
