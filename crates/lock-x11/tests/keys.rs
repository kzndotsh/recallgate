use recallgate_lock_x11::keys::choice_index_from_name;

#[test]
fn maps_digit_keys_one_through_four() {
    assert_eq!(choice_index_from_name("1"), Some(0));
    assert_eq!(choice_index_from_name("4"), Some(3));
}

#[test]
fn maps_keypad_digits() {
    assert_eq!(choice_index_from_name("KP_2"), Some(1));
}

#[test]
fn ignores_non_choice_keys() {
    assert_eq!(choice_index_from_name("Escape"), None);
}
