//! The picture panel: when a picture is selected, the left panel turns into its editor, like the
//! text panel, in three tabs. Edit: quick actions as illustrated tiles (remove, blur or change
//! the background, black and white, fill the frame, colours to the brand) and compact buttons
//! opening pop-ups (adjust, crop and shape, corners, fit, position, opacity). Effects: the filters
//! as previews of this very picture, and their strength. Animation: as for every object.

use serde_json::json;

use super::text_panel::{PanelTab, label, segmented, swatches, tile_back};
use super::{Actions, element_animation};
use crate::art::{self, Art};
use crate::theme::Tokens;
use crate::{App, thumbs};
use genha_doc::{Element, ElementKind, ImageEl};
use genha_i18n::{N_, tr};

/// Draws the panel for picture `id`.
pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: genha_doc::NodeId) {
    let Some(d) = app.session.active() else { return };
    let Some(el) = d.doc.element(id).ok().cloned() else { return };
    let ElementKind::Image(img) = &el.kind else { return };
    let img = img.clone();
    let brand: Vec<genha_color::Color> = d.doc.brand.colors.iter().map(|c| c.color).collect();
    let mut acts = Actions { ml: crate::models::panel(app), ..Default::default() };
    let mut tab = app.editor.panel_tab;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr("Picture")).size(19.0).strong().color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = egui::Button::new(egui::RichText::new("×").size(18.0).color(t.text_dim)).frame(false);
            if ui.add(close).on_hover_text(tr("Close (the left panel shows its tab again)")).clicked() {
                app.editor.panel_closed = Some(id);
            }
        });
    });
    ui.add_space(8.0);
    segmented(ui, t, &mut tab, &super::text_panel::ALL_TABS);
    ui.add_space(14.0);
    app.editor.panel_tab = tab;

    match tab {
        PanelTab::Edit => {
            quick_actions(app, ui, t, &brand, &mut acts);
            ui.add_space(6.0);
            crate::models::draw(ui, &mut acts.ml);
            ui.add_space(12.0);
            tools(ui, t, &el, &img, &mut acts);
        }
        PanelTab::Effects => filters(app, ui, t, &el, &img, &mut acts),
        PanelTab::Animation => element_animation(ui, t, &el, &mut acts),
    }

    crate::models::apply(app, acts.ml);
    for (cmd, p) in acts.commands {
        if crate::models::is_background(cmd) {
            crate::models::start(app, ui.ctx(), cmd, p);
        } else {
            app.run(cmd, &p);
        }
    }
    if acts.preview {
        crate::motion::play(app, ui.ctx());
    }
}

/// A quick action: illustration, label, hint, and the command with its parameters (none: a pop-up
/// with colours).
type QuickAction = (&'static str, &'static str, &'static str, Option<(&'static str, serde_json::Value)>);

