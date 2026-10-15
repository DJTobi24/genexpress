//! The left panel's tabs (Templates, Elements, Text, Media, Brand, Layers) in the style of the
//! object panels: a large title, a rounded search field, category chips, pictures on rounded tiles
//! in a grid, and small headings over groups.

use serde_json::{Value, json};

use crate::glyphs::{self, Glyph};
use crate::theme::{Tokens, to_egui};
use crate::{App, io, thumbs};
use genha_doc::ElementKind;
use genha_i18n::{N_, tr};

/// The tab's title.
fn title(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.label(egui::RichText::new(text).size(19.0).strong().color(t.text));
    ui.add_space(10.0);
}

/// A small heading over a group.
fn section(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.add_space(14.0);
    ui.label(egui::RichText::new(text).size(12.5).strong().color(t.text_dim));
    ui.add_space(6.0);
}

/// A rounded search field with a magnifier.
fn search(ui: &mut egui::Ui, t: &Tokens, text: &mut String, hint: &str) {
    egui::Frame::new().fill(t.panel_alt).corner_radius(12).inner_margin(egui::Margin::symmetric(10, 7)).show(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            let c = rect.center() - egui::vec2(1.5, 1.5);
            ui.painter().circle_stroke(c, 5.0, egui::Stroke::new(1.6, t.text_dim));
            ui.painter().line_segment([c + egui::vec2(3.6, 3.6), c + egui::vec2(7.0, 7.0)], egui::Stroke::new(1.8, t.text_dim));
            ui.add(egui::TextEdit::singleline(text).hint_text(hint).frame(egui::Frame::NONE).desired_width(f32::INFINITY));
        });
    });
}

/// A row of chips; the one at `chosen` is filled. Returns the chip clicked.
fn chips(ui: &mut egui::Ui, t: &Tokens, labels: &[&str], chosen: usize) -> Option<usize> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (i, label) in labels.iter().enumerate() {
            let on = i == chosen;
            let b = egui::Button::new(egui::RichText::new(*label).size(12.0).color(if on { t.accent_text } else { t.text }))
                .fill(if on { t.accent } else { t.panel_alt })
                .corner_radius(14)
                .min_size(egui::vec2(0.0, 28.0));
            if ui.add(b).clicked() {
                out = Some(i);
            }
        }
    });
    out
}

/// A tile's background: soft when hovered.
fn tile(ui: &egui::Ui, t: &Tokens, rect: egui::Rect, hovered: bool) {
    ui.painter().rect_filled(rect, 10, if hovered { t.accent_soft } else { t.panel_alt });
}

