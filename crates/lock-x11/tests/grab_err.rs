use recallgate_lock_x11::error::{keyboard_grab_error, GrabError, GrabStatusCode};

#[test]
fn grab_err_maps_busy() {
    assert_eq!(keyboard_grab_error(GrabStatusCode::AlreadyGrabbed as u8), GrabError::GrabBusy);
}

#[test]
fn grab_err_maps_other_status() {
    assert_eq!(
        keyboard_grab_error(GrabStatusCode::Frozen as u8),
        GrabError::GrabFailed(GrabStatusCode::Frozen as u8)
    );
}
