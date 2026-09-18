#![cfg(feature = "x11")]

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as RandrConnectionExt;
use x11rb::protocol::xproto::{
    ChangeGCAux, ConnectionExt as XprotoConnectionExt, CreateGCAux, CreateWindowAux, EventMask,
    GrabMode, GrabStatus, Rectangle, Screen, Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use xkeysym::{KeyCode, RawKeysym};

use crate::error::{keyboard_grab_error, GrabError};
use crate::ipc::ShowPrompt;
use crate::keys;
use crate::paths;

const WRONG_ANSWER_FLASH: Duration = Duration::from_millis(1500);
const LINE_HEIGHT: i16 = 18;

struct MonitorRect {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

struct OutputSurface {
    window: Window,
    gc: u32,
    width: u16,
    height: u16,
}

struct KeyboardMapping {
    min_keycode: u8,
    keysyms_per_keycode: u8,
    keysyms: Vec<RawKeysym>,
}

pub fn run(prompt_rx: Receiver<ShowPrompt>) -> Result<(), GrabError> {
    let (conn, screen_num) =
        RustConnection::connect(None).map_err(|err| GrabError::Display(err.to_string()))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let black = screen.black_pixel;
    let white = screen.white_pixel;

    let font = conn.generate_id().map_err(map_conn_err)?;
    conn.open_font(font, b"9x15").map_err(map_conn_err)?;
    let keyboard = load_keyboard(&conn)?;

    loop {
        let prompt = prompt_rx.recv().map_err(|err| GrabError::Display(err.to_string()))?;
        let monitors = monitor_rects(&conn, root, screen.width_in_pixels, screen.height_in_pixels)?;
        let surfaces = create_surfaces(&conn, screen, root, font, black, white, &monitors)?;
        grab_root(&conn, root)?;
        for surface in &surfaces {
            draw_prompt(&conn, surface, white, &prompt, None)?;
        }
        conn.flush().map_err(map_conn_err)?;

        let mut answered = false;
        let mut unlock_at: Option<Instant> = None;

        loop {
            if let Some(deadline) = unlock_at {
                if Instant::now() >= deadline {
                    break;
                }
            }

            while let Some(event) = conn.poll_for_event().map_err(map_conn_err)? {
                if let Event::KeyPress(key_event) = event {
                    if answered {
                        continue;
                    }
                    let Some(choice) = choice_from_keycode(&keyboard, key_event.detail) else {
                        continue;
                    };
                    answered = true;
                    let wrong = choice != prompt.correct_index;
                    if wrong {
                        let flash = prompt
                            .choices
                            .get(prompt.correct_index as usize)
                            .map(|answer| format!("Correct: {answer}"));
                        for surface in &surfaces {
                            draw_prompt(&conn, surface, white, &prompt, flash.as_deref())?;
                        }
                        conn.flush().map_err(map_conn_err)?;
                    }
                    match paths::submit_choice(choice) {
                        Ok(_) if wrong => {
                            unlock_at = Some(Instant::now() + WRONG_ANSWER_FLASH);
                        }
                        Ok(_) => break,
                        Err(err) => {
                            eprintln!("submit failed: {err}");
                            answered = false;
                        }
                    }
                }
            }

            if !answered {
                std::thread::sleep(Duration::from_millis(16));
            } else if unlock_at.is_none() {
                break;
            } else {
                std::thread::sleep(Duration::from_millis(16));
            }
        }

        release_grab(&conn)?;
        destroy_surfaces(&conn, &surfaces)?;
        conn.flush().map_err(map_conn_err)?;
    }
}

fn map_conn_err<E: std::fmt::Display>(err: E) -> GrabError {
    GrabError::Display(err.to_string())
}

fn load_keyboard(conn: &RustConnection) -> Result<KeyboardMapping, GrabError> {
    let setup = conn.setup();
    let count = setup.max_keycode - setup.min_keycode + 1;
    let reply = conn
        .get_keyboard_mapping(setup.min_keycode, count)
        .map_err(map_conn_err)?
        .reply()
        .map_err(map_conn_err)?;
    Ok(KeyboardMapping {
        min_keycode: setup.min_keycode,
        keysyms_per_keycode: reply.keysyms_per_keycode,
        keysyms: reply.keysyms.iter().copied().collect(),
    })
}

fn monitor_rects(
    conn: &RustConnection,
    root: Window,
    fallback_width: u16,
    fallback_height: u16,
) -> Result<Vec<MonitorRect>, GrabError> {
    let fallback = vec![MonitorRect { x: 0, y: 0, width: fallback_width, height: fallback_height }];
    let Ok(cookie) = conn.randr_get_monitors(root, true) else {
        return Ok(fallback);
    };
    let Ok(reply) = cookie.reply() else {
        return Ok(fallback);
    };
    if reply.monitors.is_empty() {
        return Ok(fallback);
    }
    Ok(reply
        .monitors
        .into_iter()
        .map(|monitor| MonitorRect {
            x: monitor.x as i16,
            y: monitor.y as i16,
            width: monitor.width,
            height: monitor.height,
        })
        .collect())
}

fn create_surfaces(
    conn: &RustConnection,
    screen: &Screen,
    root: Window,
    font: u32,
    black: u32,
    white: u32,
    monitors: &[MonitorRect],
) -> Result<Vec<OutputSurface>, GrabError> {
    let mut surfaces = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let window = conn.generate_id().map_err(map_conn_err)?;
        let gc = conn.generate_id().map_err(map_conn_err)?;
        conn.create_window(
            screen.root_depth,
            window,
            root,
            monitor.x,
            monitor.y,
            monitor.width,
            monitor.height,
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
            &CreateWindowAux::new()
                .background_pixel(black)
                .event_mask(EventMask::KEY_PRESS)
                .override_redirect(1),
        )
        .map_err(map_conn_err)?;
        conn.map_window(window).map_err(map_conn_err)?;
        conn.create_gc(
            gc,
            window,
            &CreateGCAux::new().foreground(white).background(black).font(font),
        )
        .map_err(map_conn_err)?;
        surfaces.push(OutputSurface { window, gc, width: monitor.width, height: monitor.height });
    }
    Ok(surfaces)
}

fn destroy_surfaces(conn: &RustConnection, surfaces: &[OutputSurface]) -> Result<(), GrabError> {
    for surface in surfaces {
        conn.destroy_window(surface.window).map_err(map_conn_err)?;
        conn.free_gc(surface.gc).map_err(map_conn_err)?;
    }
    Ok(())
}

fn grab_root(conn: &RustConnection, root: Window) -> Result<(), GrabError> {
    let time = x11rb::CURRENT_TIME;
    let keyboard = conn
        .grab_keyboard(false, root, time, GrabMode::ASYNC, GrabMode::ASYNC)
        .map_err(map_conn_err)?
        .reply()
        .map_err(map_conn_err)?;
    if keyboard.status != GrabStatus::SUCCESS {
        return Err(keyboard_grab_error(grab_status_code(keyboard.status)));
    }
    let pointer = conn
        .grab_pointer(
            false,
            root,
            EventMask::NO_EVENT,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            root,
            x11rb::NONE,
            time,
        )
        .map_err(map_conn_err)?
        .reply()
        .map_err(map_conn_err)?;
    if pointer.status != GrabStatus::SUCCESS {
        let _ = conn.ungrab_keyboard(time);
        let _ = conn.ungrab_pointer(time);
        return Err(keyboard_grab_error(grab_status_code(pointer.status)));
    }
    Ok(())
}

fn release_grab(conn: &RustConnection) -> Result<(), GrabError> {
    let time = x11rb::CURRENT_TIME;
    conn.ungrab_keyboard(time).map_err(map_conn_err)?;
    conn.ungrab_pointer(time).map_err(map_conn_err)?;
    Ok(())
}

fn draw_prompt(
    conn: &RustConnection,
    surface: &OutputSurface,
    fg: u32,
    prompt: &ShowPrompt,
    flash: Option<&str>,
) -> Result<(), GrabError> {
    conn.change_gc(surface.gc, &ChangeGCAux::new().foreground(fg)).map_err(map_conn_err)?;
    conn.poly_fill_rectangle(
        surface.window,
        surface.gc,
        &[Rectangle { x: 0, y: 0, width: surface.width, height: surface.height }],
    )
    .map_err(map_conn_err)?;

    let mut lines = vec![prompt.stem.clone()];
    for (index, choice) in prompt.choices.iter().enumerate() {
        lines.push(format!("{}: {choice}", index + 1));
    }
    if let Some(flash) = flash {
        lines.push(flash.to_string());
    }

    let total_height = lines.len() as i16 * LINE_HEIGHT;
    let mut y = ((surface.height as i16 - total_height) / 2).max(LINE_HEIGHT);
    for line in lines {
        let x = ((surface.width as i16 - line.len() as i16 * 7) / 2).max(8);
        conn.image_text8(surface.window, surface.gc, x, y, line.as_bytes())
            .map_err(map_conn_err)?;
        y += LINE_HEIGHT;
    }
    Ok(())
}

fn grab_status_code(status: GrabStatus) -> u8 {
    match status {
        GrabStatus::SUCCESS => 0,
        GrabStatus::ALREADY_GRABBED => 1,
        GrabStatus::INVALID_TIME => 2,
        GrabStatus::NOT_VIEWABLE => 3,
        GrabStatus::FROZEN => 4,
        _ => 255,
    }
}

fn choice_from_keycode(mapping: &KeyboardMapping, keycode: u8) -> Option<u8> {
    let keysym = xkeysym::keysym(
        KeyCode::new(keycode.into()),
        0,
        KeyCode::new(mapping.min_keycode.into()),
        mapping.keysyms_per_keycode,
        &mapping.keysyms,
    )?;
    let name = keysym.name()?.to_string();
    keys::choice_index_from_name(&name)
}
