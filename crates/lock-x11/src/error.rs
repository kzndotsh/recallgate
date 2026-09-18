use std::fmt;

/// Result of an X11 keyboard or pointer grab attempt (Xlib status codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabStatusCode {
    Success = 0,
    AlreadyGrabbed = 1,
    InvalidTime = 2,
    NotViewable = 3,
    Frozen = 4,
}

impl GrabStatusCode {
    pub fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Success),
            1 => Some(Self::AlreadyGrabbed),
            2 => Some(Self::InvalidTime),
            3 => Some(Self::NotViewable),
            4 => Some(Self::Frozen),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrabError {
    GrabBusy,
    GrabFailed(u8),
    Display(String),
}

impl fmt::Display for GrabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GrabBusy => write!(f, "grab_busy"),
            Self::GrabFailed(code) => write!(f, "grab failed with status {code}"),
            Self::Display(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn keyboard_grab_error(status: u8) -> GrabError {
    match GrabStatusCode::from_raw(status) {
        Some(GrabStatusCode::Success) => {
            GrabError::Display("unexpected success in error mapper".into())
        }
        Some(GrabStatusCode::AlreadyGrabbed) => GrabError::GrabBusy,
        Some(GrabStatusCode::InvalidTime)
        | Some(GrabStatusCode::NotViewable)
        | Some(GrabStatusCode::Frozen) => GrabError::GrabFailed(status),
        None => GrabError::GrabFailed(status),
    }
}
