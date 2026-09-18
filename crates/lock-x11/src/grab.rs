use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as RandrConnectionExt;
use x11rb::protocol::xproto::{
    ChangeGCAux, ChangeWindowAttributesAux, ConnectionExt as XprotoConnectionExt, CreateGCAux,
    CreateWindowAux, Cursor, EventMask, GrabMode, GrabStatus, Rectangle, Screen, Window,
    WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use xkeysym::{KeyCode, RawKeysym};

use crate::error::{keyboard_grab_error, GrabError};
use crate::hatch::{HatchOutcome, HatchTracker};
use crate::ipc::{IncomingShow, ShowAck, ShowPrompt};
use crate::keys;
use crate::paths;

const WRONG_ANSWER_FLASH: Duration = Duration::from_millis(1500);
const CARD_PAD: i16 = 32;
const ROW_GAP: i16 = 8;
const ROW_PAD_Y: i16 = 8;

struct MonitorRect {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

struct OutputSurface {
    window: Window,
    gc: u32,
    origin_x: i16,
    origin_y: i16,
    width: u16,
    height: u16,
    choice_rows: [Rectangle; 4],
}

struct CoreFont {
    id: u32,
    cell_w: i16,
    ascent: i16,
    height: i16,
}

struct Palette {
    bg: u32,
    card: u32,
    border: u32,
    row: u32,
    text: u32,
    muted: u32,
    accent: u32,
    warn: u32,
}

struct KeyboardMapping {
    min_keycode: u8,
    keysyms_per_keycode: u8,
    keysyms: Vec<RawKeysym>,
}

const XK_ESCAPE: u32 = 0xff1b;
const XK_RETURN: u32 = 0xff0d;
const XK_KP_ENTER: u32 = 0xff8d;
const X_SHIFT: u16 = 1;
const X_CONTROL: u16 = 4;

pub fn run(prompt_rx: Receiver<IncomingShow>) -> Result<(), GrabError> {
    let (conn, screen_num) =
        RustConnection::connect(None).map_err(|err| GrabError::Display(err.to_string()))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let font = open_lock_font(&conn)?;
    let palette = palette_for(screen);
    let keyboard = load_keyboard(&conn)?;
    let pointer = open_lock_cursor(&conn, root)?;
    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new()
            .event_mask(EventMask::KEY_PRESS | EventMask::KEY_RELEASE | EventMask::BUTTON_PRESS),
    )
    .map_err(map_conn_err)?;

    loop {
        let incoming = prompt_rx.recv().map_err(|err| GrabError::Display(err.to_string()))?;
        let prompt = incoming.prompt;
        let monitors = monitor_rects(&conn, root, screen.width_in_pixels, screen.height_in_pixels)?;
        let mut surfaces = create_surfaces(
            &conn,
            screen,
            root,
            font.id,
            palette.bg,
            palette.text,
            pointer,
            &monitors,
        )?;
        if let Err(err) = grab_root(&conn, root, pointer) {
            let _ = incoming.ack.send(if err == GrabError::GrabBusy {
                ShowAck::GrabBusy
            } else {
                ShowAck::Unsupported
            });
            destroy_surfaces(&conn, &surfaces)?;
            if err == GrabError::GrabBusy {
                return Err(err);
            }
            continue;
        }
        let _ = incoming.ack.send(ShowAck::Ok);
        for surface in &mut surfaces {
            draw_prompt(&conn, surface, &font, &palette, &prompt, None, false)?;
        }
        conn.flush().map_err(map_conn_err)?;
        drain_events(&conn)?;
        if let Some(surface) = surfaces.first() {
            conn.warp_pointer(
                x11rb::NONE,
                surface.window,
                0,
                0,
                0,
                0,
                (surface.width / 2) as i16,
                (surface.height / 2) as i16,
            )
            .map_err(map_conn_err)?;
        }
        conn.flush().map_err(map_conn_err)?;

        let mut answered = false;
        let mut unlock_at: Option<Instant> = None;
        let mut hatch = HatchTracker::new();
        let mut escape_held = false;
        let mut last_ctrl = false;
        let mut last_shift = false;
        let mut flash: Option<String> = None;
        let mut abort_prompt = false;

        loop {
            if let Some(deadline) = unlock_at {
                if Instant::now() >= deadline {
                    break;
                }
            }

            while let Some(event) = conn.poll_for_event().map_err(map_conn_err)? {
                match event {
                    Event::KeyPress(key_event) => {
                        let (ctrl, shift) = x_mods(u16::from(key_event.state));
                        last_ctrl = ctrl;
                        last_shift = shift;
                        let keysym = raw_keysym(&keyboard, key_event.detail);
                        if keysym == Some(XK_ESCAPE) {
                            escape_held = true;
                        }
                        let hatch_out = hatch.on_chord(ctrl, shift, escape_held, Instant::now());
                        if hatch_out == HatchOutcome::Confirming || hatch.is_confirming() {
                            if !abort_prompt {
                                abort_prompt = true;
                                for surface in &mut surfaces {
                                    draw_prompt(
                                        &conn,
                                        surface,
                                        &font,
                                        &palette,
                                        &prompt,
                                        flash.as_deref(),
                                        true,
                                    )?;
                                }
                                conn.flush().map_err(map_conn_err)?;
                            }
                            let text_out = match keysym {
                                Some(XK_RETURN | XK_KP_ENTER) => hatch.on_text('\n'),
                                Some(sym) => match char_from_keysym(sym) {
                                    Some(ch) => hatch.on_text(ch),
                                    None => HatchOutcome::Confirming,
                                },
                                None => HatchOutcome::Confirming,
                            };
                            if text_out == HatchOutcome::Completed {
                                if let Err(err) = paths::abort_hatch() {
                                    eprintln!("abort failed: {err}");
                                } else {
                                    answered = true;
                                    break;
                                }
                            }
                            continue;
                        }
                        if answered {
                            continue;
                        }
                        let Some(choice) = choice_from_keycode(&keyboard, key_event.detail) else {
                            continue;
                        };
                        answered = true;
                        let wrong = choice != prompt.correct_index;
                        if wrong {
                            flash = prompt
                                .choices
                                .get(prompt.correct_index as usize)
                                .map(|answer| format!("Correct: {answer}"));
                            for surface in &mut surfaces {
                                draw_prompt(
                                    &conn,
                                    surface,
                                    &font,
                                    &palette,
                                    &prompt,
                                    flash.as_deref(),
                                    abort_prompt,
                                )?;
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
                    Event::Expose(_) => {
                        for surface in &mut surfaces {
                            draw_prompt(
                                &conn,
                                surface,
                                &font,
                                &palette,
                                &prompt,
                                flash.as_deref(),
                                abort_prompt,
                            )?;
                        }
                        conn.flush().map_err(map_conn_err)?;
                    }
                    Event::ButtonPress(button) if button.detail == 1 && !answered && !abort_prompt => {
                        let Some(choice) = surfaces.iter().find_map(|surface| {
                            choice_at(surface, button.root_x, button.root_y)
                        }) else {
                            continue;
                        };
                        answered = true;
                        let wrong = choice != prompt.correct_index;
                        if wrong {
                            flash = prompt
                                .choices
                                .get(prompt.correct_index as usize)
                                .map(|answer| format!("Correct: {answer}"));
                            for surface in &mut surfaces {
                                draw_prompt(
                                    &conn,
                                    surface,
                                    &font,
                                    &palette,
                                    &prompt,
                                    flash.as_deref(),
                                    abort_prompt,
                                )?;
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
                    Event::KeyRelease(key_event) => {
                        let (ctrl, shift) = x_mods(u16::from(key_event.state));
                        last_ctrl = ctrl;
                        last_shift = shift;
                        if raw_keysym(&keyboard, key_event.detail) == Some(XK_ESCAPE) {
                            escape_held = false;
                        }
                        hatch.on_chord(ctrl, shift, escape_held, Instant::now());
                    }
                    _ => {}
                }
            }

            if escape_held {
                let _ = hatch.on_chord(last_ctrl, last_shift, true, Instant::now());
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
        drain_events(&conn)?;
        conn.flush().map_err(map_conn_err)?;
    }
}

fn drain_events(conn: &RustConnection) -> Result<(), GrabError> {
    while conn.poll_for_event().map_err(map_conn_err)?.is_some() {}
    Ok(())
}

fn x_mods(state: u16) -> (bool, bool) {
    (state & X_CONTROL != 0, state & X_SHIFT != 0)
}

fn raw_keysym(mapping: &KeyboardMapping, keycode: u8) -> Option<u32> {
    xkeysym::keysym(
        KeyCode::new(keycode.into()),
        0,
        KeyCode::new(mapping.min_keycode.into()),
        mapping.keysyms_per_keycode,
        &mapping.keysyms,
    )
    .map(u32::from)
}

fn char_from_keysym(keysym: u32) -> Option<char> {
    char::from_u32(keysym).filter(char::is_ascii_alphabetic)
}

fn map_conn_err<E: std::fmt::Display>(err: E) -> GrabError {
    GrabError::Display(err.to_string())
}

fn open_lock_font(conn: &RustConnection) -> Result<CoreFont, GrabError> {
    let mut names: Vec<Vec<u8>> = [
        &b"10x20"[..],
        b"-misc-fixed-medium-r-normal--20-200-75-75-c-100-iso8859-1",
        b"9x15",
        b"-misc-fixed-medium-r-normal--15-140-75-75-c-90-iso8859-1",
        b"fixed",
        b"6x13",
        b"-misc-fixed-medium-r-normal--13-120-75-75-c-80-iso8859-1",
    ]
    .into_iter()
    .map(Vec::from)
    .collect();
    if let Ok(cookie) = conn.list_fonts(64, b"*-iso8859-1") {
        if let Ok(listed) = cookie.reply() {
            for font_name in listed.names {
                names.push(font_name.name);
            }
        }
    }
    for name in names {
        if name == b"cursor" {
            continue;
        }
        let font = conn.generate_id().map_err(map_conn_err)?;
        conn.open_font(font, &name).map_err(map_conn_err)?;
        match conn.query_font(font).map_err(map_conn_err)?.reply() {
            Ok(info) if info.font_ascent > 0 && info.max_bounds.character_width > 0 => {
                return Ok(CoreFont {
                    id: font,
                    cell_w: info.max_bounds.character_width.max(6),
                    ascent: info.font_ascent,
                    height: (info.font_ascent + info.font_descent).max(12),
                });
            }
            _ => {
                let _ = conn.close_font(font);
            }
        }
    }
    Err(GrabError::Display("no core bitmap font available".into()))
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

fn palette_for(screen: &Screen) -> Palette {
    if screen.root_depth >= 24 {
        Palette {
            bg: rgb(0x0d, 0x11, 0x17),
            card: rgb(0x16, 0x1b, 0x22),
            border: rgb(0x30, 0x36, 0x3d),
            row: rgb(0x21, 0x27, 0x30),
            text: rgb(0xe6, 0xed, 0xf3),
            muted: rgb(0x8b, 0x94, 0x9e),
            accent: rgb(0x58, 0xa6, 0xff),
            warn: rgb(0xf8, 0x51, 0x49),
        }
    } else {
        Palette {
            bg: screen.black_pixel,
            card: screen.black_pixel,
            border: screen.white_pixel,
            row: screen.black_pixel,
            text: screen.white_pixel,
            muted: screen.white_pixel,
            accent: screen.white_pixel,
            warn: screen.white_pixel,
        }
    }
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
        keysyms: reply.keysyms.to_vec(),
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
            x: monitor.x,
            y: monitor.y,
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
    cursor: Cursor,
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
                .event_mask(
                    EventMask::KEY_PRESS
                        | EventMask::KEY_RELEASE
                        | EventMask::EXPOSURE
                        | EventMask::BUTTON_PRESS,
                )
                .override_redirect(1)
                .cursor(cursor),
        )
        .map_err(map_conn_err)?;
        conn.map_window(window).map_err(map_conn_err)?;
        conn.create_gc(
            gc,
            window,
            &CreateGCAux::new().foreground(white).background(black).font(font),
        )
        .map_err(map_conn_err)?;
        surfaces.push(OutputSurface {
            window,
            gc,
            origin_x: monitor.x,
            origin_y: monitor.y,
            width: monitor.width,
            height: monitor.height,
            choice_rows: [Rectangle { x: 0, y: 0, width: 0, height: 0 }; 4],
        });
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

fn grab_root(conn: &RustConnection, root: Window, cursor: Cursor) -> Result<(), GrabError> {
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
            EventMask::BUTTON_PRESS,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            root,
            cursor,
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

fn open_lock_cursor(conn: &RustConnection, root: Window) -> Result<Cursor, GrabError> {
    const SIZE: u16 = 16;
    let source = conn.generate_id().map_err(map_conn_err)?;
    let mask = conn.generate_id().map_err(map_conn_err)?;
    let source_gc = conn.generate_id().map_err(map_conn_err)?;
    let mask_gc = conn.generate_id().map_err(map_conn_err)?;
    let cursor = conn.generate_id().map_err(map_conn_err)?;
    conn.create_pixmap(1, source, root, SIZE, SIZE).map_err(map_conn_err)?;
    conn.create_pixmap(1, mask, root, SIZE, SIZE).map_err(map_conn_err)?;
    conn.create_gc(source_gc, source, &CreateGCAux::new().foreground(0).background(0))
        .map_err(map_conn_err)?;
    conn.create_gc(mask_gc, mask, &CreateGCAux::new().foreground(0).background(0))
        .map_err(map_conn_err)?;
    let clear = Rectangle { x: 0, y: 0, width: SIZE, height: SIZE };
    conn.poly_fill_rectangle(source, source_gc, &[clear]).map_err(map_conn_err)?;
    conn.poly_fill_rectangle(mask, mask_gc, &[clear]).map_err(map_conn_err)?;
    conn.change_gc(source_gc, &ChangeGCAux::new().foreground(1))
        .map_err(map_conn_err)?;
    conn.change_gc(mask_gc, &ChangeGCAux::new().foreground(1))
        .map_err(map_conn_err)?;
    conn.poly_fill_rectangle(
        source,
        source_gc,
        &[Rectangle { x: 3, y: 3, width: 10, height: 10 }],
    )
    .map_err(map_conn_err)?;
    conn.poly_fill_rectangle(
        mask,
        mask_gc,
        &[Rectangle { x: 2, y: 2, width: 12, height: 12 }],
    )
    .map_err(map_conn_err)?;
    conn.create_cursor(
        cursor, source, mask, 0xffff, 0xffff, 0xffff, 0, 0, 0, 8, 8,
    )
    .map_err(map_conn_err)?;
    conn.free_gc(source_gc).map_err(map_conn_err)?;
    conn.free_gc(mask_gc).map_err(map_conn_err)?;
    conn.free_pixmap(source).map_err(map_conn_err)?;
    conn.free_pixmap(mask).map_err(map_conn_err)?;
    Ok(cursor)
}

fn release_grab(conn: &RustConnection) -> Result<(), GrabError> {
    let time = x11rb::CURRENT_TIME;
    conn.ungrab_keyboard(time).map_err(map_conn_err)?;
    conn.ungrab_pointer(time).map_err(map_conn_err)?;
    Ok(())
}

fn fill_rect(
    conn: &RustConnection,
    surface: &OutputSurface,
    color: u32,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
) -> Result<(), GrabError> {
    conn.change_gc(surface.gc, &ChangeGCAux::new().foreground(color).line_width(1)).map_err(map_conn_err)?;
    conn.poly_fill_rectangle(
        surface.window,
        surface.gc,
        &[Rectangle { x, y, width, height }],
    )
    .map_err(map_conn_err)?;
    Ok(())
}

fn stroke_rect(
    conn: &RustConnection,
    surface: &OutputSurface,
    color: u32,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
) -> Result<(), GrabError> {
    conn.change_gc(surface.gc, &ChangeGCAux::new().foreground(color).line_width(2))
        .map_err(map_conn_err)?;
    conn.poly_rectangle(
        surface.window,
        surface.gc,
        &[Rectangle { x, y, width, height }],
    )
    .map_err(map_conn_err)?;
    Ok(())
}

fn draw_label(
    conn: &RustConnection,
    surface: &OutputSurface,
    run: LabelRun<'_>,
) -> Result<(), GrabError> {
    conn.change_gc(
        surface.gc,
        &ChangeGCAux::new().foreground(run.ink).background(run.paper).font(run.font.id),
    )
    .map_err(map_conn_err)?;
    let bytes: Vec<u8> = run.text.chars().filter(|ch| ch.is_ascii()).map(|ch| ch as u8).collect();
    if bytes.is_empty() {
        return Ok(());
    }
    conn.image_text8(surface.window, surface.gc, run.x, run.y, &bytes).map_err(map_conn_err)?;
    Ok(())
}

struct LabelRun<'a> {
    font: &'a CoreFont,
    ink: u32,
    paper: u32,
    x: i16,
    y: i16,
    text: &'a str,
}

fn text_width(font: &CoreFont, text: &str) -> i16 {
    i16::try_from(text.chars().filter(|ch| ch.is_ascii()).count()).unwrap_or(i16::MAX) * font.cell_w
}

fn draw_prompt(
    conn: &RustConnection,
    surface: &mut OutputSurface,
    font: &CoreFont,
    palette: &Palette,
    prompt: &ShowPrompt,
    flash: Option<&str>,
    abort_prompt: bool,
) -> Result<(), GrabError> {
    fill_rect(conn, surface, palette.bg, 0, 0, surface.width, surface.height)?;

    let row_h = font.height + ROW_PAD_Y * 2;
    let mut inner_w = text_width(font, &prompt.stem);
    for (index, choice) in prompt.choices.iter().enumerate() {
        inner_w = inner_w.max(text_width(font, &format!("{}  {choice}", index + 1)));
    }
    if let Some(flash) = flash {
        inner_w = inner_w.max(text_width(font, flash));
    }
    if abort_prompt {
        inner_w = inner_w.max(text_width(font, "Type ABORT then Enter"));
    }

    let wanted_w = (inner_w + CARD_PAD * 2).max(320);
    let max_w = (i32::from(surface.width) - 48).max(320);
    let card_w = u16::try_from(i32::from(wanted_w).min(max_w)).unwrap_or(surface.width);
    let extra = u16::from(flash.is_some()) + u16::from(abort_prompt);
    let rows = 5 + extra;
    let wanted_h = CARD_PAD * 2 + i16::from(rows as u8) * row_h + i16::from(rows.saturating_sub(1) as u8) * ROW_GAP;
    let max_h = (i32::from(surface.height) - 48).max(i32::from(wanted_h));
    let card_h = u16::try_from(i32::from(wanted_h).min(max_h)).unwrap_or(surface.height);
    let card_x = ((surface.width.saturating_sub(card_w)) / 2) as i16;
    let card_y = ((surface.height.saturating_sub(card_h)) / 2) as i16;

    fill_rect(conn, surface, palette.card, card_x, card_y, card_w, card_h)?;
    stroke_rect(conn, surface, palette.border, card_x, card_y, card_w, card_h)?;

    let mut y = card_y + CARD_PAD;
    let text_x = card_x + CARD_PAD;
    draw_label(
        conn,
        surface,
        LabelRun {
            font,
            ink: palette.text,
            paper: palette.card,
            x: text_x,
            y: y + font.ascent,
            text: &prompt.stem,
        },
    )?;
    y += row_h + ROW_GAP;

    for (index, choice) in prompt.choices.iter().enumerate() {
        let row = Rectangle {
            x: text_x - 8,
            y,
            width: card_w.saturating_sub((CARD_PAD as u16).saturating_mul(2).saturating_sub(16)),
            height: row_h as u16,
        };
        surface.choice_rows[index] = row;
        fill_rect(conn, surface, palette.row, row.x, row.y, row.width, row.height)?;
        let key = format!("{}", index + 1);
        draw_label(
            conn,
            surface,
            LabelRun {
                font,
                ink: palette.accent,
                paper: palette.row,
                x: text_x,
                y: y + font.ascent + ROW_PAD_Y,
                text: &key,
            },
        )?;
        draw_label(
            conn,
            surface,
            LabelRun {
                font,
                ink: palette.text,
                paper: palette.row,
                x: text_x + font.cell_w * 3,
                y: y + font.ascent + ROW_PAD_Y,
                text: choice,
            },
        )?;
        y += row_h + ROW_GAP;
    }

    if let Some(flash) = flash {
        draw_label(
            conn,
            surface,
            LabelRun {
                font,
                ink: palette.warn,
                paper: palette.card,
                x: text_x,
                y: y + font.ascent,
                text: flash,
            },
        )?;
        y += row_h + ROW_GAP;
    }
    if abort_prompt {
        draw_label(
            conn,
            surface,
            LabelRun {
                font,
                ink: palette.muted,
                paper: palette.card,
                x: text_x,
                y: y + font.ascent,
                text: "Type ABORT then Enter",
            },
        )?;
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

fn choice_at(surface: &OutputSurface, root_x: i16, root_y: i16) -> Option<u8> {
    let x = root_x - surface.origin_x;
    let y = root_y - surface.origin_y;
    surface.choice_rows.iter().enumerate().find_map(|(index, row)| {
        let right = row.x + row.width as i16;
        let bottom = row.y + row.height as i16;
        (x >= row.x && y >= row.y && x < right && y < bottom).then_some(index as u8)
    })
}

fn choice_from_keycode(mapping: &KeyboardMapping, keycode: u8) -> Option<u8> {
    let keysym = xkeysym::keysym(
        KeyCode::new(keycode.into()),
        0,
        KeyCode::new(mapping.min_keycode.into()),
        mapping.keysyms_per_keycode,
        &mapping.keysyms,
    )?;
    let raw = u32::from(keysym);
    if let Some(index) = keys::choice_from_keysym(raw) {
        return Some(index);
    }
    keys::choice_index_from_name(keysym.name()?)
}
