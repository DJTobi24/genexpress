//! The canvas: the current page rendered by `genha-render`, with selection handles, snapping
//! guides, distance hints and gesture previews drawn on top. Pointer input goes to the engine's
//! tools; the hints are what the tools report (`genha_tools::smart`), this only draws them.

use std::sync::Arc;

use crate::App;
use crate::theme::Tokens;
use genha_doc::{Document, ElementKind};
use genha_geom::{Frame, Point};
use genha_i18n::tr;
use genha_tools::{Handle, Measure, MeasureKind, PointerEvent, PointerKind, ToolId};

#[derive(Default)]
pub struct CanvasState {
    texture: Option<egui::TextureHandle>,
    /// What the texture shows: document pointer, page, zoom (bits), pixels per point (bits),
    /// playback time (bits; MAX when not playing: a posed page's pointer can repeat).
    key: Option<(usize, usize, u32, u32, u64)>,
    pointer_down: bool,
    last_pos: Option<Point>,
    /// Shift, Alt and snapping (no Ctrl/Cmd) at the last pointer event.
    last_keys: (bool, bool, bool),
    panning: Option<egui::Pos2>,
    /// The document drawn last: (stored document, gesture preview, document with the preview
    /// applied). Reused while neither changes, and kept alive so the texture key (its address)
    /// cannot be taken by a new document.
    shown: Option<(Arc<Document>, Option<genha_tools::Preview>, Arc<Document>)>,
    /// Text being edited in place (UI side of the engine's `text_editing`).
    pub text: crate::textedit::TextEditUi,
}

/// Largest side of the canvas render in pixels (bigger zooms render the visible scale capped).
const MAX_RENDER_SIDE: f32 = 6000.0;

pub fn zoom_by(app: &mut App, factor: f32, around: Option<egui::Vec2>) {
    let old = app.view.zoom;
    let new = (old * factor).clamp(0.02, 16.0);
    if let Some(p) = around {
        // Keep the page point under `p` (canvas-local) where it is.
        app.view.pan = p - (p - app.view.pan) * (new / old);
    }
    app.view.zoom = new;
}

pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let Some(mut display) = display_document(app) else { return };
    let Some(d) = app.session.active() else { return };
    let page_index = d.page;
    let selection = d.selection.clone();
    let playback = crate::motion::playback_frame(app, ui.ctx(), &mut display, page_index);
    let Some(page) = display.pages.get(page_index) else { return };
    let (pw, ph) = (page.width as f32, page.height as f32);

    // A fixed id: the canvas keeps keyboard focus while text is edited on it.
    let (area, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    let resp = ui.interact(area, egui::Id::new("genha_canvas"), egui::Sense::click_and_drag());
    let painter = ui.painter_at(area);
    if app.view.fit_pending && area.width() > 50.0 {
        let z = ((area.width() - 80.0) / pw).min((area.height() - 80.0) / ph).clamp(0.02, 4.0);
        app.view.zoom = z;
        app.view.pan = egui::vec2((area.width() - pw * z) / 2.0, (area.height() - ph * z) / 2.0);
        app.view.fit_pending = false;
    }
    let zoom = app.view.zoom;
    let origin = area.min + app.view.pan;
    let to_screen = |p: Point| egui::pos2(origin.x + p.x as f32 * zoom, origin.y + p.y as f32 * zoom);
    let to_page = |p: egui::Pos2| Point::new(f64::from((p.x - origin.x) / zoom), f64::from((p.y - origin.y) / zoom));
    let page_rect = egui::Rect::from_min_size(origin, egui::vec2(pw * zoom, ph * zoom));

    // Page shadow, transparency checkerboard and the rendered page.
    painter.rect_filled(page_rect.translate(egui::vec2(0.0, 3.0)).expand(1.0), 2, egui::Color32::from_black_alpha(40));
    checkerboard(&painter, page_rect);
    update_texture(app, ui.ctx(), &display, page_index, playback);
    if let Some(tex) = &app.canvas.texture {
        painter.image(tex.id(), page_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    }
    if playback.is_some() {
        // A click stops the playback; nothing is edited while the page plays.
        if resp.clicked() {
            crate::motion::stop(app);
        }
        return;
    }

    // Overlays.
    let preview = app.session.preview.clone();
    let hovered = resp.hover_pos().and_then(|p| genha_tools::hit_test(page, to_page(p)).map(|e| e.id));
    if let Some(h) = hovered.filter(|h| !selection.contains(h))
        && app.session.tools.tool == ToolId::Select
        && let Some(el) = page.find(h)
    {
        outline(&painter, &el.frame, &to_screen, egui::Stroke::new(1.0, t.selection.gamma_multiply(0.7)));
    }
    for id in &selection {
        if let Some(el) = page.find(*id) {
            outline(&painter, &el.frame, &to_screen, egui::Stroke::new(1.5, t.selection));
        }
    }
    // Handles are hidden while text is edited in place.
    if app.session.tools.tool == ToolId::Select
        && app.session.text_editing.is_none()
        && let Some(f) = genha_tools::selection_frame(&display, &selection)
    {
        if selection.len() > 1 {
            outline(&painter, &f, &to_screen, egui::Stroke::new(1.0, t.selection.gamma_multiply(0.6)));
        }
        handles(&painter, &f, zoom, &to_screen, t);
    }
    if let Some(p) = &preview {
        let guide = egui::Stroke::new(1.0, t.guide);
        for x in &p.guides_x {
            let sx = origin.x + *x as f32 * zoom;
            painter.line_segment([egui::pos2(sx, page_rect.top() - 20.0), egui::pos2(sx, page_rect.bottom() + 20.0)], guide);
        }
        for y in &p.guides_y {
            let sy = origin.y + *y as f32 * zoom;
            painter.line_segment([egui::pos2(page_rect.left() - 20.0, sy), egui::pos2(page_rect.right() + 20.0, sy)], guide);
        }
        if let Some(m) = p.marquee {
            let r = egui::Rect::from_two_pos(to_screen(Point::new(m.x0, m.y0)), to_screen(Point::new(m.x1, m.y1)));
            painter.rect_filled(r, 0, t.selection.gamma_multiply(0.12));
            painter.rect_stroke(r, 0, egui::Stroke::new(1.0, t.selection), egui::StrokeKind::Inside);
        }
        draw_measures(&painter, &p.measures, &to_screen, t);
        if let Some(f) = p.creating {
            outline(&painter, &f, &to_screen, egui::Stroke::new(1.5, t.selection));
            let c = to_screen(Point::new(f.x + f.width, f.y + f.height));
            painter.text(
                c + egui::vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                format!("{:.0} × {:.0}", f.width, f.height),
                egui::FontId::proportional(11.0),
                t.text_dim,
            );
        }
    }

    // Alt (Option) over another object: the distances from the selection to it (or to the page).
    let alt = ui.input(|i| i.modifiers.alt);
    if alt
        && preview.is_none()
        && !app.session.tools.is_dragging()
        && app.session.tools.tool == ToolId::Select
        && app.session.text_editing.is_none()
        && !selection.is_empty()
        && let Some(hp) = resp.hover_pos()
    {
        if let Some(h) = hovered.filter(|h| !selection.contains(h))
            && let Some(el) = page.find(h)
        {
            outline(&painter, &el.frame, &to_screen, egui::Stroke::new(1.0, t.measure));
        }
        let ms = genha_tools::hover_measures(&display, page_index, &selection, to_page(hp));
        draw_measures(&painter, &ms, &to_screen, t);
        ui.ctx().request_repaint();
    }

    // Zoom (Ctrl + wheel / pinch) and scroll-to-pan.
    if resp.hovered() {
        let (scroll, zoom_delta, pointer) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
        if zoom_delta != 1.0 {
            zoom_by(app, zoom_delta, pointer.map(|p| p - area.min));
        } else if scroll != egui::Vec2::ZERO {
            app.view.pan += scroll;
        }
    }

    let view = crate::textedit::View { to_screen: &to_screen, to_page: &to_page, zoom };
    if !crate::textedit::show(app, ui, &resp, &display, page_index, &view, t) {
        pointer_input(app, ui, &resp, area, &to_page, &display, page_index);
    }
    pointer_shape(app, ui, &resp, &display, page_index, &selection, &to_page, t);
    if app.session.tools.is_dragging() || app.session.preview.is_some() {
        ui.ctx().request_repaint();
    }

    // The bar over the selection, and the zoom control.
    let selection_rect = (app.session.tools.tool == ToolId::Select
        && app.session.text_editing.is_none()
        && !app.session.tools.is_dragging()
        && !ui.input(|i| i.pointer.any_down()))
    .then(|| genha_tools::selection_frame(&display, &selection))
    .flatten()
    .map(|f| {
        let c = corners(&f).map(&to_screen);
        egui::Rect::from_points(&c)
    });
    // The bar fades in when a selection settles (after a drag, a click).
    let shown = ui.ctx().animate_bool_with_time(egui::Id::new("selection_bar_shown"), selection_rect.is_some(), 0.16);
    if let Some(r) = selection_rect {
        selection_bar(app, ui.ctx(), area, r, t, shown);
    }
    zoom_control(app, ui.ctx(), area, t);
}

/// The pointer's shape over the canvas: what a press would do there, or what the drag does.
#[allow(clippy::too_many_arguments)]
fn pointer_shape(
    app: &App,
    ui: &egui::Ui,
    resp: &egui::Response,
    doc: &Document,
    page_index: usize,
    selection: &[genha_doc::NodeId],
    to_page: &impl Fn(egui::Pos2) -> Point,
    t: &Tokens,
) {
    use egui::CursorIcon as C;
    use genha_tools::GestureKind;
    let ctx = ui.ctx();
    let gesture = app.session.tools.gesture();
    if !resp.hovered() && gesture == GestureKind::Idle && app.canvas.panning.is_none() {
        return;
    }
    let frame = genha_tools::selection_frame(doc, selection);
    let rotation = frame.map_or(0.0, |f| f.rotation);
    let space = ui.input(|i| i.key_down(egui::Key::Space));
    let icon = if app.canvas.panning.is_some() {
        C::Grabbing
    } else {
        match gesture {
            GestureKind::Resize(h) => resize_cursor(h, rotation),
            GestureKind::Rotate => return rotate_cursor(ctx, t),
            GestureKind::Move => C::Move,
            GestureKind::Create => C::Crosshair,
            GestureKind::Pan => C::Grabbing,
            GestureKind::Marquee => C::Default,
            GestureKind::Idle => {
                let Some(pos) = resp.hover_pos() else { return };
                let p = to_page(pos);
                match app.session.tools.tool {
                    _ if space => C::Grab,
                    ToolId::Hand => C::Grab,
                    ToolId::Rect | ToolId::Ellipse | ToolId::Line => C::Crosshair,
                    ToolId::Text => C::Text,
                    ToolId::Select => {
                        let page = doc.pages.get(page_index);
                        let edited = app.session.text_editing.as_ref().map(|e| e.id);
                        let over = page.and_then(|pg| genha_tools::hit_test(pg, p));
                        let handle =
                            (edited.is_none()).then_some(frame).flatten().and_then(|f| genha_tools::hit_handle(&f, p, f64::from(app.view.zoom)));
                        match (handle, over) {
                            (Some(Handle::Rotate), _) => return rotate_cursor(ctx, t),
                            (Some(h), _) => resize_cursor(h, rotation),
                            (None, Some(el)) if edited == Some(el.id) => C::Text,
                            (None, Some(el)) if !el.locked => C::Move,
                            _ => C::Default,
                        }
                    }
                }
            }
        }
    };
    ctx.set_cursor_icon(icon);
}

/// The resize arrows for `handle` on a box turned by `rotation` degrees.
fn resize_cursor(handle: Handle, rotation: f64) -> egui::CursorIcon {
    use egui::CursorIcon as C;
    // The direction the handle pulls in, on the unturned box (y points down).
    let base = match handle {
        Handle::Left | Handle::Right => 0.0,
        Handle::TopLeft | Handle::BottomRight => 45.0,
        Handle::Top | Handle::Bottom => 90.0,
        Handle::TopRight | Handle::BottomLeft => 135.0,
        Handle::Rotate => return C::Default,
    };
    let turned = if rotation.is_finite() { base + rotation } else { base };
    match ((turned / 45.0).round().rem_euclid(4.0)) as u8 {
        0 => C::ResizeHorizontal,
        1 => C::ResizeNwSe,
        2 => C::ResizeVertical,
        _ => C::ResizeNeSw,
    }
}

/// A turning arrow drawn at the pointer (no system cursor has one).
fn rotate_cursor(ctx: &egui::Context, t: &Tokens) {
    let Some(pos) = ctx.pointer_hover_pos().or_else(|| ctx.pointer_interact_pos()) else { return };
    ctx.set_cursor_icon(egui::CursorIcon::None);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("rotate_cursor")));
    let r = egui::Rect::from_center_size(pos, egui::vec2(22.0, 22.0));
    painter.circle_filled(pos, 12.0, egui::Color32::from_white_alpha(200));
    crate::glyphs::draw(&painter, crate::glyphs::Glyph::Redo, r, t.selection);
}

