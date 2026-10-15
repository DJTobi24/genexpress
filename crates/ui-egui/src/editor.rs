//! The editor: top bar, left panels, page strip, status bar, command palette, shortcuts.

use serde_json::{Value, json};

use crate::theme::Tokens;
use crate::{App, LeftTab, io, thumbs};
use genha_doc::{ElementKind, NodeId, preset};
use genha_i18n::{N_, tr, tr_n};

#[derive(Default)]
pub struct EditorState {
    pub palette_open: bool,
    pub palette_query: String,
    pub resize_copy: bool,
    pub qr_text: String,
    pub template_query: String,
    /// The template category the Templates tab shows (none: all).
    pub template_category: Option<&'static str>,
    /// A template clicked while the design has content: asks how to use it (new design, replace
    /// this page, add a page).
    pub template_ask: Option<&'static str>,
    /// The text and picture panels' tab (Edit, Effects, Animation).
    pub panel_tab: crate::properties::PanelTab,
    /// The object the text or picture panel was closed for (× or a rail tab): the left panel
    /// shows its tab until another object is selected.
    pub panel_closed: Option<NodeId>,
    /// The strength of the picture's filter as last set in the picture panel (none: full).
    pub filter_strength: Option<f32>,
    pub renaming: Option<String>,
    pub new_brand_color: [u8; 3],
    pub versions_open: bool,
    pub version_name: String,
    /// A video being exported on a worker thread (desktop; the browser exports in place).
    #[cfg(not(target_arch = "wasm32"))]
    pub video_job: Option<VideoJob>,
}

/// An MP4 export running on its own thread, so the editor stays responsive: progress in the
/// status bar, a Cancel button, the result when it ends. It runs `export.mp4`'s own work
/// ([`genha_engine::cmd::motion::video_file`]) on a copy of the design.
#[cfg(not(target_arch = "wasm32"))]
pub struct VideoJob {
    path: std::path::PathBuf,
    /// Frames rendered and to render.
    progress: std::sync::Arc<(std::sync::atomic::AtomicUsize, std::sync::atomic::AtomicUsize)>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    done: std::sync::mpsc::Receiver<Result<Value, String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl VideoJob {
    pub(crate) fn start(app: &App, ctx: &egui::Context, params: Value, path: std::path::PathBuf) -> Option<VideoJob> {
        use std::sync::atomic::Ordering;
        let d = app.session.active()?;
        let (doc, page, fonts) = (d.doc.clone(), d.page, app.session.fonts.clone());
        let progress = std::sync::Arc::new((std::sync::atomic::AtomicUsize::new(0), std::sync::atomic::AtomicUsize::new(0)));
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tx, done) = std::sync::mpsc::channel();
        let (p, c, ctx) = (progress.clone(), cancel.clone(), ctx.clone());
        let spawned = std::thread::Builder::new().name("video export".into()).spawn(move || {
            let r = genha_engine::cmd::motion::video_file(&doc, &fonts, page, &params, &mut |n, total| {
                p.0.store(n, Ordering::Relaxed);
                p.1.store(total, Ordering::Relaxed);
                ctx.request_repaint();
                !c.load(Ordering::Relaxed)
            });
            // Nothing to do if the editor has gone away: it no longer waits for the result.
            tx.send(r.map_err(|e| e.to_string())).ok();
            ctx.request_repaint();
        });
        spawned.ok().map(|_| VideoJob { path, progress, cancel, done })
    }

    /// Asks the export to stop (it ends at the next frame and reports that it was cancelled).
    pub(crate) fn cancel(&self) {
        self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Picks up the result of a finished video export.
#[cfg(not(target_arch = "wasm32"))]
fn poll_video(app: &mut App) {
    let Some(job) = &app.editor.video_job else { return };
    let msg = match job.done.try_recv() {
        Ok(Ok(_)) => tr!("Downloaded {path}", path = job.path.display()),
        Ok(Err(e)) => e,
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => tr!("The video export stopped unexpectedly"),
    };
    app.editor.video_job = None;
    app.set_status(msg);
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    app.thumbs.begin_frame();
    #[cfg(not(target_arch = "wasm32"))]
    poll_video(app);
    shortcuts(app, ui.ctx());
    top_bar(app, ui, &t);
    status_bar(app, ui, &t);
    page_strip(app, ui, &t);
    left_rail(app, ui, &t);
    // Nothing selected: a text box selected again gets its panel again.
    if app.session.active().is_some_and(|d| d.selection.is_empty()) && app.session.text_editing.is_none() {
        app.editor.panel_closed = None;
    }
    // A selected object is edited in the left panel (nothing selected: the rail's tab).
    let context = crate::properties::context_target(app);
    egui::Panel::left("left_panel").resizable(true).default_size(290.0).min_size(260.0).frame(panel_frame(&t)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match context {
            Some((id, kind)) => crate::properties::context_panel(app, ui, &t, id, kind),
            None => left_panel(app, ui, &t),
        });
    });
    crate::layers::show(app, ui, &t);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| crate::canvas::show(app, ui, &t));
    palette(app, ui.ctx(), &t);
    versions_window(app, ui.ctx(), &t);
    template_dialog(app, ui.ctx(), &t);
    toast(app, ui.ctx(), &t);
    crate::models::show(app, ui.ctx());
}

