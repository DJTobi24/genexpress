//! The left panel for shapes, charts, tables and groups (and for several objects at once), in the
//! style of the text and picture panels: a title, Edit / Effects / Animation where they apply,
//! large controls, and Position and Opacity in pop-ups.

use serde_json::json;

use super::text_panel::{PanelTab, label, segmented, swatches, tile_back};
use super::{Actions, align_buttons, common_frame, data_props, element_animation};
use crate::App;
use crate::theme::Tokens;
use genha_doc::{Element, ElementKind, Fill, ShapeEl, ShapeKind};
use genha_i18n::{N_, tr, tr_n};

/// The panel's title and close button; a click on × closes it for `id`.
fn header(app: &mut App, ui: &mut egui::Ui, t: &Tokens, title: &str, id: genha_doc::NodeId) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(19.0).strong().color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = egui::Button::new(egui::RichText::new("×").size(18.0).color(t.text_dim)).frame(false);
            if ui.add(close).on_hover_text(tr("Close (the left panel shows its tab again)")).clicked() {
                app.editor.panel_closed = Some(id);
            }
        });
    });
    ui.add_space(8.0);
}

/// Runs what the panel asked for.
fn finish(app: &mut App, ui: &egui::Ui, acts: Actions) {
    for (cmd, p) in acts.commands {
        app.run(cmd, &p);
    }
    if acts.preview {
        crate::motion::play(app, ui.ctx());
    }
}

/// Draws the panel for a shape, chart, table or group.
pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: genha_doc::NodeId) {
    let Some(d) = app.session.active() else { return };
    let Some(el) = d.doc.element(id).ok().cloned() else { return };
    let brand: Vec<genha_color::Color> = d.doc.brand.colors.iter().map(|c| c.color).collect();
    let mut acts = Actions::default();
    let (title, tabs): (&str, &[PanelTab]) = match &el.kind {
        ElementKind::Shape(_) => (tr("Shape"), &super::text_panel::ALL_TABS),
        ElementKind::Chart(_) => (tr("Chart"), &[PanelTab::Edit, PanelTab::Animation]),
        ElementKind::Table(_) => (tr("Table"), &[PanelTab::Edit, PanelTab::Animation]),
        ElementKind::Group(_) => (tr("Group"), &[PanelTab::Edit, PanelTab::Animation]),
        // Text and pictures have their own panels.
        _ => return,
    };
    header(app, ui, t, title, id);
    let mut tab = app.editor.panel_tab;
    segmented(ui, t, &mut tab, tabs);
    app.editor.panel_tab = tab;
    ui.add_space(14.0);
    match (tab, &el.kind) {
        (PanelTab::Animation, _) => element_animation(ui, t, &el, &mut acts),
        (PanelTab::Effects, ElementKind::Shape(sh)) => shadow(ui, t, sh, id.0, &mut acts),
        (_, ElementKind::Shape(sh)) => {
            shape_edit(ui, t, &el, sh, &brand, &mut acts);
            ui.add_space(14.0);
            position_and_opacity(ui, t, &el, &mut acts);
            ui.add_space(12.0);
            arrange(ui, t, &el, &mut acts);
        }
        (_, ElementKind::Chart(c)) => {
            position_and_opacity(ui, t, &el, &mut acts);
            ui.add_space(6.0);
            data_props::chart(ui, t, id.0, c, &brand, &mut acts);
            ui.add_space(12.0);
            arrange(ui, t, &el, &mut acts);
        }
        (_, ElementKind::Table(tb)) => {
            position_and_opacity(ui, t, &el, &mut acts);
            ui.add_space(6.0);
            data_props::table(ui, t, id.0, tb, el.frame.width, &brand, &mut acts);
            ui.add_space(12.0);
            arrange(ui, t, &el, &mut acts);
        }
        (_, ElementKind::Group(g)) => {
            ui.label(egui::RichText::new(tr_n!("Group of {n} object", "Group of {n} objects", g.children.len())).color(t.text_dim));
            ui.add_space(8.0);
            if chip(ui, t, tr("Ungroup"), egui::vec2(ui.available_width(), 36.0)).clicked() {
                acts.run("object.ungroup", json!({}));
            }
            ui.add_space(14.0);
            position_and_opacity(ui, t, &el, &mut acts);
            ui.add_space(12.0);
            arrange(ui, t, &el, &mut acts);
        }
        _ => {}
    }
    finish(app, ui, acts);
}

