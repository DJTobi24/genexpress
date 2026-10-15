//! My designs: the project folders, a grid of design cards (thumbnail, name, when it was edited),
//! search, sort, and each design's menu (open, rename, duplicate, move to a folder, delete). Cards
//! are dragged onto a folder to move them. Everything acts through `design.*` and `folder.*`.

use serde_json::json;

use crate::App;
use crate::art::{Art, Icon};
use crate::home::column;
use crate::popups::Modal;
use crate::theme::Tokens;
use genha_engine::cmd::designs::{DesignEntry, FolderFilter, Sort, filtered};
use genha_i18n::{tr, tr_n};

/// What a design card asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardAction {
    Open,
    Rename,
    Duplicate,
    Move(Option<String>),
    Delete,
    HideRecent,
}

/// A design being dragged (its id).
#[derive(Clone, Debug)]
pub struct Dragged(pub String);

/// The card's thumbnail texture.
fn thumbnail(app: &mut App, ctx: &egui::Context, e: &DesignEntry) -> Option<egui::TextureHandle> {
    let session = &app.session;
    app.thumbs.get(ctx, &format!("design:{}", e.id), e.version, || {
        let png = genha_engine::cmd::designs::thumbnail(session, &e.id)?;
        let img = genha_effects::decode(&png).ok()?;
        Some(genha_render::Raster { width: img.width(), height: img.height(), rgba: img.into_raw() })
    })
}

