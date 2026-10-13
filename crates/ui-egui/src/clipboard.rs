//! Copy, cut and paste in the editor. Objects go through the engine's clipboard (`edit.copy`,
//! `edit.cut`, `edit.paste`); a picture on the system clipboard is placed with `image.paste`.
//!
//! - Desktop: egui-winit turns Cmd+C, Cmd+X and Cmd+V into Copy, Cut and Paste events instead of
//!   key presses, so they are handled here. Paste places a picture from the system clipboard
//!   (read with arboard) when there is one, else the copied objects. egui-winit reports Cmd+V only
//!   when the clipboard also holds text, so a picture alone is pasted with Edit › Paste Picture
//!   (the command palette, Cmd+K).
//! - Browser: the shortcuts arrive as key presses and the editor's shortcuts run them; a picture
//!   comes with the page's paste event (`web.rs`), which then drops the object paste of that
//!   Cmd+V. Edit › Paste Picture reads the clipboard through the browser's Clipboard API.
//!
//! Copying objects also puts their text (or a short note) on the system clipboard, so an older
//! picture there doesn't win over them at the next paste.

use serde_json::json;

use crate::{App, Screen};

/// After `edit.copy` / `edit.cut`: what the system clipboard should hold.
pub(crate) fn copied(app: &mut App) {
    let els = &app.session.clipboard.elements;
    let texts: Vec<&str> = els
        .iter()
        .filter_map(|e| match &e.kind {
            genha_doc::ElementKind::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    let n = els.len();
    app.clip_out = Some(if texts.is_empty() { format!("GenHa: {n} object{}", if n == 1 { "" } else { "s" }) } else { texts.join("\n") });
}

/// Called once per frame.
pub(crate) fn process(app: &mut App, ctx: &egui::Context) {
    if let Some(text) = app.clip_out.take() {
        ctx.copy_text(text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    desktop_shortcuts(app, ctx);
}

#[cfg(not(target_arch = "wasm32"))]
fn desktop_shortcuts(app: &mut App, ctx: &egui::Context) {
    if app.screen != Screen::Editor || ctx.egui_wants_keyboard_input() || app.session.text_editing.is_some() {
        return;
    }
    let events: Vec<egui::Event> =
        ctx.input(|i| i.events.iter().filter(|e| matches!(e, egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_))).cloned().collect());
    for e in events {
        match e {
            egui::Event::Copy => run_if_enabled(app, "edit.copy"),
            egui::Event::Cut => run_if_enabled(app, "edit.cut"),
            _ => match system_picture() {
                Some(png) => place(app, &png),
                None => run_if_enabled(app, "edit.paste"),
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_if_enabled(app: &mut App, id: &str) {
    if genha_engine::find_command(id).is_some_and(|s| (s.enabled)(&app.session).is_ok()) {
        app.run(id, &json!({}));
    }
}

/// Places picture bytes into the open design (on the home screen: starts a design from them).
pub(crate) fn place(app: &mut App, bytes: &[u8]) {
    if app.screen == Screen::Editor && app.session.active().is_some() {
        use base64::Engine as _;
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        app.run("image.paste", &json!({"data": data, "name": "Pasted picture"}));
    } else {
        match genha_engine::cmd::file::open_bytes(&mut app.session, "Pasted picture", bytes) {
            Ok(_) => {
                app.screen = Screen::Editor;
                app.view.fit_pending = true;
            }
            Err(e) => app.set_status(e.to_string()),
        }
    }
}

/// Edit › Paste Picture: the picture on the system clipboard, placed.
pub(crate) fn paste_picture(app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    match system_picture() {
        Some(png) => place(app, &png),
        None => app.set_status("There is no picture on the clipboard"),
    }
    // The browser reads the clipboard asynchronously; the picture arrives a frame later.
    #[cfg(target_arch = "wasm32")]
    {
        app.set_status("Reading the clipboard…");
        crate::web::read_clipboard_picture();
    }
}

/// The picture on the system clipboard as PNG bytes.
#[cfg(not(target_arch = "wasm32"))]
fn system_picture() -> Option<Vec<u8>> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    let (w, h) = (u32::try_from(img.width).ok()?, u32::try_from(img.height).ok()?);
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > genha_render::MAX_PIXELS || img.bytes.len() != (w as usize) * (h as usize) * 4 {
        return None;
    }
    genha_render::encode_png(&genha_render::Raster { width: w, height: h, rgba: img.bytes.into_owned() }).ok()
}
