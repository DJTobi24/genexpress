//! The home screen's Templates, Brand and Settings pages.

use serde_json::json;

use crate::App;
use crate::home::{Chip, TEMPLATE_CHIPS, chips, column, template_card, use_template};
use crate::theme::Tokens;
use genha_i18n::{tr, tr_n};

fn title(ui: &mut egui::Ui, t: &Tokens, text: &str, sub: &str) {
    ui.add_space(22.0);
    ui.label(egui::RichText::new(text).size(28.0).strong().color(t.text));
    ui.label(egui::RichText::new(sub).color(t.text_dim));
    ui.add_space(16.0);
}

/// Every template, with category chips and a search.
pub fn templates(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().id_salt("templates_scroll").auto_shrink([false, false]).show(ui, |ui| {
        column(ui, |ui| {
            let all = genha_templates::catalog();
            title(
                ui,
                t,
                tr("Templates"),
                &tr_n!(
                    "{n} original template, made in code. Free to use and change.",
                    "{n} original templates, made in code. Free to use and change.",
                    all.len()
                ),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.home.templates_query)
                    .hint_text(tr("Search templates"))
                    .desired_width(320.0)
                    .margin(egui::vec2(10.0, 6.0)),
            );
            ui.add_space(10.0);
            chips(ui, t, &mut app.home.templates_chip);
            ui.add_space(16.0);
            let chip = TEMPLATE_CHIPS.get(app.home.templates_chip).map_or(Chip::All, |c| c.1);
            let q = app.home.templates_query.clone();
            let list: Vec<_> = all
                .into_iter()
                .filter(|tp| chip.keeps(tp))
                .filter(|tp| {
                    let mut texts: Vec<&str> = vec![tp.name, tp.category, tr(tp.category)];
                    texts.extend(tp.tags.iter().copied());
                    genha_engine::cmd::designs::matches(&q, &texts)
                })
                .collect();
            if list.is_empty() {
                ui.label(egui::RichText::new(tr("No templates match.")).color(t.text_dim));
                return;
            }
            let gap = 16.0;
            let avail = ui.available_width();
            let per = (((avail + gap) / (180.0 + gap)).floor() as usize).max(1);
            let w = ((avail - gap * (per as f32 - 1.0)) / per as f32).floor();
            let mut chosen = None;
            for row in list.chunks(per) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for tp in row {
                        if template_card(app, ui, t, tp, w).clicked() {
                            chosen = Some(tp.id);
                        }
                    }
                });
                ui.add_space(gap);
            }
            if let Some(id) = chosen {
                use_template(app, id);
            }
        });
    });
}

/// Palettes to start a design with as its brand colours.
pub fn brand(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().id_salt("brand_scroll").auto_shrink([false, false]).show(ui, |ui| {
        column(ui, |ui| {
            title(
                ui,
                t,
                tr("Brand"),
                tr("A design's brand kit holds its colours, fonts and logos (Brand in the editor). Start a design with one of these palettes as its brand colours:"),
            );
            let gap = 14.0;
            let avail = ui.available_width();
            let per = (((avail + gap) / (230.0 + gap)).floor() as usize).max(1);
            let w = ((avail - gap * (per as f32 - 1.0)) / per as f32).floor();
            let mut chosen = None;
            let palettes = genha_templates::palettes();
            for row in palettes.chunks(per) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for pal in row {
                        let (rect, r) = ui.allocate_exact_size(egui::vec2(w, 112.0), egui::Sense::click());
                        let p = ui.painter_at(rect.expand(2.0));
                        p.rect_filled(rect, 14, t.panel);
                        p.rect_stroke(rect, 14, egui::Stroke::new(1.0, if r.hovered() { t.accent } else { t.border }), egui::StrokeKind::Inside);
                        let n = pal.colors.len().max(1) as f32;
                        let band = egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 10.0), egui::vec2(w - 20.0, 58.0));
                        let sw = band.width() / n;
                        for (i, c) in pal.colors.iter().enumerate() {
                            let cell = egui::Rect::from_min_size(band.min + egui::vec2(i as f32 * sw, 0.0), egui::vec2(sw, band.height()));
                            let radius = match i {
                                0 => egui::CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 },
                                _ if i + 1 == pal.colors.len() => egui::CornerRadius { ne: 8, se: 8, nw: 0, sw: 0 },
                                _ => egui::CornerRadius::ZERO,
                            };
                            p.rect_filled(cell, radius, crate::theme::to_egui(*c));
                        }
                        p.text(egui::pos2(rect.left() + 12.0, band.bottom() + 12.0), egui::Align2::LEFT_TOP, tr(pal.name), egui::FontId::proportional(13.5), t.text);
                        if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr("New design with these brand colours")).clicked() {
                            chosen = Some(pal.id.to_string());
                        }
                    }
                });
                ui.add_space(gap);
            }
            if let Some(id) = chosen {
                crate::popups::open_size_picker(app, Some(id));
            }
        });
    });
}

/// A settings group: a card with a title.
fn group(ui: &mut egui::Ui, t: &Tokens, heading: &str, f: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(t.panel).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(14).inner_margin(18).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(heading).size(16.0).strong().color(t.text));
        ui.add_space(6.0);
        f(ui);
    });
    ui.add_space(14.0);
}