/// Draws `tex` (or a placeholder) inside `rect`, keeping its proportions.
fn picture(ui: &egui::Ui, t: &Tokens, tex: Option<&egui::TextureHandle>, rect: egui::Rect, w: f32, h: f32) {
    match tex {
        Some(tex) => {
            let size = thumbs::fit_size(w, h, rect.size());
            ui.painter().image(
                tex.id(),
                egui::Rect::from_center_size(rect.center(), size),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => {
            ui.painter().rect_filled(rect.shrink(4.0), 6, t.border);
        }
    }
}

/// Columns of `n` tiles across the panel's width, `gap` apart: the tile width.
fn column_width(ui: &egui::Ui, n: usize, gap: f32) -> f32 {
    ((ui.available_width() - gap * (n.saturating_sub(1)) as f32) / n.max(1) as f32).floor()
}

// ---------------------------------------------------------------- templates

pub fn templates(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    title(ui, t, tr("Templates"));
    search(ui, t, &mut app.editor.template_query, tr("Search templates"));
    ui.add_space(8.0);
    let catalog = genha_templates::catalog();
    let mut categories: Vec<&'static str> = vec![];
    for tp in &catalog {
        if !categories.contains(&tp.category) {
            categories.push(tp.category);
        }
    }
    let mut labels = vec![tr("All")];
    labels.extend(categories.iter().map(|c| tr(c)));
    let chosen = app.editor.template_category.and_then(|c| categories.iter().position(|x| *x == c)).map_or(0, |i| i + 1);
    if let Some(i) = chips(ui, t, &labels, chosen) {
        app.editor.template_category = i.checked_sub(1).and_then(|i| categories.get(i).copied());
    }
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(tr("Click: use (asks first when your design has content) · right-click: add as a page")).size(11.0).color(t.text_dim),
    );
    ui.add_space(8.0);
    let q = app.editor.template_query.to_lowercase();
    let category = app.editor.template_category;
    let ctx = ui.ctx().clone();
    let mut chosen: Option<(&'static str, bool)> = None;
    let w = column_width(ui, 2, 10.0);
    let shown: Vec<_> = catalog
        .into_iter()
        .filter(|tp| category.is_none_or(|c| c == tp.category))
        .filter(|tp| {
            q.is_empty()
                || tp.name.to_lowercase().contains(&q)
                || tp.category.to_lowercase().contains(&q)
                || tr(tp.category).to_lowercase().contains(&q)
                || tp.tags.iter().any(|g| g.contains(&q))
        })
        .collect();
    if shown.is_empty() {
        ui.label(egui::RichText::new(tr("No templates match.")).color(t.text_dim));
    }
    egui::Grid::new("tab_templates").spacing(egui::vec2(10.0, 12.0)).show(ui, |ui| {
        for (i, tp) in shown.iter().enumerate() {
            let fonts = app.session.fonts.clone();
            let id = tp.id;
            let tex = app.thumbs.get(&ctx, &format!("template:{id}"), 0, || {
                let doc = genha_templates::instantiate(id, None)?;
                genha_render::render_thumbnail(&doc, 0, &fonts, 360).ok()
            });
            let (rect, r) = ui.allocate_exact_size(egui::vec2(w, w + 22.0), egui::Sense::click());
            let pic = egui::Rect::from_min_size(rect.min, egui::vec2(w, w));
            tile(ui, t, pic, r.hovered());
            picture(ui, t, tex.as_ref(), pic.shrink(8.0), tp.width as f32, tp.height as f32);
            let galley = ui.painter().layout(tp.name.to_string(), egui::FontId::proportional(11.5), t.text, w);
            ui.painter().galley(egui::pos2(rect.left() + 2.0, pic.bottom() + 4.0), galley, t.text);
            let r = r.on_hover_text(format!("{} — {}", tp.name, tr(tp.category)));
            if r.clicked() {
                chosen = Some((id, false));
            }
            if r.secondary_clicked() {
                chosen = Some((id, true));
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    if let Some((id, as_page)) = chosen {
        let has_content = app.session.active().is_some_and(|d| d.doc.pages.iter().any(|p| !p.elements.is_empty()));
        if as_page {
            app.run("template.use", &json!({"id": id, "as_page": true}));
            app.view.fit_pending = true;
        } else if has_content {
            // Never replace someone's work without asking.
            app.editor.template_ask = Some(id);
        } else {
            // An empty design simply takes the template.
            app.run("template.use", &json!({"id": id, "replace": true}));
            app.view.fit_pending = true;
        }
    }
}

// ---------------------------------------------------------------- elements

/// The chart kinds offered, with their hints.
const CHARTS: [(&str, &str, &str); 8] = [
    (N_("Bar"), "bar", N_("Add a bar chart")),
    (N_("Horizontal bar"), "horizontal-bar", N_("Add a horizontal bar chart")),
    (N_("Line"), "line", N_("Add a line chart")),
    (N_("Area"), "area", N_("Add an area chart")),
    (N_("Pie"), "pie", N_("Add a pie chart")),
    (N_("Donut"), "donut", N_("Add a donut chart")),
    (N_("Progress"), "progress", N_("Add a progress bar")),
    (N_("Ring"), "ring", N_("Add a progress ring")),
];

pub fn elements(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    title(ui, t, tr("Elements"));
    let ctx = ui.ctx().clone();
    let mut add: Option<&'static str> = None;
    let lib = genha_templates::shape_library();
    let mut cats: Vec<&str> = vec![];
    for s in &lib {
        if !cats.contains(&s.category) {
            cats.push(s.category);
        }
    }
    let side = column_width(ui, 5, 6.0);
    for cat in cats {
        section(ui, t, tr(cat));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
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
                let (rect, r) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
                tile(ui, t, rect, r.hovered());
                picture(ui, t, tex.as_ref(), rect.shrink(side * 0.18), 1.0, 1.0);
                if r.on_hover_text(tr(item.label)).clicked() {
                    add = Some(item.id);
                }
            }
        });
    }
    if let Some(id) = add {
        app.run("library.addShape", &json!({"id": id}));
    }

    section(ui, t, tr("Charts & tables"));
    let w = column_width(ui, 3, 8.0);
    egui::Grid::new("tab_charts").spacing(egui::vec2(8.0, 8.0)).show(ui, |ui| {
        let mut n = 0;
        for (label, kind, hint) in CHARTS.iter().copied().chain(std::iter::once((N_("Table"), "table", N_("Add a 3 x 3 table")))) {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(w, 70.0), egui::Sense::click());
            tile(ui, t, rect, r.hovered());
            let ink = if r.hovered() { t.accent } else { t.text_dim };
            chart_icon(ui.painter(), egui::Rect::from_center_size(rect.center() - egui::vec2(0.0, 9.0), egui::vec2(34.0, 26.0)), kind, ink, t);
            ui.painter().text(
                rect.center_bottom() - egui::vec2(0.0, 11.0),
                egui::Align2::CENTER_CENTER,
                tr(label),
                egui::FontId::proportional(11.0),
                t.text,
            );
            if r.on_hover_text(tr(hint)).clicked() {
                if kind == "table" {
                    app.run("table.add", &json!({}));
                } else {
                    app.run("chart.add", &json!({"kind": kind}));
                }
            }
            n += 1;
            if n % 3 == 0 {
                ui.end_row();
            }
        }
    });

    section(ui, t, tr("QR code"));
    ui.horizontal(|ui| {
        let field = egui::TextEdit::singleline(&mut app.editor.qr_text).hint_text("https://…").desired_width(ui.available_width() - 70.0);
        ui.add(field);
        let ok = !app.editor.qr_text.trim().is_empty();
        let b = egui::Button::new(egui::RichText::new(tr("Add")).color(t.accent_text)).fill(t.accent).corner_radius(10);
        if ui.add_enabled(ok, b).on_hover_text(tr("Add QR code")).clicked() {
            let text = app.editor.qr_text.trim().to_string();
            app.run("element.qrCode", &json!({"text": text}));
        }
    });

    section(ui, t, tr("Collage"));
    let pictures =
        app.session.active().map(|d| d.selected_elements().iter().filter(|e| matches!(e.kind, ElementKind::Image(_))).count()).unwrap_or(0);
    if pictures == 0 {
        ui.label(egui::RichText::new(tr("Select pictures on the page to arrange them as a collage.")).size(11.5).color(t.text_dim));
    } else {
        let mut chosen = None;
        let side = column_width(ui, 4, 8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for l in genha_layout::collage_layouts(pictures) {
                let (rect, r) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
                tile(ui, t, rect, r.hovered());
                let inner = rect.shrink(6.0);
                for c in &l.cells {
                    let cell = egui::Rect::from_min_max(
                        egui::pos2(inner.left() + c.x0 as f32 * inner.width(), inner.top() + c.y0 as f32 * inner.height()),
                        egui::pos2(inner.left() + c.x1 as f32 * inner.width(), inner.top() + c.y1 as f32 * inner.height()),
                    );
                    ui.painter().rect_filled(cell.shrink(1.5), 3, if r.hovered() { t.accent } else { t.border });
                }
                if r.on_hover_text(tr(l.label)).clicked() {
                    chosen = Some(l.id);
                }
            }
        });
        if let Some(id) = chosen {
            app.run("layout.collage", &json!({"layout": id}));
        }
    }
}

/// A small drawing of a chart kind (or a table).
fn chart_icon(p: &egui::Painter, r: egui::Rect, kind: &str, ink: egui::Color32, t: &Tokens) {
    let soft = ink.gamma_multiply(0.45);
    let st = egui::Stroke::new(2.0, ink);
    let (l, top, w, h) = (r.left(), r.top(), r.width(), r.height());
    let at = |x: f32, y: f32| egui::pos2(l + x * w, top + y * h);
    match kind {
        "bar" => {
            for (i, v) in [0.5_f32, 0.8, 0.35, 1.0].into_iter().enumerate() {
                let x = 0.06 + i as f32 * 0.24;
                p.rect_filled(egui::Rect::from_min_max(at(x, 1.0 - v), at(x + 0.16, 1.0)), 2, if i == 3 { ink } else { soft });
            }
        }
        "horizontal-bar" => {
            for (i, v) in [0.6_f32, 1.0, 0.4].into_iter().enumerate() {
                let y = 0.05 + i as f32 * 0.34;
                p.rect_filled(egui::Rect::from_min_max(at(0.0, y), at(v, y + 0.24)), 2, if i == 1 { ink } else { soft });
            }
        }
        "line" | "area" => {
            let pts = vec![at(0.0, 0.8), at(0.3, 0.45), at(0.55, 0.6), at(1.0, 0.1)];
            if kind == "area" {
                // Two convex parts under the line.
                p.add(egui::Shape::convex_polygon(
                    vec![at(0.0, 0.8), at(0.3, 0.45), at(0.55, 0.6), at(0.55, 1.0), at(0.0, 1.0)],
                    soft,
                    egui::Stroke::NONE,
                ));
                p.add(egui::Shape::convex_polygon(vec![at(0.55, 0.6), at(1.0, 0.1), at(1.0, 1.0), at(0.55, 1.0)], soft, egui::Stroke::NONE));
            }
            p.add(egui::Shape::line(pts, st));
        }
        "pie" | "donut" | "ring" => {
            let c = r.center();
            let rad = h * 0.5;
            p.circle_filled(c, rad, soft);
            let slice: Vec<egui::Pos2> = std::iter::once(c)
                .chain((0..=10).map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * 0.7 * i as f32 / 10.0;
                    c + egui::vec2(a.cos(), a.sin()) * rad
                }))
                .collect();
            p.add(egui::Shape::convex_polygon(slice, ink, egui::Stroke::NONE));
            if kind != "pie" {
                p.circle_filled(c, rad * 0.55, t.panel_alt);
            }
        }
        "progress" => {
            let bar = egui::Rect::from_min_max(at(0.0, 0.38), at(1.0, 0.62));
            p.rect_filled(bar, 5, soft);
            p.rect_filled(egui::Rect::from_min_max(bar.min, egui::pos2(bar.left() + bar.width() * 0.65, bar.bottom())), 5, ink);
        }
        _ => {
            // A table: a grid with a coloured header row.
            let grid = r.shrink2(egui::vec2(w * 0.05, 0.0));
            p.rect_filled(egui::Rect::from_min_max(grid.min, egui::pos2(grid.right(), grid.top() + grid.height() / 3.0)), 2, ink);
            p.rect_stroke(grid, 2, egui::Stroke::new(1.4, ink), egui::StrokeKind::Inside);
            for k in 1..3 {
                let y = grid.top() + grid.height() * k as f32 / 3.0;
                p.line_segment([egui::pos2(grid.left(), y), egui::pos2(grid.right(), y)], egui::Stroke::new(1.0, soft));
                let x = grid.left() + grid.width() * k as f32 / 3.0;
                p.line_segment([egui::pos2(x, grid.top()), egui::pos2(x, grid.bottom())], egui::Stroke::new(1.0, soft));
            }
        }
    }
}