/// What the bar over the selection can do.
const BAR_ACTIONS: usize = 5;

/// A small bar of icons over (or under) the selection: duplicate, delete, lock, forward, backward
/// and more.
fn selection_bar(app: &mut App, ctx: &egui::Context, area: egui::Rect, sel: egui::Rect, t: &Tokens, shown: f32) {
    use crate::glyphs::{Glyph, button};
    let size = egui::vec2(BAR_ACTIONS as f32 * 34.0 + 34.0 + 16.0, 42.0);
    // Above the selection (clear of the rotation knob), else below, kept inside the canvas.
    let above = sel.top() - 52.0 - size.y;
    let y = if above > area.top() + 6.0 { above } else { (sel.bottom() + 14.0).min(area.bottom() - size.y - 6.0) };
    let x = (sel.center().x - size.x / 2.0).clamp(area.left() + 6.0, (area.right() - size.x - 6.0).max(area.left() + 6.0));
    let locked = app.session.active().is_some_and(|d| d.selected_elements().iter().any(|e| e.locked));
    let several = app.session.active().is_some_and(|d| d.selection.len() > 1);
    let mut action: Option<&'static str> = None;
    let y = y + 6.0 * (1.0 - shown);
    egui::Area::new(egui::Id::new("selection_bar")).order(egui::Order::Foreground).fixed_pos(egui::pos2(x, y)).show(ctx, |ui| {
        ui.set_opacity(shown);
        egui::Frame::new()
            .fill(t.panel)
            .corner_radius(12)
            .stroke(egui::Stroke::new(1.0, t.border))
            .shadow(egui::epaint::Shadow { offset: [0, 3], blur: 12, spread: 0, color: egui::Color32::from_black_alpha(40) })
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (g, hint, cmd) in [
                        (Glyph::Duplicate, genha_i18n::N_("Duplicate (Ctrl+D)"), "edit.duplicate"),
                        (Glyph::Trash, genha_i18n::N_("Delete"), "edit.delete"),
                        (
                            if locked { Glyph::Unlock } else { Glyph::Lock },
                            if locked { genha_i18n::N_("Unlock") } else { genha_i18n::N_("Lock") },
                            if locked { "object.unlock" } else { "object.lock" },
                        ),
                        (Glyph::Forward, genha_i18n::N_("Bring forward"), "object.bringForward"),
                        (Glyph::Backward, genha_i18n::N_("Send backward"), "object.sendBackward"),
                    ] {
                        if button(ui, t, g, false, 32.0).on_hover_text(tr(hint)).clicked() {
                            action = Some(cmd);
                        }
                    }
                    let r = button(ui, t, Glyph::More, false, 32.0).on_hover_text(tr("More"));
                    egui::Popup::menu(&r).show(|ui| {
                        ui.set_min_width(200.0);
                        let mut item = |ui: &mut egui::Ui, label: &str, cmd: &'static str| {
                            if ui.button(label).clicked() {
                                action = Some(cmd);
                                ui.close();
                            }
                        };
                        item(ui, tr("Bring to Front"), "object.bringToFront");
                        item(ui, tr("Send to Back"), "object.sendToBack");
                        ui.separator();
                        item(ui, tr("Flip H"), "object.flipHorizontal");
                        item(ui, tr("Flip V"), "object.flipVertical");
                        if several {
                            item(ui, tr("Group"), "object.group");
                        }
                        item(ui, tr("Ungroup"), "object.ungroup");
                        ui.separator();
                        item(ui, tr("Copy style"), "style.copy");
                        item(ui, tr("Paste style"), "style.paste");
                    });
                });
            });
    });
    if let Some(cmd) = action {
        app.run(cmd, &serde_json::json!({}));
    }
}

