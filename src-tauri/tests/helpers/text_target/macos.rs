//! NSWindow with an NSTextView, driven by a manual event loop so stdin
//! commands can be handled on the main thread.

use crate::{ready, reply, spawn_stdin, Cmd};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEventMask, NSTextView,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, NSString};

fn focus(app: &NSApplication, window: &NSWindow, view: &Retained<NSTextView>) {
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    window.makeKeyAndOrderFront(None);
    window.makeFirstResponder(Some(view));
}

pub fn run() {
    let Some(mtm) = MainThreadMarker::new() else {
        std::process::exit(2);
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    app.finishLaunching();

    let frame = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(480.0, 240.0));
    // SAFETY: standard NSWindow initialiser with valid arguments.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            frame,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: the window is owned by this Retained, not released on close.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str("LocalYapper text target"));
    let view = NSTextView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(480.0, 240.0)),
    );
    window.setContentView(Some(&view));
    focus(&app, &window, &view);

    let (tx, rx) = std::sync::mpsc::channel::<Cmd>();
    spawn_stdin(move |c| {
        let _ = tx.send(c);
    });
    let mut announced = false;
    let started = std::time::Instant::now();
    loop {
        // SAFETY: NSDefaultRunLoopMode is an immutable framework constant.
        let mode = unsafe { NSDefaultRunLoopMode };
        let until = NSDate::dateWithTimeIntervalSinceNow(0.01);
        if let Some(ev) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
            NSEventMask::Any,
            Some(&until),
            mode,
            true,
        ) {
            app.sendEvent(&ev);
        }
        if !announced && started.elapsed().as_millis() > 300 {
            focus(&app, &window, &view);
            ready();
            announced = true;
        }
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Cmd::Get => reply(&view.string().to_string()),
                Cmd::Focus => {
                    focus(&app, &window, &view);
                    reply("");
                }
                Cmd::Clear => {
                    view.setString(&NSString::from_str(""));
                    reply("");
                }
                Cmd::Quit => return,
            }
        }
    }
}