fn panel_frame(t: &Tokens) -> egui::Frame {
    egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 10)).stroke(egui::Stroke::new(1.0, t.border))
}

fn top_bar(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::top("top_bar")
        .exact_size(54.0)
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(10, 9)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                document_group(app, ui, t);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    file_group(app, ui, t);
                });
            });
            // The tools in the middle of the bar, when there is room between the two groups.
            let tools_width = genha_tools::ToolId::ALL.len() as f32 * 38.0 + 8.0;
            let middle = egui::Rect::from_center_size(full.center(), egui::vec2(tools_width, 38.0));
            if full.width() > 1100.0 {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(middle).layout(egui::Layout::left_to_right(egui::Align::Center)));
                tools(app, &mut child, t);
            }
        });
}

/// The tools as icons on one rounded strip.
fn tools(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    use genha_tools::ToolId;
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 12, t.panel_alt);
    ui.add_space(4.0);
    ui.spacing_mut().item_spacing.x = 2.0;
    for tool in ToolId::ALL {
        let on = app.session.tools.tool == tool;
        let r = crate::glyphs::button(ui, t, crate::glyphs::Glyph::tool(tool), on, 36.0);
        if r.on_hover_text(format!("{} ({})", tr(tool.label()), tool.shortcut())).clicked() {
            app.run("tool.set", &json!({"tool": tool.id()}));
        }
    }
}

/// Home, the design's name, undo and redo, resize.
fn document_group(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    use crate::glyphs::{Glyph, button, labelled};
    if button(ui, t, Glyph::Home, false, 36.0).on_hover_text(tr("Home")).clicked() {
        app.go_home();
    }
    ui.add_space(4.0);
    let title = app.session.active().map(|d| crate::shown_name(&d.doc.title)).unwrap_or_default();
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
            let text = egui::RichText::new(format!("{title}{}", if dirty { " •" } else { "" })).size(14.5).strong().color(t.text);
            let label = egui::Label::new(text).truncate().sense(egui::Sense::click());
            if ui.add_sized(egui::vec2(190.0, 30.0), label).on_hover_text(tr("Click to rename")).clicked() {
                app.editor.renaming = Some(title);
            }
        }
    }
    ui.add_space(4.0);
    let can_undo = app.session.active().is_some_and(|d| !d.history.undo.is_empty());
    let can_redo = app.session.active().is_some_and(|d| !d.history.redo.is_empty());
    if ui.add_enabled_ui(can_undo, |ui| button(ui, t, Glyph::Undo, false, 36.0)).inner.on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
        app.run("edit.undo", &json!({}));
    }
    if ui.add_enabled_ui(can_redo, |ui| button(ui, t, Glyph::Redo, false, 36.0)).inner.on_hover_text(tr("Redo (Ctrl+Shift+Z)")).clicked() {
        app.run("edit.redo", &json!({}));
    }
    ui.add_space(6.0);
    let r = labelled(ui, t, Glyph::Resize, tr("Resize"), false).on_hover_text(tr("Another size for this page, laid out again"));
    egui::Popup::menu(&r).show(|ui| resize_menu(app, ui));
}

