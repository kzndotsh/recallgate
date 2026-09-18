#![cfg(feature = "ui")]

use std::cell::RefCell;
use std::rc::Rc;

use glib::clone;
use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};
use gtk4_session_lock::Instance as SessionLockInstance;

use crate::ipc::ShowPrompt;
use crate::paths;

const WRONG_ANSWER_FLASH: std::time::Duration = std::time::Duration::from_millis(1500);

pub fn run_app(app: Application, prompt_rx: std::sync::mpsc::Receiver<ShowPrompt>) {
    let answered = Rc::new(RefCell::new(false));
    let pending: Rc<RefCell<Option<ShowPrompt>>> = Rc::new(RefCell::new(None));
    let session: Rc<RefCell<Option<SessionLockInstance>>> = Rc::new(RefCell::new(None));

    let pending_activate = pending.clone();
    let session_activate = session.clone();
    let answered_activate = answered.clone();
    app.connect_activate(move |app| {
        let lock = SessionLockInstance::new();
        let answered_monitor = answered_activate.clone();
        lock.connect_monitor(clone!(
            #[weak]
            app,
            #[strong]
            pending_activate,
            #[strong]
            answered_monitor,
            move |lock, monitor| {
                let Some(prompt) = pending_activate.borrow().clone() else {
                    return;
                };
                present_monitor(app, lock, monitor, prompt, answered_monitor.clone());
            }
        ));
        lock.connect_unlocked(clone!(
            #[weak]
            app,
            move |_| app.quit()
        ));
        lock.connect_failed(clone!(
            #[weak]
            app,
            move |_| {
                eprintln!("unsupported");
                app.quit();
            }
        ));
        *session_activate.borrow_mut() = Some(lock);
    });

    let pending_poll = pending.clone();
    let session_poll = session.clone();
    let answered_poll = answered.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        if let Ok(prompt) = prompt_rx.try_recv() {
            *answered_poll.borrow_mut() = false;
            *pending_poll.borrow_mut() = Some(prompt);
            if let Some(lock) = session_poll.borrow().as_ref() {
                lock.lock();
            }
        }
        glib::ControlFlow::Continue
    });

    app.run();
}

fn present_monitor(
    app: &Application,
    session: &SessionLockInstance,
    monitor: gtk4::gdk::Monitor,
    prompt: ShowPrompt,
    answered: Rc<RefCell<bool>>,
) {
    let window = ApplicationWindow::new(app);
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

    for (index, choice) in prompt.choices.iter().enumerate() {
        let button = Button::with_label(&format!("{}: {choice}", index + 1));
        let prompt = prompt.clone();
        let flash = flash.clone();
        let session = session.clone();
        let answered_click = answered.clone();
        button.connect_clicked(clone!(move |_| handle_choice(
            index as u8,
            &prompt,
            &flash,
            &session,
            &answered_click
        )));
        root.append(&button);
    }

    window.set_child(Some(&root));

    let controller = gtk4::EventControllerKey::new();
    let prompt_keys = prompt.clone();
    let flash_keys = flash.clone();
    let session_keys = session.clone();
    let answered_keys = answered.clone();
    controller.connect_key_pressed(clone!(move |_, _, keyval, _, _| {
        let name = gtk4::gdk::Key::from(keyval).name();
        let Some(name) = name.as_ref().map(|n| n.as_str()) else {
            return glib::Propagation::Proceed;
        };
        let Some(index) = crate::keys::choice_index_from_name(name) else {
            return glib::Propagation::Proceed;
        };
        handle_choice(index, &prompt_keys, &flash_keys, &session_keys, &answered_keys);
        glib::Propagation::Stop
    }));
    window.add_controller(controller);

    session.assign_window_to_monitor(&window, &monitor);
}

fn handle_choice(
    chosen: u8,
    prompt: &ShowPrompt,
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
