//! macOS: files opened from Finder (double-click, "Open With", dropping on the Dock icon).
//!
//! macOS does not pass such files on the command line: it launches the app (or uses the running
//! one) and sends it an "open documents" event, which AppKit hands to the application delegate's
//! `application:openURLs:`. winit never installs a delegate (it leaves that to the app, see
//! `winit::platform::macos`), so GenHa installs a minimal one that only implements this method and
//! queues the paths; [`take_opened`] collects them on the next frame. The delegate is set after
//! the event loop is created (winit makes `NSApplication` then) and before it runs, so files that
//! launched the app arrive too (AppKit sends them before `applicationDidFinishLaunching:`).
//!
//! The only unsafe code in the app: declaring an Objective-C class requires it (the macro cannot
//! check that the method signatures match AppKit's), and the signature below is AppKit's.
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationDelegate};
use objc2_foundation::{NSArray, NSURL};

/// Paths macOS asked GenHa to open, not yet opened.
static OPENED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// The UI's context, to wake it when a file arrives while it is idle.
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

define_class!(
    // SAFETY: NSObject has no subclassing requirements and `OpenDelegate` does not implement Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "GenHaOpenDelegate"]
    struct OpenDelegate;

    // SAFETY: NSObjectProtocol has no extra requirements.
    unsafe impl NSObjectProtocol for OpenDelegate {}

    // SAFETY: the method's selector and signature are those of NSApplicationDelegate.
    unsafe impl NSApplicationDelegate for OpenDelegate {
        #[unsafe(method(application:openURLs:))]
        fn application_open_urls(&self, _application: &NSApplication, urls: &NSArray<NSURL>) {
            let paths: Vec<PathBuf> = urls.iter().filter(|u| u.isFileURL()).filter_map(|u| u.path()).map(|p| PathBuf::from(p.to_string())).collect();
            if paths.is_empty() {
                return;
            }
            log::info!("macOS asked to open {} file(s)", paths.len());
            match OPENED.lock() {
                Ok(mut q) => q.extend(paths),
                // A poisoned queue only means a panic elsewhere while it was held; the paths are
                // still worth opening.
                Err(poisoned) => poisoned.into_inner().extend(paths),
            }
            if let Some(ctx) = CONTEXT.get() {
                ctx.request_repaint();
            }
        }
    }
);

impl OpenDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: `init` on an NSObject subclass without ivars that need initialising.
        unsafe { msg_send![super(this), init] }
    }
}

/// Keeps the delegate alive: `NSApplication` holds its delegate weakly.
pub struct Installed {
    _delegate: Option<Retained<OpenDelegate>>,
}

/// Installs the delegate. Call after the winit event loop exists and before it runs.
pub fn install() -> Installed {
    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("not on the main thread: files opened from Finder will not reach GenHa");
        return Installed { _delegate: None };
    };
    let delegate = OpenDelegate::new(mtm);
    let app = NSApplication::sharedApplication(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    Installed { _delegate: Some(delegate) }
}

/// Lets the delegate wake the UI.
pub fn set_context(ctx: &egui::Context) {
    // Set once; a second call (there is none) would keep the first context, which is fine.
    let _ = CONTEXT.set(ctx.clone());
}

/// Paths opened from Finder since the last call.
pub fn take_opened() -> Vec<PathBuf> {
    match OPENED.lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    }
}
