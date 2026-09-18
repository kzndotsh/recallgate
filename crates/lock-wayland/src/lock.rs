use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use glib::clone;
use gtk4::gdk::{Key, ModifierType};
use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};
use gtk4_session_lock::Instance as SessionLockInstance;

use crate::hatch::{HatchOutcome, HatchTracker};
use crate::ipc::{IncomingShow, ShowAck};
use crate::paths;

const WRONG_ANSWER_FLASH: std::time::Duration = std::time::Duration::from_millis(1500);

struct ChordKeys {
    ctrl: bool,
    shift: bool,
    escape: bool,
}

pub fn run_app(app: Application, prompt_rx: std::sync::mpsc::Receiver<IncomingShow>) {
    let answered = Rc::new(RefCell::new(false));
    let pending: Rc<RefCell<Option<IncomingShow>>> = Rc::new(RefCell::new(None));
    let session: Rc<RefCell<Option<SessionLockInstance>>> = Rc::new(RefCell::new(None));
    let locked = Rc::new(RefCell::new(false));
    let hatch = Rc::new(RefCell::new(HatchTracker::new()));
    let chord = Rc::new(RefCell::new(ChordKeys { ctrl: false, shift: false, escape: false }));

    let pending_activate = pending.clone();
    let session_activate = session.clone();
    let answered_activate = answered.clone();
    let hatch_activate = hatch.clone();
    let chord_activate = chord.clone();
    let locked_activate = locked.clone();
    app.connect_activate(move |app| {
        let lock = SessionLockInstance::new();
        let answered_monitor = answered_activate.clone();
        let hatch_monitor = hatch_activate.clone();
        let chord_monitor = chord_activate.clone();
        lock.connect_monitor(clone!(
            #[weak]
            app,
            #[strong]
            pending_activate,
            #[strong]
            answered_monitor,
            #[strong]
            hatch_monitor,
            #[strong]
            chord_monitor,
            move |lock, monitor| {
                let Some(incoming) = pending_activate.borrow().as_ref() else {
                    return;
                };
                let prompt = incoming.prompt.clone();
                present_monitor(
                    app,
                    lock,
                    monitor,
                    prompt,
                    answered_monitor.clone(),
                    hatch_monitor.clone(),
                    chord_monitor.clone(),
                );
            }
        ));
        lock.connect_unlocked(clone!(
            #[strong]
            locked_activate,
            move |_| {
                *locked_activate.borrow_mut() = false;
            }
        ));
        lock.connect_failed(|_| {
            eprintln!("unsupported");
        });
        *session_activate.borrow_mut() = Some(lock);
    });

    let pending_poll = pending.clone();
    let session_poll = session.clone();
    let answered_poll = answered.clone();
    let hatch_poll = hatch.clone();
    let locked_poll = locked.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        if let Ok(incoming) = prompt_rx.try_recv() {
            *answered_poll.borrow_mut() = false;
            *hatch_poll.borrow_mut() = HatchTracker::new();
            let already_locked = *locked_poll.borrow();
            *pending_poll.borrow_mut() = Some(incoming);
            if let Some(lock) = session_poll.borrow().as_ref() {
                if !already_locked {
                    lock.lock();
                    *locked_poll.borrow_mut() = true;
                }
                if let Some(pending_show) = pending_poll.borrow().as_ref() {
                    let _ = pending_show.ack.send(ShowAck::Ok);
                }
            } else if let Some(pending_show) = pending_poll.borrow().as_ref() {
                let _ = pending_show.ack.send(ShowAck::Unsupported);
            }
        }
        glib::ControlFlow::Continue
    });

    let hatch_tick = hatch.clone();
    let chord_tick = chord.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        let keys = chord_tick.borrow();
        let _ =
            hatch_tick.borrow_mut().on_chord(keys.ctrl, keys.shift, keys.escape, Instant::now());
        glib::ControlFlow::Continue
    });

    app.run();
}

fn finish_abort(session: Option<&SessionLockInstance>) {
    if let Err(err) = paths::abort_hatch() {
        eprintln!("abort failed: {err}");
        return;
    }
    if let Some(session) = session {
        session.unlock();
    }
}

