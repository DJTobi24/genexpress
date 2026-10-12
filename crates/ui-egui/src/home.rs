//! The home screen: start from a size, a template or a picture; quick actions; open designs.

use serde_json::json;

use crate::theme::Tokens;
use crate::{App, io, thumbs};
use genexpress_doc::preset;

pub struct HomeState {
    pub query: String,
    pub category: Option<&'static str>,
    pub custom_w: f64,
    pub custom_h: f64,
}

impl Default for HomeState {
    fn default() -> Self {
        HomeState { query: String::new(), category: None, custom_w: 1080.0, custom_h: 1080.0 }
    }
}

/// File-in, file-out picture tools.
pub const QUICK_ACTIONS: &[(&str, &str, &str)] = &[
    ("remove-background", "Remove background", "Cut out the subject of a photo with a plain backdrop"),
    ("resize", "Resize picture", "Scale a picture to 50%"),
    ("convert-png", "Convert to PNG", "Any picture to PNG"),
    ("convert-jpeg", "Convert to JPEG", "Any picture to JPEG"),
    ("convert-webp", "Convert to WebP", "Any picture to WebP"),
    ("auto-crop", "Trim borders", "Remove empty or uniform edges"),
    ("grayscale", "Black & white", "Turn a photo black and white"),
];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    app.thumbs.begin_frame();
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.bg).inner_margin(egui::Margin::symmetric(32, 20))).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            header(app, ui, &t);
            ui.add_space(18.0);
            start(app, ui, &t);
            ui.add_space(22.0);
            templates(app, ui, &t);
            ui.add_space(22.0);
            quick_actions(app, ui, &t);
            if !app.session.docs.is_empty() {
                ui.add_space(22.0);
                open_designs(app, ui, &t);
            }
            ui.add_space(30.0);
            if let Some((msg, _)) = &app.status {
                ui.colored_label(t.text_dim, msg);
            }
        });
    });
}

fn header(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("GenExpress").size(30.0).strong().color(t.accent));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Design anything — open source, made in Rust.").size(15.0).color(t.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if app.dark { "☀ Light" } else { "🌙 Dark" }).clicked() {
                app.dark = !app.dark;
            }
            if ui.button("📂 Open…").clicked()
                && let Some(p) = io::pick_open(&[io::DESIGNS, io::PICTURES])
            {
                app.run("file.open", &json!({"path": p.display().to_string()}));
            }
        });
    });
}

fn start(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.label(egui::RichText::new("What do you want to create?").size(20.0).strong());
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.home.query).hint_text("🔍 Search sizes and templates").desired_width(280.0));
        if ui.selectable_label(app.home.category.is_none(), "All").clicked() {
            app.home.category = None;
        }
        for c in preset::categories() {
            if ui.selectable_label(app.home.category == Some(c), c).clicked() {
                app.home.category = Some(c);
            }
        }
    });
    ui.add_space(8.0);
    let q = app.home.query.to_lowercase();
    let cat = app.home.category;
    ui.horizontal_wrapped(|ui| {
        // Custom size first.
        egui::Frame::new().fill(t.panel).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(12).inner_margin(10).show(ui, |ui| {
            ui.set_width(170.0);
            ui.set_height(118.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Custom size").strong());
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut app.home.custom_w).range(1.0..=16384.0).suffix(" px").speed(1.0));
                    ui.label("×");
                });
                ui.add(egui::DragValue::new(&mut app.home.custom_h).range(1.0..=16384.0).suffix(" px").speed(1.0));
                if ui.add(egui::Button::new(egui::RichText::new("Create").color(t.accent_text)).fill(t.accent)).clicked() {
                    let (w, h) = (app.home.custom_w, app.home.custom_h);
                    app.run("file.new", &json!({"width": w, "height": h, "title": "Custom design"}));
                }
            });
        });
        for p in preset::PRESETS {
            if cat.is_some_and(|c| c != p.category) {
                continue;
            }
            if !q.is_empty() && !p.label.to_lowercase().contains(&q) && !p.category.to_lowercase().contains(&q) {
                continue;
            }
            let r = egui::Frame::new()
                .fill(t.panel)
                .stroke(egui::Stroke::new(1.0, t.border))
                .corner_radius(12)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.set_width(130.0);
                    ui.set_height(118.0);
                    ui.vertical_centered(|ui| {
                        let sz = thumbs::fit_size(p.width as f32, p.height as f32, egui::vec2(70.0, 56.0));
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(70.0, 60.0), egui::Sense::hover());
                        let r = egui::Rect::from_center_size(rect.center(), sz);
                        ui.painter().rect_filled(r, 4, t.accent_soft);
                        ui.painter().rect_stroke(r, 4, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
                        ui.label(egui::RichText::new(p.label).size(12.5));
                        ui.label(egui::RichText::new(format!("{:.0} × {:.0}", p.width, p.height)).size(11.0).color(t.text_dim));
                    });
                })
                .response
                .interact(egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if r.clicked() {
                app.run("file.new", &json!({"preset": p.id}));
            }
        }
    });
}