/// Zoom out, the zoom (a click fits the page), zoom in, at the canvas's bottom right.
fn zoom_control(app: &mut App, ctx: &egui::Context, area: egui::Rect, t: &Tokens) {
    use crate::glyphs::{Glyph, button};
    let size = egui::vec2(32.0 * 3.0 + 58.0 + 12.0, 40.0);
    let pos = area.right_bottom() - size - egui::vec2(14.0, 14.0);
    egui::Area::new(egui::Id::new("zoom_control")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        egui::Frame::new().fill(t.panel).corner_radius(12).stroke(egui::Stroke::new(1.0, t.border)).inner_margin(egui::Margin::same(4)).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if button(ui, t, Glyph::Minus, false, 32.0).on_hover_text(tr("Zoom out")).clicked() {
                        zoom_by(app, 0.8, None);
                    }
                    let label = egui::Button::new(egui::RichText::new(format!("{:.0}%", app.view.zoom * 100.0)).size(12.5).color(t.text))
                        .frame(false)
                        .min_size(egui::vec2(58.0, 32.0));
                    if ui.add(label).on_hover_text(tr("Fit page (Ctrl+0)")).clicked() {
                        app.view.fit_pending = true;
                    }
                    if button(ui, t, Glyph::Plus, false, 32.0).on_hover_text(tr("Zoom in")).clicked() {
                        zoom_by(app, 1.25, None);
                    }
                    if button(ui, t, Glyph::Fit, false, 32.0).on_hover_text(tr("Fit page (Ctrl+0)")).clicked() {
                        app.view.fit_pending = true;
                    }
                });
            },
        );
    });
}

