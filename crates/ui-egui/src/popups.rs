//! The home screen's pop-ups (size picker, video and presentation starters, QR code, renaming and
//! deleting designs and folders) and its search field with the results under it.

use serde_json::json;

use crate::App;
use crate::art::{Art, Icon};
use crate::home::{Page, create, go, run_quick_action, use_template};
use crate::theme::{FOLDER_COLORS, Tokens};
use genha_doc::preset;
use genha_engine::cmd::designs::FolderFilter;
use genha_i18n::{tr, tr_n};

/// A pop-up over the home screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Modal {
    /// Pick a size (or type one) for a new design; with a palette from the Brand page.
    SizePicker {
        query: String,
        palette: Option<String>,
    },
    Video,
    Presentation,
    Qr {
        text: String,
    },
    RenameDesign {
        id: String,
        title: String,
    },
    DeleteDesigns {
        ids: Vec<String>,
        title: String,
        stored: usize,
    },
    /// `color`: 0 for none, else 1 + an index into [`FOLDER_COLORS`].
    NewFolder {
        name: String,
        color: usize,
    },
    RenameFolder {
        id: String,
        name: String,
    },
    FolderColor {
        id: String,
        name: String,
    },
    DeleteFolder {
        id: String,
        name: String,
        delete_designs: bool,
    },
}

impl Modal {
    /// The name the control channel uses for it.
    pub fn id(&self) -> &'static str {
        match self {
            Modal::SizePicker { .. } => "size",
            Modal::Video => "video",
            Modal::Presentation => "presentation",
            Modal::Qr { .. } => "qr",
            Modal::RenameDesign { .. } => "rename-design",
            Modal::DeleteDesigns { .. } => "delete-designs",
            Modal::NewFolder { .. } => "new-folder",
            Modal::RenameFolder { .. } => "rename-folder",
            Modal::FolderColor { .. } => "folder-color",
            Modal::DeleteFolder { .. } => "delete-folder",
        }
    }
}

pub fn open_size_picker(app: &mut App, palette: Option<String>) {
    app.home.search_open = false;
    app.home.modal = Some(Modal::SizePicker { query: String::new(), palette });
}

/// Draws the open pop-up, if any.
pub fn show(app: &mut App, ctx: &egui::Context, t: &Tokens) {
    let Some(mut modal) = app.home.modal.take() else { return };
    let screen = ctx.content_rect();
    let wide = matches!(modal, Modal::SizePicker { .. } | Modal::Video | Modal::Presentation);
    let width = if wide { (screen.width() - 80.0).clamp(320.0, 900.0) } else { (screen.width() - 80.0).clamp(280.0, 440.0) };
    let frame = egui::Frame::new().fill(t.panel).corner_radius(18).inner_margin(22).stroke(egui::Stroke::new(1.0, t.border));
    let mut keep = true;
    let r = egui::Modal::new(egui::Id::new("home_modal")).frame(frame).backdrop_color(t.scrim).show(ctx, |ui| {
        ui.set_width(width);
        match &mut modal {
            Modal::SizePicker { query, palette } => {
                let palette = palette.clone();
                keep = size_picker(app, ui, t, query, palette.as_deref());
            }
            Modal::Video => keep = video(app, ui, t),
            Modal::Presentation => keep = presentation(app, ui, t),
            Modal::Qr { text } => keep = qr(app, ui, t, text),
            Modal::RenameDesign { id, title } => match rename(ui, t, tr("Rename design"), title) {
                None => {}
                Some(None) => keep = false,
                Some(Some(name)) => {
                    app.run("design.rename", &json!({"id": id, "title": name}));
                    keep = false;
                }
            },
            Modal::DeleteDesigns { ids, title, stored } => keep = delete_designs(app, ui, t, ids, title, *stored),
            Modal::NewFolder { name, color } => keep = new_folder(app, ui, t, name, color),
            Modal::RenameFolder { id, name } => match rename(ui, t, tr("Rename folder"), name) {
                None => {}
                Some(None) => keep = false,
                Some(Some(name)) => {
                    app.run("folder.rename", &json!({"id": id, "name": name}));
                    keep = false;
                }
            },
            Modal::FolderColor { id, name } => keep = folder_color(app, ui, t, id, name),
            Modal::DeleteFolder { id, name, delete_designs } => keep = delete_folder(app, ui, t, id, name, delete_designs),
        }
    });
    if r.should_close() {
        keep = false;
    }
    // A pop-up opened from inside this one replaces it.
    if keep && app.home.modal.is_none() {
        app.home.modal = Some(modal);
    }
}