/// A design's card; what was asked of it. `recent`: on the home page's row (its menu can take it
/// off that row).
pub fn design_card(app: &mut App, ui: &mut egui::Ui, t: &Tokens, e: &DesignEntry, width: f32, recent: bool) -> Option<CardAction> {
    let ctx = ui.ctx().clone();
    let tex = thumbnail(app, &ctx, e);
    let box_h = (width * 0.75).round();
    let (rect, r) = ui.allocate_exact_size(egui::vec2(width, box_h + 50.0), egui::Sense::click_and_drag());
    let mut out = None;
    let p = ui.painter_at(rect.expand(2.0));
    let boxr = egui::Rect::from_min_size(rect.min, egui::vec2(width, box_h));
    p.rect_filled(boxr, 12, t.panel_alt);
    let (w, h) = match &tex {
        Some(tx) => (tx.size()[0] as f32, tx.size()[1] as f32),
        None if e.width > 0.0 && e.height > 0.0 => (e.width as f32, e.height as f32),
        None => (4.0, 3.0),
    };
    let img = egui::Rect::from_center_size(boxr.center(), crate::thumbs::fit_size(w, h, boxr.size() - egui::vec2(22.0, 22.0)));
    p.rect_filled(img.translate(egui::vec2(0.0, 2.0)), 3, t.art_shadow);
    match &tex {
        Some(tx) => {
            p.image(tx.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        None => {
            p.rect_filled(img, 3, t.panel);
        }
    }
    let open = app.session.docs.iter().any(|d| d.library_id.as_deref() == Some(e.id.as_str()));
    if open {
        let chip = egui::Rect::from_min_size(boxr.min + egui::vec2(8.0, 8.0), egui::vec2(12.0, 12.0));
        p.circle_filled(chip.center(), 5.0, t.accent);
    }
    let hovered = r.hovered() || r.context_menu_opened();
    if hovered {
        p.rect_stroke(boxr, 12, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    }
    let title = crate::shown_name(&e.title);
    let name = egui::WidgetText::from(egui::RichText::new(&title).size(13.0).strong().color(t.text)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width - 4.0,
        egui::TextStyle::Body,
    );
    p.galley(egui::pos2(rect.left() + 2.0, boxr.bottom() + 7.0), name, t.text);
    let when = crate::home::ago(crate::now(), e.edited.max(e.created));
    let mut sub = if when.is_empty() { String::new() } else { genha_i18n::tr!("Edited {when}", when = when) };
    if e.path.is_none() && cfg!(not(target_arch = "wasm32")) && sub.is_empty() {
        sub = tr("Kept in GenHa").to_string();
    }
    // Among all designs, the folder it is in.
    if !recent
        && app.home.folder == FolderFilter::All
        && let Some(f) = e.folder.as_deref().and_then(|f| app.library.folder(f))
    {
        sub = if sub.is_empty() { f.name.clone() } else { format!("{sub}, {}", f.name) };
    }
    let sub = egui::WidgetText::from(egui::RichText::new(sub).size(11.5).color(t.text_dim)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width - 4.0,
        egui::TextStyle::Body,
    );
    p.galley(egui::pos2(rect.left() + 2.0, boxr.bottom() + 27.0), sub, t.text_dim);
    // The ⋯ button (on hover) and the context menu offer the same.
    let dots = egui::Rect::from_min_size(egui::pos2(boxr.right() - 34.0, boxr.top() + 8.0), egui::vec2(26.0, 26.0));
    let dots_id = ui.id().with(("design_menu", &e.id));
    let menu_open = egui::Popup::is_id_open(ui.ctx(), dots_id);
    if hovered || menu_open {
        let dr = ui.interact(dots, dots_id.with("button"), egui::Sense::click());
        p.circle_filled(dots.center(), 13.0, if dr.hovered() { t.accent_soft } else { t.panel });
        Art::new(&mut app.thumbs, &ctx, app.session.fonts.clone()).icon(&p, Icon::Dots, dots.shrink(4.0), t.text);
        egui::Popup::menu(&dr).id(dots_id).show(|ui| {
            if let Some(a) = menu(app, ui, e, recent) {
                out = Some(a);
            }
        });
    }
    r.context_menu(|ui| {
        if let Some(a) = menu(app, ui, e, recent) {
            out = Some(a);
        }
    });
    if r.drag_started() {
        r.dnd_set_drag_payload(Dragged(e.id.clone()));
    }
    if r.dragged() {
        // A small copy of the card follows the pointer.
        if let Some(pos) = ctx.pointer_interact_pos() {
            let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("design_drag"));
            let dp = ctx.layer_painter(layer);
            let card = egui::Rect::from_center_size(pos + egui::vec2(0.0, 30.0), egui::vec2(120.0, 34.0));
            dp.rect_filled(card, 10, t.panel);
            dp.rect_stroke(card, 10, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
            let text = egui::WidgetText::from(egui::RichText::new(&title).size(12.0).color(t.text)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                104.0,
                egui::TextStyle::Body,
            );
            dp.galley(card.left_center() + egui::vec2(8.0, -7.0), text, t.text);
        }
    }
    let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
    let r = match &e.path {
        Some(path) => r.on_hover_text(path),
        None => r,
    };
    if r.clicked() && out.is_none() {
        out = Some(CardAction::Open);
    }
    out
}

/// A design's menu entries.
fn menu(app: &App, ui: &mut egui::Ui, e: &DesignEntry, recent: bool) -> Option<CardAction> {
    let mut out = None;
    if ui.button(tr("Open")).clicked() {
        out = Some(CardAction::Open);
    }
    if ui.button(tr("Rename…")).clicked() {
        out = Some(CardAction::Rename);
    }
    if ui.button(tr("Duplicate")).clicked() {
        out = Some(CardAction::Duplicate);
    }
    ui.menu_button(tr("Move to folder"), |ui| {
        if ui.add_enabled(e.folder.is_some(), egui::Button::new(tr("No folder"))).clicked() {
            out = Some(CardAction::Move(None));
        }
        if !app.library.folders.is_empty() {
            ui.separator();
        }
        for f in &app.library.folders {
            if ui.add_enabled(e.folder.as_deref() != Some(f.id.as_str()), egui::Button::new(&f.name)).clicked() {
                out = Some(CardAction::Move(Some(f.id.clone())));
            }
        }
    });
    ui.separator();
    if recent && ui.button(tr("Remove from recent designs")).clicked() {
        out = Some(CardAction::HideRecent);
    }
    let delete = if e.path.is_some() { tr("Remove from My designs…") } else { tr("Delete…") };
    if ui.button(delete).clicked() {
        out = Some(CardAction::Delete);
    }
    if out.is_some() {
        ui.close();
    }
    out
}

/// Carries out what a card asked for.
pub fn act(app: &mut App, e: &DesignEntry, a: CardAction) {
    match a {
        CardAction::Open => {
            app.run("design.open", &json!({"id": e.id}));
        }
        CardAction::Rename => app.home.modal = Some(Modal::RenameDesign { id: e.id.clone(), title: e.title.clone() }),
        CardAction::Duplicate => {
            if app.run("design.duplicate", &json!({"id": e.id})).is_some() {
                app.set_status(genha_i18n::tr!("Duplicated {title}", title = crate::shown_name(&e.title)));
            }
        }
        CardAction::Move(folder) => {
            app.run("design.move", &json!({"ids": [e.id], "folder": folder}));
        }
        CardAction::Delete => {
            app.home.modal = Some(Modal::DeleteDesigns { ids: vec![e.id.clone()], title: e.title.clone(), stored: usize::from(e.path.is_none()) })
        }
        CardAction::HideRecent => {
            app.run("recent.remove", &json!({"id": e.id}));
        }
    }
}

/// Makes `r` a place to drop a dragged design: highlighted while one is held over it, moved into
/// `folder` when dropped. Whether one was dropped.
pub fn drop_target(app: &mut App, ui: &egui::Ui, r: &egui::Response, folder: Option<String>, t: &Tokens) -> bool {
    if r.dnd_hover_payload::<Dragged>().is_some() {
        ui.painter().rect_stroke(r.rect.shrink(2.0), 12, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
    }
    if let Some(d) = r.dnd_release_payload::<Dragged>() {
        app.run("design.move", &json!({"ids": [d.0.clone()], "folder": folder}));
        return true;
    }
    false
}

/// The My designs page.
pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().id_salt("designs_scroll").auto_shrink([false, false]).show(ui, |ui| {
        column(ui, |ui| {
            ui.add_space(22.0);
            let title = match &app.home.folder {
                FolderFilter::Folder(id) => app.library.folder(id).map(|f| f.name.clone()).unwrap_or_else(|| tr("My designs").to_string()),
                FolderFilter::Unfiled => tr("Not in a folder").to_string(),
                FolderFilter::All => tr("My designs").to_string(),
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).size(28.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let b = egui::Button::new(egui::RichText::new(tr("New design")).color(t.accent_text))
                        .fill(t.accent)
                        .corner_radius(16)
                        .min_size(egui::vec2(0.0, 32.0));
                    if ui.add(b).clicked() {
                        crate::popups::open_size_picker(app, None);
                    }
                    if ui.add(egui::Button::new(tr("New folder")).corner_radius(16).min_size(egui::vec2(0.0, 32.0))).clicked() {
                        app.home.modal = Some(Modal::NewFolder { name: String::new(), color: 0 });
                    }
                });
            });
            ui.label(egui::RichText::new(where_kept()).color(t.text_dim));
            ui.add_space(16.0);
            folder_strip(app, ui, t);
            ui.add_space(18.0);
            toolbar(app, ui, t);
            ui.add_space(12.0);
            grid(app, ui, t);
            ui.add_space(40.0);
        });
    });
}