/// Draws the panel for several selected objects: align, distribute, group.
pub fn several(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: genha_doc::NodeId) {
    let Some(d) = app.session.active() else { return };
    let els: Vec<Element> = d.selected_elements().into_iter().cloned().collect();
    let mut acts = Actions::default();
    header(app, ui, t, &tr_n!("{n} object", "{n} objects", els.len()), id);
    label(ui, t, tr("Align to each other"));
    align_buttons(ui, &mut acts, "selection");
    ui.add_space(12.0);
    label(ui, t, tr("Spread evenly"));
    let half = ((ui.available_width() - 8.0) / 2.0).floor();
    ui.horizontal(|ui| {
        if chip(ui, t, tr("Horizontally"), egui::vec2(half, 34.0)).clicked() {
            acts.run("object.distribute", json!({"axis": "horizontal"}));
        }
        if chip(ui, t, tr("Vertically"), egui::vec2(half, 34.0)).clicked() {
            acts.run("object.distribute", json!({"axis": "vertical"}));
        }
    });
    ui.add_space(12.0);
    label(ui, t, tr("Arrange"));
    ui.horizontal_wrapped(|ui| {
        for (name, cmd) in [(N_("Group"), "object.group"), (N_("Duplicate"), "edit.duplicate"), (N_("Paste style"), "style.paste")] {
            if chip(ui, t, tr(name), egui::Vec2::ZERO).clicked() {
                acts.run(cmd, json!({}));
            }
        }
        if danger_chip(ui, t, tr("Delete")).clicked() {
            acts.run("edit.delete", json!({}));
        }
    });
    if els.iter().all(|e| matches!(e.kind, ElementKind::Image(_))) {
        ui.add_space(12.0);
        ui.label(egui::RichText::new(tr("Pick a layout in Elements › Collage.")).size(11.5).color(t.text_dim));
    }
    finish(app, ui, acts);
}