/// The document to draw: the stored one with the gesture preview applied, built once per change.
fn display_document(app: &mut App) -> Option<Arc<Document>> {
    let base = app.session.active().map(|d| Arc::clone(&d.doc))?;
    if app.session.text_editing.is_none()
        && let Some((b, p, shown)) = &app.canvas.shown
        && Arc::ptr_eq(b, &base)
        && *p == app.session.preview
    {
        return Some(Arc::clone(shown));
    }
    let shown = app.session.display_document()?;
    app.canvas.shown = Some((base, app.session.preview.clone(), Arc::clone(&shown)));
    Some(shown)
}

fn checkerboard(p: &egui::Painter, r: egui::Rect) {
    p.rect_filled(r, 0, egui::Color32::from_gray(250));
    let s = 12.0;
    let (cols, rows) = ((r.width() / s).ceil() as i32, (r.height() / s).ceil() as i32);
    if cols * rows > 40_000 {
        return;
    }
    for y in 0..rows {
        for x in 0..cols {
            if (x + y) % 2 == 0 {
                let cell = egui::Rect::from_min_size(egui::pos2(r.left() + x as f32 * s, r.top() + y as f32 * s), egui::vec2(s, s)).intersect(r);
                p.rect_filled(cell, 0, egui::Color32::from_gray(230));
            }
        }
    }
}

/// Renders the page into the canvas texture when it changed. During playback (`playback` is the
/// time) a new picture is drawn every frame, at a lower resolution.
fn update_texture(app: &mut App, ctx: &egui::Context, doc: &Arc<Document>, page: usize, playback: Option<f64>) {
    let quality = if playback.is_some() { crate::motion::PLAYBACK_QUALITY } else { 1.0 };
    let ppp = ctx.pixels_per_point();
    let Some(pg) = doc.pages.get(page) else { return };
    let mut scale = app.view.zoom * ppp * quality.clamp(0.1, 1.0);
    let side = (pg.width.max(pg.height) as f32) * scale;
    if side > MAX_RENDER_SIDE {
        scale *= MAX_RENDER_SIDE / side;
    }
    let key = (Arc::as_ptr(doc) as usize, page, scale.to_bits(), ppp.to_bits(), playback.map_or(u64::MAX, f64::to_bits));
    if app.canvas.key == Some(key) && app.canvas.texture.is_some() {
        return;
    }
    // A page without background shows the checkerboard; any other background is drawn.
    let transparent = matches!(pg.background, genha_doc::Fill::None);
    let opts = genha_render::RenderOptions { scale: f64::from(scale), transparent_background: transparent };
    match app.renderer.render_page(doc, page, &app.session.fonts, &opts) {
        Ok(r) => {
            let img = crate::thumbs::to_color_image(&r);
            match &mut app.canvas.texture {
                Some(tex) => tex.set(img, egui::TextureOptions::LINEAR),
                None => app.canvas.texture = Some(ctx.load_texture("canvas", img, egui::TextureOptions::LINEAR)),
            }
            app.canvas.key = Some(key);
        }
        Err(e) => {
            app.canvas.key = Some(key);
            app.set_status(genha_i18n::tr!("Could not draw the page: {error}", error = e));
        }
    }
}

