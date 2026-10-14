//! The editor: top bar, left panels, page strip, status bar, command palette, shortcuts.

use serde_json::{Value, json};

use crate::theme::{Tokens, to_egui};
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
    /// The top bar's groups as last drawn (document, tools, file, view, commands), in points.
    pub top_bar_widths: Option<[f32; 5]>,
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
    // A selected text box or picture is edited in the left panel; the right-hand panel stays
    // closed.
    let context = crate::properties::context_target(app);
    egui::Panel::left("left_panel").resizable(true).default_size(290.0).min_size(260.0).frame(panel_frame(&t)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match context {
            Some((id, kind)) => crate::properties::context_panel(app, ui, &t, id, kind),
            None => left_panel(app, ui, &t),
        });
    });
    if context.is_none() {
        egui::Panel::right("properties").resizable(true).default_size(290.0).min_size(240.0).frame(panel_frame(&t)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| crate::properties::show(app, ui, &t));
        });
    }
    crate::layers::show(app, ui, &t);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| crate::canvas::show(app, ui, &t));
    palette(app, ui.ctx(), &t);
    versions_window(app, ui.ctx(), &t);
    template_dialog(app, ui.ctx(), &t);
    crate::models::show(app, ui.ctx());
}

fn panel_frame(t: &Tokens) -> egui::Frame {
    egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 10)).stroke(egui::Stroke::new(1.0, t.border))
}

fn top_bar(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    // One row when everything fits (text is longer in some languages, windows can be narrow);
    // otherwise two: document and file actions above, tools and view below. The groups' widths
    // are measured as they are drawn (estimated from their labels until then).
    let gaps = 4.0 * (ui.ctx().global_style().spacing.item_spacing.x * 2.0 + 2.0) + 24.0;
    let need = app.editor.top_bar_widths.map_or_else(|| top_bar_width(app, ui), |w| w.iter().sum::<f32>() + gaps);
    let one_row = need <= ui.available_width();
    let mut widths = [0.0f32; 5];
    let mut measure = |i: usize, r: egui::Response| {
        if let Some(w) = widths.get_mut(i) {
            *w = r.rect.width();
        }
    };
    egui::Panel::top("top_bar")
        .exact_size(if one_row { 48.0 } else { 84.0 })
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(12, 8)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            if one_row {
                ui.horizontal_centered(|ui| {
                    measure(0, ui.scope(|ui| document_group(app, ui, t)).response);
                    ui.separator();
                    measure(1, ui.scope(|ui| tools(app, ui, t)).response);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        measure(2, ui.scope(|ui| file_group(app, ui, t)).response);
                        ui.separator();
                        measure(3, ui.scope(|ui| view_group(app, ui)).response);
                        measure(4, ui.scope(|ui| commands_button(app, ui)).response);
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    measure(0, ui.scope(|ui| document_group(app, ui, t)).response);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        measure(2, ui.scope(|ui| file_group(app, ui, t)).response);
                        measure(4, ui.scope(|ui| commands_button(app, ui)).response);
                    });
                });
                ui.horizontal(|ui| {
                    measure(1, ui.scope(|ui| tools(app, ui, t)).response);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        measure(3, ui.scope(|ui| view_group(app, ui)).response);
                    });
                });
            }
        });
    app.editor.top_bar_widths = Some(widths);
}

/// About how wide the top bar is on one row, from its labels in the UI's language.
fn top_bar_width(app: &App, ui: &egui::Ui) -> f32 {
    // The app's style (the theme may only reach this frame's widgets next frame).
    let style = ui.ctx().global_style();
    let font = egui::TextStyle::Button.resolve(&style);
    let pad = style.spacing.button_padding.x * 2.0 + style.spacing.item_spacing.x;
    let w = |s: &str| ui.painter().layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE).size().x + pad;
    let sep = style.spacing.item_spacing.x * 2.0 + 2.0;
    let title = app.session.active().map(|d| crate::shown_name(&d.doc.title)).unwrap_or_default();
    let labels = [
        "GenHa",
        "Undo",
        "Redo",
        "Resize",
        "Download",
        "Versions",
        "Save",
        "Present",
        if tr("Stop").len() > tr("Play").len() { "Stop" } else { "Play" },
        "Snap",
        "Fit",
        "Commands",
    ];
    let tools: f32 = genha_tools::ToolId::ALL.iter().map(|tool| w(tr(tool.label()))).sum();
    labels.iter().map(|l| w(tr(l))).sum::<f32>() + tools + w(&format!("{title} •")) + w("100%") + w("+") + w("-") + 7.0 * sep
}