/// Download, present, play, save and the rest under "more" (right to left).
fn file_group(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    use crate::glyphs::{Glyph, button, labelled};
    let r = labelled(ui, t, Glyph::Download, tr("Download"), true).on_hover_text(tr("Export as PNG, JPEG, SVG, PDF, GIF, animated PNG or MP4 video"));
    egui::Popup::menu(&r).show(|ui| download_menu(app, ui));
    if button(ui, t, Glyph::Present, false, 36.0).on_hover_text(tr("Show the design full screen, page by page (F5)")).clicked() {
        crate::motion::start_presenting(app, ui.ctx());
    }
    let playing = app.motion.playing.is_some();
    let r = button(ui, t, if playing { Glyph::Stop } else { Glyph::Play }, playing, 36.0);
    if r.on_hover_text(if playing { tr("Stop") } else { tr("Play this page's animations") }).clicked() {
        if playing {
            crate::motion::stop(app);
        } else {
            crate::motion::play(app, ui.ctx());
        }
    }
    if button(ui, t, Glyph::Save, false, 36.0).on_hover_text(tr("Save (Ctrl+S)")).clicked() {
        save(app, false);
    }
    let r = button(ui, t, Glyph::More, false, 36.0).on_hover_text(tr("More"));
    egui::Popup::menu(&r).show(|ui| {
        ui.set_min_width(230.0);
        if ui.button(tr("Commands…  (Ctrl+K)")).clicked() {
            app.editor.palette_open = true;
            ui.close();
        }
        if ui.button(tr("Version history")).clicked() {
            app.editor.versions_open = true;
            ui.close();
        }
        if ui.button(tr("Save as…")).clicked() {
            save(app, true);
            ui.close();
        }
        ui.separator();
        let mut snapping = app.session.snapping;
        if ui
            .checkbox(&mut snapping, tr("Smart guides"))
            .on_hover_text(tr("Smart guides: snap to the page, other objects, equal gaps and sizes (hold Ctrl/Cmd while dragging to skip)"))
            .changed()
        {
            app.run("view.snapping", &json!({"on": snapping}));
        }
        let mut strip = app.session.layer_strip;
        if ui
            .checkbox(&mut strip, tr("Layer strip"))
            .on_hover_text(tr("The layer strip: every object as a picture, the front at the top; drag one up or down to restack it"))
            .changed()
        {
            app.run("view.layerStrip", &json!({"on": strip}));
        }
    });
    // Narrow windows: the tools sit here instead of the middle of the bar.
    if ui.available_width() > 0.0 && ui.ctx().content_rect().width() <= 1100.0 {
        ui.add_space(8.0);
        let width = genha_tools::ToolId::ALL.len() as f32 * 38.0 + 8.0;
        ui.allocate_ui_with_layout(egui::vec2(width, 38.0), egui::Layout::left_to_right(egui::Align::Center), |ui| tools(app, ui, t));
    }
}