fn corners(f: &Frame) -> [Point; 4] {
    let m = f.to_page();
    [m * Point::new(0.0, 0.0), m * Point::new(f.width, 0.0), m * Point::new(f.width, f.height), m * Point::new(0.0, f.height)]
}

fn outline(p: &egui::Painter, f: &Frame, to_screen: &impl Fn(Point) -> egui::Pos2, stroke: egui::Stroke) {
    let c = corners(f).map(to_screen);
    p.add(egui::Shape::closed_line(c.to_vec(), stroke));
}

/// Distance hints, equal-spacing and same-size markers: a line with end ticks and a label.
fn draw_measures(p: &egui::Painter, measures: &[Measure], to_screen: &impl Fn(Point) -> egui::Pos2, t: &Tokens) {
    for m in measures {
        let color = match m.kind {
            MeasureKind::Distance => t.measure,
            MeasureKind::EqualSpacing | MeasureKind::SameSize => t.guide,
        };
        let horizontal = m.horizontal();
        let (mut a, mut b) = (to_screen(m.from), to_screen(m.to));
        if m.kind == MeasureKind::SameSize {
            // Drawn just outside the object's top (widths) or left (heights) side.
            let off = if horizontal { egui::vec2(0.0, -8.0) } else { egui::vec2(-8.0, 0.0) };
            a += off;
            b += off;
        }
        let stroke = egui::Stroke::new(1.0, color);
        p.line_segment([a, b], stroke);
        let tick = if horizontal { egui::vec2(0.0, 4.0) } else { egui::vec2(4.0, 0.0) };
        for e in [a, b] {
            p.line_segment([e - tick, e + tick], stroke);
        }
        if m.kind == MeasureKind::EqualSpacing {
            // A band across the gap marks it as one of a set of equal gaps.
            let band = if horizontal { egui::vec2(0.0, 3.0) } else { egui::vec2(3.0, 0.0) };
            p.rect_filled(egui::Rect::from_two_pos(a - band, b + band), 0, color.gamma_multiply(0.35));
        }
        let galley = p.layout_no_wrap(m.label(), egui::FontId::proportional(11.0), egui::Color32::WHITE);
        let size = galley.size() + egui::vec2(8.0, 4.0);
        let mid = a + (b - a) / 2.0;
        // The label sits on the line; a same-size label sits beyond it.
        let centre = if m.kind == MeasureKind::SameSize {
            mid + if horizontal { egui::vec2(0.0, -size.y / 2.0 - 4.0) } else { egui::vec2(-size.x / 2.0 - 4.0, 0.0) }
        } else {
            mid
        };
        let r = egui::Rect::from_center_size(centre, size);
        p.rect_filled(r, 3, color);
        p.galley(r.min + egui::vec2(4.0, 2.0), galley, egui::Color32::WHITE);
    }
}