/// A shape's fill, border and corners.
fn shape_edit(ui: &mut egui::Ui, t: &Tokens, el: &Element, sh: &ShapeEl, brand: &[genha_color::Color], acts: &mut Actions) {
    let id = el.id.0;
    let key = |what: &str| format!("{what}:{id}");
    label(ui, t, tr("Fill"));
    let fill = sh.fill.primary_color().unwrap_or(genha_color::Color::TRANSPARENT);
    if let Some(c) = swatches(ui, t, fill, brand) {
        acts.run("style.fill", json!({"color": c.to_hex(), "merge_key": key("fill")}));
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if chip(ui, t, tr("Gradient"), egui::Vec2::ZERO).clicked() {
            let second = brand.iter().find(|b| **b != fill).copied().unwrap_or(genha_color::Color::WHITE);
            acts.run("style.fill", json!({"fill": Fill::linear(45.0, fill, second)}));
        }
        if chip(ui, t, tr("No fill"), egui::Vec2::ZERO).clicked() {
            acts.run("style.fill", json!({"color": "none"}));
        }
    });
    ui.add_space(14.0);

    label(ui, t, tr("Border"));
    let stroke = sh.stroke.clone();
    let current = match &stroke {
        None => 0,
        Some(s) if s.dash.is_empty() => 1,
        Some(_) => 2,
    };
    ui.horizontal(|ui| {
        for (i, name) in [N_("None"), N_("Solid"), N_("Dashed")].into_iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(78.0, 48.0), egui::Sense::click());
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = current == i;
            tile_back(ui, t, rect, on, r.hovered());
            let ink = if on { t.accent } else { t.text };
            let line = [rect.center_top() + egui::vec2(-22.0, 16.0), rect.center_top() + egui::vec2(22.0, 16.0)];
            match i {
                0 => {
                    ui.painter().line_segment(line, egui::Stroke::new(1.0, t.text_dim.gamma_multiply(0.5)));
                }
                1 => {
                    ui.painter().line_segment(line, egui::Stroke::new(3.0, ink));
                }
                _ => {
                    for sh in egui::Shape::dashed_line(&line, egui::Stroke::new(3.0, ink), 6.0, 4.0) {
                        ui.painter().add(sh);
                    }
                }
            }
            ui.painter().text(
                rect.center_bottom() - egui::vec2(0.0, 11.0),
                egui::Align2::CENTER_CENTER,
                tr(name),
                egui::FontId::proportional(11.5),
                ink,
            );
            if r.clicked() && !on {
                let width = stroke.as_ref().map_or(4.0, |s| s.width.max(1.0));
                let p = match i {
                    0 => json!({"none": true}),
                    1 => json!({"width": width, "dash": []}),
                    _ => json!({"width": width, "dash": [width * 3.0, width * 2.0]}),
                };
                acts.run("style.stroke", p);
            }
        }
    });
    if let Some(st) = &stroke {
        ui.add_space(6.0);
        if let Some(c) = swatches(ui, t, st.color, brand) {
            acts.run("style.stroke", json!({"color": c.to_hex(), "merge_key": key("strokecolor")}));
        }
        let mut w = st.width;
        if ui.add(egui::Slider::new(&mut w, 0.5..=100.0).text(tr("Width"))).changed() {
            acts.run("style.stroke", json!({"width": w, "merge_key": key("strokewidth")}));
        }
    }
    if let ShapeKind::Rect { corner_radius } = &sh.shape {
        ui.add_space(14.0);
        label(ui, t, tr("Corners"));
        let mut r = *corner_radius;
        if ui.add(egui::Slider::new(&mut r, 0.0..=(el.frame.width.min(el.frame.height) / 2.0).max(1.0))).changed() {
            acts.run("style.cornerRadius", json!({"radius": r, "merge_key": key("radius")}));
        }
    }
}

/// A shape's shadow: none, soft or hard as pictures, then its softness and distance.
fn shadow(ui: &mut egui::Ui, t: &Tokens, sh: &ShapeEl, id: u64, acts: &mut Actions) {
    label(ui, t, tr("Shadow"));
    let current = match &sh.shadow {
        None => 0,
        Some(s) if s.blur >= 1.0 => 1,
        Some(_) => 2,
    };
    ui.horizontal(|ui| {
        for (i, name) in [N_("None"), N_("Soft"), N_("Hard")].into_iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(78.0, 70.0), egui::Sense::click());
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = current == i;
            tile_back(ui, t, rect, on, r.hovered());
            let card = egui::Rect::from_center_size(rect.center() - egui::vec2(3.0, 9.0), egui::vec2(30.0, 24.0));
            match i {
                1 => {
                    for k in 0..5 {
                        ui.painter().rect_filled(card.translate(egui::vec2(4.0, 4.0)).expand(k as f32), 4, t.text.gamma_multiply(0.05));
                    }
                }
                2 => {
                    ui.painter().rect_filled(card.translate(egui::vec2(5.0, 5.0)), 3, t.text.gamma_multiply(0.6));
                }
                _ => {}
            }
            ui.painter().rect_filled(card, 3, t.panel);
            ui.painter().rect_stroke(card, 3, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            let ink = if on { t.accent } else { t.text };
            ui.painter().text(
                rect.center_bottom() - egui::vec2(0.0, 10.0),
                egui::Align2::CENTER_CENTER,
                tr(name),
                egui::FontId::proportional(11.5),
                ink,
            );
            if r.clicked() && !on {
                let p = match i {
                    0 => json!({"none": true}),
                    1 => json!({"blur": 18.0, "offset_x": 6.0, "offset_y": 8.0}),
                    _ => json!({"blur": 0.0, "offset_x": 8.0, "offset_y": 8.0}),
                };
                acts.run("style.shadow", p);
            }
        }
    });
    let Some(s) = &sh.shadow else {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(tr("Pick a shadow to lift the shape off the page.")).size(11.5).color(t.text_dim));
        return;
    };
    ui.add_space(8.0);
    let mut blur = s.blur;
    if ui.add(egui::Slider::new(&mut blur, 0.0..=100.0).text(tr("Softness"))).changed() {
        acts.run("style.shadow", json!({"blur": blur, "merge_key": format!("shadowblur:{id}")}));
    }
    let mut dist = s.offset_x.hypot(s.offset_y);
    if ui.add(egui::Slider::new(&mut dist, 0.0..=100.0).text(tr("Distance"))).changed() {
        let d = dist / std::f64::consts::SQRT_2;
        acts.run("style.shadow", json!({"offset_x": d, "offset_y": d, "merge_key": format!("shadowoff:{id}")}));
    }
}