// ---------------------------------------------------------------- text

pub fn text(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    title(ui, t, tr("Text"));
    for (role, label, size) in
        [("heading", N_("Add a heading"), 21.0), ("subheading", N_("Add a subheading"), 16.0), ("body", N_("Add body text"), 13.0)]
    {
        let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), size + 26.0), egui::Sense::click());
        tile(ui, t, rect, r.hovered());
        let font = egui::FontId::new(size, egui::FontFamily::Proportional);
        let galley = ui.painter().layout_no_wrap(tr(label).to_string(), font, t.text);
        let pos = egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0);
        if role != "body" {
            // egui has no bold face: the letters drawn twice, a hair apart.
            ui.painter().galley(pos + egui::vec2(0.6, 0.0), galley.clone(), t.text);
        }
        ui.painter().galley(pos, galley, t.text);
        if r.clicked() {
            // The new box's text, in the language of the UI.
            let text = match role {
                "heading" => tr("Your headline"),
                "subheading" => tr("A subheading"),
                _ => tr("Write something here"),
            };
            app.run("text.add", &json!({"text": text, "role": role}));
        }
        ui.add_space(6.0);
    }
    section(ui, t, tr("Text styles"));
    let ctx = ui.ctx().clone();
    let w = column_width(ui, 2, 8.0);
    let mut chosen = None;
    egui::Grid::new("tab_text_styles").spacing(egui::vec2(8.0, 8.0)).show(ui, |ui| {
        for (i, p) in genha_templates::text_presets().into_iter().enumerate() {
            let fonts = app.session.fonts.clone();
            let tex = app
                .thumbs
                .get(&ctx, &format!("textstyle:{}:{}", genha_i18n::language().code(), p.id), 0, || crate::properties::preset_picture(&p, &fonts));
            let (rect, r) = ui.allocate_exact_size(egui::vec2(w, 62.0), egui::Sense::click());
            let light_ink = p.style.color.luminance() > 0.6;
            ui.painter().rect_filled(rect, 10, if light_ink { t.thumb_dark } else { t.thumb_light });
            if r.hovered() {
                ui.painter().rect_stroke(rect, 10, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
            }
            if let Some(tex) = &tex {
                let size = tex.size_vec2();
                picture(ui, t, Some(tex), rect.shrink(6.0), size.x, size.y);
            }
            if r.on_hover_text(tr(p.label)).clicked() {
                chosen = Some((p.id, tr(p.sample)));
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    if let Some((id, sample)) = chosen {
        app.run("text.addPreset", &json!({"id": id, "text": sample}));
    }
    section(ui, t, tr("Fonts"));
    crate::fonts::add_fonts_ui(app, ui);
}

// ---------------------------------------------------------------- media

pub fn media(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    title(ui, t, tr("Media"));
    // A large tile to upload pictures; they can be dropped on the window too.
    let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 104.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 14, if r.hovered() { t.accent_soft } else { t.panel_alt });
    let pts = vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
    for s in egui::Shape::dashed_line(&pts, egui::Stroke::new(1.4, if r.hovered() { t.accent } else { t.border }), 6.0, 4.0) {
        ui.painter().add(s);
    }
    glyphs::draw(
        ui.painter(),
        Glyph::Download,
        egui::Rect::from_center_size(rect.center() - egui::vec2(0.0, 18.0), egui::vec2(24.0, 24.0)),
        t.accent,
    );
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 10.0),
        egui::Align2::CENTER_CENTER,
        tr("Upload picture…"),
        egui::FontId::proportional(14.0),
        t.text,
    );
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 30.0),
        egui::Align2::CENTER_CENTER,
        tr("…or drop pictures on the window."),
        egui::FontId::proportional(11.0),
        t.text_dim,
    );
    if r.clicked()
        && let Some(p) = io::pick_open(&[io::PICTURES])
    {
        app.run("image.place", &json!({"path": p.display().to_string()}));
    }
    let Some(d) = app.session.active() else { return };
    let assets: Vec<(u64, String, u32, u32)> = d.doc.assets.iter().map(|(id, a)| (id.0, a.name.clone(), a.width, a.height)).collect();
    if assets.is_empty() {
        return;
    }
    section(ui, t, tr("Your pictures"));
    let doc = d.doc.clone();
    let design = d.recovery_id;
    let ctx = ui.ctx().clone();
    let mut place = None;
    let side = column_width(ui, 3, 8.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for (id, name, w, h) in assets {
            let tex = app.thumbs.get(&ctx, &format!("asset:{design}:{id}"), 0, || {
                let a = doc.assets.get(&genha_doc::AssetId(id))?;
                let img = genha_effects::decode(&a.data).ok()?;
                let img = genha_effects::resize(&img, 200, 200, genha_geom::Fit::Contain).ok()?;
                Some(genha_render::Raster { width: img.width(), height: img.height(), rgba: img.into_raw() })
            });
            let (rect, r) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
            tile(ui, t, rect, r.hovered());
            picture(ui, t, tex.as_ref(), rect.shrink(4.0), w as f32, h as f32);
            if r.on_hover_text(name).clicked() {
                place = Some(id);
            }
        }
    });
    if let Some(id) = place {
        app.run("image.placeAsset", &json!({"asset": id}));
    }
}

