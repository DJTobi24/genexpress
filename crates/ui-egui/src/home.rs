//! The home screen: start from a size, a template or a picture; quick actions; open designs.

use serde_json::json;

use crate::theme::Tokens;
use crate::{App, io, thumbs};
use genha_doc::preset;

pub struct HomeState {
    pub query: String,
    pub category: Option<&'static str>,
    pub custom_w: f64,
    pub custom_h: f64,
    pub all_sizes: bool,
}

impl Default for HomeState {
    fn default() -> Self {
        HomeState { query: String::new(), category: None, custom_w: 1080.0, custom_h: 1080.0, all_sizes: false }
    }
}

/// File-in, file-out picture tools.
pub const QUICK_ACTIONS: &[(&str, &str, &str)] = &[
    ("remove-background", "Remove background", "Cut out the subject of a photo with a plain backdrop"),
    ("resize", "Resize picture", "Scale a picture to 50%"),
    ("convert-png", "Convert to PNG", "Any picture to PNG"),
    ("convert-jpeg", "Convert to JPEG", "Any picture to JPEG"),
    ("convert-webp", "Convert to WebP", "Any picture to WebP"),
    ("blur-background", "Blur background", "Keep the subject sharp, blur what is behind"),
    ("change-background", "White background", "Replace a plain backdrop with white"),
    ("pixelate", "Pixelate", "Big blocky pixels"),
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
            recovered(app, ui, &t);
            recent(app, ui, &t);
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
        ui.label(egui::RichText::new("GenHa").size(30.0).strong().color(t.accent));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Design anything — open source, made in Rust.").size(15.0).color(t.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if app.dark { "Light mode" } else { "Dark mode" }).clicked() {
                app.dark = !app.dark;
            }
            if ui.button("Open…").clicked()
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
        ui.add(egui::TextEdit::singleline(&mut app.home.query).hint_text("Search sizes and templates").desired_width(280.0));
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
    let mut items: Vec<Option<&'static preset::PagePreset>> = vec![None];
    items.extend(
        preset::PRESETS
            .iter()
            .filter(|p| cat.is_none_or(|c| c == p.category))
            .filter(|p| q.is_empty() || p.label.to_lowercase().contains(&q) || p.category.to_lowercase().contains(&q))
            .map(Some),
    );
    // Without a filter, the most used sizes; "More sizes" shows them all.
    let limited = cat.is_none() && q.is_empty() && !app.home.all_sizes;
    if limited {
        let popular = [
            "instagram-post",
            "instagram-story",
            "facebook-post",
            "youtube-thumbnail",
            "linkedin-post",
            "flyer-a4",
            "poster-a3",
            "presentation",
            "logo",
            "business-card",
        ];
        items.retain(|i| i.is_none_or(|p| popular.contains(&p.id)));
    }
    let mut create: Option<serde_json::Value> = None;
    rows(ui, items, 152.0, |ui, item| match item {
        None => {
            if let Some(p) = custom_card(app, ui, t) {
                create = Some(p);
            }
        }
        Some(p) => {
            if preset_card(ui, t, p).clicked() {
                create = Some(json!({"preset": p.id}));
            }
        }
    });
    if ui.link(if app.home.all_sizes { "Fewer sizes" } else { "More sizes…" }).clicked() {
        app.home.all_sizes = !app.home.all_sizes;
    }
    if let Some(p) = create {
        app.run("file.new", &p);
    }
}

/// Lays `items` out in rows of cards `item_w` wide (outer width), as many per row as fit.
fn rows<T>(ui: &mut egui::Ui, items: Vec<T>, item_w: f32, mut f: impl FnMut(&mut egui::Ui, T)) {
    let gap = ui.spacing().item_spacing.x;
    let per = (((ui.available_width() + gap) / (item_w + gap)).floor() as usize).max(1);
    let mut it = items.into_iter().peekable();
    while it.peek().is_some() {
        ui.horizontal(|ui| {
            for _ in 0..per {
                if let Some(x) = it.next() {
                    f(ui, x);
                }
            }
        });
        ui.add_space(4.0);
    }
}

fn custom_card(app: &mut App, ui: &mut egui::Ui, t: &Tokens) -> Option<serde_json::Value> {
    let mut out = None;
    egui::Frame::new().fill(t.panel).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(12).inner_margin(10).show(ui, |ui| {
        ui.set_width(130.0);
        ui.set_height(118.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Custom size").strong());
            ui.add(egui::DragValue::new(&mut app.home.custom_w).range(1.0..=16384.0).prefix("W ").suffix(" px").speed(1.0));
            ui.add(egui::DragValue::new(&mut app.home.custom_h).range(1.0..=16384.0).prefix("H ").suffix(" px").speed(1.0));
            if ui.add(egui::Button::new(egui::RichText::new("Create").color(t.accent_text)).fill(t.accent)).clicked() {
                out = Some(json!({"width": app.home.custom_w, "height": app.home.custom_h, "title": "Custom design"}));
            }
        });
    });
    out
}