/// Home, the design's name, undo and redo, resize.
fn document_group(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    if ui.button(egui::RichText::new("GenHa").strong().color(t.accent)).on_hover_text(tr("Home")).clicked() {
        app.go_home();
    }
    ui.separator();
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
            if ui
                .add(egui::Label::new(egui::RichText::new(format!("{title}{}", if dirty { " •" } else { "" })).strong()).sense(egui::Sense::click()))
                .on_hover_text(tr("Click to rename"))
                .clicked()
            {
                app.editor.renaming = Some(title);
            }
        }
    }
    ui.separator();
    let can_undo = app.session.active().is_some_and(|d| !d.history.undo.is_empty());
    let can_redo = app.session.active().is_some_and(|d| !d.history.redo.is_empty());
    if ui.add_enabled(can_undo, egui::Button::new(tr("Undo"))).on_hover_text(tr("Undo (Ctrl+Z)")).clicked() {
        app.run("edit.undo", &json!({}));
    }
    if ui.add_enabled(can_redo, egui::Button::new(tr("Redo"))).on_hover_text(tr("Redo (Ctrl+Shift+Z)")).clicked() {
        app.run("edit.redo", &json!({}));
    }
    ui.separator();
    resize_menu(app, ui);
}

/// Download, versions, save (right to left).
fn file_group(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.menu_button(egui::RichText::new(tr("Download")).strong().color(t.accent), |ui| download_menu(app, ui))
        .response
        .on_hover_text(tr("Export as PNG, JPEG, SVG, PDF, GIF, animated PNG or MP4 video"));
    if ui.button(tr("Versions")).on_hover_text(tr("Version history")).clicked() {
        app.editor.versions_open = !app.editor.versions_open;
    }
    if ui.button(tr("Save")).on_hover_text(tr("Save (Ctrl+S)")).clicked() {
        save(app, false);
    }
}

/// Present, play, snapping, zoom (right to left).
fn view_group(app: &mut App, ui: &mut egui::Ui) {
    if ui.button(tr("Present")).on_hover_text(tr("Show the design full screen, page by page (F5)")).clicked() {
        crate::motion::start_presenting(app, ui.ctx());
    }
    let playing = app.motion.playing.is_some();
    if ui
        .add(egui::Button::new(if playing { tr("Stop") } else { tr("Play") }).selected(playing))
        .on_hover_text(tr("Play this page's animations"))
        .clicked()
    {
        if playing {
            crate::motion::stop(app);
        } else {
            crate::motion::play(app, ui.ctx());
        }
    }
    ui.separator();
    let snapping = app.session.snapping;
    if ui
        .add(egui::Button::new(tr("Snap")).selected(snapping))
        .on_hover_text(tr("Smart guides: snap to the page, other objects, equal gaps and sizes (hold Ctrl/Cmd while dragging to skip)"))
        .clicked()
    {
        app.run("view.snapping", &json!({"on": !snapping}));
    }
    let strip = app.session.layer_strip;
    if ui
        .add(egui::Button::new(tr("Layers")).selected(strip))
        .on_hover_text(tr("The layer strip: every object as a picture, the front at the top; drag one up or down to restack it"))
        .clicked()
    {
        app.run("view.layerStrip", &json!({"on": !strip}));
    }
    if ui.button(tr("Fit")).on_hover_text(tr("Fit page (Ctrl+0)")).clicked() {
        app.view.fit_pending = true;
    }
    ui.label(format!("{:.0}%", app.view.zoom * 100.0));
    if ui.button("+").clicked() {
        crate::canvas::zoom_by(app, 1.25, None);
    }
    if ui.button("-").clicked() {
        crate::canvas::zoom_by(app, 0.8, None);
    }
}

fn commands_button(app: &mut App, ui: &mut egui::Ui) {
    if ui.button(tr("Commands")).on_hover_text(tr("All commands (Ctrl+K)")).clicked() {
        app.editor.palette_open = true;
    }
}

