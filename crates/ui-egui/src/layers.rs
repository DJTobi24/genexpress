//! The layer strip beside the canvas: every object of the page as a picture, the front at the
//! top. Click one to select it (Shift adds to the selection), drag it up or down to restack it
//! (`object.reorder`): dropped at the top it is in front of everything, at the bottom behind. No
//! names in the tiles; hovering shows one. `view.layerStrip` shows or hides it.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use serde_json::{Value, json};

use crate::theme::Tokens;
use crate::{App, thumbs};
use genha_doc::NodeId;
use genha_i18n::{N_, tr};

/// The strip's width (points).
pub const WIDTH: f32 = 100.0;
/// A tile's side (points).
const TILE: f32 = 72.0;
/// Space between tiles (points).
const GAP: f32 = 10.0;
/// The pictures' longest side (pixels; tiles show them at up to twice their size on Retina).
const PICTURE: u32 = 144;

/// What the strip remembers between frames.
#[derive(Default)]
pub struct LayerStripState {
    /// The object being dragged and the row it started in (row 0 is the front).
    dragging: Option<(NodeId, usize)>,
    /// Per picture: whether the object is mostly light (it then gets a dark tile).
    light: HashMap<String, bool>,
}

/// Where an object dragged from display row `from` (row 0 is the front) and dropped in gap
/// `slot` (gap 0 is above the first row, gap `n` below the last) goes in the page's stacking
/// order (0 is the back), or `None` when it stays where it is.
pub fn drop_index(n: usize, from: usize, slot: usize) -> Option<usize> {
    if from >= n || slot > n {
        return None;
    }
    // The row it lands in once taken out of its old one.
    let row = if slot > from { slot - 1 } else { slot };
    (row != from).then(|| n - 1 - row)
}

/// Whether a picture is mostly light: the average brightness of its visible pixels.
fn is_light(r: &genha_render::Raster) -> bool {
    let (mut sum, mut weight) = (0.0_f64, 0.0_f64);
    for p in r.rgba.as_chunks::<4>().0.iter().step_by(3) {
        let a = f64::from(p[3]) / 255.0;
        let l = 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2]);
        sum += l * a;
        weight += a;
    }
    weight > 0.0 && sum / weight > 170.0
}

/// A stamp that changes when the object changes (its picture is drawn again).
fn stamp(el: &genha_doc::Element) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // An object that cannot be written out gets stamp 0: its picture is drawn once.
    serde_json::to_string(el).unwrap_or_default().hash(&mut h);
    h.finish()
}

/// Shows the strip as a panel on the canvas' right.
pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    if !app.session.layer_strip || app.session.active().is_none() {
        return;
    }
    egui::Panel::right("layer_strip")
        .exact_size(WIDTH)
        .resizable(false)
        .frame(egui::Frame::new().fill(t.panel_alt).inner_margin(egui::Margin::symmetric(0, 8)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| strip(app, ui, t));
}