/// A pop-up's title with a close button; whether it was closed.
fn header(app: &mut App, ui: &mut egui::Ui, t: &Tokens, title: &str, sub: Option<&str>) -> bool {
    let mut closed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(21.0).strong().color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
            if r.hovered() {
                ui.painter().circle_filled(rect.center(), 14.0, t.panel_alt);
            }
            Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()).icon(ui.painter(), Icon::Close, rect.shrink(5.0), t.text_dim);
            closed = r.on_hover_text(tr("Close")).clicked();
        });
    });
    if let Some(sub) = sub {
        ui.label(egui::RichText::new(sub).color(t.text_dim));
    }
    ui.add_space(12.0);
    closed
}

fn primary(ui: &mut egui::Ui, t: &Tokens, label: &str, enabled: bool) -> bool {
    let b =
        egui::Button::new(egui::RichText::new(label).color(t.accent_text).strong()).fill(t.accent).corner_radius(16).min_size(egui::vec2(96.0, 32.0));
    ui.add_enabled(enabled, b).clicked()
}

fn secondary(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(label).corner_radius(16).min_size(egui::vec2(88.0, 32.0))).clicked()
}

/// A size card: the page's shape, its name and size.
fn size_card(ui: &mut egui::Ui, t: &Tokens, label: &str, w: f64, h: f64, sub: Option<&str>) -> egui::Response {
    sized_card(ui, t, 142.0, label, w, h, sub)
}

fn sized_card(ui: &mut egui::Ui, t: &Tokens, card_w: f32, label: &str, w: f64, h: f64, sub: Option<&str>) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(card_w, 126.0), egui::Sense::click());
    let p = ui.painter_at(rect.expand(2.0));
    p.rect_filled(rect, 12, if r.hovered() { t.accent_soft } else { t.panel_alt });
    if r.hovered() {
        p.rect_stroke(rect, 12, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    }
    let shape_box = egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 10.0), egui::vec2(rect.width(), 58.0));
    let sz = crate::thumbs::fit_size(w as f32, h as f32, egui::vec2(72.0, 52.0));
    let sr = egui::Rect::from_center_size(shape_box.center(), sz);
    p.rect_filled(sr.translate(egui::vec2(0.0, 2.0)), 4, t.art_shadow);
    p.rect_filled(sr, 4, t.art_paper);
    p.rect_stroke(sr, 4, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    let name = egui::WidgetText::from(egui::RichText::new(label).size(12.5).color(t.text)).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        rect.width() - 14.0,
        egui::TextStyle::Body,
    );
    let name_h = name.size().y.min(34.0);
    p.galley(egui::pos2(rect.center().x - name.size().x / 2.0, rect.top() + 74.0), name, t.text);
    let dims = sub.map(str::to_string).unwrap_or_else(|| format!("{w:.0} × {h:.0} px"));
    p.text(egui::pos2(rect.center().x, rect.top() + 78.0 + name_h), egui::Align2::CENTER_TOP, dims, egui::FontId::proportional(11.0), t.text_dim);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn size_picker(app: &mut App, ui: &mut egui::Ui, t: &Tokens, query: &mut String, palette: Option<&str>) -> bool {
    if header(app, ui, t, tr("Create a new design"), Some(tr("Pick a size; you can change it later with Resize."))) {
        return false;
    }
    let mut create_with: Option<serde_json::Value> = None;
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(query).hint_text(tr("Search sizes")).desired_width(240.0).margin(egui::vec2(10.0, 6.0)));
        ui.add_space(16.0);
        ui.label(egui::RichText::new(tr("Custom size")).strong());
        ui.add(egui::DragValue::new(&mut app.home.custom_w).range(1.0..=16384.0).prefix(tr("W ")).suffix(" px").speed(1.0));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut app.home.custom_h).range(1.0..=16384.0).prefix(tr("H ")).suffix(" px").speed(1.0));
        if primary(ui, t, tr("Create"), true) {
            create_with = Some(json!({"width": app.home.custom_w, "height": app.home.custom_h, "title": tr("Custom design")}));
        }
    });
    ui.add_space(10.0);
    let q = query.trim().to_string();
    let max_h = (ui.ctx().content_rect().height() - 260.0).clamp(160.0, 520.0);
    egui::ScrollArea::vertical().max_height(max_h).min_scrolled_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
        let gap = 10.0;
        let avail = ui.available_width() - 4.0;
        let per = (((avail + gap) / (142.0 + gap)).floor()).max(1.0);
        let card_w = ((avail - gap * (per - 1.0)) / per).floor();
        let mut any = false;
        for cat in preset::categories() {
            let list: Vec<&preset::PagePreset> = preset::PRESETS
                .iter()
                .filter(|p| p.category == cat)
                .filter(|p| genha_engine::cmd::designs::matches(&q, &[p.label, tr(p.label), p.category, tr(p.category), p.id]))
                .collect();
            if list.is_empty() {
                continue;
            }
            any = true;
            ui.label(egui::RichText::new(tr(cat)).size(14.0).strong().color(t.text_dim));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                for p in list {
                    if sized_card(ui, t, card_w, tr(p.label), p.width, p.height, None).clicked() {
                        create_with = Some(json!({"preset": p.id, "title": tr(p.label)}));
                    }
                }
            });
            ui.add_space(12.0);
        }
        if !any {
            ui.label(egui::RichText::new(tr("No sizes match. Type your own above.")).color(t.text_dim));
        }
    });
    match create_with {
        Some(params) => {
            create(app, params, palette);
            false
        }
        None => true,
    }
}