// ---------------------------------------------------------------- brand

pub fn brand(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    title(ui, t, tr("Brand kit"));
    let Some(d) = app.session.active() else { return };
    let brand = d.doc.brand.clone();
    ui.label(egui::RichText::new(crate::shown_name(&brand.name)).strong().color(t.text));
    section(ui, t, tr("Colours"));
    let mut remove = None;
    let mut set: Option<(usize, genha_color::Color)> = None;
    let mut add: Option<genha_color::Color> = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for (i, c) in brand.colors.iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click());
            ui.painter().circle_filled(rect.center(), 15.0, to_egui(c.color));
            ui.painter().circle_stroke(rect.center(), 15.0, egui::Stroke::new(1.0, t.border));
            if r.hovered() {
                ui.painter().circle_stroke(rect.center(), 17.5, egui::Stroke::new(1.5, t.accent));
            }
            let r = r.on_hover_text(format!("{}  {}", crate::shown_name(&c.name), c.color.to_hex()));
            egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                let mut col = to_egui(c.color);
                if egui::widgets::color_picker::color_picker_color32(ui, &mut col, egui::widgets::color_picker::Alpha::Opaque) {
                    set = Some((i, crate::theme::from_egui(col)));
                }
                if ui.button(egui::RichText::new(tr("Remove")).color(t.danger)).clicked() {
                    remove = Some(i);
                    ui.close();
                }
            });
        }
        // A "+" to add a colour.
        let (rect, r) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click());
        ui.painter().circle_filled(rect.center(), 15.0, if r.hovered() { t.accent_soft } else { t.panel_alt });
        glyphs::draw(ui.painter(), Glyph::Plus, egui::Rect::from_center_size(rect.center(), egui::vec2(14.0, 14.0)), t.text);
        let r = r.on_hover_text(tr("Add colour"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            let [rr, g, b] = app.editor.new_brand_color;
            let mut col = egui::Color32::from_rgb(rr, g, b);
            egui::widgets::color_picker::color_picker_color32(ui, &mut col, egui::widgets::color_picker::Alpha::Opaque);
            app.editor.new_brand_color = [col.r(), col.g(), col.b()];
            if ui.button(tr("Add colour")).clicked() {
                add = Some(genha_color::Color::rgb(col.r(), col.g(), col.b()));
                ui.close();
            }
        });
    });
    if let Some((i, c)) = set {
        app.run("brand.setColor", &json!({"index": i, "color": c.to_hex(), "merge_key": format!("brandcolor:{i}")}));
    }
    if let Some(i) = remove {
        app.run("brand.removeColor", &json!({"index": i}));
    }
    if let Some(c) = add {
        app.run("brand.addColor", &json!({"color": c.to_hex()}));
    }
    let has_picture = app.session.active().is_some_and(|d| d.selected_elements().iter().any(|e| matches!(e.kind, ElementKind::Image(_))));
    ui.add_space(6.0);
    if ui.add_enabled(has_picture, chip(t, tr("Colours from selected picture"))).clicked() {
        app.run("brand.fromPicture", &json!({}));
    }

    section(ui, t, tr("Fonts"));
    let families = app.font_families().to_vec();
    let (mut heading_font, mut body_font) = (brand.heading_font.clone(), brand.body_font.clone());
    crate::editor::font_combo(ui, "brand_heading", tr("Headings"), &mut heading_font, &families);
    crate::editor::font_combo(ui, "brand_body", tr("Body"), &mut body_font, &families);
    if heading_font != brand.heading_font || body_font != brand.body_font {
        app.run("brand.setFonts", &json!({"heading": heading_font, "body": body_font}));
    }

    section(ui, t, tr("Apply"));
    ui.horizontal_wrapped(|ui| {
        if ui.add(chip(t, tr("Apply brand to this page"))).clicked() {
            app.run("brand.apply", &json!({}));
        }
        if ui.add(chip(t, tr("Apply brand to all pages"))).clicked() {
            app.run("brand.apply", &json!({"all_pages": true}));
        }
    });

    section(ui, t, tr("Palettes"));
    let mut chosen = None;
    let w = column_width(ui, 2, 8.0);
    egui::Grid::new("tab_palettes").spacing(egui::vec2(8.0, 8.0)).show(ui, |ui| {
        for (i, p) in genha_templates::palettes().into_iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(w, 58.0), egui::Sense::click());
            tile(ui, t, rect, r.hovered());
            let stripe = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(w - 16.0, 24.0));
            let n = p.colors.len().max(1) as f32;
            for (k, c) in p.colors.iter().enumerate() {
                let x0 = stripe.left() + stripe.width() * k as f32 / n;
                let x1 = stripe.left() + stripe.width() * (k + 1) as f32 / n;
                ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(x0, stripe.top()), egui::pos2(x1, stripe.bottom())), 0, to_egui(*c));
            }
            ui.painter().rect_stroke(stripe, 4, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);
            ui.painter().text(
                egui::pos2(rect.left() + 9.0, rect.bottom() - 13.0),
                egui::Align2::LEFT_CENTER,
                tr(p.name),
                egui::FontId::proportional(11.0),
                t.text,
            );
            if r.on_hover_text(tr("Use as brand colours")).clicked() {
                chosen = Some(p.id);
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    if let Some(id) = chosen {
        app.run("brand.usePalette", &json!({"id": id}));
    }
}