fn where_kept() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        tr("Every design you make or open, kept in this browser.")
    } else {
        tr("Every design you make or open. Designs not saved to a file are kept in GenHa's data folder.")
    }
}

/// All designs, those in no folder, and each folder (with its menu); designs drop onto them.
fn folder_strip(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let lib = &app.library;
    let unfiled = lib.designs.iter().filter(|e| e.folder.is_none()).count();
    let mut items: Vec<(FolderFilter, String, usize, Option<egui::Color32>)> = vec![
        (FolderFilter::All, tr("All designs").to_string(), lib.designs.len(), None),
        (FolderFilter::Unfiled, tr("Not in a folder").to_string(), unfiled, None),
    ];
    for f in &lib.folders {
        let color = f.color.as_deref().and_then(genha_color::Color::parse).map(crate::theme::to_egui);
        items.push((FolderFilter::Folder(f.id.clone()), f.name.clone(), lib.count(&f.id), color));
    }
    let mut chosen = None;
    let mut menu_action: Option<(String, &str)> = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
        for (filter, name, count, color) in items {
            let on = app.home.folder == filter;
            let (rect, r) = ui.allocate_exact_size(egui::vec2(178.0, 58.0), egui::Sense::click());
            let p = ui.painter_at(rect.expand(2.0));
            p.rect_filled(rect, 12, if on { t.accent_soft } else { t.panel });
            p.rect_stroke(rect, 12, egui::Stroke::new(1.0, if on || r.hovered() { t.accent } else { t.border }), egui::StrokeKind::Inside);
            let icon = match filter {
                FolderFilter::All => Icon::Grid,
                _ => Icon::Folder,
            };
            let ic = egui::Rect::from_min_size(rect.min + egui::vec2(12.0, 15.0), egui::vec2(28.0, 28.0));
            let tint = color.unwrap_or(if on { t.accent } else { t.text_dim });
            Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()).icon(&p, icon, ic, tint);
            let label = egui::WidgetText::from(egui::RichText::new(&name).size(13.5).strong().color(t.text)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                110.0,
                egui::TextStyle::Body,
            );
            p.galley(egui::pos2(rect.left() + 50.0, rect.top() + 11.0), label, t.text);
            p.text(
                egui::pos2(rect.left() + 50.0, rect.top() + 32.0),
                egui::Align2::LEFT_TOP,
                tr_n!("{n} design", "{n} designs", count),
                egui::FontId::proportional(11.5),
                t.text_dim,
            );
            let target = match &filter {
                FolderFilter::Folder(id) => Some(Some(id.clone())),
                FolderFilter::Unfiled => Some(None),
                FolderFilter::All => None,
            };
            if let Some(folder) = target
                && drop_target(app, ui, &r, folder, t)
            {
                continue;
            }
            if let FolderFilter::Folder(id) = &filter {
                r.context_menu(|ui| {
                    if ui.button(tr("Rename…")).clicked() {
                        menu_action = Some((id.clone(), "rename"));
                        ui.close();
                    }
                    if ui.button(tr("Colour…")).clicked() {
                        menu_action = Some((id.clone(), "color"));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr("Delete folder…")).clicked() {
                        menu_action = Some((id.clone(), "delete"));
                        ui.close();
                    }
                });
            }
            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                chosen = Some(filter);
            }
        }
    });
    if let Some(f) = chosen {
        app.home.folder = f;
    }
    if let Some((id, what)) = menu_action {
        folder_action(app, &id, what);
    }
}

