#![cfg(feature = "ui")]

use std::cell::RefCell;
use std::rc::Rc;

use glib::clone;
use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};
use gtk4_session_lock::Instance as SessionLockInstance;

use crate::ipc::ShowPrompt;
use crate::paths;

pub fn run_app(app: Application, prompt_rx: std::sync::mpsc::Receiver<ShowPrompt>) {
    let pending: Rc<RefCell<Option<ShowPrompt>>> = Rc::new(RefCell::new(None));
    let session: Rc<RefCell<Option<SessionLockInstance>>> = Rc::new(RefCell::new(None));

    let pending_activate = pending.clone();
    let session_activate = session.clone();
    app.connect_activate(move |app| {
        let lock = SessionLockInstance::new();
        lock.connect_monitor(clone!(
            #[weak]
            app,
            #[strong]
            pending_activate,
            move |lock, monitor| {
                let Some(prompt) = pending_activate.borrow().clone() else {
                    return;
                };
                present_monitor(app, lock, monitor, prompt);
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
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        if let Ok(prompt) = prompt_rx.try_recv() {
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
        button.connect_clicked(clone!(move |_| handle_choice(
            index as u8,
            &prompt,
            &flash,
            &session
        )));
        root.append(&button);
    }

    window.set_child(Some(&root));

    let controller = gtk4::EventControllerKey::new();
    let prompt_keys = prompt.clone();
    let flash_keys = flash.clone();
    let session_keys = session.clone();
    controller.connect_key_pressed(clone!(move |_, _, keyval, _, _| {
        let name = gtk4::gdk::Key::from(keyval).name();
        let Some(name) = name.as_ref().map(|n| n.as_str()) else {
            return glib::Propagation::Proceed;
        };
        let Some(index) = crate::keys::choice_index_from_name(name) else {
            return glib::Propagation::Proceed;
        };
        handle_choice(index, &prompt_keys, &flash_keys, &session_keys);
        glib::Propagation::Stop
    }));
    window.add_controller(controller);

    session.assign_window_to_monitor(&window, &monitor);
}

fn handle_choice(chosen: u8, prompt: &ShowPrompt, flash: &Label, session: &SessionLockInstance) {
    if chosen != prompt.correct_index {
        let correct = &prompt.choices[prompt.correct_index as usize];
        flash.set_text(&format!("Correct: {correct}"));
        flash.set_visible(true);
    }
    match paths::submit_choice(chosen) {
        Ok(_) => session.unlock(),
        Err(err) => eprintln!("submit failed: {err}"),
    }
}
