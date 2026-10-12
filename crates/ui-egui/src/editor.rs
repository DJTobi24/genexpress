//! The editor: top bar, left panels, page strip, status bar, command palette, shortcuts.

use serde_json::{Value, json};

use crate::theme::{Tokens, to_egui};
use crate::{App, LeftTab, io, thumbs};
use genha_doc::{ElementKind, NodeId, preset};

#[derive(Default)]
pub struct EditorState {
    pub palette_open: bool,
    pub palette_query: String,
    pub resize_copy: bool,
    pub qr_text: String,
    pub template_query: String,
    pub renaming: Option<String>,
    pub new_brand_color: [u8; 3],
    pub versions_open: bool,
    pub version_name: String,
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    app.thumbs.begin_frame();
    shortcuts(app, ui.ctx());
    top_bar(app, ui, &t);
    status_bar(app, ui, &t);
    page_strip(app, ui, &t);
    left_rail(app, ui, &t);
    egui::Panel::left("left_panel").resizable(true).default_size(290.0).min_size(240.0).frame(panel_frame(&t)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| left_panel(app, ui, &t));
    });
    egui::Panel::right("properties").resizable(true).default_size(290.0).min_size(240.0).frame(panel_frame(&t)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| crate::properties::show(app, ui, &t));
    });
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| crate::canvas::show(app, ui, &t));
    palette(app, ui.ctx(), &t);
    versions_window(app, ui.ctx(), &t);
}

fn panel_frame(t: &Tokens) -> egui::Frame {
    egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 10)).stroke(egui::Stroke::new(1.0, t.border))
}

fn top_bar(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::top("top_bar")
        .exact_size(48.0)
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 8)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button(egui::RichText::new("GenHa").strong().color(t.accent)).on_hover_text("Home").clicked() {
                    app.screen = crate::Screen::Home;
                }
                ui.separator();
                let title = app.session.active().map(|d| d.doc.title.clone()).unwrap_or_default();
                match &mut app.editor.renaming {
                    Some(name) => {
                        let r = ui.add(egui::TextEdit::singleline(name).desired_width(200.0));
                        r.request_focus();
                        if r.lost_focus() {
                            let name = name.clone();
                            app.editor.renaming = None;
                            app.run("document.rename", &json!({"title": name}));
                        }
                    }
                    None => {
                        let dirty = app.session.active().is_some_and(|d| d.dirty);
                        if ui
                            .add(
                                egui::Label::new(egui::RichText::new(format!("{title}{}", if dirty { " •" } else { "" })).strong())
                                    .sense(egui::Sense::click()),
                            )
                            .on_hover_text("Click to rename")
                            .clicked()
                        {
                            app.editor.renaming = Some(title);
                        }
                    }
                }
                ui.separator();
                let can_undo = app.session.active().is_some_and(|d| !d.history.undo.is_empty());
                let can_redo = app.session.active().is_some_and(|d| !d.history.redo.is_empty());
                if ui.add_enabled(can_undo, egui::Button::new("Undo")).on_hover_text("Undo (Ctrl+Z)").clicked() {
                    app.run("edit.undo", &json!({}));
                }
                if ui.add_enabled(can_redo, egui::Button::new("Redo")).on_hover_text("Redo (Ctrl+Shift+Z)").clicked() {
                    app.run("edit.redo", &json!({}));
                }
                ui.separator();
                resize_menu(app, ui);
                ui.separator();
                tools(app, ui, t);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button(egui::RichText::new("Download").strong().color(t.accent), |ui| download_menu(app, ui))
                        .response
                        .on_hover_text("Export as PNG, JPEG, SVG or PDF");
                    if ui.button("Versions").on_hover_text("Version history").clicked() {
                        app.editor.versions_open = !app.editor.versions_open;
                    }
                    if ui.button("Save").on_hover_text("Save (Ctrl+S)").clicked() {
                        save(app, false);
                    }
                    ui.separator();
                    if ui.button("Fit").on_hover_text("Fit page (Ctrl+0)").clicked() {
                        app.view.fit_pending = true;
                    }
                    ui.label(format!("{:.0}%", app.view.zoom * 100.0));
                    if ui.button("+").clicked() {
                        crate::canvas::zoom_by(app, 1.25, None);
                    }
                    if ui.button("-").clicked() {
                        crate::canvas::zoom_by(app, 0.8, None);
                    }
                    if ui.button("Commands").on_hover_text("All commands (Ctrl+K)").clicked() {
                        app.editor.palette_open = true;
                    }
                });
            });
        });
}