/// Rename, colour or delete folder `id` (each asks first).
pub fn folder_action(app: &mut App, id: &str, what: &str) {
    let Some(f) = app.library.folder(id).cloned() else { return };
    app.home.modal = Some(match what {
        "rename" => Modal::RenameFolder { id: f.id, name: f.name },
        "color" => Modal::FolderColor { id: f.id, name: f.name },
        _ => Modal::DeleteFolder { id: f.id, name: f.name, delete_designs: false },
    });
}

fn toolbar(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal(|ui| {
        let search = egui::TextEdit::singleline(&mut app.home.designs_query)
            .hint_text(tr("Search your designs"))
            .desired_width(280.0)
            .margin(egui::vec2(10.0, 6.0));
        ui.add(search);
        if let FolderFilter::Folder(id) = app.home.folder.clone() {
            let mut act = None;
            ui.menu_button(tr("Folder"), |ui| {
                if ui.button(tr("Rename…")).clicked() {
                    act = Some("rename");
                }
                if ui.button(tr("Colour…")).clicked() {
                    act = Some("color");
                }
                if ui.button(tr("Delete folder…")).clicked() {
                    act = Some("delete");
                }
            });
            if let Some(a) = act {
                folder_action(app, &id, a);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = |s: Sort| match s {
                Sort::Edited => tr("Last edited"),
                Sort::Opened => tr("Last opened"),
                Sort::Created => tr("Newest"),
                Sort::Name => tr("Name"),
            };
            egui::ComboBox::from_id_salt("designs_sort").selected_text(label(app.home.sort)).show_ui(ui, |ui| {
                for s in [Sort::Edited, Sort::Opened, Sort::Created, Sort::Name] {
                    ui.selectable_value(&mut app.home.sort, s, label(s));
                }
            });
            ui.label(egui::RichText::new(tr("Sort by")).color(t.text_dim));
        });
    });
}

fn grid(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let lib = app.library.clone();
    let list: Vec<DesignEntry> = filtered(&lib, &app.home.folder, &app.home.designs_query, app.home.sort).into_iter().cloned().collect();
    if list.is_empty() {
        empty(app, ui, t);
        return;
    }
    let gap = 16.0;
    let avail = ui.available_width();
    let per = (((avail + gap) / (200.0 + gap)).floor() as usize).max(1);
    let w = ((avail - gap * (per as f32 - 1.0)) / per as f32).floor();
    let mut acts = vec![];
    for row in list.chunks(per) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for e in row {
                if let Some(a) = design_card(app, ui, t, e, w, false) {
                    acts.push((e.clone(), a));
                }
            }
        });
        ui.add_space(gap);
    }
    for (e, a) in acts {
        act(app, &e, a);
    }
}

fn empty(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(220.0, 120.0), egui::Sense::hover());
        let [a, b, ink] = t.tiles[1];
        crate::art::gradient_rect(ui.painter(), rect, 18.0, a, b);
        crate::art::scene(
            &mut Art::new(&mut app.thumbs, ui.ctx(), app.session.fonts.clone()),
            ui.painter(),
            t,
            crate::art::Scene::NewDesign,
            rect.shrink(12.0),
            ink,
        );
        ui.add_space(14.0);
        let (title, hint) = if app.home.designs_query.trim().is_empty() {
            (tr("Nothing here yet"), tr("Designs you make or open show up here. Drag them onto a folder to sort them."))
        } else {
            (tr("No designs match your search"), tr("Try other words, or look in all designs."))
        };
        ui.label(egui::RichText::new(title).size(18.0).strong());
        ui.label(egui::RichText::new(hint).color(t.text_dim));
        ui.add_space(10.0);
        if ui
            .add(
                egui::Button::new(egui::RichText::new(tr("New design")).color(t.accent_text))
                    .fill(t.accent)
                    .corner_radius(16)
                    .min_size(egui::vec2(0.0, 32.0)),
            )
            .clicked()
        {
            crate::popups::open_size_picker(app, None);
        }
    });
}