/// Quick actions as tiles with illustrations, three to a row.
fn quick_actions(app: &mut App, ui: &mut egui::Ui, t: &Tokens, brand: &[genha_color::Color], acts: &mut Actions) {
    label(ui, t, tr("Quick actions"));
    let ctx = ui.ctx().clone();
    let fonts = app.session.fonts.clone();
    let mut art = Art::new(&mut app.thumbs, &ctx, fonts);
    let actions: [QuickAction; 6] = [
        (
            "remove-background",
            N_("Remove background"),
            N_("Finds the subject and removes everything else"),
            Some(("image.removeBackground", json!({}))),
        ),
        ("blur-background", N_("Blur background"), N_("Keeps the subject sharp and blurs the rest"), Some(("image.blurBackground", json!({})))),
        ("change-background", N_("Background colour"), N_("Puts the subject on a colour"), None),
        ("grayscale", N_("Black & white"), N_("The picture in black and white"), Some(("image.filter", json!({"preset": "mono", "intensity": 100})))),
        ("auto-crop", N_("Fill frame"), N_("The picture fills its frame, cropped where needed"), Some(("image.setFit", json!({"fit": "cover"})))),
        ("brand-colours", N_("Colours → brand"), N_("The picture's main colours become the brand's colours"), Some(("brand.fromPicture", json!({})))),
    ];
    let width = ui.available_width();
    let side = ((width - 16.0) / 3.0).floor();
    egui::Grid::new("pic_actions").spacing(egui::vec2(8.0, 8.0)).show(ui, |ui| {
        for (i, (illustration, name, hint, cmd)) in actions.into_iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(side, side + 18.0), egui::Sense::click());
            let pic = egui::Rect::from_min_size(rect.min, egui::vec2(side, side * 0.78));
            tile_back(ui, t, pic, false, r.hovered());
            let ink = if r.hovered() { t.accent } else { t.text_dim };
            if illustration == "brand-colours" {
                // Three swatches with a picture beside them.
                let c = pic.center();
                ui.painter().rect_filled(
                    egui::Rect::from_center_size(c - egui::vec2(10.0, 0.0), egui::vec2(26.0, 22.0)),
                    3,
                    ink.gamma_multiply(0.35),
                );
                for (k, a) in [1.0_f32, 0.6, 0.3].into_iter().enumerate() {
                    ui.painter().circle_filled(c + egui::vec2(16.0, -10.0 + k as f32 * 10.0), 4.0, ink.gamma_multiply(a));
                }
            } else {
                art::action(&mut art, ui.painter(), t, illustration, pic.shrink(4.0), ink);
            }
            let ink = if r.hovered() { t.accent } else { t.text };
            let galley = ui.painter().layout(tr(name).to_string(), egui::FontId::proportional(11.0), ink, side);
            ui.painter().galley(egui::pos2(rect.center().x - galley.size().x / 2.0, pic.bottom() + 3.0), galley, ink);
            let r = r.on_hover_text(tr(hint));
            match cmd {
                Some((cmd, p)) => {
                    if r.clicked() {
                        acts.run(cmd, p);
                    }
                }
                None => {
                    egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                        ui.set_min_width(250.0);
                        label(ui, t, tr("Put the subject on"));
                        if let Some(c) = swatches(ui, t, genha_color::Color::WHITE, brand) {
                            acts.run("image.changeBackground", json!({"color": c.to_hex()}));
                            ui.close();
                        }
                    });
                }
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
}

/// The picture's other tools as two columns of buttons, each opening a pop-up.
fn tools(ui: &mut egui::Ui, t: &Tokens, el: &Element, img: &ImageEl, acts: &mut Actions) {
    let id = el.id.0;
    let key = |what: &str| format!("{what}:{id}");
    let half = ((ui.available_width() - 8.0) / 2.0).floor();
    let button = |ui: &mut egui::Ui, icon: ToolIcon, text: &str| -> egui::Response {
        let (rect, r) = ui.allocate_exact_size(egui::vec2(half, 36.0), egui::Sense::click());
        tile_back(ui, t, rect, false, r.hovered());
        let ink = if r.hovered() { t.accent } else { t.text };
        tool_icon(ui.painter(), egui::Rect::from_center_size(rect.left_center() + egui::vec2(20.0, 0.0), egui::vec2(16.0, 16.0)), icon, ink);
        ui.painter().text(rect.left_center() + egui::vec2(36.0, 0.0), egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(13.0), ink);
        r
    };
    egui::Grid::new("pic_tools").spacing(egui::vec2(8.0, 8.0)).show(ui, |ui| {
        let r = button(ui, ToolIcon::Adjust, tr("Adjust"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(260.0);
            let a = &img.adjustments;
            for (name, field, v, lo, hi) in [
                (N_("Brightness"), "brightness", a.brightness, -100.0, 100.0),
                (N_("Contrast"), "contrast", a.contrast, -100.0, 100.0),
                (N_("Saturation"), "saturation", a.saturation, -100.0, 100.0),
                (N_("Warmth"), "warmth", a.warmth, -100.0, 100.0),
                (N_("Sharpen"), "sharpen", a.sharpen, 0.0, 100.0),
                (N_("Blur"), "blur", a.blur, 0.0, 100.0),
            ] {
                let mut x = v;
                if ui.add(egui::Slider::new(&mut x, lo..=hi).text(tr(name))).changed() {
                    acts.run("image.adjust", json!({field: x, "merge_key": key(field)}));
                }
            }
            if ui.button(tr("Reset adjustments")).clicked() {
                acts.run("image.adjust", json!({"reset": true}));
            }
        });
        let r = button(ui, ToolIcon::Crop, tr("Crop & shape"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(260.0);
            let c = img.crop;
            let (mut cx, mut cy, mut cw, mut ch) = (c.x, c.y, c.width, c.height);
            let mut changed = false;
            changed |= ui.add(egui::Slider::new(&mut cx, 0.0..=0.95).text(tr("Left"))).changed();
            changed |= ui.add(egui::Slider::new(&mut cy, 0.0..=0.95).text(tr("Top"))).changed();
            changed |= ui.add(egui::Slider::new(&mut cw, 0.05..=1.0).text(tr("Width"))).changed();
            changed |= ui.add(egui::Slider::new(&mut ch, 0.05..=1.0).text(tr("Height"))).changed();
            if changed {
                acts.run("image.crop", json!({"x": cx, "y": cy, "width": cw, "height": ch, "merge_key": key("crop")}));
            }
            ui.add_space(6.0);
            label(ui, t, tr("Shape"));
            ui.horizontal_wrapped(|ui| {
                for (name, shape) in [
                    (N_("None"), ""),
                    (N_("Circle"), "circle"),
                    (N_("Rounded"), "rounded"),
                    (N_("Star"), "star"),
                    (N_("Hexagon"), "polygon"),
                    (N_("Triangle"), "triangle"),
                ] {
                    if ui.button(tr(name)).clicked() {
                        acts.run("image.mask", if shape.is_empty() { json!({"none": true}) } else { json!({"shape": shape}) });
                    }
                }
            });
        });
        ui.end_row();
        let r = button(ui, ToolIcon::Corners, tr("Corners"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(240.0);
            let mut radius = img.corner_radius;
            let most = (el.frame.width.min(el.frame.height) / 2.0).max(1.0);
            if ui.add(egui::Slider::new(&mut radius, 0.0..=most).text(tr("Corners"))).changed() {
                acts.run("style.cornerRadius", json!({"radius": radius, "merge_key": key("radius")}));
            }
        });
        let r = button(ui, ToolIcon::Fit, tr("Fit"));
        egui::Popup::menu(&r).show(|ui| {
            ui.set_min_width(200.0);
            if ui.button(tr("Fill frame")).clicked() {
                acts.run("image.setFit", json!({"fit": "cover"}));
            }
            if ui.button(tr("Fit inside")).clicked() {
                acts.run("image.setFit", json!({"fit": "contain"}));
            }
        });
        ui.end_row();
        let r = button(ui, ToolIcon::Position, tr("Position"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(240.0);
            label(ui, t, tr("Align on the page"));
            super::align_buttons(ui, acts, "page");
            ui.add_space(6.0);
            super::common_frame(ui, el, acts);
        });
        let r = button(ui, ToolIcon::Opacity, tr("Opacity"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(240.0);
            let mut o = el.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut o, 0.0..=100.0).suffix("%").text(tr("Opacity"))).changed() {
                acts.run("object.setOpacity", json!({"opacity": o / 100.0, "merge_key": key("opacity")}));
            }
        });
        ui.end_row();
    });
}