/// Templates with tag `tag`, as a row of cards; the one clicked.
fn template_row(app: &mut App, ui: &mut egui::Ui, t: &Tokens, tag: &str, salt: &str) -> Option<&'static str> {
    let list: Vec<_> = genha_templates::catalog().into_iter().filter(|tp| tp.tags.contains(&tag)).collect();
    let mut chosen = None;
    egui::ScrollArea::horizontal().id_salt(salt).auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            for tp in &list {
                if crate::home::template_card(app, ui, t, tp, 150.0).clicked() {
                    chosen = Some(tp.id);
                }
            }
        });
    });
    chosen
}

fn video(app: &mut App, ui: &mut egui::Ui, t: &Tokens) -> bool {
    if header(app, ui, t, tr("Create a video"), Some(tr("A blank video with an animated title, or an animated template."))) {
        return false;
    }
    let mut blank = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
        for (id, label) in [("video-portrait", tr("Story 9:16")), ("video-square", tr("Square 1:1")), ("video-landscape", tr("Wide 16:9"))] {
            if let Some(p) = preset::find(id)
                && size_card(ui, t, label, p.width, p.height, Some(&format!("{:.0} × {:.0}", p.width, p.height))).clicked()
            {
                blank = Some((id, p.width, p.height));
            }
        }
    });
    ui.add_space(14.0);
    ui.label(egui::RichText::new(tr("Animated templates")).size(14.0).strong().color(t.text_dim));
    ui.add_space(4.0);
    let chosen = template_row(app, ui, t, "animated", "video_templates");
    if let Some((id, w, h)) = blank {
        if create(app, json!({"preset": id, "title": tr("Video")}), None) {
            let size = (w.min(h) * 0.09).round();
            app.run(
                "text.add",
                &json!({"text": tr("Your video title"), "role": "heading", "x": w * 0.1, "y": h * 0.42, "width": w * 0.8,
                        "style": {"font_size": size, "align": "center", "bold": true}}),
            );
            app.run("animation.applyToPage", &json!({"kind": "rise"}));
            app.run("page.setDuration", &json!({"seconds": 5}));
            app.run("select.none", &json!({}));
        }
        return false;
    }
    if let Some(id) = chosen {
        use_template(app, id);
        return false;
    }
    true
}

fn presentation(app: &mut App, ui: &mut egui::Ui, t: &Tokens) -> bool {
    if header(app, ui, t, tr("Create a presentation"), Some(tr("16:9 slides: start blank or from a deck."))) {
        return false;
    }
    let mut blank = false;
    ui.horizontal_wrapped(|ui| {
        if let Some(p) = preset::find("presentation")
            && size_card(ui, t, tr("Blank presentation"), p.width, p.height, Some("16:9")).clicked()
        {
            blank = true;
        }
    });
    ui.add_space(14.0);
    ui.label(egui::RichText::new(tr("Presentation templates")).size(14.0).strong().color(t.text_dim));
    ui.add_space(4.0);
    let chosen = template_row(app, ui, t, "presentation", "deck_templates");
    if blank {
        create(app, json!({"preset": "presentation", "title": tr("Presentation")}), None);
        return false;
    }
    if let Some(id) = chosen {
        use_template(app, id);
        return false;
    }
    true
}