fn strip(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let Some(d) = app.session.active() else { return };
    let doc = d.doc.clone();
    let page_index = d.page;
    let design = d.recovery_id;
    let selection = d.selection.clone();
    let Some(page) = doc.pages.get(page_index) else { return };
    let ctx = ui.ctx().clone();
    let mut action: Option<(&'static str, Value)> = None;

    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(tr("Layers")).size(11.0).color(t.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);
            if ui.small_button("×").on_hover_text(tr("Hide the layer strip (View › Layer Strip shows it again)")).clicked() {
                action = Some(("view.layerStrip", json!({"on": false})));
            }
        });
    });
    ui.add_space(4.0);

    // Front first.
    let rows: Vec<&genha_doc::Element> = page.elements.iter().rev().collect();
    let n = rows.len();
    if n == 0 {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| ui.label(egui::RichText::new(tr("Empty page")).size(11.0).color(t.text_dim)));
    }
    let pointer = ctx.pointer_interact_pos();
    let mut row_rects: Vec<egui::Rect> = Vec::with_capacity(n);
    let mut released: Option<(NodeId, usize)> = None;

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            for (row, el) in rows.iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(TILE, TILE), egui::Sense::click_and_drag());
                row_rects.push(rect);
                let (tex, on_light_content) = picture(app, &ctx, &doc, page_index, design, el);
                let dragged = app.layers.dragging.is_some_and(|(id, _)| id == el.id);
                let bg = if on_light_content { t.thumb_dark } else { t.thumb_light };
                let painter = ui.painter();
                painter.rect_filled(rect, 8, bg);
                if let Some(tex) = &tex {
                    let size = tex.size_vec2();
                    let shown = thumbs::fit_size(size.x, size.y, egui::vec2(TILE - 10.0, TILE - 10.0));
                    let img = egui::Rect::from_center_size(rect.center(), shown);
                    let tint = if el.hidden || dragged { egui::Color32::from_white_alpha(90) } else { egui::Color32::WHITE };
                    painter.image(tex.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
                }
                let on = selection.contains(&el.id);
                let stroke = if on {
                    egui::Stroke::new(2.5, t.accent)
                } else if resp.hovered() {
                    egui::Stroke::new(1.5, t.text_dim)
                } else {
                    egui::Stroke::new(1.0, t.border)
                };
                painter.rect_stroke(rect, 8, stroke, egui::StrokeKind::Outside);
                if el.locked {
                    padlock(painter, rect.right_bottom() + egui::vec2(-12.0, -12.0), t);
                }
                let mut name = crate::element_name(el);
                if el.hidden {
                    name = format!("{name} ({})", tr("hidden"));
                }
                let resp = resp.on_hover_text(name);
                if resp.clicked() {
                    let ids: Vec<u64> = if ui.input(|i| i.modifiers.shift) {
                        let mut s: Vec<u64> = selection.iter().map(|i| i.0).collect();
                        if !s.contains(&el.id.0) {
                            s.push(el.id.0);
                        }
                        s
                    } else {
                        vec![el.id.0]
                    };
                    action = Some(("select.set", json!({"ids": ids})));
                }
                if resp.drag_started() {
                    app.layers.dragging = Some((el.id, row));
                    if !on {
                        action = Some(("select.set", json!({"ids": [el.id.0]})));
                    }
                }
                if resp.drag_stopped()
                    && let Some(dr) = app.layers.dragging.take()
                {
                    released = Some(dr);
                }
                let (id, hidden, locked) = (el.id.0, el.hidden, el.locked);
                resp.context_menu(|ui| {
                    for (label, cmd) in [(N_("Bring to Front"), "object.bringToFront"), (N_("Send to Back"), "object.sendToBack")] {
                        if ui.button(tr(label)).clicked() {
                            action = Some((cmd, json!({"ids": [id]})));
                            ui.close();
                        }
                    }
                    ui.separator();
                    let (label, cmd) = if hidden { (N_("Show"), "object.show") } else { (N_("Hide"), "object.hide") };
                    if ui.button(tr(label)).clicked() {
                        action = Some((cmd, json!({"ids": [id]})));
                        ui.close();
                    }
                    let (label, cmd) = if locked { (N_("Unlock"), "object.unlock") } else { (N_("Lock"), "object.lock") };
                    if ui.button(tr(label)).clicked() {
                        action = Some((cmd, json!({"ids": [id]})));
                        ui.close();
                    }
                    if ui.button(tr("Delete")).clicked() {
                        action = Some(("edit.delete", json!({"ids": [id]})));
                        ui.close();
                    }
                });
                ui.add_space(GAP);
            }
        });
        // While dragging: a line where the object would land, and its picture under the pointer;
        // near the strip's ends the list scrolls.
        if let (Some((id, from)), Some(p)) = (app.layers.dragging, pointer) {
            let slot = slot_at(&row_rects, p.y);
            if drop_index(n, from, slot).is_some() {
                let y = match (slot, row_rects.first(), row_rects.last()) {
                    (0, Some(first), _) => first.top() - GAP / 2.0,
                    (s, _, Some(last)) if s >= n => last.bottom() + GAP / 2.0,
                    (s, _, _) => row_rects.get(s).map_or(0.0, |r| r.top() - GAP / 2.0),
                };
                let x = ui.max_rect().center().x;
                ui.painter().line_segment([egui::pos2(x - TILE / 2.0, y), egui::pos2(x + TILE / 2.0, y)], egui::Stroke::new(3.0, t.accent));
            }
            let clip = ui.clip_rect();
            if p.y < clip.top() + 24.0 {
                ui.scroll_with_delta(egui::vec2(0.0, 6.0));
            } else if p.y > clip.bottom() - 24.0 {
                ui.scroll_with_delta(egui::vec2(0.0, -6.0));
            }
            if let Some(tex) = row_rects.get(from).and(app.thumbs.cached(&picture_key(design, page.id, id))) {
                let size = tex.size_vec2();
                let shown = thumbs::fit_size(size.x, size.y, egui::vec2(TILE * 0.8, TILE * 0.8));
                let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("layer_strip_drag"));
                // Left of the pointer, so the drop line in the strip stays visible.
                let img = egui::Rect::from_center_size(p - egui::vec2(TILE * 0.8, 0.0), shown);
                let painter = ctx.layer_painter(layer);
                painter.rect_filled(img.expand(4.0), 6, t.panel);
                painter.image(tex.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                painter.rect_stroke(img.expand(4.0), 6, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Outside);
            }
            ctx.request_repaint();
        }
    });

    // A drag that ended outside every tile (the pointer was released elsewhere) ends here too.
    if released.is_none() && app.layers.dragging.is_some() && !ctx.input(|i| i.pointer.any_down()) {
        released = app.layers.dragging.take();
    }
    if let Some((id, from)) = released
        && let Some(p) = pointer
        && let Some(index) = drop_index(n, from, slot_at(&row_rects, p.y))
    {
        action = Some(("object.reorder", json!({"id": id.0, "index": index})));
    }
    if let Some((cmd, p)) = action {
        app.run(cmd, &p);
    }
}