fn tools(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    use genha_tools::ToolId;
    for tool in ToolId::ALL {
        let icon = tool.label();
        let on = app.session.tools.tool == tool;
        let b = egui::Button::new(egui::RichText::new(icon).color(if on { t.accent_text } else { t.text })).selected(on).fill(if on {
            t.accent
        } else {
            t.panel_alt
        });
        if ui.add(b).on_hover_text(format!("{} ({})", tool.label(), tool.shortcut())).clicked() {
            app.run("tool.set", &json!({"tool": tool.id()}));
        }
    }
}

fn resize_menu(app: &mut App, ui: &mut egui::Ui) {
    ui.menu_button("Resize", |ui| {
        ui.checkbox(&mut app.editor.resize_copy, "Keep this page, add a resized copy");
        ui.separator();
        for cat in preset::categories() {
            ui.menu_button(cat, |ui| {
                for p in preset::PRESETS.iter().filter(|p| p.category == cat) {
                    if ui.button(format!("{}  ({:.0}×{:.0})", p.label, p.width, p.height)).clicked() {
                        let copy = app.editor.resize_copy;
                        app.run("page.resize", &json!({"preset": p.id, "copy": copy}));
                        app.view.fit_pending = true;
                        ui.close();
                    }
                }
            });
        }
    });
}

fn download_menu(app: &mut App, ui: &mut egui::Ui) {
    let name = app.session.active().map(|d| io::file_stem(&d.doc.title)).unwrap_or_else(|| "design".into());
    let entries: [(&str, &str, &str, Value); 6] = [
        ("PNG", "export.png", "png", json!({})),
        ("PNG — transparent background", "export.png", "png", json!({"transparent": true})),
        ("PNG — 2× size", "export.png", "png", json!({"scale": 2.0})),
        ("JPEG", "export.jpeg", "jpg", json!({})),
        ("SVG", "export.svg", "svg", json!({})),
        ("PDF — all pages", "export.pdf", "pdf", json!({})),
    ];
    for (label, cmd, ext, extra) in entries {
        if ui.button(label).clicked() {
            ui.close();
            if let Some(path) = io::pick_save(&format!("{name}.{ext}"), (ext, &[ext])) {
                let mut p = extra;
                if let Some(o) = p.as_object_mut() {
                    o.insert("path".into(), json!(path.display().to_string()));
                }
                if app.run(cmd, &p).is_some() {
                    app.set_status(format!("Downloaded {}", path.display()));
                }
            }
        }
    }
}

pub fn save(app: &mut App, save_as: bool) {
    let has_path = app.session.active().is_some_and(|d| d.path.is_some());
    if has_path && !save_as {
        app.run("file.save", &json!({}));
        return;
    }
    let name = app.session.active().map(|d| io::file_stem(&d.doc.title)).unwrap_or_else(|| "design".into());
    if let Some(path) = io::pick_save(&format!("{name}.genha"), io::DESIGNS)
        && app.run("file.saveAs", &json!({"path": path.display().to_string()})).is_some()
    {
        app.set_status(format!("Saved {}", path.display()));
    }
}

fn status_bar(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::bottom("status").exact_size(24.0).frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 3))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                if let Some(d) = app.session.active() {
                    if let Some(p) = d.doc.pages.get(d.page) {
                        ui.colored_label(
                            t.text_dim,
                            format!("{}  ·  {:.0} × {:.0} px  ·  page {} of {}", p.name, p.width, p.height, d.page + 1, d.doc.pages.len()),
                        );
                    }
                    if !d.selection.is_empty() {
                        ui.colored_label(t.text_dim, format!("·  {} selected", d.selection.len()));
                    }
                }
                if let Some((msg, at)) = &app.status
                    && crate::now() - at < 8.0
                {
                    ui.separator();
                    ui.colored_label(t.text, msg);
                }
            });
        },
    );
}