/// Position and Opacity as two buttons opening pop-ups.
pub(super) fn position_and_opacity(ui: &mut egui::Ui, t: &Tokens, el: &Element, acts: &mut Actions) {
    let half = ((ui.available_width() - 8.0) / 2.0).floor();
    ui.horizontal(|ui| {
        let r = chip(ui, t, tr("Position"), egui::vec2(half, 34.0));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(240.0);
            label(ui, t, tr("Align on the page"));
            align_buttons(ui, acts, "page");
            ui.add_space(6.0);
            common_frame(ui, el, acts);
        });
        let r = chip(ui, t, tr("Opacity"), egui::vec2(half, 34.0));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(240.0);
            let mut o = el.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut o, 0.0..=100.0).suffix("%").text(tr("Opacity"))).changed() {
                acts.run("object.setOpacity", json!({"opacity": o / 100.0, "merge_key": format!("opacity:{}", el.id.0)}));
            }
        });
    });
}

/// Order, flip, duplicate, styles, lock and delete as small buttons.
fn arrange(ui: &mut egui::Ui, t: &Tokens, el: &Element, acts: &mut Actions) {
    label(ui, t, tr("Arrange"));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (name, cmd) in [
            (N_("Forward"), "object.bringForward"),
            (N_("Backward"), "object.sendBackward"),
            (N_("Flip H"), "object.flipHorizontal"),
            (N_("Flip V"), "object.flipVertical"),
            (N_("Duplicate"), "edit.duplicate"),
            (N_("Copy style"), "style.copy"),
            (N_("Paste style"), "style.paste"),
        ] {
            if chip(ui, t, tr(name), egui::Vec2::ZERO).clicked() {
                acts.run(cmd, json!({}));
            }
        }
        let (name, cmd) = if el.locked { (tr("Unlock"), "object.unlock") } else { (tr("Lock"), "object.lock") };
        if chip(ui, t, name, egui::Vec2::ZERO).clicked() {
            acts.run(cmd, json!({}));
        }
        if danger_chip(ui, t, tr("Delete")).clicked() {
            acts.run("edit.delete", json!({}));
        }
    });
}

/// A rounded button in the panels' style, at least `min` large (zero: as large as its text).
pub(super) fn chip(ui: &mut egui::Ui, t: &Tokens, text: &str, min: egui::Vec2) -> egui::Response {
    let b = egui::Button::new(egui::RichText::new(text).size(12.5).color(t.text))
        .fill(t.panel_alt)
        .corner_radius(10)
        .min_size(egui::vec2(min.x, min.y.max(30.0)));
    ui.add(b)
}

/// A rounded button for removing things.
fn danger_chip(ui: &mut egui::Ui, t: &Tokens, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(12.5).color(t.danger)).fill(t.panel_alt).corner_radius(10).min_size(egui::vec2(0.0, 30.0)),
    )
}