fn tools(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    use genha_tools::ToolId;
    for tool in ToolId::ALL {
        let icon = tr(tool.label());
        let on = app.session.tools.tool == tool;
        let b = egui::Button::new(egui::RichText::new(icon).color(if on { t.accent_text } else { t.text })).selected(on).fill(if on {
            t.accent
        } else {
            t.panel_alt
        });
        if ui.add(b).on_hover_text(format!("{icon} ({})", tool.shortcut())).clicked() {
            app.run("tool.set", &json!({"tool": tool.id()}));
        }
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

fn download_menu(app: &mut App, ui: &mut egui::Ui) {
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
        let button = ui.add_enabled(!busy, egui::Button::new(tr(label)));
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
                if let Some((msg, at)) = &app.status
                    && crate::now() - at < 8.0
                {
                    ui.separator();
                    ui.colored_label(t.text, msg);
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
                            if ui.button(tr("Duplicate page")).clicked() {
                                action = Some(("page.duplicate", json!({"index": i})));
                                ui.close();
                            }
                            if ui.button(tr("Delete page")).clicked() {
                                action = Some(("page.delete", json!({"index": i})));
                                ui.close();
                            }
                            if i > 0 && ui.button(tr("Move left")).clicked() {
                                action = Some(("page.move", json!({"from": i, "to": i - 1})));
                                ui.close();
                            }
                            if i + 1 < doc.pages.len() && ui.button(tr("Move right")).clicked() {
                                action = Some(("page.move", json!({"from": i, "to": i + 1})));
                                ui.close();
                            }
                        });
                    }
                    if ui.add(egui::Button::new(egui::RichText::new(tr("+\nAdd page")).size(13.0)).min_size(egui::vec2(90.0, 80.0))).clicked() {
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
        .exact_size(78.0)
        .frame(egui::Frame::new().fill(t.panel).inner_margin(egui::Margin::symmetric(6, 10)).stroke(egui::Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            let ctx = ui.ctx().clone();
            let fonts = app.session.fonts.clone();
            let mut picked = None;
            ui.vertical_centered(|ui| {
                for tab in LeftTab::ALL {
                    // The tab whose content shows (none while the text or picture panel is open).
                    let on = app.left_tab == tab && crate::properties::context_target(app).is_none();
                    let (rect, r) = ui.allocate_exact_size(egui::vec2(66.0, 58.0), egui::Sense::click());
                    let back = egui::Rect::from_center_size(rect.center_top() + egui::vec2(0.0, 19.0), egui::vec2(44.0, 32.0));
                    if on {
                        ui.painter().rect_filled(back, 12, t.accent_soft);
                    } else if r.hovered() {
                        ui.painter().rect_filled(back, 12, t.panel_alt);
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
    heading(ui, tr("Templates"));
    ui.add(egui::TextEdit::singleline(&mut app.editor.template_query).hint_text(tr("Search templates")));
    ui.label(
        egui::RichText::new(tr("Click: use (asks first when your design has content) · right-click: add as a page")).size(11.0).color(t.text_dim),
    );
    let q = app.editor.template_query.to_lowercase();
    let ctx = ui.ctx().clone();
    let mut chosen: Option<(&'static str, bool)> = None;
    ui.horizontal_wrapped(|ui| {
        for tp in genha_templates::catalog() {
            if !q.is_empty()
                && !tp.name.to_lowercase().contains(&q)
                && !tp.category.to_lowercase().contains(&q)
                && !tr(tp.category).to_lowercase().contains(&q)
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
            let r = r.on_hover_text(format!("{} — {}", tp.name, tr(tp.category)));
            if r.clicked() {
                chosen = Some((tp.id, false));
            }
            if r.secondary_clicked() {
                chosen = Some((tp.id, true));
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

fn elements_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, tr("Shapes & icons"));
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
        ui.label(egui::RichText::new(tr(cat)).color(t.text_dim).size(12.0));
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
                if r.on_hover_text(tr(item.label)).clicked() {
                    add = Some(item.id);
                }
            }
        });
    }
    if let Some(id) = add {
        app.run("library.addShape", &json!({"id": id}));
    }
    ui.separator();
    heading(ui, tr("Charts & tables"));
    ui.horizontal_wrapped(|ui| {
        for (label, kind, hint) in [
            (N_("Bar"), "bar", N_("Add a bar chart")),
            (N_("Horizontal bar"), "horizontal-bar", N_("Add a horizontal bar chart")),
            (N_("Line"), "line", N_("Add a line chart")),
            (N_("Area"), "area", N_("Add an area chart")),
            (N_("Pie"), "pie", N_("Add a pie chart")),
            (N_("Donut"), "donut", N_("Add a donut chart")),
            (N_("Progress"), "progress", N_("Add a progress bar")),
            (N_("Ring"), "ring", N_("Add a progress ring")),
        ] {
            if ui.button(tr(label)).on_hover_text(tr(hint)).clicked() {
                app.run("chart.add", &json!({"kind": kind}));
            }
        }
        if ui.button(tr("Table")).on_hover_text(tr("Add a 3 x 3 table")).clicked() {
            app.run("table.add", &json!({}));
        }
    });
    ui.separator();
    heading(ui, tr("QR code"));
    ui.add(egui::TextEdit::singleline(&mut app.editor.qr_text).hint_text("https://…"));
    if ui.add_enabled(!app.editor.qr_text.trim().is_empty(), egui::Button::new(tr("Add QR code"))).clicked() {
        let text = app.editor.qr_text.trim().to_string();
        app.run("element.qrCode", &json!({"text": text}));
    }
    ui.separator();
    heading(ui, tr("Collage"));
    let pictures =
        app.session.active().map(|d| d.selected_elements().iter().filter(|e| matches!(e.kind, ElementKind::Image(_))).count()).unwrap_or(0);
    if pictures == 0 {
        ui.colored_label(t.text_dim, tr("Select pictures on the page to arrange them as a collage."));
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

fn text_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, tr("Text"));
    for (role, label, size) in
        [("heading", N_("Add a heading"), 22.0), ("subheading", N_("Add a subheading"), 17.0), ("body", N_("Add body text"), 13.0)]
    {
        let b = egui::Button::new(egui::RichText::new(tr(label)).size(size).strong()).min_size(egui::vec2(ui.available_width(), 40.0));
        if ui.add(b).clicked() {
            // The new box's text, in the language of the UI.
            let text = match role {
                "heading" => tr("Your headline"),
                "subheading" => tr("A subheading"),
                _ => tr("Write something here"),
            };
            app.run("text.add", &json!({"text": text, "role": role}));
        }
    }
    ui.separator();
    heading(ui, tr("Text styles"));
    for p in genha_templates::text_presets() {
        let color = to_egui(p.style.color);
        let mut rt = egui::RichText::new(tr(p.sample)).size((p.style.font_size as f32 / 4.0).clamp(11.0, 26.0));
        if p.style.bold {
            rt = rt.strong();
        }
        if p.style.italic {
            rt = rt.italics();
        }
        let readable = if t.dark && color.r() < 80 && color.g() < 80 && color.b() < 80 { t.text } else { color };
        let b = egui::Button::new(rt.color(readable)).min_size(egui::vec2(ui.available_width(), 36.0));
        if ui.add(b).on_hover_text(tr(p.label)).clicked() {
            app.run("text.addPreset", &json!({"id": p.id, "text": tr(p.sample)}));
        }
    }
    ui.separator();
    heading(ui, tr("Fonts"));
    crate::fonts::add_fonts_ui(app, ui);
}

fn media_tab(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    heading(ui, tr("Your pictures"));
    if ui
        .add(
            egui::Button::new(egui::RichText::new(tr("Upload picture…")).color(t.accent_text))
                .fill(t.accent)
                .min_size(egui::vec2(ui.available_width(), 36.0)),
        )
        .clicked()
        && let Some(p) = io::pick_open(&[io::PICTURES])
    {
        app.run("image.place", &json!({"path": p.display().to_string()}));
    }
    ui.label(egui::RichText::new(tr("…or drop pictures on the window.")).size(11.0).color(t.text_dim));
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
    heading(ui, tr("Brand kit"));
    let Some(d) = app.session.active() else { return };
    let brand = d.doc.brand.clone();
    ui.label(egui::RichText::new(crate::shown_name(&brand.name)).strong());
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("Colours")).color(t.text_dim));
    let mut set: Option<(usize, genha_color::Color)> = None;
    let mut remove = None;
    for (i, c) in brand.colors.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut col = to_egui(c.color);
            if ui.color_edit_button_srgba(&mut col).changed() {
                set = Some((i, crate::theme::from_egui(col)));
            }
            ui.label(format!("{}  {}", crate::shown_name(&c.name), c.color.to_hex()));
            if ui.small_button("x").on_hover_text(tr("Remove")).clicked() {
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
        if ui.button(tr("Add colour")).clicked() {
            let c = genha_color::Color::rgb(col.r(), col.g(), col.b());
            app.run("brand.addColor", &json!({"color": c.to_hex()}));
        }
    });
    let has_picture = app.session.active().is_some_and(|d| d.selected_elements().iter().any(|e| matches!(e.kind, ElementKind::Image(_))));
    if ui.add_enabled(has_picture, egui::Button::new(tr("Colours from selected picture"))).clicked() {
        app.run("brand.fromPicture", &json!({}));
    }
    ui.separator();
    ui.label(egui::RichText::new(tr("Fonts")).color(t.text_dim));
    let families = app.font_families().to_vec();
    let (mut heading_font, mut body_font) = (brand.heading_font.clone(), brand.body_font.clone());
    font_combo(ui, "brand_heading", tr("Headings"), &mut heading_font, &families);
    font_combo(ui, "brand_body", tr("Body"), &mut body_font, &families);
    if heading_font != brand.heading_font || body_font != brand.body_font {
        app.run("brand.setFonts", &json!({"heading": heading_font, "body": body_font}));
    }
    ui.separator();
    if ui.button(tr("Apply brand to this page")).clicked() {
        app.run("brand.apply", &json!({}));
    }
    if ui.button(tr("Apply brand to all pages")).clicked() {
        app.run("brand.apply", &json!({"all_pages": true}));
    }
    ui.separator();
    heading(ui, tr("Palettes"));
    let mut chosen = None;
    for p in genha_templates::palettes() {
        let r = ui
            .horizontal(|ui| {
                for c in &p.colors {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4, to_egui(*c));
                }
                ui.label(tr(p.name));
            })
            .response
            .interact(egui::Sense::click());
        if r.on_hover_text(tr("Use as brand colours")).clicked() {
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
    heading(ui, tr("Layers"));
    let Some(d) = app.session.active() else { return };
    let Some(page) = d.doc.pages.get(d.page) else { return };
    let sel = d.selection.clone();
    let rows: Vec<(NodeId, String, &'static str, bool, bool)> =
        page.elements.iter().rev().map(|e| (e.id, crate::element_name(e), e.kind.type_name(), e.hidden, e.locked)).collect();
    if rows.is_empty() {
        ui.colored_label(t.text_dim, tr("This page is empty. Add text, shapes or pictures from the left."));
        return;
    }
    let mut action: Option<(&str, Value)> = None;
    for (id, name, kind, hidden, locked) in rows {
        let on = sel.contains(&id);
        ui.horizontal(|ui| {
            let eye = if hidden { tr("show") } else { tr("hide") };
            if ui.small_button(eye).on_hover_text(if hidden { tr("Show") } else { tr("Hide") }).clicked() {
                action = Some((if hidden { "object.show" } else { "object.hide" }, json!({"ids": [id.0]})));
            }
            let lock = if locked { tr("unlock") } else { tr("lock") };
            if ui.small_button(lock).on_hover_text(if locked { tr("Unlock") } else { tr("Lock") }).clicked() {
                action = Some((if locked { "object.unlock" } else { "object.lock" }, json!({"ids": [id.0]})));
            }
            let text = egui::RichText::new(format!("{name}  ")).color(if hidden { t.text_dim } else { t.text });
            if ui.selectable_label(on, text).on_hover_text(tr(kind)).clicked() {
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
            (N_("Forward"), "object.bringForward"),
            (N_("Backward"), "object.sendBackward"),
            (N_("To front"), "object.bringToFront"),
            (N_("To back"), "object.sendToBack"),
        ] {
            if ui.add_enabled(!sel.is_empty(), egui::Button::new(tr(label))).clicked() {
                action = Some((cmd, json!({})));
            }
        }
    });
    if let Some((cmd, p)) = action {
        app.run(cmd, &p);
    }
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