/// A rounded button in the tabs' style.
fn chip(t: &Tokens, text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_string()).size(12.5).color(t.text))
        .fill(t.panel_alt)
        .corner_radius(10)
        .min_size(egui::vec2(0.0, 30.0))
}

// ---------------------------------------------------------------- layers

pub fn layers(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    title(ui, t, tr("Layers"));
    let Some(d) = app.session.active() else { return };
    let doc = d.doc.clone();
    let page_index = d.page;
    let design = d.recovery_id;
    let sel = d.selection.clone();
    let Some(page) = doc.pages.get(page_index) else { return };
    if page.elements.is_empty() {
        ui.label(egui::RichText::new(tr("This page is empty. Add text, shapes or pictures from the left.")).color(t.text_dim));
        return;
    }
    ui.label(egui::RichText::new(tr("The front at the top. Drag tiles in the strip beside the canvas to restack.")).size(11.0).color(t.text_dim));
    ui.add_space(8.0);
    let ctx = ui.ctx().clone();
    let mut action: Option<(&str, Value)> = None;
    for el in page.elements.iter().rev() {
        let on = sel.contains(&el.id);
        let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), egui::Sense::click());
        if on {
            ui.painter().rect_filled(rect, 10, t.accent_soft);
        } else if r.hovered() {
            ui.painter().rect_filled(rect, 10, t.panel_alt);
        }
        let pic = egui::Rect::from_min_size(rect.min + egui::vec2(6.0, 6.0), egui::vec2(36.0, 36.0));
        let (tex, light) = crate::layers::picture(app, &ctx, &doc, page_index, design, el);
        ui.painter().rect_filled(pic, 6, if light { t.thumb_dark } else { t.thumb_light });
        if let Some(tex) = &tex {
            let size = tex.size_vec2();
            picture(ui, t, Some(tex), pic.shrink(3.0), size.x, size.y);
        }
        let name = crate::element_name(el);
        let ink = if el.hidden { t.text_dim } else { t.text };
        let galley = ui.painter().layout(name, egui::FontId::proportional(13.0), ink, rect.width() - 130.0);
        ui.painter().galley(egui::pos2(pic.right() + 10.0, rect.center().y - galley.size().y / 2.0), galley, ink);
        // Lock and visibility toggles at the row's end.
        let lock_r = egui::Rect::from_center_size(rect.right_center() - egui::vec2(20.0, 0.0), egui::vec2(30.0, 30.0));
        let eye_r = egui::Rect::from_center_size(rect.right_center() - egui::vec2(54.0, 0.0), egui::vec2(30.0, 30.0));
        let lock = ui.interact(lock_r, ui.id().with(("lock", el.id.0)), egui::Sense::click());
        let eye = ui.interact(eye_r, ui.id().with(("eye", el.id.0)), egui::Sense::click());
        let show_toggles = r.hovered() || lock.hovered() || eye.hovered() || el.locked || el.hidden;
        if show_toggles {
            let dim = |hov: bool, active: bool| {
                if hov {
                    t.accent
                } else if active {
                    t.text
                } else {
                    t.text_dim
                }
            };
            glyphs::draw(ui.painter(), if el.locked { Glyph::Lock } else { Glyph::Unlock }, lock_r.shrink(8.0), dim(lock.hovered(), el.locked));
            glyphs::draw(ui.painter(), if el.hidden { Glyph::EyeOff } else { Glyph::Eye }, eye_r.shrink(8.0), dim(eye.hovered(), el.hidden));
        }
        if lock.on_hover_text(if el.locked { tr("Unlock") } else { tr("Lock") }).clicked() {
            action = Some((if el.locked { "object.unlock" } else { "object.lock" }, json!({"ids": [el.id.0]})));
        } else if eye.on_hover_text(if el.hidden { tr("Show") } else { tr("Hide") }).clicked() {
            action = Some((if el.hidden { "object.show" } else { "object.hide" }, json!({"ids": [el.id.0]})));
        } else if r.on_hover_text(tr(el.kind.type_name())).clicked() {
            let ids = if ui.input(|i| i.modifiers.shift) {
                let mut s: Vec<u64> = sel.iter().map(|i| i.0).collect();
                s.push(el.id.0);
                s
            } else {
                vec![el.id.0]
            };
            action = Some(("select.set", json!({"ids": ids})));
        }
        ui.add_space(2.0);
    }
    section(ui, t, tr("Arrange"));
    ui.horizontal_wrapped(|ui| {
        for (label, cmd) in [
            (N_("Forward"), "object.bringForward"),
            (N_("Backward"), "object.sendBackward"),
            (N_("To front"), "object.bringToFront"),
            (N_("To back"), "object.sendToBack"),
        ] {
            if ui.add_enabled(!sel.is_empty(), chip(t, tr(label))).clicked() {
                action = Some((cmd, json!({})));
            }
        }
    });
    if let Some((cmd, p)) = action {
        app.run(cmd, &p);
    }
}