fn page_strip(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::bottom("pages")
        .exact_size(112.0)
        .frame(egui::Frame::new().fill(t.panel_alt).inner_margin(egui::Margin::symmetric(12, 8)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            let Some(d) = app.session.active() else { return };
            let doc = d.doc.clone();
            let current = d.page;
            let revision = d.revision;
            let ctx = ui.ctx().clone();
            let mut goto = None;
            let mut action: Option<(&str, Value)> = None;
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, page) in doc.pages.iter().enumerate() {
                        let fonts = app.session.fonts.clone();
                        let stamp = if i == current { revision } else { 0 };
                        let key = format!("page:{}:{}", d_key(&doc), page.id.0);
                        let tex = app.thumbs.get(&ctx, &key, stamp, || genha_render::render_thumbnail(&doc, i, &fonts, 160).ok());
                        let size = thumbs::fit_size(page.width as f32, page.height as f32, egui::vec2(110.0, 72.0));
                        let (rect, r) = ui.allocate_exact_size(egui::vec2(116.0, 92.0), egui::Sense::click());
                        let img = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 40.0), size);
                        match &tex {
                            Some(tex) => {
                                ui.painter().image(
                                    tex.id(),
                                    img,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            }
                            None => {
                                ui.painter().rect_filled(img, 2, t.border);
                            }
                        }
                        let stroke = if i == current { egui::Stroke::new(2.5, t.accent) } else { egui::Stroke::new(1.0, t.border) };
                        ui.painter().rect_stroke(img.expand(2.0), 4, stroke, egui::StrokeKind::Outside);
                        ui.painter().text(
                            egui::pos2(rect.center().x, rect.bottom() - 8.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", i + 1),
                            egui::FontId::proportional(11.0),
                            t.text_dim,
                        );
                        if r.clicked() {
                            goto = Some(i);
                        }
                        r.context_menu(|ui| {
                            if ui.button("Duplicate page").clicked() {
                                action = Some(("page.duplicate", json!({"index": i})));
                                ui.close();
                            }
                            if ui.button("Delete page").clicked() {
                                action = Some(("page.delete", json!({"index": i})));
                                ui.close();
                            }
                            if i > 0 && ui.button("Move left").clicked() {
                                action = Some(("page.move", json!({"from": i, "to": i - 1})));
                                ui.close();
                            }
                            if i + 1 < doc.pages.len() && ui.button("Move right").clicked() {
                                action = Some(("page.move", json!({"from": i, "to": i + 1})));
                                ui.close();
                            }
                        });
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("+\nAdd page").size(13.0)).min_size(egui::vec2(90.0, 80.0))).clicked() {
                        action = Some(("page.add", json!({})));
                    }
                });
            });
            if let Some(i) = goto {
                app.run("page.goto", &json!({"index": i}));
                app.view.fit_pending = true;
            }
            if let Some((cmd, p)) = action {
                app.run(cmd, &p);
                app.view.fit_pending = true;
            }
        });
}

/// A key that changes when another document is active (thumbnails per document).
fn d_key(doc: &std::sync::Arc<genha_doc::Document>) -> u64 {
    doc.pages.first().map(|p| p.id.0).unwrap_or(0) ^ (doc.next_id << 20)
}

fn left_rail(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::left("rail")
        .exact_size(72.0)
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(6, 10)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                for tab in LeftTab::ALL {
                    let on = app.left_tab == tab;
                    let text = egui::RichText::new(tab.label()).size(12.0).color(if on { t.accent } else { t.text_dim });
                    let b = egui::Button::new(text)
                        .min_size(egui::vec2(80.0, 40.0))
                        .fill(if on { t.accent_soft } else { egui::Color32::TRANSPARENT })
                        .frame(on);
                    if ui.add(b).clicked() {
                        app.left_tab = tab;
                    }
                    ui.add_space(2.0);
                }
            });
        });
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(16.0).strong());
    ui.add_space(4.0);
}

fn left_panel(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    match app.left_tab {
        LeftTab::Templates => templates_tab(app, ui, t),
        LeftTab::Elements => elements_tab(app, ui, t),
        LeftTab::Text => text_tab(app, ui, t),
        LeftTab::Media => media_tab(app, ui, t),
        LeftTab::Brand => brand_tab(app, ui, t),
        LeftTab::Layers => layers_tab(app, ui, t),
    }
}