fn templates(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.label(egui::RichText::new("Start from a template").size(20.0).strong());
    ui.add_space(6.0);
    let q = app.home.query.to_lowercase();
    let list: Vec<_> = genexpress_templates::catalog()
        .into_iter()
        .filter(|tp| {
            let preset_cat = preset::find(tp.preset).map(|p| p.category);
            app.home.category.is_none_or(|c| preset_cat == Some(c))
                && (q.is_empty()
                    || tp.name.to_lowercase().contains(&q)
                    || tp.category.to_lowercase().contains(&q)
                    || tp.tags.iter().any(|g| g.contains(&q)))
        })
        .collect();
    if list.is_empty() {
        ui.colored_label(t.text_dim, "No templates match.");
        return;
    }
    let ctx = ui.ctx().clone();
    ui.horizontal_wrapped(|ui| {
        for tp in list {
            let fonts = app.session.fonts.clone();
            let tex = app.thumbs.get(&ctx, &format!("template:{}", tp.id), 0, || {
                let doc = genexpress_templates::instantiate(tp.id, None)?;
                genexpress_render::render_thumbnail(&doc, 0, &fonts, 360).ok()
            });
            let r = template_card(ui, t, tp.name, tp.category, tp.width as f32, tp.height as f32, tex.as_ref());
            if r.clicked() {
                app.run("template.use", &json!({"id": tp.id}));
            }
        }
    });
}

pub(crate) fn template_card(
    ui: &mut egui::Ui,
    t: &Tokens,
    name: &str,
    category: &str,
    w: f32,
    h: f32,
    tex: Option<&egui::TextureHandle>,
) -> egui::Response {
    egui::Frame::new()
        .fill(t.panel)
        .stroke(egui::Stroke::new(1.0, t.border))
        .corner_radius(12)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_width(180.0);
            ui.vertical(|ui| {
                let box_size = egui::vec2(180.0, 150.0);
                let (rect, _) = ui.allocate_exact_size(box_size, egui::Sense::hover());
                ui.painter().rect_filled(rect, 8, t.panel_alt);
                let r = egui::Rect::from_center_size(rect.center(), thumbs::fit_size(w, h, box_size - egui::vec2(12.0, 12.0)));
                match tex {
                    Some(tex) => {
                        ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    }
                    None => {
                        ui.painter().rect_filled(r, 4, t.border);
                    }
                }
                ui.label(egui::RichText::new(name).strong().size(13.0));
                ui.label(egui::RichText::new(category).size(11.0).color(t.text_dim));
            });
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn quick_actions(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.label(egui::RichText::new("Quick actions").size(20.0).strong());
    ui.label(egui::RichText::new("One click, file in, file out — nothing leaves your computer.").color(t.text_dim));
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for (id, label, hint) in QUICK_ACTIONS {
            let r = ui.add(egui::Button::new(egui::RichText::new(*label).size(14.0)).min_size(egui::vec2(170.0, 44.0))).on_hover_text(*hint);
            if r.clicked() {
                run_quick_action(app, id);
            }
        }
    });
}

fn run_quick_action(app: &mut App, action: &str) {
    let Some(input) = io::pick_open(&[io::PICTURES]) else { return };
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into());
    let (ext, params) = match action {
        "remove-background" => ("png", json!({"action": "remove-background"})),
        "resize" => ("png", json!({"action": "resize", "scale": 0.5})),
        "convert-png" => ("png", json!({"action": "convert"})),
        "convert-jpeg" => ("jpg", json!({"action": "convert"})),
        "convert-webp" => ("webp", json!({"action": "convert"})),
        "auto-crop" => ("png", json!({"action": "auto-crop"})),
        _ => ("png", json!({"action": "filter", "preset": "mono"})),
    };
    let Some(output) = io::pick_save(&format!("{stem}-{action}.{ext}"), (ext, &[ext])) else { return };
    let mut p = params;
    if let Some(o) = p.as_object_mut() {
        o.insert("input".into(), json!(input.display().to_string()));
        o.insert("output".into(), json!(output.display().to_string()));
    }
    if app.run("quick.image", &p).is_some() {
        app.set_status(format!("Saved {}", output.display()));
    }
}

fn open_designs(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.label(egui::RichText::new("Open designs").size(20.0).strong());
    ui.add_space(6.0);
    let names: Vec<(usize, String, usize)> = app.session.docs.iter().enumerate().map(|(i, d)| (i, d.display_name(), d.doc.pages.len())).collect();
    ui.horizontal_wrapped(|ui| {
        for (i, name, pages) in names {
            let label = format!("{name}\n{pages} page{}", if pages == 1 { "" } else { "s" });
            if ui.add(egui::Button::new(label).min_size(egui::vec2(170.0, 48.0)).fill(t.panel)).clicked() {
                app.run("document.switch", &json!({"index": i}));
                app.screen = crate::Screen::Editor;
            }
        }
    });
}
