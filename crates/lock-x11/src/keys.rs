//! Map keyboard digits 1–4 to MCQ choice indices 0..3.

pub fn choice_index_from_name(name: &str) -> Option<u8> {
    match name {
        "1" | "KP_1" => Some(0),
        "2" | "KP_2" => Some(1),
        "3" | "KP_3" => Some(2),
        "4" | "KP_4" => Some(3),
        _ => None,
    }
}