fn templates_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, "Templates");
    ui.add(egui::TextEdit::singleline(&mut app.editor.template_query).hint_text("Search templates"));
    ui.label(egui::RichText::new("Click: open as a new design · right-click: add as a page").size(11.0).color(t.text_dim));
    let q = app.editor.template_query.to_lowercase();
    let ctx = ui.ctx().clone();
    let mut chosen: Option<(&'static str, bool)> = None;
    ui.horizontal_wrapped(|ui| {
        for tp in genha_templates::catalog() {
            if !q.is_empty()
                && !tp.name.to_lowercase().contains(&q)
                && !tp.category.to_lowercase().contains(&q)
                && !tp.tags.iter().any(|g| g.contains(&q))
            {
                continue;
            }
            let fonts = app.session.fonts.clone();
            let tex = app.thumbs.get(&ctx, &format!("template:{}", tp.id), 0, || {
                let doc = genha_templates::instantiate(tp.id, None)?;
                genha_render::render_thumbnail(&doc, 0, &fonts, 360).ok()
            });
            let size = thumbs::fit_size(tp.width as f32, tp.height as f32, egui::vec2(122.0, 122.0));
            let (rect, r) = ui.allocate_exact_size(egui::vec2(126.0, 126.0), egui::Sense::click());
            let img = egui::Rect::from_center_size(rect.center(), size);
            match &tex {
                Some(tex) => {
                    ui.painter().image(tex.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
                None => {
                    ui.painter().rect_filled(img, 4, t.border);
                }
            }
            if r.hovered() {
                ui.painter().rect_stroke(img, 4, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Outside);
            }
            let r = r.on_hover_text(format!("{} — {}", tp.name, tp.category));
            if r.clicked() {
                chosen = Some((tp.id, false));
            }
            if r.secondary_clicked() {
                chosen = Some((tp.id, true));
            }
        }
    });
    if let Some((id, as_page)) = chosen {
        app.run("template.use", &json!({"id": id, "as_page": as_page}));
        app.view.fit_pending = true;
    }
}

fn elements_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, "Shapes & icons");
    let ctx = ui.ctx().clone();
    let mut add: Option<&'static str> = None;
    let lib = genha_templates::shape_library();
    let mut cats: Vec<&str> = vec![];
    for s in &lib {
        if !cats.contains(&s.category) {
            cats.push(s.category);
        }
    }
    for cat in cats {
        ui.label(egui::RichText::new(cat).color(t.text_dim).size(12.0));
        ui.horizontal_wrapped(|ui| {
            for item in lib.iter().filter(|s| s.category == cat) {
                let fonts = app.session.fonts.clone();
                let accent = crate::theme::from_egui(t.accent);
                let tex = app.thumbs.get(&ctx, &format!("shape:{}:{}", item.id, t.dark), 0, || {
                    let mut doc = genha_doc::Document::new("shape", 100.0, 100.0);
                    if let Some(p) = doc.pages.first_mut() {
                        p.background = genha_doc::Fill::None;
                    }
                    let id = doc.alloc_id();
                    let is_line = matches!(item.shape, genha_doc::ShapeKind::Line);
                    let shape = genha_doc::ShapeEl {
                        shape: item.shape.clone(),
                        fill: if is_line { genha_doc::Fill::None } else { genha_doc::Fill::solid(accent) },
                        stroke: is_line.then(|| genha_doc::StrokeStyle { color: accent, width: 8.0, dash: vec![] }),
                        shadow: None,
                    };
                    let el = genha_doc::Element::new(
                        id,
                        genha_doc::Frame::new(10.0, if is_line { 46.0 } else { 10.0 }, 80.0, if is_line { 8.0 } else { 80.0 }),
                        ElementKind::Shape(shape),
                    );
                    doc.pages.first_mut()?.elements.push(el);
                    genha_render::render_thumbnail(&doc, 0, &fonts, 96).ok()
                });
                let (rect, r) = ui.allocate_exact_size(egui::vec2(54.0, 54.0), egui::Sense::click());
                if r.hovered() {
                    ui.painter().rect_filled(rect, 8, t.accent_soft);
                }
                if let Some(tex) = &tex {
                    ui.painter().image(
                        tex.id(),
                        rect.shrink(5.0),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                if r.on_hover_text(item.label).clicked() {
                    add = Some(item.id);
                }
            }
        });
    }
    if let Some(id) = add {
        app.run("library.addShape", &json!({"id": id}));
    }
    ui.separator();
    heading(ui, "QR code");
    ui.add(egui::TextEdit::singleline(&mut app.editor.qr_text).hint_text("https://…"));
    if ui.add_enabled(!app.editor.qr_text.trim().is_empty(), egui::Button::new("Add QR code")).clicked() {
        let text = app.editor.qr_text.trim().to_string();
        app.run("element.qrCode", &json!({"text": text}));
    }
    ui.separator();
    heading(ui, "Collage");
    let pictures =
        app.session.active().map(|d| d.selected_elements().iter().filter(|e| matches!(e.kind, ElementKind::Image(_))).count()).unwrap_or(0);
    if pictures == 0 {
        ui.colored_label(t.text_dim, "Select pictures on the page to arrange them as a collage.");
    } else {
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            for l in genha_layout::collage_layouts(pictures) {
                let (rect, r) = ui.allocate_exact_size(egui::vec2(60.0, 60.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 6, t.panel_alt);
                for c in &l.cells {
                    let cell = egui::Rect::from_min_max(
                        egui::pos2(rect.left() + 4.0 + c.x0 as f32 * 52.0, rect.top() + 4.0 + c.y0 as f32 * 52.0),
                        egui::pos2(rect.left() + 4.0 + c.x1 as f32 * 52.0, rect.top() + 4.0 + c.y1 as f32 * 52.0),
                    );
                    ui.painter().rect_filled(cell.shrink(1.0), 2, if r.hovered() { t.accent } else { t.border });
                }
                if r.on_hover_text(l.label).clicked() {
                    chosen = Some(l.id);
                }
            }
        });
        if let Some(id) = chosen {
            app.run("layout.collage", &json!({"layout": id}));
        }
    }
}

fn text_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, "Text");
    for (role, label, size) in [("heading", "Add a heading", 22.0), ("subheading", "Add a subheading", 17.0), ("body", "Add body text", 13.0)] {
        let b = egui::Button::new(egui::RichText::new(label).size(size).strong()).min_size(egui::vec2(ui.available_width(), 40.0));
        if ui.add(b).clicked() {
            let text = match role {
                "heading" => "Your headline",
                "subheading" => "A subheading",
                _ => "Write something here",
            };
            app.run("text.add", &json!({"text": text, "role": role}));
        }
    }
    ui.separator();
    heading(ui, "Text styles");
    for p in genha_templates::text_presets() {
        let color = to_egui(p.style.color);
        let mut rt = egui::RichText::new(p.sample).size((p.style.font_size as f32 / 4.0).clamp(11.0, 26.0));
        if p.style.bold {
            rt = rt.strong();
        }
        if p.style.italic {
            rt = rt.italics();
        }
        let readable = if t.dark && color.r() < 80 && color.g() < 80 && color.b() < 80 { t.text } else { color };
        let b = egui::Button::new(rt.color(readable)).min_size(egui::vec2(ui.available_width(), 36.0));
        if ui.add(b).on_hover_text(p.label).clicked() {
            app.run("text.addPreset", &json!({"id": p.id}));
        }
    }
}

