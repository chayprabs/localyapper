//! GTK 3 TextView.

use crate::{clicks, ready, reply, spawn_stdin, Cmd, CLICKS};
use gtk::prelude::*;

pub fn run() {
    if gtk::init().is_err() {
        eprintln!("text_target: no display");
        std::process::exit(2);
    }
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("LocalYapper text target");
    window.set_default_size(480, 240);
    window.move_(200, 200);
    let scroll = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    let view = gtk::TextView::new();
    scroll.add(&view);
    window.add(&scroll);
    view.connect_button_press_event(|_, ev| {
        if ev.button() == 1 {
            CLICKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        gtk::glib::Propagation::Proceed
    });
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        gtk::glib::Propagation::Stop
    });
    window.show_all();
    window.present();
    view.grab_focus();

    let (tx, rx) = std::sync::mpsc::channel::<Cmd>();
    spawn_stdin(move |c| {
        let _ = tx.send(c);
    });
    let (w, v) = (window.clone(), view.clone());
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Cmd::Get => {
                    let text = v
                        .buffer()
                        .and_then(|b| {
                            let (s, e) = b.bounds();
                            b.text(&s, &e, false)
                        })
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    reply(&text);
                }
                Cmd::Focus => {
                    w.present();
                    v.grab_focus();
                    reply("");
                }
                Cmd::Clear => {
                    if let Some(buf) = v.buffer() {
                        buf.set_text("");
                    }
                    reply("");
                }
                Cmd::Cover => {
                    let monitor = gtk::gdk::Display::default()
                        .and_then(|d| d.primary_monitor().or_else(|| d.monitor(0)));
                    if let Some(m) = monitor {
                        let g = m.geometry();
                        w.move_(g.x(), g.y());
                        w.resize(g.width(), g.height());
                    }
                    reply("");
                }
                Cmd::Clicks => reply(&clicks()),
                Cmd::Quit => {
                    gtk::main_quit();
                    return gtk::glib::ControlFlow::Break;
                }
            }
        }
        gtk::glib::ControlFlow::Continue
    });
    // Report ready once the window has been mapped and focused.
    let ready_view = view.clone();
    gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
        ready_view.grab_focus();
        ready();
    });
    gtk::main();
}