fn qr(app: &mut App, ui: &mut egui::Ui, t: &Tokens, text: &mut String) -> bool {
    if header(app, ui, t, tr("QR code"), Some(tr("A square design with a QR code for a link or a text."))) {
        return false;
    }
    let r = ui.add(egui::TextEdit::singleline(text).hint_text("https://…").desired_width(f32::INFINITY).margin(egui::vec2(10.0, 8.0)));
    r.request_focus();
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.add_space(12.0);
    let mut go_on = enter && !text.trim().is_empty();
    let mut cancel = false;
    ui.horizontal(|ui| {
        if primary(ui, t, tr("Create design"), !text.trim().is_empty()) {
            go_on = true;
        }
        if secondary(ui, tr("Cancel")) {
            cancel = true;
        }
    });
    if go_on {
        let value = text.trim().to_string();
        if create(app, json!({"preset": "instagram-post", "title": tr("QR code")}), None) {
            app.run("element.qrCode", &json!({"text": value, "size": 640}));
        }
        return false;
    }
    !cancel
}

/// A name field with Save and Cancel: `None` while open, `Some(None)` when cancelled, the name
/// when saved.
fn rename(ui: &mut egui::Ui, t: &Tokens, title: &str, name: &mut String) -> Option<Option<String>> {
    ui.label(egui::RichText::new(title).size(19.0).strong());
    ui.add_space(10.0);
    let r = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY).margin(egui::vec2(10.0, 8.0)));
    r.request_focus();
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.add_space(12.0);
    let mut out = None;
    ui.horizontal(|ui| {
        if primary(ui, t, tr("Save"), !name.trim().is_empty()) || (enter && !name.trim().is_empty()) {
            out = Some(Some(name.trim().to_string()));
        }
        if secondary(ui, tr("Cancel")) {
            out = Some(None);
        }
    });
    out
}

fn delete_designs(app: &mut App, ui: &mut egui::Ui, t: &Tokens, ids: &[String], title: &str, stored: usize) -> bool {
    let (heading, body) = if stored > 0 {
        (
            tr("Delete this design?"),
            genha_i18n::tr!("“{title}” is kept only in GenHa. Deleting it can't be undone.", title = crate::shown_name(title)),
        )
    } else {
        (
            tr("Remove from My designs?"),
            genha_i18n::tr!("“{title}” is removed from the list. Its file stays where it is.", title = crate::shown_name(title)),
        )
    };
    ui.label(egui::RichText::new(heading).size(19.0).strong());
    ui.add_space(8.0);
    ui.label(egui::RichText::new(body).color(t.text_dim));
    ui.add_space(14.0);
    let mut keep = true;
    ui.horizontal(|ui| {
        let label = if stored > 0 { tr("Delete") } else { tr("Remove") };
        let b = egui::Button::new(egui::RichText::new(label).color(t.accent_text).strong())
            .fill(t.danger)
            .corner_radius(16)
            .min_size(egui::vec2(96.0, 32.0));
        if ui.add(b).clicked() {
            app.run("design.remove", &json!({"ids": ids}));
            keep = false;
        }
        if secondary(ui, tr("Cancel")) {
            keep = false;
        }
    });
    keep
}

/// Colour swatches: none, then [`FOLDER_COLORS`]; `chosen` as in [`Modal::NewFolder`].
fn swatches(ui: &mut egui::Ui, t: &Tokens, chosen: &mut usize) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for i in 0..=FOLDER_COLORS.len() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
            let c = rect.center();
            match i.checked_sub(1).and_then(|k| FOLDER_COLORS.get(k)) {
                Some(col) => {
                    ui.painter().circle_filled(c, 11.0, *col);
                }
                None => {
                    ui.painter().circle_stroke(c, 10.0, egui::Stroke::new(1.5, t.text_dim));
                    ui.painter().line_segment([c + egui::vec2(-7.0, 7.0), c + egui::vec2(7.0, -7.0)], egui::Stroke::new(1.5, t.text_dim));
                }
            }
            if *chosen == i {
                ui.painter().circle_stroke(c, 13.5, egui::Stroke::new(2.0, t.accent));
            }
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if i == 0 { tr("No colour") } else { tr("Colour") });
            if r.clicked() {
                *chosen = i;
                changed = true;
            }
        }
    });
    changed
}