fn media_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, "Your pictures");
    if ui
        .add(
            egui::Button::new(egui::RichText::new("Upload picture…").color(t.accent_text))
                .fill(t.accent)
                .min_size(egui::vec2(ui.available_width(), 36.0)),
        )
        .clicked()
        && let Some(p) = io::pick_open(&[io::PICTURES])
    {
        app.run("image.place", &json!({"path": p.display().to_string()}));
    }
    ui.label(egui::RichText::new("…or drop pictures on the window.").size(11.0).color(t.text_dim));
    ui.add_space(6.0);
    let Some(d) = app.session.active() else { return };
    let assets: Vec<(u64, String, u32, u32)> = d.doc.assets.iter().map(|(id, a)| (id.0, a.name.clone(), a.width, a.height)).collect();
    let doc = d.doc.clone();
    let ctx = ui.ctx().clone();
    let mut place = None;
    ui.horizontal_wrapped(|ui| {
        for (id, name, w, h) in assets {
            let tex = app.thumbs.get(&ctx, &format!("asset:{}:{id}", d_key(&doc)), 0, || {
                let a = doc.assets.get(&genha_doc::AssetId(id))?;
                let img = genha_effects::decode(&a.data).ok()?;
                let img = genha_effects::resize(&img, 160, 160, genha_geom::Fit::Contain).ok()?;
                Some(genha_render::Raster { width: img.width(), height: img.height(), rgba: img.into_raw() })
            });
            let (rect, r) = ui.allocate_exact_size(egui::vec2(80.0, 80.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 6, t.panel_alt);
            if let Some(tex) = tex {
                let sz = thumbs::fit_size(w as f32, h as f32, egui::vec2(76.0, 76.0));
                ui.painter().image(
                    tex.id(),
                    egui::Rect::from_center_size(rect.center(), sz),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            if r.on_hover_text(name).clicked() {
                place = Some(id);
            }
        }
    });
    if let Some(id) = place {
        app.run("image.placeAsset", &json!({"asset": id}));
    }
}

fn brand_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, "Brand kit");
    let Some(d) = app.session.active() else { return };
    let brand = d.doc.brand.clone();
    ui.label(egui::RichText::new(&brand.name).strong());
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Colours").color(t.text_dim));
    let mut set: Option<(usize, genha_color::Color)> = None;
    let mut remove = None;
    for (i, c) in brand.colors.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut col = to_egui(c.color);
            if ui.color_edit_button_srgba(&mut col).changed() {
                set = Some((i, crate::theme::from_egui(col)));
            }
            ui.label(format!("{}  {}", c.name, c.color.to_hex()));
            if ui.small_button("x").on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some((i, c)) = set {
        app.run("brand.setColor", &json!({"index": i, "color": c.to_hex(), "merge_key": format!("brandcolor:{i}")}));
    }
    if let Some(i) = remove {
        app.run("brand.removeColor", &json!({"index": i}));
    }
    ui.horizontal(|ui| {
        let [r, g, b] = app.editor.new_brand_color;
        let mut col = egui::Color32::from_rgb(r, g, b);
        ui.color_edit_button_srgba(&mut col);
        app.editor.new_brand_color = [col.r(), col.g(), col.b()];
        if ui.button("Add colour").clicked() {
            let c = genha_color::Color::rgb(col.r(), col.g(), col.b());
            app.run("brand.addColor", &json!({"color": c.to_hex()}));
        }
    });
    let has_picture = app.session.active().is_some_and(|d| d.selected_elements().iter().any(|e| matches!(e.kind, ElementKind::Image(_))));
    if ui.add_enabled(has_picture, egui::Button::new("Colours from selected picture")).clicked() {
        app.run("brand.fromPicture", &json!({}));
    }
    ui.separator();
    ui.label(egui::RichText::new("Fonts").color(t.text_dim));
    let families = app.font_families().to_vec();
    let (mut heading_font, mut body_font) = (brand.heading_font.clone(), brand.body_font.clone());
    font_combo(ui, "brand_heading", "Headings", &mut heading_font, &families);
    font_combo(ui, "brand_body", "Body", &mut body_font, &families);
    if heading_font != brand.heading_font || body_font != brand.body_font {
        app.run("brand.setFonts", &json!({"heading": heading_font, "body": body_font}));
    }
    ui.separator();
    if ui.button("Apply brand to this page").clicked() {
        app.run("brand.apply", &json!({}));
    }
    if ui.button("Apply brand to all pages").clicked() {
        app.run("brand.apply", &json!({"all_pages": true}));
    }
    ui.separator();
    heading(ui, "Palettes");
    let mut chosen = None;
    for p in genha_templates::palettes() {
        let r = ui
            .horizontal(|ui| {
                for c in &p.colors {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4, to_egui(*c));
                }
                ui.label(p.name);
            })
            .response
            .interact(egui::Sense::click());
        if r.on_hover_text("Use as brand colours").clicked() {
            chosen = Some(p.id);
        }
    }
    if let Some(id) = chosen {
        app.run("brand.usePalette", &json!({"id": id}));
    }
}