fn handles(p: &egui::Painter, f: &Frame, zoom: f32, to_screen: &impl Fn(Point) -> egui::Pos2, t: &Tokens) {
    let top_mid = to_screen(Handle::Top.position(f, f64::from(zoom)));
    let knob = to_screen(Handle::Rotate.position(f, f64::from(zoom)));
    p.line_segment([top_mid, knob], egui::Stroke::new(1.0, t.selection));
    for h in Handle::ALL {
        let c = to_screen(h.position(f, f64::from(zoom)));
        if h == Handle::Rotate {
            p.circle_filled(c, 6.0, t.handle_fill);
            p.circle_stroke(c, 6.0, egui::Stroke::new(1.5, t.selection));
        } else {
            let r = egui::Rect::from_center_size(c, egui::vec2(9.0, 9.0));
            p.rect_filled(r, 2, t.handle_fill);
            p.rect_stroke(r, 2, egui::Stroke::new(1.5, t.selection), egui::StrokeKind::Inside);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pointer_input(
    app: &mut App,
    ui: &egui::Ui,
    resp: &egui::Response,
    area: egui::Rect,
    to_page: &impl Fn(egui::Pos2) -> Point,
    doc: &Arc<Document>,
    page: usize,
) {
    let (pos, pressed, down, released, mods, middle_down, double) = ui.input(|i| {
        (
            i.pointer.interact_pos(),
            i.pointer.primary_pressed(),
            i.pointer.primary_down(),
            i.pointer.primary_released(),
            i.modifiers,
            i.pointer.middle_down() || (i.key_down(egui::Key::Space) && i.pointer.primary_down()),
            i.pointer.button_double_clicked(egui::PointerButton::Primary),
        )
    });
    // Middle button or Space+drag pans whatever the tool.
    if middle_down && resp.hovered() || app.canvas.panning.is_some() && middle_down {
        if let Some(p) = pos {
            if let Some(last) = app.canvas.panning {
                app.view.pan += p - last;
            }
            app.canvas.panning = Some(p);
        }
        return;
    }
    app.canvas.panning = None;
    let Some(pos) = pos else { return };
    // Double-click on a text element edits it in place.
    if double && resp.hovered() && app.session.tools.tool == ToolId::Select {
        let pt = to_page(pos);
        if let Some(pg) = doc.pages.get(page)
            && let Some(el) = genha_tools::hit_test(pg, pt)
            && matches!(el.kind, ElementKind::Text(_))
            && !el.locked
        {
            app.canvas.pointer_down = false;
            app.session.tools.cancel();
            app.session.preview = None;
            let now = ui.input(|i| i.time);
            crate::textedit::begin_at(app, doc, page, el.id, pt, pos, now);
            return;
        }
    }
    let ev = |kind| PointerEvent { kind, pos: to_page(pos), shift: mods.shift, alt: mods.alt };
    // Ctrl (Cmd on macOS) held while dragging skips snapping for the moment.
    let snap = !mods.command;
    let keys = (mods.shift, mods.alt, snap);
    let zoom = f64::from(app.view.zoom);
    if pressed && resp.hovered() && area.contains(pos) {
        app.canvas.pointer_down = true;
        app.canvas.last_pos = Some(to_page(pos));
        app.canvas.last_keys = keys;
        send(app, ev(PointerKind::Down), zoom, snap);
    } else if app.canvas.pointer_down && down {
        let p = to_page(pos);
        // Pressing or releasing Shift, Alt or Ctrl/Cmd updates the drag without moving the pointer.
        if app.canvas.last_pos != Some(p) || app.canvas.last_keys != keys {
            app.canvas.last_pos = Some(p);
            app.canvas.last_keys = keys;
            send(app, ev(PointerKind::Move), zoom, snap);
        }
    } else if app.canvas.pointer_down && (released || !down) {
        app.canvas.pointer_down = false;
        send(app, ev(PointerKind::Up), zoom, snap);
        app.session.preview = None;
        // After drawing a shape or text, go back to selecting (like most design tools).
        if matches!(app.session.tools.tool, ToolId::Rect | ToolId::Ellipse | ToolId::Line | ToolId::Text) {
            app.session.tools.set_tool(ToolId::Select);
        }
    }
}

fn send(app: &mut App, ev: PointerEvent, zoom: f64, snap: bool) {
    // The tool finds the page's and the other objects' guides itself.
    match app.session.tool_event_with(ev, zoom, (&[], &[]), snap) {
        Ok(out) => {
            if let Some(d) = out.pan {
                app.view.pan += egui::vec2(d.x as f32, d.y as f32) * app.view.zoom;
            }
        }
        Err(e) => app.set_status(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::resize_cursor;
    use egui::CursorIcon as C;
    use genha_tools::Handle;

    #[test]
    fn resize_arrows_follow_the_handle_and_the_rotation() {
        assert_eq!(resize_cursor(Handle::Right, 0.0), C::ResizeHorizontal);
        assert_eq!(resize_cursor(Handle::Top, 0.0), C::ResizeVertical);
        assert_eq!(resize_cursor(Handle::TopLeft, 0.0), C::ResizeNwSe);
        assert_eq!(resize_cursor(Handle::BottomLeft, 0.0), C::ResizeNeSw);
        // A box turned a quarter: the side handles pull up and down.
        assert_eq!(resize_cursor(Handle::Right, 90.0), C::ResizeVertical);
        assert_eq!(resize_cursor(Handle::Top, -90.0), C::ResizeHorizontal);
        // Turned 45°: the side handle pulls diagonally.
        assert_eq!(resize_cursor(Handle::Right, 45.0), C::ResizeNwSe);
        assert_eq!(resize_cursor(Handle::Right, -45.0), C::ResizeNeSw);
        assert_eq!(resize_cursor(Handle::Right, f64::NAN), C::ResizeHorizontal);
        assert_eq!(resize_cursor(Handle::Rotate, 0.0), C::Default);
    }
}