fn resize_menu(app: &mut App, ui: &mut egui::Ui) {
    ui.menu_button(tr("Resize"), |ui| {
        ui.checkbox(&mut app.editor.resize_copy, tr("Keep this page, add a resized copy"));
        ui.separator();
        for cat in preset::categories() {
            ui.menu_button(tr(cat), |ui| {
                for p in preset::PRESETS.iter().filter(|p| p.category == cat) {
                    if ui.button(format!("{}  ({:.0}×{:.0})", tr(p.label), p.width, p.height)).clicked() {
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

/// A row of the Download menu: a coloured badge with the format, then the label.
fn format_row(ui: &mut egui::Ui, t: &Tokens, ext: &str, label: &str) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(280.0, 36.0), egui::Sense::click());
    let enabled = ui.is_enabled();
    let r = if enabled { r.on_hover_cursor(egui::CursorIcon::PointingHand) } else { r };
    let k = crate::theme::hover_fade(ui.ctx(), rect, r.hovered() && enabled);
    if k > 0.0 {
        ui.painter().rect_filled(rect, 8, t.accent_soft.gamma_multiply(k));
    }
    let color = match ext {
        "png" => egui::Color32::from_rgb(0x25, 0x63, 0xeb),
        "jpg" => egui::Color32::from_rgb(0x0e, 0x9f, 0x8e),
        "svg" => egui::Color32::from_rgb(0xe8, 0x8a, 0x17),
        "pdf" => egui::Color32::from_rgb(0xd9, 0x30, 0x30),
        "gif" => egui::Color32::from_rgb(0xdb, 0x27, 0x77),
        _ => t.accent,
    };
    let badge = egui::Rect::from_min_size(rect.left_center() + egui::vec2(8.0, -11.0), egui::vec2(40.0, 22.0));
    ui.painter().rect_filled(badge, 6, if enabled { color } else { t.border });
    ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, ext.to_uppercase(), egui::FontId::proportional(10.5), egui::Color32::WHITE);
    let ink = if enabled { t.text } else { t.text_dim };
    ui.painter().text(rect.left_center() + egui::vec2(58.0, 0.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(13.0), ink);
    r
}

fn download_menu(app: &mut App, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let name = app.session.active().map(|d| io::file_stem(&d.doc.title)).unwrap_or_else(|| "design".into());
    let entries: [(&str, &str, &str, Value); 10] = [
        ("PNG", "export.png", "png", json!({})),
        (N_("PNG — transparent background"), "export.png", "png", json!({"transparent": true})),
        (N_("PNG — 2× size"), "export.png", "png", json!({"scale": 2.0})),
        ("JPEG", "export.jpeg", "jpg", json!({})),
        ("SVG", "export.svg", "svg", json!({})),
        (N_("PDF — all pages"), "export.pdf", "pdf", json!({})),
        (N_("PDF — print (bleed, crop marks)"), "export.pdf", "pdf", json!({"print": true})),
        (N_("GIF (animated)"), "export.gif", "gif", json!({})),
        (N_("Animated PNG"), "export.apng", "png", json!({})),
        (N_("MP4 video"), "export.mp4", "mp4", json!({})),
    ];
    for (label, cmd, ext, extra) in entries {
        let video = cmd == "export.mp4";
        #[cfg(not(target_arch = "wasm32"))]
        let busy = video && app.editor.video_job.is_some();
        #[cfg(target_arch = "wasm32")]
        let busy = false;
        let button = ui.add_enabled_ui(!busy, |ui| format_row(ui, &t, ext, tr(label))).inner;
        let button = if video { button.on_hover_text(tr("Every page in turn, each for its duration (30 frames per second)")) } else { button };
        if button.clicked() {
            ui.close();
            #[cfg(target_arch = "wasm32")]
            crate::web::export(app, cmd, ext, &extra, &name);
            if let Some(path) = io::pick_save(&format!("{name}.{ext}"), (ext, &[ext])) {
                let mut p = extra;
                if let Some(o) = p.as_object_mut() {
                    o.insert("path".into(), json!(path.display().to_string()));
                }
                #[cfg(not(target_arch = "wasm32"))]
                if video {
                    app.editor.video_job = VideoJob::start(app, ui.ctx(), p, path);
                    if app.editor.video_job.is_none() {
                        app.set_status(tr("Could not start the video export"));
                    }
                    continue;
                }
                if app.run(cmd, &p).is_some() {
                    app.set_status(tr!("Downloaded {path}", path = path.display()));
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
    #[cfg(target_arch = "wasm32")]
    crate::web::save(app, &name);
    if let Some(path) = io::pick_save(&format!("{name}.genha"), io::DESIGNS)
        && app.run("file.saveAs", &json!({"path": path.display().to_string()})).is_some()
    {
        app.set_status(tr!("Saved {path}", path = path.display()));
    }
}

fn status_bar(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::bottom("status").exact_size(24.0).frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 3))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                if let Some(d) = app.session.active() {
                    if let Some(p) = d.doc.pages.get(d.page) {
                        let page = tr!("page {page} of {pages}", page = d.page + 1, pages = d.doc.pages.len());
                        ui.colored_label(t.text_dim, format!("{}  ·  {:.0} × {:.0} px  ·  {page}", crate::shown_name(&p.name), p.width, p.height));
                    }
                    if !d.selection.is_empty() {
                        ui.colored_label(t.text_dim, format!("·  {}", tr!("{n} selected", n = d.selection.len())));
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(job) = &app.editor.video_job {
                    use std::sync::atomic::Ordering;
                    let (n, total) = (job.progress.0.load(Ordering::Relaxed), job.progress.1.load(Ordering::Relaxed));
                    ui.separator();
                    ui.add(egui::ProgressBar::new(if total > 0 { n as f32 / total as f32 } else { 0.0 }).desired_width(120.0));
                    ui.colored_label(t.text, tr!("Exporting video… {n} of {total} frames", n = n, total = total));
                    if ui.small_button(tr("Cancel")).clicked() {
                        job.cancel();
                    }
                }
            });
        },
    );
}

fn page_strip(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::bottom("pages")
        .exact_size(118.0)
        .frame(egui::Frame::new().fill(t.panel_alt).inner_margin(egui::Margin::symmetric(14, 10)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            let Some(d) = app.session.active() else { return };
            let doc = d.doc.clone();
            let current = d.page;
            let revision = d.revision;
            let ctx = ui.ctx().clone();
            let mut goto = None;
            let mut action: Option<(&str, Value)> = None;
            let count = doc.pages.len();
            let menu = |ui: &mut egui::Ui, i: usize, action: &mut Option<(&str, Value)>| {
                if ui.button(tr("Duplicate page")).clicked() {
                    *action = Some(("page.duplicate", json!({"index": i})));
                    ui.close();
                }
                if i > 0 && ui.button(tr("Move left")).clicked() {
                    *action = Some(("page.move", json!({"from": i, "to": i - 1})));
                    ui.close();
                }
                if i + 1 < count && ui.button(tr("Move right")).clicked() {
                    *action = Some(("page.move", json!({"from": i, "to": i + 1})));
                    ui.close();
                }
                if count > 1 && ui.button(egui::RichText::new(tr("Delete page")).color(t.danger)).clicked() {
                    *action = Some(("page.delete", json!({"index": i})));
                    ui.close();
                }
            };
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (i, page) in doc.pages.iter().enumerate() {
                        let fonts = app.session.fonts.clone();
                        let stamp = if i == current { revision } else { 0 };
                        let key = format!("page:{}:{}", d_key(&doc), page.id.0);
                        let tex = app.thumbs.get(&ctx, &key, stamp, || genha_render::render_thumbnail(&doc, i, &fonts, 200).ok());
                        let (rect, r) = ui.allocate_exact_size(egui::vec2(120.0, 96.0), egui::Sense::click());
                        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
                        // Lifts a little under the pointer.
                        let lift = ctx.animate_bool_with_time(egui::Id::new(("page_lift", page.id.0)), r.hovered(), 0.15);
                        let card = rect.translate(egui::vec2(0.0, -2.0 * lift));
                        if lift > 0.0 {
                            ui.painter().rect_filled(card.translate(egui::vec2(0.0, 3.0)), 12, egui::Color32::from_black_alpha((26.0 * lift) as u8));
                        }
                        ui.painter().rect_filled(card, 12, t.panel);
                        let size = thumbs::fit_size(page.width as f32, page.height as f32, card.shrink(10.0).size());
                        let img = egui::Rect::from_center_size(card.center(), size);
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
                                ui.painter().rect_filled(img, 3, t.border);
                            }
                        }
                        let stroke = if i == current { egui::Stroke::new(2.5, t.accent) } else { egui::Stroke::new(1.0, t.border) };
                        ui.painter().rect_stroke(card, 12, stroke, egui::StrokeKind::Inside);
                        // The page number on a small badge.
                        let badge = egui::Rect::from_min_size(card.left_bottom() + egui::vec2(6.0, -22.0), egui::vec2(22.0, 16.0));
                        ui.painter().rect_filled(badge, 8, if i == current { t.accent } else { t.text.gamma_multiply(0.7) });
                        ui.painter().text(
                            badge.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", i + 1),
                            egui::FontId::proportional(10.5),
                            t.accent_text,
                        );
                        // "…" for the page's menu, under the pointer.
                        let dots = egui::Rect::from_min_size(card.right_top() + egui::vec2(-28.0, 4.0), egui::vec2(24.0, 22.0));
                        let dots_r = ui.interact(dots, egui::Id::new(("page_menu", page.id.0)), egui::Sense::click());
                        if r.hovered() || dots_r.hovered() {
                            ui.painter().rect_filled(dots, 8, if dots_r.hovered() { t.accent_soft } else { t.panel });
                            crate::glyphs::draw(ui.painter(), crate::glyphs::Glyph::More, dots.shrink(4.0), t.text);
                        }
                        egui::Popup::menu(&dots_r).show(|ui| menu(ui, i, &mut action));
                        if r.clicked() {
                            goto = Some(i);
                        }
                        r.context_menu(|ui| menu(ui, i, &mut action));
                    }
                    // A tile to add a page.
                    let (rect, r) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), egui::Sense::click());
                    let r = r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr("Add page"));
                    let k = ctx.animate_bool_with_time(r.id.with("hover"), r.hovered(), 0.12);
                    ui.painter().rect_filled(rect, 12, t.panel.lerp_to_gamma(t.accent_soft, k));
                    let pts = vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
                    for sh in egui::Shape::dashed_line(&pts, egui::Stroke::new(1.3, t.border.lerp_to_gamma(t.accent, k)), 6.0, 4.0) {
                        ui.painter().add(sh);
                    }
                    crate::glyphs::draw(
                        ui.painter(),
                        crate::glyphs::Glyph::Plus,
                        egui::Rect::from_center_size(rect.center(), egui::vec2(26.0, 26.0)),
                        t.text.lerp_to_gamma(t.accent, k),
                    );
                    if r.clicked() {
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

/// The latest message as a small card that rises at the bottom of the window and fades away.
fn toast(app: &App, ctx: &egui::Context, t: &Tokens) {
    const SHOWN: f64 = 4.0;
    let Some((msg, at)) = &app.status else { return };
    let age = crate::now() - at;
    if !(0.0..SHOWN).contains(&age) {
        return;
    }
    // In over 0.2 s, out over the last 0.4 s.
    let alpha = ((age / 0.2).min(1.0).min((SHOWN - age) / 0.4)).clamp(0.0, 1.0) as f32;
    let rise = 10.0 * (1.0 - (age / 0.2).min(1.0) as f32);
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("toast"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -150.0 + rise))
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
            ui.set_max_width((screen.width() - 80.0).clamp(200.0, 560.0));
            egui::Frame::new()
                .fill(t.text)
                .corner_radius(14)
                .inner_margin(egui::Margin::symmetric(16, 10))
                .shadow(egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: egui::Color32::from_black_alpha(50) })
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(msg).color(t.panel).size(13.0));
                });
        });
    ctx.request_repaint();
}