fn color_hex(i: usize) -> Option<String> {
    i.checked_sub(1).and_then(|k| FOLDER_COLORS.get(k)).map(|c| crate::theme::from_egui(*c).to_hex())
}

fn new_folder(app: &mut App, ui: &mut egui::Ui, t: &Tokens, name: &mut String, color: &mut usize) -> bool {
    ui.label(egui::RichText::new(tr("New folder")).size(19.0).strong());
    ui.label(egui::RichText::new(tr("Keep the designs of one project together.")).color(t.text_dim));
    ui.add_space(10.0);
    let r = ui.add(egui::TextEdit::singleline(name).hint_text(tr("Folder name")).desired_width(f32::INFINITY).margin(egui::vec2(10.0, 8.0)));
    if !ui.ctx().memory(|m| m.focused().is_some()) {
        r.request_focus();
    }
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.add_space(8.0);
    swatches(ui, t, color);
    ui.add_space(12.0);
    let mut keep = true;
    let mut make = enter && !name.trim().is_empty();
    ui.horizontal(|ui| {
        if primary(ui, t, tr("Create folder"), !name.trim().is_empty()) {
            make = true;
        }
        if secondary(ui, tr("Cancel")) {
            keep = false;
        }
    });
    if make {
        if let Some(r) = app.run("folder.create", &json!({"name": name.trim(), "color": color_hex(*color)}))
            && let Some(id) = r.get("id").and_then(|v| v.as_str())
        {
            app.home.page = Page::Designs;
            app.home.folder = FolderFilter::Folder(id.to_string());
        }
        keep = false;
    }
    keep
}

fn folder_color(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: &str, name: &str) -> bool {
    ui.label(egui::RichText::new(genha_i18n::tr!("Colour of “{name}”", name = name)).size(19.0).strong());
    ui.add_space(10.0);
    let current = app.library.folder(id).and_then(|f| f.color.clone());
    let mut chosen = FOLDER_COLORS.iter().position(|c| Some(crate::theme::from_egui(*c).to_hex()) == current).map_or(0, |i| i + 1);
    if swatches(ui, t, &mut chosen) {
        app.run("folder.setColor", &json!({"id": id, "color": color_hex(chosen)}));
    }
    ui.add_space(12.0);
    !primary(ui, t, tr("Done"), true)
}

fn delete_folder(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: &str, name: &str, delete_designs: &mut bool) -> bool {
    let n = app.library.count(id);
    ui.label(egui::RichText::new(genha_i18n::tr!("Delete the folder “{name}”?", name = name)).size(19.0).strong());
    ui.add_space(8.0);
    let body = if n == 0 {
        tr("The folder is empty.").to_string()
    } else {
        tr_n!("Its {n} design stays in My designs, outside any folder.", "Its {n} designs stay in My designs, outside any folder.", n)
    };
    ui.label(egui::RichText::new(body).color(t.text_dim));
    if n > 0 {
        ui.add_space(6.0);
        crate::theme::switch(ui, delete_designs, tr("Also delete the designs in it (files saved elsewhere are kept)"));
    }
    ui.add_space(14.0);
    let mut keep = true;
    ui.horizontal(|ui| {
        let b = egui::Button::new(egui::RichText::new(tr("Delete folder")).color(t.accent_text).strong())
            .fill(t.danger)
            .corner_radius(16)
            .min_size(egui::vec2(96.0, 32.0));
        if ui.add(b).clicked() {
            if app.run("folder.delete", &json!({"id": id, "keep_designs": !*delete_designs})).is_some()
                && app.home.folder == FolderFilter::Folder(id.to_string())
            {
                app.home.folder = FolderFilter::All;
            }
            keep = false;
        }
        if secondary(ui, tr("Cancel")) {
            keep = false;
        }
    });
    keep
}

// ---------- search ----------