pub fn settings(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().id_salt("settings_scroll").auto_shrink([false, false]).show(ui, |ui| {
        crate::home::column_max(ui, 860.0, |ui| {
            title(ui, t, tr("Settings"), tr("Kept on this computer between runs. GenHa needs no account."));
            group(ui, t, tr("Your name"), |ui| {
                ui.label(egui::RichText::new(tr("The home page greets you with it.")).color(t.text_dim));
                let draft = app.home.name_draft.get_or_insert_with(|| app.session.user_name.clone());
                let r = ui
                    .add(egui::TextEdit::singleline(draft).hint_text(tr("Your name (optional)")).desired_width(280.0).margin(egui::vec2(10.0, 6.0)));
                if r.lost_focus() {
                    let name = app.home.name_draft.take().unwrap_or_default();
                    if name.trim() != app.session.user_name {
                        app.run("settings.set", &json!({"name": name}));
                    }
                }
            });
            group(ui, t, tr("Appearance"), |ui| {
                ui.horizontal(|ui| {
                    for (dark, label) in [(false, tr("Light")), (true, tr("Dark"))] {
                        if ui.selectable_label(app.dark == dark, label).clicked() && app.dark != dark {
                            app.dark = dark;
                            app.run("settings.set", &json!({"dark": dark}));
                        }
                    }
                });
            });
            group(ui, t, tr("Language"), |ui| language_picker(app, ui));
            group(ui, t, tr("Network"), |ui| {
                let mut on = app.session.network;
                if crate::theme::switch(ui, &mut on, tr("Allow GenHa to use the network")).changed() {
                    app.run("settings.network", &json!({"on": on}));
                }
                ui.label(egui::RichText::new(tr("Only to download the optional background removal model when you ask for it.")).color(t.text_dim));
            });
            group(ui, t, tr("Editor"), |ui| {
                let mut on = app.session.snapping;
                if crate::theme::switch(ui, &mut on, tr("Smart guides: snap while moving and resizing")).changed() {
                    app.run("view.snapping", &json!({"on": on}));
                }
            });
            group(ui, t, tr("Fonts"), |ui| fonts(app, ui, t));
            group(ui, t, tr("Your data"), |ui| {
                let place = app.session.store.as_ref().map(|s| s.locate("designs"));
                let text = match place {
                    Some(p) if cfg!(not(target_arch = "wasm32")) => {
                        genha_i18n::tr!("My designs, Data Recovery copies and added fonts are kept in {place}.", place = p)
                    }
                    Some(_) => tr("My designs, Data Recovery copies and added fonts are kept in this browser's storage.").to_string(),
                    None => tr("Nothing is kept between runs in this window.").to_string(),
                };
                ui.label(egui::RichText::new(text).color(t.text_dim));
            });
        });
    });
}

/// The UI's language: the system's, or one chosen (`ui.setLanguage`); it switches at once.
fn language_picker(app: &mut App, ui: &mut egui::Ui) {
    let current = app.session.language;
    let system = genha_i18n::Lang::system();
    let label = |s: genha_i18n::Setting| match s {
        genha_i18n::Setting::System => genha_i18n::tr!("System language ({name})", name = system.native_name()),
        genha_i18n::Setting::Fixed(l) => l.native_name().to_string(),
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt("ui_language")
        .selected_text(label(current))
        .width(240.0)
        .show_ui(ui, |ui| {
            let choices = std::iter::once(genha_i18n::Setting::System).chain(genha_i18n::Lang::ALL.map(genha_i18n::Setting::Fixed));
            for s in choices {
                if ui.selectable_label(s == current, label(s)).clicked() {
                    chosen = Some(s);
                }
            }
        })
        .response
        .on_hover_text(tr("Language of the user interface"));
    if let Some(s) = chosen {
        app.run("ui.setLanguage", &json!({"lang": s.id()}));
    }
}

fn fonts(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let n = app.font_families().len().saturating_sub(4);
    let hint = if cfg!(target_arch = "wasm32") {
        tr_n!(
            "{n} font family. Add font files (kept in this browser), or use the fonts installed on this computer.",
            "{n} font families. Add font files (kept in this browser), or use the fonts installed on this computer.",
            n
        )
    } else {
        tr_n!("{n} font family: the installed fonts and the ones you added.", "{n} font families: the installed fonts and the ones you added.", n)
    };
    ui.label(egui::RichText::new(hint).color(t.text_dim));
    ui.add_space(6.0);
    crate::fonts::add_fonts_ui(app, ui);
    let added: Vec<(String, String)> = app
        .session
        .execute("font.list", &json!({}))
        .ok()
        .and_then(|v| v.get("added").and_then(|a| a.as_array()).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| Some((v.get("key")?.as_str()?.to_string(), v.get("name")?.as_str()?.to_string())))
        .collect();
    if added.is_empty() {
        return;
    }
    let mut remove = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(tr("Added:")).color(t.text_dim));
        for (key, name) in &added {
            if ui.small_button(format!("{name}  ×")).on_hover_text(tr("Don't load this font next time")).clicked() {
                remove = Some(key.clone());
            }
        }
    });
    if let Some(key) = remove {
        app.run("font.remove", &json!({"key": key}));
    }
}
