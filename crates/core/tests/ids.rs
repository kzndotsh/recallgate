use std::str::FromStr;

use recallgate_core::{GateId, IdError, ItemId};

#[test]
fn from_str_accepts_uuid_v4() {
    let id = ItemId::new();
    let parsed = ItemId::from_str(&id.to_string()).expect("parse");
    assert_eq!(parsed, id);
}

#[test]
fn from_str_rejects_non_v4() {
    // UUID v3 (MD5)
    let v3 = "6fa459ea-ee8a-3ca2-894e-db77e160355e";
    assert_eq!(ItemId::from_str(v3), Err(IdError::NotVersion4));
    assert_eq!(GateId::from_str(v3), Err(IdError::NotVersion4));
}