/// What a search result opens.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Hit {
    Design(String),
    Template(&'static str),
    Size(&'static str),
    Action(&'static str),
    AllTemplates,
}

/// The search field at the top of the home page; its results show under it while it is used.
pub fn search_field(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let avail = ui.available_width();
    let w = avail.min(700.0);
    let id = egui::Id::new("home_search");
    let focused = ui.ctx().memory(|m| m.has_focus(id));
    let mut field_rect = egui::Rect::NOTHING;
    let mut submitted = false;
    ui.horizontal(|ui| {
        ui.add_space(((avail - w) / 2.0).max(0.0));
        let frame = egui::Frame::new()
            .fill(t.panel)
            .stroke(egui::Stroke::new(if focused { 1.5 } else { 1.0 }, if focused { t.accent } else { t.border }))
            .corner_radius(24)
            .inner_margin(egui::Margin::symmetric(16, 9));
        let resp = frame.show(ui, |ui| {
            ui.set_width(w - 32.0);
            ui.horizontal(|ui| {
                let (ir, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()).icon(ui.painter(), Icon::Lib("search"), ir, t.text_dim);
                let te = egui::TextEdit::singleline(&mut app.home.search)
                    .id(id)
                    .hint_text(tr("Search templates, sizes, quick actions and your designs"))
                    .frame(egui::Frame::NONE)
                    .font(egui::FontId::proportional(15.5))
                    .desired_width(f32::INFINITY);
                let r = ui.add(te);
                if r.changed() || r.gained_focus() {
                    app.home.search_open = true;
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submitted = true;
                }
            });
        });
        field_rect = resp.response.rect;
    });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.home.search_open = false;
    }
    // A click outside the field and its results closes them.
    let outside = ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer.interact_pos().is_some_and(|p| !field_rect.contains(p) && !app.home.search_rect.is_some_and(|r| r.contains(p)))
    });
    if outside {
        app.home.search_open = false;
    }
    let query = app.home.search.trim().to_string();
    if !app.home.search_open || query.is_empty() {
        app.home.search_rect = None;
        return;
    }
    let results = genha_engine::cmd::home::search(&app.session, &query, 5);
    let first = results
        .designs
        .first()
        .map(|e| Hit::Design(e.id.clone()))
        .or_else(|| results.templates.first().map(|tp| Hit::Template(tp.id)))
        .or_else(|| results.sizes.first().map(|p| Hit::Size(p.id)))
        .or_else(|| results.actions.first().map(|a| Hit::Action(a.id)));
    let mut hit = if submitted { first.clone() } else { None };
    let area = egui::Area::new(egui::Id::new("home_search_results"))
        .order(egui::Order::Foreground)
        .fixed_pos(field_rect.left_bottom() + egui::vec2(0.0, 6.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).fill(t.panel).corner_radius(16).inner_margin(10).show(ui, |ui| {
                ui.set_width(field_rect.width() - 20.0);
                if results.is_empty() {
                    ui.label(egui::RichText::new(genha_i18n::tr!("Nothing found for “{query}”", query = query)).color(t.text_dim));
                    return;
                }
                egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                    let group = |ui: &mut egui::Ui, title: &str| {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(title).size(12.0).strong().color(t.text_dim));
                    };
                    if !results.designs.is_empty() {
                        group(ui, tr("Your designs"));
                        for e in &results.designs {
                            let sub = crate::home::ago(crate::now(), e.edited.max(e.created));
                            if row(
                                app,
                                ui,
                                t,
                                RowArt::Design(e.clone()),
                                &crate::shown_name(&e.title),
                                &sub,
                                first == Some(Hit::Design(e.id.clone())),
                            ) {
                                hit = Some(Hit::Design(e.id.clone()));
                            }
                        }
                    }
                    if !results.templates.is_empty() {
                        group(ui, tr("Templates"));
                        for tp in &results.templates {
                            if row(app, ui, t, RowArt::Template(tp.clone()), tp.name, tr(tp.category), first == Some(Hit::Template(tp.id))) {
                                hit = Some(Hit::Template(tp.id));
                            }
                        }
                        let more =
                            ui.add(egui::Label::new(egui::RichText::new(tr("All matching templates")).color(t.accent)).sense(egui::Sense::click()));
                        if more.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            hit = Some(Hit::AllTemplates);
                        }
                    }
                    if !results.sizes.is_empty() {
                        group(ui, tr("Sizes"));
                        for p in &results.sizes {
                            let sub = format!("{:.0} × {:.0} px, {}", p.width, p.height, tr(p.category));
                            if row(app, ui, t, RowArt::Size(p.width, p.height), tr(p.label), &sub, first == Some(Hit::Size(p.id))) {
                                hit = Some(Hit::Size(p.id));
                            }
                        }
                    }
                    if !results.actions.is_empty() {
                        group(ui, tr("Quick actions"));
                        for a in &results.actions {
                            if row(app, ui, t, RowArt::Action(a.id), tr(a.label), tr(a.hint), first == Some(Hit::Action(a.id))) {
                                hit = Some(Hit::Action(a.id));
                            }
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(tr("Enter opens the first result")).size(11.0).color(t.text_dim));
                });
            });
        });
    app.home.search_rect = Some(area.response.rect);
    if let Some(h) = hit {
        app.home.search_open = false;
        match h {
            Hit::Design(id) => {
                app.run("design.open", &json!({"id": id}));
            }
            Hit::Template(id) => use_template(app, id),
            Hit::Size(id) => {
                let label = preset::find(id).map(|p| tr(p.label)).unwrap_or_default();
                create(app, json!({"preset": id, "title": label}), None);
            }
            Hit::Action(id) => run_quick_action(app, id),
            Hit::AllTemplates => {
                app.home.templates_query = query;
                app.home.templates_chip = 0;
                go(app, Page::Templates);
            }
        }
    }
}