fn present_monitor(
    app: &Application,
    session: &SessionLockInstance,
    monitor: gtk4::gdk::Monitor,
    prompt: crate::ipc::ShowPrompt,
    answered: Rc<RefCell<bool>>,
    hatch: Rc<RefCell<HatchTracker>>,
    chord: Rc<RefCell<ChordKeys>>,
) {
    let window = ApplicationWindow::new(app);
    window.connect_close_request(|_| glib::Propagation::Stop);

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let stem = Label::new(Some(&prompt.stem));
    stem.add_css_class("title-1");
    root.append(&stem);

    let flash = Label::new(None);
    flash.set_visible(false);
    root.append(&flash);

    let hatch_hint = Label::new(None);
    hatch_hint.set_visible(false);
    root.append(&hatch_hint);

    for (index, choice) in prompt.choices.iter().enumerate() {
        let button = Button::with_label(&format!("{}: {choice}", index + 1));
        let prompt = prompt.clone();
        let flash = flash.clone();
        let session = session.clone();
        let answered_click = answered.clone();
        let hatch_click = hatch.clone();
        button.connect_clicked(clone!(move |_| {
            if hatch_click.borrow().is_confirming() {
                return;
            }
            handle_choice(index as u8, &prompt, &flash, &session, &answered_click);
        }));
        root.append(&button);
    }

    window.set_child(Some(&root));

    let controller = gtk4::EventControllerKey::new();
    let prompt_keys = prompt.clone();
    let flash_keys = flash.clone();
    let session_keys = session.clone();
    let answered_keys = answered.clone();
    let hatch_keys = hatch.clone();
    let chord_keys = chord.clone();
    let hint_keys = hatch_hint.clone();
    controller.connect_key_pressed(clone!(move |_, key, _, state| {
        update_chord(&chord_keys, state, key, true);
        let outcome = hatch_keys.borrow_mut().on_chord(
            chord_keys.borrow().ctrl,
            chord_keys.borrow().shift,
            chord_keys.borrow().escape,
            Instant::now(),
        );
        if outcome == HatchOutcome::Confirming || hatch_keys.borrow().is_confirming() {
            hint_keys.set_text("Type ABORT then Enter");
            hint_keys.set_visible(true);
            if key == Key::Return || key == Key::KP_Enter {
                if hatch_keys.borrow_mut().on_text('\n') == HatchOutcome::Completed {
                    finish_abort(Some(&session_keys));
                }
                return glib::Propagation::Stop;
            }
            if let Some(ch) = key.to_unicode() {
                if hatch_keys.borrow_mut().on_text(ch) == HatchOutcome::Completed {
                    finish_abort(Some(&session_keys));
                }
            }
            return glib::Propagation::Stop;
        }
        let Some(name) = key.name() else {
            return glib::Propagation::Proceed;
        };
        let Some(index) = crate::keys::choice_index_from_name(name.as_str()) else {
            return glib::Propagation::Proceed;
        };
        handle_choice(index, &prompt_keys, &flash_keys, &session_keys, &answered_keys);
        glib::Propagation::Stop
    }));
    let chord_release = chord.clone();
    controller.connect_key_released(move |_, key, _, state| {
        update_chord(&chord_release, state, key, false);
    });
    window.add_controller(controller);

    session.assign_window_to_monitor(&window, &monitor);
}

fn update_chord(chord: &Rc<RefCell<ChordKeys>>, state: ModifierType, key: Key, pressed: bool) {
    let mut keys = chord.borrow_mut();
    keys.ctrl = state.contains(ModifierType::CONTROL_MASK);
    keys.shift = state.contains(ModifierType::SHIFT_MASK);
    if key == Key::Escape {
        keys.escape = pressed;
    }
}

fn handle_choice(
    chosen: u8,
    prompt: &crate::ipc::ShowPrompt,
    flash: &Label,
    session: &SessionLockInstance,
    answered: &Rc<RefCell<bool>>,
) {
    if *answered.borrow() {
        return;
    }
    *answered.borrow_mut() = true;

    let wrong = chosen != prompt.correct_index;
    if wrong {
        let Some(correct) = prompt.choices.get(prompt.correct_index as usize) else {
            *answered.borrow_mut() = false;
            return;
        };
        flash.set_text(&format!("Correct: {correct}"));
        flash.set_visible(true);
    }

    match paths::submit_choice(chosen) {
        Ok(_) if wrong => {
            let session = session.clone();
            glib::timeout_add_local(WRONG_ANSWER_FLASH, move || {
                session.unlock();
                glib::ControlFlow::Break
            });
        }
        Ok(_) => session.unlock(),
        Err(err) => {
            eprintln!("submit failed: {err}");
            *answered.borrow_mut() = false;
        }
    }
}
