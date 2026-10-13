//! Motion in the UI: playing the current page's animations on the canvas and present mode
//! (the design full-window, page by page, animations playing). This is UI-only state; what an
//! animation looks like comes from `genha_motion::page_at`, and edits go through engine commands.

use std::sync::Arc;

use crate::App;
use crate::theme::Tokens;
use genha_doc::{AnimationKind, Document};

/// Largest side of a present-mode frame in pixels.
const MAX_PRESENT_SIDE: f32 = 2400.0;
/// Canvas render scale while playing (fewer pixels per frame keeps playback smooth).
pub const PLAYBACK_QUALITY: f32 = 0.6;

#[derive(Default)]
pub struct MotionState {
    /// The canvas is playing the current page's animation.
    pub playing: Option<Playback>,
    pub present: Option<Present>,
    /// Choices in the page panel's "Animate all elements".
    pub panel: PanelChoices,
}

#[derive(Clone, Copy, Debug)]
pub struct Playback {
    pub page: usize,
    /// `egui` input time when it started, seconds.
    pub started: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelChoices {
    pub kind: AnimationKind,
    pub stagger: f64,
}

impl Default for PanelChoices {
    fn default() -> Self {
        PanelChoices { kind: AnimationKind::Rise, stagger: genha_engine::cmd::motion::DEFAULT_STAGGER }
    }
}

pub struct Present {
    pub page: usize,
    pub started: f64,
    pub show_notes: bool,
    texture: Option<egui::TextureHandle>,
    /// What the texture shows: document pointer, page, time (bits; MAX when at rest), scale (bits).
    key: Option<(usize, usize, u64, u32)>,
}

fn time(ctx: &egui::Context) -> f64 {
    ctx.input(|i| i.time)
}

/// Starts (or restarts) playing the current page on the canvas.
pub fn play(app: &mut App, ctx: &egui::Context) {
    let page = app.session.active().map(|d| d.page).unwrap_or(0);
    app.motion.playing = Some(Playback { page, started: time(ctx) });
    ctx.request_repaint();
}

pub fn stop(app: &mut App) {
    app.motion.playing = None;
}

/// While playing, replaces `display` with the page as it looks now and asks for the next frame.
/// Returns the playback time while the canvas plays (it then shows no handles and takes no edits).
pub fn playback_frame(app: &mut App, ctx: &egui::Context, display: &mut Arc<Document>, page: usize) -> Option<f64> {
    let pb = app.motion.playing?;
    let Some(pg) = display.pages.get(page) else {
        app.motion.playing = None;
        return None;
    };
    let t = time(ctx) - pb.started;
    if pb.page != page || t > genha_motion::page_duration(pg) || !genha_motion::is_animated(pg) {
        app.motion.playing = None;
        if !genha_motion::is_animated(pg) {
            app.set_status("Nothing on this page is animated (Animation in the properties panel)");
        }
        return None;
    }
    let posed = genha_motion::page_at(pg, t);
    let mut doc = (**display).clone();
    if let Some(slot) = doc.pages.get_mut(page) {
        *slot = posed;
    }
    *display = Arc::new(doc);
    ctx.request_repaint();
    Some(t)
}

/// Enters present mode at the current page (full screen where the platform allows).
pub fn start_presenting(app: &mut App, ctx: &egui::Context) {
    let page = app.session.active().map(|d| d.page).unwrap_or(0);
    app.motion.playing = None;
    if app.session.text_editing.is_some() {
        app.run("text.endEdit", &serde_json::json!({}));
    }
    app.session.tools.cancel();
    app.session.preview = None;
    app.motion.present = Some(Present { page, started: time(ctx), show_notes: false, texture: None, key: None });
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
}

/// Shows page `index` in present mode, playing it from the start.
pub fn present_page(app: &mut App, ctx: &egui::Context, index: usize) {
    if let Some(p) = app.motion.present.as_mut() {
        p.page = index;
        p.started = time(ctx);
    }
}

pub fn stop_presenting(app: &mut App, ctx: &egui::Context) {
    if let Some(p) = app.motion.present.take() {
        // Back in the editor on the page last shown.
        app.run("page.goto", &serde_json::json!({"index": p.page}));
        app.view.fit_pending = true;
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
}

/// Present mode: the current page fitted on a dark stage, its animations playing.
/// Right, Down, Space, Page Down or a click: next page; Left, Up, Page Up or a right-click: back;
/// N: speaker notes; Esc: back to the editor.
pub fn present(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let t = Tokens::get(&ctx);
    let Some(doc) = app.session.active().map(|d| Arc::clone(&d.doc)) else {
        app.motion.present = None;
        return;
    };
    let pages = doc.pages.len();
    let (mut next, mut prev, exit, notes) = ctx.input_mut(|i| {
        let none = egui::Modifiers::NONE;
        let next = [egui::Key::ArrowRight, egui::Key::ArrowDown, egui::Key::Space, egui::Key::PageDown, egui::Key::Enter]
            .into_iter()
            .fold(false, |a, k| i.consume_key(none, k) | a);
        let prev = [egui::Key::ArrowLeft, egui::Key::ArrowUp, egui::Key::PageUp, egui::Key::Backspace]
            .into_iter()
            .fold(false, |a, k| i.consume_key(none, k) | a);
        (next, prev, i.consume_key(none, egui::Key::Escape), i.consume_key(none, egui::Key::N))
    });
    if exit {
        stop_presenting(app, &ctx);
        return;
    }
    let now = time(&ctx);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.stage)).show(ui, |ui| {
        let area = ui.max_rect();
        let resp = ui.allocate_rect(area, egui::Sense::click());
        next |= resp.clicked();
        prev |= resp.secondary_clicked();
        let Some(state) = app.motion.present.as_mut() else { return };
        if notes {
            state.show_notes = !state.show_notes;
        }
        if next && state.page + 1 < pages {
            state.page += 1;
            state.started = now;
        } else if prev && state.page > 0 {
            state.page -= 1;
            state.started = now;
        }
        state.page = state.page.min(pages.saturating_sub(1));
        let index = state.page;
        let Some(pg) = doc.pages.get(index) else { return };
        let (pw, ph) = (pg.width as f32, pg.height as f32);
        let fit = (area.width() / pw).min(area.height() / ph);
        let fit = if fit.is_finite() && fit > 0.0 { fit } else { 1.0 };
        let rect = egui::Rect::from_center_size(area.center(), egui::vec2(pw * fit, ph * fit));

        let elapsed = now - state.started;
        let duration = genha_motion::page_duration(pg);
        let animating = genha_motion::is_animated(pg) && elapsed < duration;
        let at = if animating { elapsed.max(0.0) } else { duration };
        let mut scale = fit * ctx.pixels_per_point();
        let side = pw.max(ph) * scale;
        if side > MAX_PRESENT_SIDE {
            scale *= MAX_PRESENT_SIDE / side;
        }
        let key = (Arc::as_ptr(&doc) as usize, index, if animating { at.to_bits() } else { u64::MAX }, scale.to_bits());
        if state.key != Some(key) || state.texture.is_none() {
            let mut frame_doc = (*doc).clone();
            if let Some(slot) = frame_doc.pages.get_mut(index) {
                *slot = genha_motion::page_at(pg, at);
            }
            let opts = genha_render::RenderOptions { scale: f64::from(scale), transparent_background: false };
            match app.renderer.render_page(&frame_doc, index, &app.session.fonts, &opts) {
                Ok(r) => {
                    let img = crate::thumbs::to_color_image(&r);
                    match &mut state.texture {
                        Some(tex) => tex.set(img, egui::TextureOptions::LINEAR),
                        None => state.texture = Some(ctx.load_texture("present", img, egui::TextureOptions::LINEAR)),
                    }
                }
                Err(e) => log::warn!("present: cannot draw page {index}: {e}"),
            }
            state.key = Some(key);
        }
        let painter = ui.painter();
        if let Some(tex) = &state.texture {
            painter.image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        let small = egui::FontId::proportional(13.0);
        painter.text(
            area.right_bottom() + egui::vec2(-16.0, -12.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{} / {}", index + 1, pages),
            small.clone(),
            t.stage_text,
        );
        painter.text(
            area.left_bottom() + egui::vec2(16.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Right or click: next   Left: back   N: notes   Esc: exit",
            small,
            t.stage_text.gamma_multiply(0.6),
        );
        if state.show_notes {
            let text = if pg.notes.trim().is_empty() { "(no speaker notes on this page)".to_string() } else { pg.notes.clone() };
            let galley = painter.layout(text, egui::FontId::proportional(18.0), egui::Color32::WHITE, (area.width() - 120.0).max(100.0));
            let pos = egui::pos2(area.center().x - galley.size().x / 2.0, area.bottom() - 48.0 - galley.size().y);
            let back = egui::Rect::from_min_size(pos, galley.size()).expand(12.0);
            painter.rect_filled(back, 8, egui::Color32::from_black_alpha(200));
            painter.galley(pos, galley, egui::Color32::WHITE);
        }
        if animating {
            ctx.request_repaint();
        }
    });
}