/// The gap (0 above the first tile, `rects.len()` below the last) nearest to height `y`.
fn slot_at(rects: &[egui::Rect], y: f32) -> usize {
    rects.iter().position(|r| y < r.center().y).unwrap_or(rects.len())
}

/// A small padlock: the object is locked.
fn padlock(painter: &egui::Painter, center: egui::Pos2, t: &Tokens) {
    painter.circle_filled(center, 8.0, t.panel);
    let body = egui::Rect::from_center_size(center + egui::vec2(0.0, 1.5), egui::vec2(8.0, 6.0));
    painter.rect_filled(body, 1, t.text);
    let top = center + egui::vec2(0.0, -1.5);
    let points: Vec<egui::Pos2> = (0..=8)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 + i as f32 / 8.0);
            top + egui::vec2(2.6 * a.cos(), 3.0 * a.sin())
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.4, t.text)));
}

/// An object's picture (drawn when it changed, a few per frame) and whether the object is mostly
/// light (it then goes on a dark tile). Shared by the strip and the Layers tab.
pub(crate) fn picture(
    app: &mut App,
    ctx: &egui::Context,
    doc: &genha_doc::Document,
    page_index: usize,
    design: u64,
    el: &genha_doc::Element,
) -> (Option<egui::TextureHandle>, bool) {
    let Some(page) = doc.pages.get(page_index) else { return (None, false) };
    let key = picture_key(design, page.id, el.id);
    let fonts = app.session.fonts.clone();
    let renderer = &mut app.renderer;
    let light = &mut app.layers.light;
    let tex = app.thumbs.get(ctx, &key, stamp(el), || {
        let r = renderer.render_element(doc, page_index, el.id, &fonts, PICTURE).ok()?;
        light.insert(key.clone(), is_light(&r));
        Some(r)
    });
    let on_light = app.layers.light.get(&key).copied().unwrap_or(false);
    (tex, on_light)
}

/// The texture key of an object's picture (designs told apart by their unique recovery id).
fn picture_key(design: u64, page: NodeId, id: NodeId) -> String {
    format!("layer:{design}:{}:{}", page.0, id.0)
}

#[cfg(test)]
mod tests {
    use super::drop_index;

    #[test]
    fn dropping_maps_rows_to_stack_positions() {
        // Three objects: rows 0, 1, 2 from the front are stack positions 2, 1, 0.
        // The back one dragged above the first row: the front.
        assert_eq!(drop_index(3, 2, 0), Some(2));
        // The front one dragged below the last row: the back.
        assert_eq!(drop_index(3, 0, 3), Some(0));
        // Dropped just above or below itself: stays.
        assert_eq!(drop_index(3, 1, 1), None);
        assert_eq!(drop_index(3, 1, 2), None);
        // One step down, one step up.
        assert_eq!(drop_index(3, 0, 2), Some(1));
        assert_eq!(drop_index(3, 2, 1), Some(1));
        // Out of range: nothing.
        assert_eq!(drop_index(3, 3, 0), None);
        assert_eq!(drop_index(3, 0, 4), None);
        assert_eq!(drop_index(0, 0, 0), None);
    }
}