/// A key that changes when another document is active (thumbnails per document).
fn d_key(doc: &std::sync::Arc<genha_doc::Document>) -> u64 {
    doc.pages.first().map(|p| p.id.0).unwrap_or(0) ^ (doc.next_id << 20)
}

fn left_rail(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::left("rail")
        .exact_size(78.0)
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(6, 10)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            let ctx = ui.ctx().clone();
            let fonts = app.session.fonts.clone();
            let mut picked = None;
            // The soft pill behind the active tab slides from tab to tab.
            let shown = crate::properties::context_target(app).is_none();
            let active = LeftTab::ALL.iter().position(|x| *x == app.left_tab).unwrap_or(0) as f32;
            let at = ui.ctx().animate_value_with_time(egui::Id::new("rail_pill"), active, 0.18);
            let first = ui.cursor().min;
            if shown {
                let center = egui::pos2(ui.max_rect().center().x, first.y + 19.0 + at * 62.0);
                ui.painter().rect_filled(egui::Rect::from_center_size(center, egui::vec2(44.0, 32.0)), 12, t.accent_soft);
            }
            ui.vertical_centered(|ui| {
                // Items 62 points apart (58 + 4), where the pill above expects them.
                ui.spacing_mut().item_spacing.y = 0.0;
                for tab in LeftTab::ALL {
                    // The tab whose content shows (none while an object's panel is open).
                    let on = app.left_tab == tab && shown;
                    let (rect, r) = ui.allocate_exact_size(egui::vec2(66.0, 58.0), egui::Sense::click());
                    let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let back = egui::Rect::from_center_size(rect.center_top() + egui::vec2(0.0, 19.0), egui::vec2(44.0, 32.0));
                    let k = crate::theme::hover_fade(ui.ctx(), back, r.hovered() && !on);
                    if k > 0.0 {
                        ui.painter().rect_filled(back, 12, t.panel_alt.gamma_multiply(k));
                    }
                    let ink = if on {
                        t.accent
                    } else if r.hovered() {
                        t.text
                    } else {
                        t.text_dim
                    };
                    let icon = match tab {
                        LeftTab::Templates => crate::art::Icon::Grid,
                        LeftTab::Elements => crate::art::Icon::Shapes,
                        LeftTab::Text => crate::art::Icon::Text,
                        LeftTab::Media => crate::art::Icon::Lib("image"),
                        LeftTab::Brand => crate::art::Icon::Brand,
                        LeftTab::Layers => crate::art::Icon::Layers,
                        LeftTab::Page => crate::art::Icon::Page,
                    };
                    let mut art = crate::art::Art::new(&mut app.thumbs, &ctx, fonts.clone());
                    art.icon(ui.painter(), icon, egui::Rect::from_center_size(back.center(), egui::vec2(20.0, 20.0)), ink);
                    ui.painter().text(
                        egui::pos2(rect.center().x, rect.bottom() - 10.0),
                        egui::Align2::CENTER_CENTER,
                        tr(tab.label()),
                        egui::FontId::proportional(11.0),
                        ink,
                    );
                    if r.clicked() {
                        picked = Some(tab);
                    }
                    ui.add_space(4.0);
                }
            });
            if let Some(tab) = picked {
                app.left_tab = tab;
                // The rail always wins over the text and picture panels.
                if let Some((id, _)) = crate::properties::context_target(app) {
                    app.editor.panel_closed = Some(id);
                }
            }
        });
}