pub(crate) fn font_combo(ui: &mut egui::Ui, id: &str, label: &str, value: &mut String, families: &[String]) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id).selected_text(value.as_str()).width(170.0).height(320.0).show_ui(ui, |ui| {
            for f in families {
                ui.selectable_value(value, f.clone(), f);
            }
        });
    });
}

fn layers_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, "Layers");
    let Some(d) = app.session.active() else { return };
    let Some(page) = d.doc.pages.get(d.page) else { return };
    let sel = d.selection.clone();
    let rows: Vec<(NodeId, String, &'static str, bool, bool)> =
        page.elements.iter().rev().map(|e| (e.id, e.display_name(), e.kind.type_name(), e.hidden, e.locked)).collect();
    if rows.is_empty() {
        ui.colored_label(t.text_dim, "This page is empty. Add text, shapes or pictures from the left.");
        return;
    }
    let mut action: Option<(&str, Value)> = None;
    for (id, name, kind, hidden, locked) in rows {
        let on = sel.contains(&id);
        ui.horizontal(|ui| {
            let eye = if hidden { "show" } else { "hide" };
            if ui.small_button(eye).on_hover_text(if hidden { "Show" } else { "Hide" }).clicked() {
                action = Some((if hidden { "object.show" } else { "object.hide" }, json!({"ids": [id.0]})));
            }
            let lock = if locked { "unlock" } else { "lock" };
            if ui.small_button(lock).on_hover_text(if locked { "Unlock" } else { "Lock" }).clicked() {
                action = Some((if locked { "object.unlock" } else { "object.lock" }, json!({"ids": [id.0]})));
            }
            let text = egui::RichText::new(format!("{name}  ")).color(if hidden { t.text_dim } else { t.text });
            if ui.selectable_label(on, text).on_hover_text(kind).clicked() {
                let ids = if ui.input(|i| i.modifiers.shift) {
                    let mut s: Vec<u64> = sel.iter().map(|i| i.0).collect();
                    s.push(id.0);
                    s
                } else {
                    vec![id.0]
                };
                action = Some(("select.set", json!({"ids": ids})));
            }
        });
    }
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        for (label, cmd) in [
            ("Forward", "object.bringForward"),
            ("Backward", "object.sendBackward"),
            ("To front", "object.bringToFront"),
            ("To back", "object.sendToBack"),
        ] {
            if ui.add_enabled(!sel.is_empty(), egui::Button::new(label)).clicked() {
                action = Some((cmd, json!({})));
            }
        }
    });
    if let Some((cmd, p)) = action {
        app.run(cmd, &p);
    }
}