fn preset_card(ui: &mut egui::Ui, t: &Tokens, p: &preset::PagePreset) -> egui::Response {
    egui::Frame::new()
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
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn templates(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.label(egui::RichText::new("Start from a template").size(20.0).strong());
    ui.add_space(6.0);
    let q = app.home.query.to_lowercase();
    let list: Vec<_> = genha_templates::catalog()
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
    let mut chosen = None;
    rows(ui, list, 198.0, |ui, tp| {
        let fonts = app.session.fonts.clone();
        let tex = app.thumbs.get(&ctx, &format!("template:{}", tp.id), 0, || {
            let doc = genha_templates::instantiate(tp.id, None)?;
            genha_render::render_thumbnail(&doc, 0, &fonts, 360).ok()
        });
        if template_card(ui, t, tp.name, tp.category, tp.width as f32, tp.height as f32, tex.as_ref()).clicked() {
            chosen = Some(tp.id);
        }
    });
    if let Some(id) = chosen {
        app.run("template.use", &json!({"id": id}));
    }
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
    let (ext, params) = match action {
        "remove-background" => ("png", json!({"action": "remove-background"})),
        "resize" => ("png", json!({"action": "resize", "scale": 0.5})),
        "convert-png" => ("png", json!({"action": "convert"})),
        "convert-jpeg" => ("jpg", json!({"action": "convert"})),
        "convert-webp" => ("webp", json!({"action": "convert"})),
        "auto-crop" => ("png", json!({"action": "auto-crop"})),
        "blur-background" => ("png", json!({"action": "blur-background"})),
        "change-background" => ("png", json!({"action": "change-background", "color": "#ffffff"})),
        "pixelate" => ("png", json!({"action": "pixelate"})),
        _ => ("png", json!({"action": "filter", "preset": "mono"})),
    };
    // In the browser the picture arrives later and the result is downloaded (web.rs).
    #[cfg(target_arch = "wasm32")]
    crate::web::pick(crate::web::Purpose::Quick { action: action.to_string(), ext, params: params.clone() }, &[io::PICTURES]);
    #[cfg(not(target_arch = "wasm32"))]
    let Some(input) = io::pick_open(&[io::PICTURES]) else { return };
    #[cfg(target_arch = "wasm32")]
    let Some(input) = None::<std::path::PathBuf> else { return };
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into());
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

fn recovered(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    if app.recovered.is_empty() {
        return;
    }
    let mut action: Option<(&str, String)> = None;
    egui::Frame::new().fill(t.accent_soft).corner_radius(12).inner_margin(12).show(ui, |ui| {
        ui.label(egui::RichText::new("Recovered designs").size(17.0).strong());
        ui.label(egui::RichText::new("GenHa closed unexpectedly last time. These designs had unsaved changes.").color(t.text_dim));
        for (path, title) in &app.recovered {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).strong());
                if ui.button("Open").clicked() {
                    action = Some(("recovery.open", path.clone()));
                }
                if ui.button("Discard").clicked() {
                    action = Some(("recovery.discard", path.clone()));
                }
            });
        }
    });
    ui.add_space(14.0);
    if let Some((cmd, path)) = action {
        app.recovered.retain(|(p, _)| *p != path);
        app.run(cmd, &json!({"path": path}));
        if cmd == "recovery.open" {
            app.screen = crate::Screen::Editor;
            app.view.fit_pending = true;
        }
    }
}

fn recent(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let files: Vec<std::path::PathBuf> = app.recent.iter().filter(|p| p.exists()).cloned().collect();
    if files.is_empty() {
        return;
    }
    ui.label(egui::RichText::new("Recent designs").size(20.0).strong());
    ui.add_space(6.0);
    let mut open = None;
    ui.horizontal_wrapped(|ui| {
        for p in files {
            let name = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if ui
                .add(egui::Button::new(egui::RichText::new(name).size(14.0)).min_size(egui::vec2(170.0, 40.0)).fill(t.panel))
                .on_hover_text(p.display().to_string())
                .clicked()
            {
                open = Some(p);
            }
        }
    });
    ui.add_space(18.0);
    if let Some(p) = open {
        app.run("file.open", &json!({"path": p.display().to_string()}));
    }
}