/// The filters as previews of this picture, three to a row; the chosen one's strength.
fn filters(app: &mut App, ui: &mut egui::Ui, t: &Tokens, el: &Element, img: &ImageEl, acts: &mut Actions) {
    let Some(d) = app.session.active() else { return };
    let doc = d.doc.clone();
    let page = d.page;
    let design = d.recovery_id;
    let ctx = ui.ctx().clone();
    label(ui, t, tr("Filters"));
    let side = ((ui.available_width() - 16.0) / 3.0).floor();
    let current = img.filter.as_deref().unwrap_or("none");
    egui::Grid::new("pic_filters").spacing(egui::vec2(8.0, 8.0)).show(ui, |ui| {
        for (i, f) in genha_effects::filter_presets().into_iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(side, side + 18.0), egui::Sense::click());
            let pic = egui::Rect::from_min_size(rect.min, egui::vec2(side, side));
            let on = f.id == current;
            tile_back(ui, t, pic, on, r.hovered());
            // This picture with the filter: the object alone, its adjustments replaced.
            let key = format!("filter:{design}:{}:{}:{}", el.id.0, img.asset.0, f.id);
            let stamp = (img.crop.x.to_bits() ^ img.crop.width.to_bits()).rotate_left(7) ^ u64::from(img.mask.is_some());
            let fonts = app.session.fonts.clone();
            let renderer = &mut app.renderer;
            let adjustments = f.adjustments.clone();
            let tex = app.thumbs.get(&ctx, &key, stamp, || {
                let mut preview = (*doc).clone();
                let target = preview.element_mut(el.id).ok()?;
                if let ElementKind::Image(im) = &mut target.kind {
                    im.adjustments = adjustments;
                }
                target.opacity = 1.0;
                renderer.render_element(&preview, page, el.id, &fonts, 160).ok()
            });
            if let Some(tex) = &tex {
                let size = tex.size_vec2();
                let shown = thumbs::fit_size(size.x, size.y, pic.shrink(5.0).size());
                ui.painter().image(
                    tex.id(),
                    egui::Rect::from_center_size(pic.center(), shown),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            ui.painter().text(
                egui::pos2(rect.center().x, pic.bottom() + 9.0),
                egui::Align2::CENTER_CENTER,
                genha_i18n::trc("filter", f.label),
                egui::FontId::proportional(11.0),
                if on { t.accent } else { t.text },
            );
            if r.clicked() && !on {
                app.editor.filter_strength = None;
                acts.run("image.filter", json!({"preset": f.id, "intensity": 100}));
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
    if current != "none" {
        ui.add_space(10.0);
        // The engine keeps only the resulting adjustments: the strength is remembered here.
        let mut strength = app.editor.filter_strength.unwrap_or(100.0);
        if ui.add(egui::Slider::new(&mut strength, 0.0..=100.0).suffix("%").text(tr("Strength"))).changed() {
            app.editor.filter_strength = Some(strength);
            acts.run("image.filter", json!({"preset": current, "intensity": strength, "merge_key": format!("filterstrength:{}", el.id.0)}));
        }
    }
}

/// The small icons on the tool buttons.
#[derive(Clone, Copy)]
enum ToolIcon {
    Adjust,
    Crop,
    Corners,
    Fit,
    Position,
    Opacity,
}

fn tool_icon(p: &egui::Painter, r: egui::Rect, icon: ToolIcon, ink: egui::Color32) {
    let s = egui::Stroke::new(1.6, ink);
    let (l, rt, top, bot, c) = (r.left(), r.right(), r.top(), r.bottom(), r.center());
    match icon {
        ToolIcon::Adjust => {
            for (i, knob) in [0.3_f32, 0.7, 0.45].into_iter().enumerate() {
                let y = top + 3.0 + i as f32 * 5.0;
                p.line_segment([egui::pos2(l, y), egui::pos2(rt, y)], s);
                p.circle_filled(egui::pos2(l + r.width() * knob, y), 2.6, ink);
            }
        }
        ToolIcon::Crop => {
            p.line_segment([egui::pos2(l + 4.0, top), egui::pos2(l + 4.0, bot - 4.0)], s);
            p.line_segment([egui::pos2(l + 4.0, bot - 4.0), egui::pos2(rt, bot - 4.0)], s);
            p.line_segment([egui::pos2(l, top + 4.0), egui::pos2(rt - 4.0, top + 4.0)], s);
            p.line_segment([egui::pos2(rt - 4.0, top + 4.0), egui::pos2(rt - 4.0, bot)], s);
        }
        ToolIcon::Corners => {
            let pts: Vec<egui::Pos2> = (0..=8)
                .map(|i| {
                    let a = std::f32::consts::FRAC_PI_2 * (2.0 + i as f32 / 8.0);
                    egui::pos2(rt + (r.width() - 2.0) * a.cos(), bot + (r.height() - 2.0) * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
        }
        ToolIcon::Fit => {
            p.rect_stroke(r.shrink(1.0), 2, s, egui::StrokeKind::Inside);
            p.line_segment([c - egui::vec2(4.0, 4.0), c + egui::vec2(4.0, 4.0)], s);
            p.line_segment([c + egui::vec2(4.0, 4.0), c + egui::vec2(4.0, 0.5)], s);
            p.line_segment([c + egui::vec2(4.0, 4.0), c + egui::vec2(0.5, 4.0)], s);
        }
        ToolIcon::Position => {
            p.line_segment([egui::pos2(l + 1.0, top), egui::pos2(l + 1.0, bot)], s);
            p.rect_filled(egui::Rect::from_min_size(egui::pos2(l + 4.0, top + 2.0), egui::vec2(10.0, 4.0)), 1, ink);
            p.rect_filled(egui::Rect::from_min_size(egui::pos2(l + 4.0, top + 9.0), egui::vec2(6.0, 4.0)), 1, ink);
        }
        ToolIcon::Opacity => {
            p.circle_stroke(c, r.width() / 2.0 - 1.0, s);
            let half: Vec<egui::Pos2> = std::iter::once(c)
                .chain((0..=12).map(|i| {
                    let a = std::f32::consts::PI * (-0.5 + i as f32 / 12.0);
                    c + egui::vec2(a.cos(), a.sin()) * (r.width() / 2.0 - 1.0)
                }))
                .collect();
            p.add(egui::Shape::convex_polygon(half, ink, egui::Stroke::NONE));
        }
    }
}