// ---------- keyboard ----------

fn parse_shortcut(s: &str) -> Option<egui::KeyboardShortcut> {
    let mut mods = egui::Modifiers::NONE;
    let mut key = None;
    for part in s.split('+') {
        match part {
            "Cmd" | "Ctrl" => mods.command = true,
            "Shift" => mods.shift = true,
            "Alt" => mods.alt = true,
            k => key = egui::Key::from_name(k),
        }
    }
    key.map(|k| egui::KeyboardShortcut::new(mods, k))
}

fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() || app.canvas.editing_text.is_some() {
        return;
    }
    let mut todo: Vec<(&'static str, Value)> = vec![];
    // Single keys without modifiers: tools.
    ctx.input_mut(|i| {
        for spec in genha_engine::command_specs() {
            let Some(sc) = spec.shortcut.and_then(parse_shortcut) else { continue };
            if sc.modifiers.is_none() && sc.logical_key != egui::Key::Delete && sc.logical_key != egui::Key::Escape {
                continue; // single letters are tool keys, handled below
            }
            if i.consume_shortcut(&sc) {
                todo.push((spec.id, json!({})));
            }
        }
        for tool in genha_tools::ToolId::ALL {
            if let Some(k) = egui::Key::from_name(tool.shortcut())
                && i.consume_key(egui::Modifiers::NONE, k)
            {
                todo.push(("tool.set", json!({"tool": tool.id()})));
            }
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace) {
            todo.push(("edit.delete", json!({})));
        }
        for (key, dx, dy) in
            [(egui::Key::ArrowLeft, -1.0, 0.0), (egui::Key::ArrowRight, 1.0, 0.0), (egui::Key::ArrowUp, 0.0, -1.0), (egui::Key::ArrowDown, 0.0, 1.0)]
        {
            for (mods, step) in [(egui::Modifiers::NONE, 1.0), (egui::Modifiers::SHIFT, 10.0)] {
                if i.consume_key(mods, key) {
                    todo.push(("object.move", json!({"dx": dx * step, "dy": dy * step, "merge_key": "nudge"})));
                }
            }
        }
        if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::K)) {
            app.editor.palette_open = true;
        }
        if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Num0)) {
            app.view.fit_pending = true;
        }
    });
    for (id, p) in todo {
        match id {
            "file.save" => save(app, false),
            "file.saveAs" => save(app, true),
            "file.open" => {
                if let Some(path) = io::pick_open(&[io::DESIGNS, io::PICTURES]) {
                    app.run("file.open", &json!({"path": path.display().to_string()}));
                }
            }
            "image.place" => {
                if let Some(path) = io::pick_open(&[io::PICTURES]) {
                    app.run("image.place", &json!({"path": path.display().to_string()}));
                }
            }
            "file.new" => app.screen = crate::Screen::Home,
            "select.none" => {
                app.session.tools.cancel();
                app.session.preview = None;
                app.run(id, &p);
            }
            "edit.delete" | "object.move" if app.session.active().is_none_or(|d| d.selection.is_empty()) => {}
            _ => {
                let enabled = genha_engine::find_command(id).is_some_and(|s| (s.enabled)(&app.session).is_ok());
                if enabled {
                    app.run(id, &p);
                }
            }
        }
    }
}

