//! Map keyboard digits 1–4 to MCQ choice indices 0..3.

const XK_1: u32 = 0x0031;
const XK_2: u32 = 0x0032;
const XK_3: u32 = 0x0033;
const XK_4: u32 = 0x0034;
const XK_KP_1: u32 = 0xffb1;
const XK_KP_2: u32 = 0xffb2;
const XK_KP_3: u32 = 0xffb3;
const XK_KP_4: u32 = 0xffb4;

pub fn choice_from_keysym(keysym: u32) -> Option<u8> {
    match keysym {
        XK_1 | XK_KP_1 => Some(0),
        XK_2 | XK_KP_2 => Some(1),
        XK_3 | XK_KP_3 => Some(2),
        XK_4 | XK_KP_4 => Some(3),
        _ => None,
    }
}

pub fn choice_index_from_name(name: &str) -> Option<u8> {
    match name {
        "1" | "KP_1" => Some(0),
        "2" | "KP_2" => Some(1),
        "3" | "KP_3" => Some(2),
        "4" | "KP_4" => Some(3),
        _ => None,
    }
}