/// What a result row shows on its left.
enum RowArt {
    Design(genha_engine::cmd::designs::DesignEntry),
    Template(genha_templates::TemplateInfo),
    Size(f64, f64),
    Action(&'static str),
}

/// One result: a picture, a name and a line under it; whether it was clicked.
fn row(app: &mut App, ui: &mut egui::Ui, t: &Tokens, art: RowArt, title: &str, sub: &str, first: bool) -> bool {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), egui::Sense::click());
    let p = ui.painter_at(rect);
    if r.hovered() || first {
        p.rect_filled(rect, 10, if r.hovered() { t.accent_soft } else { t.panel_alt });
    }
    let pic = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 5.0), egui::vec2(36.0, 36.0));
    p.rect_filled(pic, 8, t.panel_alt);
    let ctx = ui.ctx().clone();
    let tex = match &art {
        RowArt::Design(e) => {
            let session = &app.session;
            app.thumbs.get(&ctx, &format!("design:{}", e.id), e.version, || {
                let png = genha_engine::cmd::designs::thumbnail(session, &e.id)?;
                let img = genha_effects::decode(&png).ok()?;
                Some(genha_render::Raster { width: img.width(), height: img.height(), rgba: img.into_raw() })
            })
        }
        RowArt::Template(tp) => {
            let fonts = app.session.fonts.clone();
            let id = tp.id;
            app.thumbs.get(&ctx, &format!("template:{id}"), 0, || {
                let doc = genha_templates::instantiate(id, None)?;
                genha_render::render_thumbnail(&doc, 0, &fonts, 360).ok()
            })
        }
        _ => None,
    };
    match (&art, tex) {
        (_, Some(tex)) => {
            let fit = crate::thumbs::fit_size(tex.size()[0] as f32, tex.size()[1] as f32, pic.size() - egui::vec2(4.0, 4.0));
            p.image(
                tex.id(),
                egui::Rect::from_center_size(pic.center(), fit),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        (RowArt::Size(w, h), None) => {
            let fit = crate::thumbs::fit_size(*w as f32, *h as f32, pic.size() - egui::vec2(10.0, 10.0));
            let sr = egui::Rect::from_center_size(pic.center(), fit);
            p.rect_filled(sr, 2, t.art_paper);
            p.rect_stroke(sr, 2, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
        }
        (RowArt::Action(id), None) => {
            let i = genha_engine::cmd::home::QUICK_ACTIONS.iter().position(|a| a.id == *id).unwrap_or(0);
            let [a, b, ink] = t.tiles[i % 5];
            crate::art::gradient_rect(&p, pic, 8.0, a, b);
            crate::art::action(&mut Art::new(&mut app.thumbs, &ctx, app.session.fonts.clone()), &p, t, id, pic.shrink(2.0), ink);
        }
        _ => {}
    }
    let tx = pic.right() + 12.0;
    let title_g = egui::WidgetText::from(egui::RichText::new(title).size(13.5).color(t.text)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.right() - tx - 8.0,
        egui::TextStyle::Body,
    );
    p.galley(egui::pos2(tx, rect.top() + 6.0), title_g, t.text);
    let sub_g = egui::WidgetText::from(egui::RichText::new(sub).size(11.5).color(t.text_dim)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.right() - tx - 8.0,
        egui::TextStyle::Body,
    );
    p.galley(egui::pos2(tx, rect.top() + 25.0), sub_g, t.text_dim);
    r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