fn palette(app: &mut App, ctx: &egui::Context, t: &Tokens) {
    if !app.editor.palette_open {
        return;
    }
    let mut open = true;
    let mut run: Option<&'static str> = None;
    egui::Window::new("Commands")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .show(ctx, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut app.editor.palette_query).hint_text("Type a command…").desired_width(f32::INFINITY));
            r.request_focus();
            let q = app.editor.palette_query.to_lowercase();
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                let mut first = None;
                for spec in genha_engine::command_specs() {
                    if spec.menu.is_empty() || !(spec.label.to_lowercase().contains(&q) || spec.id.contains(&q)) {
                        continue;
                    }
                    let enabled = (spec.enabled)(&app.session).is_ok();
                    first.get_or_insert(spec.id);
                    let label = format!("{}{}", spec.label, spec.shortcut.map(|s| format!("    {s}")).unwrap_or_default());
                    if ui.add_enabled(enabled, egui::Button::new(label).frame(false)).on_hover_text(spec.id).clicked() {
                        run = Some(spec.id);
                    }
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    run = first;
                }
            });
            ui.colored_label(t.text_dim, "Commands that need parameters are easier from the panels.");
        });
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.editor.palette_open = false;
    }
    if let Some(id) = run {
        app.editor.palette_open = false;
        app.editor.palette_query.clear();
        match id {
            "file.save" => save(app, false),
            "file.saveAs" => save(app, true),
            _ => {
                app.run(id, &json!({}));
            }
        }
    }
}

fn versions_window(app: &mut App, ctx: &egui::Context, t: &Tokens) {
    if !app.editor.versions_open {
        return;
    }
    let list = app.session.execute("version.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let mut open = true;
    let mut action: Option<(&str, Value)> = None;
    egui::Window::new("Version history").open(&mut open).default_width(360.0).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.editor.version_name).hint_text("Name this version").desired_width(200.0));
            if ui.button("Save version").clicked() {
                action = Some(("version.save", json!({"name": app.editor.version_name.trim()})));
                app.editor.version_name.clear();
            }
        });
        ui.label(egui::RichText::new("Versions are kept inside the design file. Saving adds an automatic version.").size(11.0).color(t.text_dim));
        ui.separator();
        if list.is_empty() {
            ui.colored_label(t.text_dim, "No versions yet.");
        }
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for v in &list {
                let id = v["id"].as_u64().unwrap_or(0);
                let name = v["name"].as_str().unwrap_or("");
                let bookmarked = v["bookmarked"].as_bool().unwrap_or(false);
                let when = age(v["created"].as_u64().unwrap_or(0));
                ui.horizontal(|ui| {
                    let label = egui::RichText::new(format!("{}{name}", if bookmarked { "* " } else { "" })).strong();
                    ui.label(label);
                    ui.colored_label(t.text_dim, format!("{when} · {} pages", v["pages"].as_u64().unwrap_or(0)));
                });
                ui.horizontal(|ui| {
                    if ui.small_button("Restore").clicked() {
                        action = Some(("version.restore", json!({"id": id})));
                    }
                    if ui.small_button("Open as copy").clicked() {
                        action = Some(("version.copy", json!({"id": id})));
                    }
                    if !bookmarked && ui.small_button("Keep").on_hover_text("Bookmark: never pruned").clicked() {
                        action = Some(("version.rename", json!({"id": id, "name": name, "bookmarked": true})));
                    }
                    if ui.small_button("Delete").clicked() {
                        action = Some(("version.delete", json!({"id": id})));
                    }
                });
                ui.add_space(4.0);
            }
        });
    });
    if !open {
        app.editor.versions_open = false;
    }
    if let Some((cmd, p)) = action {
        app.run(cmd, &p);
        app.view.fit_pending = cmd != "version.save" && cmd != "version.rename" && cmd != "version.delete";
    }
}

/// "5 min ago" for a version time.
fn age(created: u64) -> String {
    let now = crate::now() as u64;
    if created == 0 || created > now {
        return "unknown time".into();
    }
    match now - created {
        s if s < 60 => "just now".into(),
        s if s < 3600 => format!("{} min ago", s / 60),
        s if s < 86_400 => format!("{} h ago", s / 3600),
        s => format!("{} days ago", s / 86_400),
    }
}