fn left_panel(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    match app.left_tab {
        LeftTab::Templates => crate::tabs::templates(app, ui, t),
        LeftTab::Elements => crate::tabs::elements(app, ui, t),
        LeftTab::Text => crate::tabs::text(app, ui, t),
        LeftTab::Media => crate::tabs::media(app, ui, t),
        LeftTab::Brand => crate::tabs::brand(app, ui, t),
        LeftTab::Layers => crate::tabs::layers(app, ui, t),
        LeftTab::Page => crate::properties::page_panel(app, ui, t),
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

/// Asks how to use a template clicked while the design has content: as a new design (the design
/// stays as it is), in place of the current page (undoable), or as an added page.
fn template_dialog(app: &mut App, ctx: &egui::Context, t: &Tokens) {
    let Some(id) = app.editor.template_ask else { return };
    let name = genha_templates::catalog().into_iter().find(|tp| tp.id == id).map(|tp| tp.name).unwrap_or(id);
    let frame = egui::Frame::new().fill(t.panel).corner_radius(18).inner_margin(22).stroke(egui::Stroke::new(1.0, t.border));
    let mut choice: Option<Option<Value>> = None;
    let r = egui::Modal::new(egui::Id::new("template_ask")).frame(frame).backdrop_color(t.scrim).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.label(egui::RichText::new(tr!("Use “{name}”?", name = name)).size(19.0).strong().color(t.text));
        ui.add_space(6.0);
        ui.label(egui::RichText::new(tr("Your design has content. How should the template be used?")).color(t.text_dim));
        ui.add_space(14.0);
        let option = |ui: &mut egui::Ui, title: &str, sub: &str, primary: bool| {
            let (fill, ink, dim) = if primary { (t.accent, t.accent_text, t.accent_text) } else { (t.panel_alt, t.text, t.text_dim) };
            let text = egui::text::LayoutJob::default();
            let mut job = text;
            job.append(title, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(14.5), color: ink, ..Default::default() });
            job.append("\n", 0.0, egui::TextFormat::default());
            job.append(sub, 0.0, egui::TextFormat { font_id: egui::FontId::proportional(12.0), color: dim, ..Default::default() });
            let b = egui::Button::new(job).fill(fill).corner_radius(12).min_size(egui::vec2(ui.available_width(), 52.0));
            ui.add(b).clicked()
        };
        if option(ui, tr("Open as a new design"), tr("Your design stays as it is"), true) {
            choice = Some(Some(json!({"id": id})));
        }
        ui.add_space(6.0);
        if option(ui, tr("Replace this page"), tr("The page becomes the template; Undo brings it back"), false) {
            choice = Some(Some(json!({"id": id, "replace": true})));
        }
        ui.add_space(6.0);
        if option(ui, tr("Add as a new page"), tr("After the current page"), false) {
            choice = Some(Some(json!({"id": id, "as_page": true})));
        }
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            if ui.add(egui::Button::new(egui::RichText::new(tr("Cancel")).color(t.text_dim)).frame(false)).clicked() {
                choice = Some(None);
            }
        });
    });
    if r.should_close() && choice.is_none() {
        choice = Some(None);
    }
    if let Some(run) = choice {
        app.editor.template_ask = None;
        if let Some(p) = run {
            app.run("template.use", &p);
            app.view.fit_pending = true;
        }
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
    if ctx.egui_wants_keyboard_input() || app.session.text_editing.is_some() {
        return;
    }
    // Enter edits the selected text box in place.
    let one_text = app.session.active().is_some_and(|d| {
        let sel = d.selected_elements();
        sel.len() == 1 && sel.iter().all(|e| matches!(e.kind, ElementKind::Text(_)) && !e.locked)
    });
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
        if one_text && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
            todo.push(("text.beginEdit", json!({})));
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::F5) {
            todo.push(("ui.present", json!({})));
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
            "file.new" => {
                app.go_home();
                crate::popups::open_size_picker(app, None);
            }

            "ui.present" => crate::motion::start_presenting(app, ctx),
            "select.none" => {
                // Esc also stops a playing animation.
                crate::motion::stop(app);
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
    egui::Window::new(tr("Commands"))
        .id(egui::Id::new("command_palette"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .show(ctx, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut app.editor.palette_query).hint_text(tr("Type a command…")).desired_width(f32::INFINITY));
            r.request_focus();
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                let mut first = None;
                for e in palette_entries(&app.editor.palette_query) {
                    let enabled = (e.spec.enabled)(&app.session).is_ok();
                    first.get_or_insert(e.spec.id);
                    let mut job = egui::text::LayoutJob::default();
                    let font = egui::TextStyle::Button.resolve(ui.style());
                    let dim = if enabled { t.text_dim } else { t.text_dim.gamma_multiply(0.6) };
                    let ink = if enabled { t.text } else { t.text_dim };
                    job.append(&format!("{}  ›  ", e.menu), 0.0, egui::TextFormat::simple(font.clone(), dim));
                    job.append(&e.label, 0.0, egui::TextFormat::simple(font.clone(), ink));
                    if let Some(sc) = e.spec.shortcut {
                        job.append(&format!("    {}", shortcut_text(sc)), 0.0, egui::TextFormat::simple(font, dim));
                    }
                    if ui.add_enabled(enabled, egui::Button::new(job).frame(false)).on_hover_text(e.spec.id).clicked() {
                        run = Some(e.spec.id);
                    }
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    run = first;
                }
            });
            ui.colored_label(t.text_dim, tr("Commands that need parameters are easier from the panels."));
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

/// A command in the palette: its menu path and label in the UI's language.
pub(crate) struct PaletteEntry {
    pub spec: &'static genha_engine::CommandSpec,
    pub menu: String,
    pub label: String,
}

/// The commands the palette lists for `query`, which matches the label and menu in the UI's
/// language or in English, or the command id.
pub(crate) fn palette_entries(query: &str) -> Vec<PaletteEntry> {
    let q = query.trim().to_lowercase();
    genha_engine::command_specs()
        .iter()
        .filter(|spec| !spec.menu.is_empty())
        .map(|spec| PaletteEntry { spec, menu: spec.menu.iter().map(|m| tr(m)).collect::<Vec<_>>().join(" › "), label: tr(spec.label).to_string() })
        .filter(|e| {
            q.is_empty()
                || e.label.to_lowercase().contains(&q)
                || e.menu.to_lowercase().contains(&q)
                || e.spec.label.to_lowercase().contains(&q)
                || e.spec.id.to_lowercase().contains(&q)
        })
        .collect()
}

fn versions_window(app: &mut App, ctx: &egui::Context, t: &Tokens) {
    if !app.editor.versions_open {
        return;
    }
    let list = app.session.execute("version.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let mut open = true;
    let mut action: Option<(&str, Value)> = None;
    egui::Window::new(tr("Version history")).id(egui::Id::new("versions")).open(&mut open).default_width(360.0).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.editor.version_name).hint_text(tr("Name this version")).desired_width(200.0));
            if ui.button(tr("Save version")).clicked() {
                action = Some(("version.save", json!({"name": app.editor.version_name.trim()})));
                app.editor.version_name.clear();
            }
        });
        ui.label(egui::RichText::new(tr("Versions are kept inside the design file. Saving adds an automatic version.")).size(11.0).color(t.text_dim));
        ui.separator();
        if list.is_empty() {
            ui.colored_label(t.text_dim, tr("No versions yet."));
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
                    ui.colored_label(t.text_dim, format!("{when} · {}", tr_n!("{n} page", "{n} pages", v["pages"].as_u64().unwrap_or(0))));
                });
                ui.horizontal(|ui| {
                    if ui.small_button(tr("Restore")).clicked() {
                        action = Some(("version.restore", json!({"id": id})));
                    }
                    if ui.small_button(tr("Open as copy")).clicked() {
                        action = Some(("version.copy", json!({"id": id})));
                    }
                    if !bookmarked && ui.small_button(tr("Keep")).on_hover_text(tr("Bookmark: never pruned")).clicked() {
                        action = Some(("version.rename", json!({"id": id, "name": name, "bookmarked": true})));
                    }
                    if ui.small_button(tr("Delete")).clicked() {
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
        return tr!("unknown time");
    }
    match now - created {
        s if s < 60 => tr!("just now"),
        s if s < 3600 => tr!("{n} min ago", n = s / 60),
        s if s < 86_400 => tr!("{n} h ago", n = s / 3600),
        s => tr_n!("{n} day ago", "{n} days ago", s / 86_400),
    }
}

/// A command's shortcut (`Cmd+Shift+Z`) as this platform and language name the keys.
pub(crate) fn shortcut_text(shortcut: &str) -> String {
    let mac = cfg!(target_os = "macos");
    shortcut
        .split('+')
        .map(|k| match k {
            "Cmd" if mac => "Cmd",
            "Cmd" | "Ctrl" => tr(N_("Ctrl")),
            "Shift" => tr(N_("Shift")),
            "Delete" => tr(N_("Del")),
            "Escape" => tr(N_("Esc")),
            k => k,
        })
        .collect::<Vec<_>>()
        .join("+")
}
