//! macOS: files opened from Finder (double-click, "Open With", dropping on the Dock icon).
//!
//! macOS does not pass such files on the command line: it launches the app (or uses the running
//! one) and sends it an "open documents" event, which AppKit hands to the application delegate's
//! `application:openURLs:`. winit 0.30 installs its own delegate (`WinitApplicationDelegate`) when
//! the event loop is created and panics in its `sendEvent:` hook when the delegate is anything
//! else, so GenHa must not replace it (doing so aborted the app on the first event). Instead
//! [`install`] adds `application:openURLs:` to winit's delegate class, which does not implement it;
//! the method queues the paths and [`take_opened`] collects them on the next frame. It runs after
//! the event loop is created and before it runs, so files that launched the app arrive too (AppKit
//! sends them before `applicationDidFinishLaunching:`).
//!
//! The only unsafe code in the app: adding a method to an Objective-C class goes through the
//! runtime's C API, and the function below must have AppKit's signature for the method.
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use objc2::MainThreadMarker;
use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::sel;
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSArray, NSURL};

/// Paths macOS asked GenHa to open, not yet opened.
static OPENED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// The UI's context, to wake it when a file arrives while it is idle.
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// The application delegate class winit 0.30 registers (`winit::platform_impl::macos::app_state`).
const WINIT_DELEGATE: &std::ffi::CStr = c"WinitApplicationDelegate";
/// `- (void)application:(NSApplication *)app openURLs:(NSArray<NSURL *> *)urls`: returns void,
/// takes self, the selector and two objects.
const OPEN_URLS_TYPES: &std::ffi::CStr = c"v@:@@";

/// `application:openURLs:`, added to winit's delegate class.
///
/// It must not unwind into AppKit (that aborts the app), so the work is wrapped in
/// `catch_unwind`; nothing in it is expected to panic.
unsafe extern "C-unwind" fn open_urls(_this: *mut AnyObject, _cmd: Sel, _app: *mut AnyObject, urls: *mut NSArray<NSURL>) {
    // SAFETY: AppKit passes a valid `NSArray<NSURL *>` (or nil) that lives for the call.
    let Some(urls) = (unsafe { urls.as_ref() }) else {
        return;
    };
    let queued = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| queue(urls)));
    if queued.is_err() {
        log::error!("could not take the files macOS asked to open");
    }
}

fn queue(urls: &NSArray<NSURL>) {
    let paths: Vec<PathBuf> = urls.iter().filter(|u| u.isFileURL()).filter_map(|u| u.path()).map(|p| PathBuf::from(p.to_string())).collect();
    if paths.is_empty() {
        return;
    }
    log::info!("macOS asked to open {} file(s)", paths.len());
    match OPENED.lock() {
        Ok(mut q) => q.extend(paths),
        // A poisoned queue only means a panic elsewhere while it was held; the paths are still
        // worth opening.
        Err(poisoned) => poisoned.into_inner().extend(paths),
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// Teaches winit's application delegate to accept files from Finder. Call after the winit event
/// loop exists and before it runs. When that is not possible (another winit version, not on the
/// main thread) GenHa still runs; files opened from Finder then do not reach it.
pub fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("not on the main thread: files opened from Finder will not reach GenHa");
        return;
    };
    let Some(class) = AnyClass::get(WINIT_DELEGATE) else {
        log::warn!("winit's application delegate was not found: files opened from Finder will not reach GenHa");
        return;
    };
    let sel = sel!(application:openURLs:);
    if class.instance_method(sel).is_some() {
        // Already there (a later winit implements it, or installed twice): keep that one.
        log::info!("the application delegate already accepts files from Finder");
        return;
    }
    let f: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>) = open_urls;
    // SAFETY: the runtime calls an `Imp` with the arguments the type encoding names; casting the
    // function pointer to the untyped `Imp` is how the runtime API takes it, and `open_urls` has
    // exactly the signature `OPEN_URLS_TYPES` describes (all arguments are pointers).
    let imp: Imp = unsafe { std::mem::transmute::<unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>), Imp>(f) };
    // SAFETY: `class` is a registered class, `sel` a valid selector, `imp` matches the encoding,
    // which is a NUL-terminated C string that outlives the call (the runtime copies it).
    let added = unsafe { objc2::ffi::class_addMethod(std::ptr::from_ref(class).cast_mut(), sel, imp, OPEN_URLS_TYPES.as_ptr()) };
    if !added.as_bool() {
        log::warn!("could not add application:openURLs: to winit's delegate: files opened from Finder will not reach GenHa");
        return;
    }
    // AppKit may remember what the delegate responds to when it is set; set it again so it sees the
    // new method. Nothing runs in between: the event loop has not started yet.
    let app = NSApplication::sharedApplication(mtm);
    if let Some(delegate) = app.delegate() {
        app.setDelegate(None);
        app.setDelegate(Some(&delegate));
    }
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
