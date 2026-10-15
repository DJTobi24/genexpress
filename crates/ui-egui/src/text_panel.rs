//! The text panel: when a text box is selected, the left panel turns into its editor, in three
//! tabs (Edit, Effects, Animation) with large, visual controls: text styles as previews, the font,
//! size, emphasis and alignment, colour swatches, spacing, straight or curved, effects as
//! pictures. The right-hand properties panel stays closed meanwhile. The left rail stays; picking
//! a rail tab (or ×) closes the panel until another text box is selected.

use serde_json::{Value, json};

use super::{Actions, color_row, element_animation};
use crate::theme::{Tokens, to_egui};
use crate::{App, thumbs};
use genha_doc::{Element, ElementKind, RangeStyle, TextAlign, TextEffect, TextEl, TextFit};
use genha_i18n::{N_, tr};

/// The tabs of the text and picture panels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelTab {
    #[default]
    Edit,
    Effects,
    Animation,
}

/// Colours offered for text besides the brand's: black, white and a bright row.
const SWATCHES: [u32; 9] = [0x000000, 0xffffff, 0xe02424, 0xf97316, 0xfacc15, 0x16a34a, 0x2563eb, 0x7c3aed, 0xdb2777];

/// Draws the panel for `id`.
pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: genha_doc::NodeId) {
    let Some(d) = app.session.active() else { return };
    let Some(el) = d.doc.element(id).ok().cloned() else { return };
    let ElementKind::Text(text) = &el.kind else { return };
    let text = text.clone();
    let brand: Vec<genha_color::Color> = d.doc.brand.colors.iter().map(|c| c.color).collect();
    let families = app.font_families().to_vec();
    let editing = genha_engine::text_selection_style(&app.session).filter(|(eid, _)| *eid == id).map(|(_, r)| r);
    let caret_only = app.session.text_editing.as_ref().is_some_and(|e| e.anchor == e.caret);
    let fit = (text.fit == TextFit::Shrink).then(|| genha_render::text_fit(&app.session.fonts, &text, el.frame.width, el.frame.height));
    let mut acts = Actions::default();
    let mut tab = app.editor.panel_tab;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr("Text")).size(19.0).strong().color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = egui::Button::new(egui::RichText::new("×").size(18.0).color(t.text_dim)).frame(false);
            if ui.add(close).on_hover_text(tr("Close (the left panel shows its tab again)")).clicked() {
                app.editor.panel_closed = Some(id);
            }
        });
    });
    ui.add_space(8.0);
    segmented(ui, t, &mut tab, &ALL_TABS);
    ui.add_space(12.0);
    app.editor.panel_tab = tab;

    match tab {
        PanelTab::Edit => {
            let shown_whole;
            let shown = match &editing {
                Some(r) => r,
                None => {
                    shown_whole = text.range_style(0..text.text.chars().count());
                    &shown_whole
                }
            };
            if editing.is_some() {
                let hint = if caret_only { tr("Changes apply to the next text you type") } else { tr("Changes apply to the selected text") };
                ui.label(egui::RichText::new(hint).size(11.5).color(t.accent));
                ui.add_space(6.0);
            }
            styles_row(app, ui, t);
            ui.add_space(14.0);
            edit_tab(ui, t, &el, &text, shown, &brand, &families, fit, &mut acts);
        }
        PanelTab::Effects => effects_tab(ui, t, &text, &brand, &mut acts),
        PanelTab::Animation => {
            element_animation(ui, t, &el, &mut acts);
        }
    }

    for (cmd, p) in acts.commands {
        app.run(cmd, &p);
    }
    if acts.preview {
        crate::motion::play(app, ui.ctx());
    }
}

/// Every tab, in order.
pub(super) const ALL_TABS: [PanelTab; 3] = [PanelTab::Edit, PanelTab::Effects, PanelTab::Animation];

/// `tabs` (some of Edit, Effects, Animation) as one rounded control; a tab not offered falls back
/// to the first.
pub(super) fn segmented(ui: &mut egui::Ui, t: &Tokens, tab: &mut PanelTab, tabs: &[PanelTab]) {
    if !tabs.contains(tab)
        && let Some(first) = tabs.first()
    {
        *tab = *first;
    }
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 18, t.panel_alt);
    let w = (rect.width() - 6.0) / tabs.len().max(1) as f32;
    // The filled pill slides to the chosen tab.
    let index = tabs.iter().position(|x| x == tab).unwrap_or(0) as f32;
    let at = ui.ctx().animate_value_with_time(ui.id().with("segmented_pill"), index, 0.18);
    let pill = egui::Rect::from_min_size(rect.min + egui::vec2(3.0 + w * at, 3.0), egui::vec2(w, 30.0));
    ui.painter().rect_filled(pill, 15, t.accent);
    for (i, which) in tabs.iter().copied().enumerate() {
        let label = match which {
            PanelTab::Edit => N_("Edit"),
            PanelTab::Effects => N_("Effects"),
            PanelTab::Animation => N_("Animation"),
        };
        let r = egui::Rect::from_min_size(rect.min + egui::vec2(3.0 + w * i as f32, 3.0), egui::vec2(w, 30.0));
        let resp = ui.interact(r, ui.id().with(("text_tab", i)), egui::Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        // The label turns light where the pill is under it.
        let cover = (1.0 - (at - i as f32).abs()).clamp(0.0, 1.0);
        if resp.hovered() && cover < 0.5 {
            ui.painter().rect_filled(r, 15, t.accent_soft);
        }
        let ink = t.text.lerp_to_gamma(t.accent_text, cover);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, tr(label), egui::FontId::proportional(13.5), ink);
        if resp.clicked() {
            *tab = which;
        }
    }
}

/// A small heading over a group of controls.
pub(super) fn label(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.label(egui::RichText::new(text).size(12.5).strong().color(t.text_dim));
    ui.add_space(4.0);
}

/// The text styles as pictures in a row that scrolls sideways; a click gives the box that style.
fn styles_row(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    label(ui, t, tr("Text styles"));
    let ctx = ui.ctx().clone();
    let mut chosen = None;
    egui::ScrollArea::horizontal().id_salt("text_styles").max_height(84.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            for p in genha_templates::text_presets() {
                let fonts = app.session.fonts.clone();
                let tex = app.thumbs.get(&ctx, &format!("textstyle:{}:{}", genha_i18n::language().code(), p.id), 0, || preset_picture(&p, &fonts));
                let (rect, r) = ui.allocate_exact_size(egui::vec2(116.0, 68.0), egui::Sense::click());
                let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
                let light_ink = p.style.color.luminance() > 0.6;
                ui.painter().rect_filled(rect, 10, if light_ink { t.thumb_dark } else { t.thumb_light });
                if let Some(tex) = &tex {
                    let size = tex.size_vec2();
                    let shown = thumbs::fit_size(size.x, size.y, rect.shrink(6.0).size());
                    ui.painter().image(
                        tex.id(),
                        egui::Rect::from_center_size(rect.center(), shown),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                let stroke = if r.hovered() { egui::Stroke::new(2.0, t.accent) } else { egui::Stroke::new(1.0, t.border) };
                ui.painter().rect_stroke(rect, 10, stroke, egui::StrokeKind::Inside);
                if r.on_hover_text(tr(p.label)).clicked() {
                    chosen = Some(p.id);
                }
            }
        });
    });
    if let Some(id) = chosen {
        app.run("text.applyPreset", &json!({"id": id}));
    }
}

/// A text style drawn on its own: its sample in its font, colour and effect.
pub(crate) fn preset_picture(p: &genha_templates::TextPreset, fonts: &genha_text::FontBook) -> Option<genha_render::Raster> {
    let mut doc = genha_doc::Document::new("style", 400.0, 220.0);
    let id = doc.alloc_id();
    let mut style = p.style.clone();
    style.font_size = 96.0;
    style.align = TextAlign::Center;
    let el = TextEl { text: tr(p.sample).to_string(), style, effect: p.effect.clone(), curve: 0.0, spans: vec![], fit: TextFit::Shrink };
    let page = doc.pages.first_mut()?;
    page.elements.push(Element::new(id, genha_geom::Frame::new(16.0, 30.0, 368.0, 160.0), ElementKind::Text(el)));
    genha_render::render_page(&doc, 0, fonts, &genha_render::RenderOptions { scale: 0.6, transparent_background: true }).ok()
}

#[allow(clippy::too_many_arguments)]
fn edit_tab(
    ui: &mut egui::Ui,
    t: &Tokens,
    el: &Element,
    text: &TextEl,
    shown: &RangeStyle,
    brand: &[genha_color::Color],
    families: &[String],
    fit: Option<genha_text::FitScale>,
    acts: &mut Actions,
) {
    let id = el.id.0;
    let key = |what: &str| format!("{what}:{id}");
    let st = &shown.style;
    let mixed = |f: &str| shown.is_mixed(f);

    label(ui, t, tr("Font"));
    let current = if mixed("font_family") { tr("Mixed fonts").to_string() } else { st.font_family.clone() };
    let width = ui.available_width();
    egui::ComboBox::from_id_salt(format!("tp_font_{id}")).selected_text(current).width(width).height(360.0).show_ui(ui, |ui| {
        for f in families {
            if ui.selectable_label(*f == st.font_family, f).clicked() {
                acts.run("text.setStyle", json!({"font_family": f}));
            }
        }
    });
    ui.add_space(10.0);

    // Size, then alignment.
    ui.horizontal(|ui| {
        let mut size = st.font_size;
        let step = |s: f64, up: bool| {
            let k = if s < 20.0 {
                1.0
            } else if s < 80.0 {
                2.0
            } else {
                4.0
            };
            if up { (s + k).min(2000.0) } else { (s - k).max(1.0) }
        };
        if pill_button(ui, t, "−", 32.0).on_hover_text(tr("Smaller")).clicked() {
            acts.run("text.setStyle", json!({"font_size": step(size, false), "merge_key": key("size")}));
        }
        let drag = egui::DragValue::new(&mut size).range(1.0..=2000.0).speed(0.5).max_decimals(0);
        let r = ui.add_sized(egui::vec2(52.0, 30.0), drag).on_hover_text(if mixed("font_size") { tr("Size (mixed)") } else { tr("Size") });
        if r.changed() {
            acts.run("text.setStyle", json!({"font_size": size, "merge_key": key("size")}));
        }
        if pill_button(ui, t, "+", 32.0).on_hover_text(tr("Larger")).clicked() {
            acts.run("text.setStyle", json!({"font_size": step(size, true), "merge_key": key("size")}));
        }
        ui.add_space(6.0);
        for (a, hint) in [(TextAlign::Left, N_("Align left")), (TextAlign::Center, N_("Centre")), (TextAlign::Right, N_("Align right"))] {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = st.align == a;
            tile_back(ui, t, rect, on, r.hovered());
            align_icon(ui.painter(), rect, a, if on { t.accent } else { t.text });
            if r.on_hover_text(tr(hint)).clicked() {
                acts.run("text.setStyle", json!({"align": a}));
            }
        }
    });
    ui.add_space(8.0);

    // Emphasis.
    ui.horizontal(|ui| {
        for (glyph, field, cmd, on) in [
            ("B", N_("bold"), "text.toggleBold", st.bold),
            ("I", N_("italic"), "text.toggleItalic", st.italic),
            ("U", N_("underline"), "text.toggleUnderline", st.underline),
            ("S", N_("strikethrough"), "text.toggleStrikethrough", st.strikethrough),
        ] {
            let on = on && !mixed(field);
            let (rect, r) = ui.allocate_exact_size(egui::vec2(38.0, 32.0), egui::Sense::click());
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            tile_back(ui, t, rect, on, r.hovered());
            let ink = if on { t.accent } else { t.text };
            let line = egui::Stroke::new(1.4, ink);
            let format = egui::TextFormat {
                font_id: egui::FontId::proportional(16.0),
                color: ink,
                italics: glyph == "I",
                underline: if glyph == "U" { line } else { egui::Stroke::NONE },
                strikethrough: if glyph == "S" { line } else { egui::Stroke::NONE },
                ..Default::default()
            };
            let galley = ui.painter().layout_job(egui::text::LayoutJob::single_section(glyph.to_string(), format));
            let pos = rect.center() - galley.size() / 2.0;
            if glyph == "B" {
                // egui has no bold face: the letter drawn twice, a hair apart.
                ui.painter().galley(pos + egui::vec2(0.8, 0.0), galley.clone(), ink);
            }
            ui.painter().galley(pos, galley, ink);
            let hint = if mixed(field) { genha_i18n::tr!("{style}: mixed", style = tr(field)) } else { tr(field).to_string() };
            if r.on_hover_text(hint).clicked() {
                acts.run(cmd, json!({}));
            }
        }
        let on = text.style.uppercase;
        let (rect, r) = ui.allocate_exact_size(egui::vec2(44.0, 32.0), egui::Sense::click());
        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
        tile_back(ui, t, rect, on, r.hovered());
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "AA", egui::FontId::proportional(13.0), if on { t.accent } else { t.text });
        if r.on_hover_text(tr("uppercase")).clicked() {
            acts.run("text.setStyle", json!({"uppercase": !on}));
        }
    });
    ui.add_space(14.0);

    label(ui, t, if mixed("color") { tr("Colour (mixed)") } else { tr("Colour") });
    if let Some(c) = swatches(ui, t, st.color, brand) {
        acts.run("text.setStyle", json!({"color": c.to_hex(), "merge_key": key("color")}));
    }
    ui.add_space(14.0);

    label(ui, t, tr("Shape"));
    ui.horizontal(|ui| {
        for (curved, name) in [(false, N_("Straight")), (true, N_("Curved"))] {
            let on = (text.curve != 0.0) == curved;
            let (rect, r) = ui.allocate_exact_size(egui::vec2(96.0, 54.0), egui::Sense::click());
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            tile_back(ui, t, rect, on, r.hovered());
            let ink = if on { t.accent } else { t.text };
            let mid = rect.center() - egui::vec2(0.0, 6.0);
            if curved {
                let pts: Vec<egui::Pos2> = (0..=16)
                    .map(|i| {
                        let a = std::f32::consts::PI * (1.1 + 0.8 * i as f32 / 16.0);
                        mid + egui::vec2(26.0 * a.cos(), 10.0 + 18.0 * a.sin())
                    })
                    .collect();
                ui.painter().add(egui::Shape::line(pts, egui::Stroke::new(2.0, ink)));
            } else {
                ui.painter().line_segment([mid - egui::vec2(24.0, 0.0), mid + egui::vec2(24.0, 0.0)], egui::Stroke::new(2.0, ink));
            }
            ui.painter().text(
                rect.center_bottom() - egui::vec2(0.0, 10.0),
                egui::Align2::CENTER_CENTER,
                tr(name),
                egui::FontId::proportional(11.5),
                ink,
            );
            if r.clicked() && !on {
                acts.run("text.setCurve", json!({"degrees": if curved { 120.0 } else { 0.0 }}));
            }
        }
    });
    if text.curve != 0.0 {
        let mut deg = text.curve;
        ui.add_space(4.0);
        if ui.add(egui::Slider::new(&mut deg, -360.0..=360.0).step_by(1.0).suffix("°").text(tr("Curve"))).changed() {
            acts.run("text.setCurve", json!({"degrees": deg, "merge_key": key("curve")}));
        }
    }
    ui.add_space(14.0);

    egui::CollapsingHeader::new(egui::RichText::new(tr("Spacing")).size(12.5).strong().color(t.text_dim)).id_salt("tp_spacing").show(ui, |ui| {
        let (mut lh, mut ls) = (text.style.line_height, text.style.letter_spacing);
        if ui.add(egui::Slider::new(&mut lh, 0.6..=3.0).text(tr("Line height"))).changed() {
            acts.run("text.setStyle", json!({"line_height": lh, "merge_key": key("lh")}));
        }
        if ui.add(egui::Slider::new(&mut ls, -200.0..=800.0).text(tr("Letter spacing"))).changed() {
            acts.run("text.setStyle", json!({"letter_spacing": ls, "merge_key": key("ls")}));
        }
    });
    let mut shrink = text.fit == TextFit::Shrink;
    let tip = tr("When the text does not fit its box, set it smaller (down to 40%) instead of growing the box");
    if ui.checkbox(&mut shrink, tr("Shrink text to fit")).on_hover_text(tip).changed() {
        acts.run("text.setFit", json!({"fit": if shrink { "shrink" } else { "none" }}));
    }
    if let Some(f) = fit.filter(|f| f.scale < 1.0 || !f.fits) {
        let percent = format!("{:.0}", f.scale * 100.0);
        let note = if f.fits {
            genha_i18n::tr!("Shown at {percent}% to fit", percent = percent)
        } else {
            genha_i18n::tr!("Too long to fit, even at {percent}%", percent = percent)
        };
        ui.label(egui::RichText::new(note).size(11.0).color(t.text_dim));
    }
    ui.add_space(14.0);

    // Position and opacity in pop-ups, so the panel stays short.
    ui.horizontal(|ui| {
        let half = (ui.available_width() - 8.0) / 2.0;
        let pos = egui::Button::new(tr("Position")).min_size(egui::vec2(half, 34.0)).corner_radius(10);
        let r = ui.add(pos);
        egui::Popup::menu(&r).show(|ui| {
            ui.set_min_width(220.0);
            label(ui, t, tr("Align on the page"));
            super::align_buttons(ui, acts, "page");
            ui.add_space(6.0);
            super::common_frame(ui, el, acts);
        });
        let op = egui::Button::new(tr("Opacity")).min_size(egui::vec2(half, 34.0)).corner_radius(10);
        let r = ui.add(op);
        egui::Popup::menu(&r).show(|ui| {
            ui.set_min_width(220.0);
            let mut o = el.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut o, 0.0..=100.0).suffix("%").text(tr("Opacity"))).changed() {
                acts.run("object.setOpacity", json!({"opacity": o / 100.0, "merge_key": key("opacity")}));
            }
        });
    });
}

/// Effects as pictures (none, shadow, outline, highlight); the chosen one's colour and strength.
fn effects_tab(ui: &mut egui::Ui, t: &Tokens, text: &TextEl, brand: &[genha_color::Color], acts: &mut Actions) {
    let st = &text.style;
    let ink = st.color;
    let effects: [(&str, TextEffect); 4] = [
        (N_("None"), TextEffect::None),
        (N_("Shadow"), TextEffect::Shadow { color: genha_color::Color::rgba(0, 0, 0, 110), offset_x: 4.0, offset_y: 4.0, blur: 6.0 }),
        (N_("Outline"), TextEffect::Outline { color: ink.readable_text(), width: (st.font_size / 18.0).max(1.0) }),
        (
            N_("Highlight"),
            TextEffect::Highlight {
                color: brand.get(1).copied().unwrap_or(genha_color::Color::rgb(255, 230, 0)),
                padding: st.font_size * 0.2,
                corner_radius: st.font_size * 0.15,
            },
        ),
    ];
    label(ui, t, tr("Effect"));
    egui::Grid::new("tp_effects").spacing(egui::vec2(10.0, 10.0)).show(ui, |ui| {
        for (i, (name, e)) in effects.into_iter().enumerate() {
            let on = std::mem::discriminant(&text.effect) == std::mem::discriminant(&e);
            let (rect, r) = ui.allocate_exact_size(egui::vec2(120.0, 76.0), egui::Sense::click());
            let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
            tile_back(ui, t, rect, on, r.hovered());
            effect_picture(ui.painter(), rect.shrink2(egui::vec2(0.0, 10.0)).translate(egui::vec2(0.0, -6.0)), t, i);
            ui.painter().text(
                rect.center_bottom() - egui::vec2(0.0, 11.0),
                egui::Align2::CENTER_CENTER,
                tr(name),
                egui::FontId::proportional(11.5),
                if on { t.accent } else { t.text },
            );
            if r.clicked() && !on {
                acts.run("text.setEffect", json!({"effect": e}));
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.add_space(14.0);
    let id = |what: &str| format!("{what}:effect");
    let with = |field: &str, v: Value| {
        let mut e = serde_json::to_value(&text.effect).unwrap_or(Value::Null);
        if let Some(o) = e.as_object_mut() {
            o.insert(field.into(), v);
        }
        e
    };
    match &text.effect {
        TextEffect::None => {
            ui.label(egui::RichText::new(tr("Pick an effect to make the text stand out.")).size(11.5).color(t.text_dim));
        }
        TextEffect::Shadow { color, blur, offset_x, offset_y } => {
            label(ui, t, tr("Shadow colour"));
            if let Some(c) = color_row(ui, *color, brand) {
                acts.run("text.setEffect", json!({"effect": with("color", json!(c.to_hex())), "merge_key": id("color")}));
            }
            let mut b = *blur;
            if ui.add(egui::Slider::new(&mut b, 0.0..=60.0).text(tr("Softness"))).changed() {
                acts.run("text.setEffect", json!({"effect": with("blur", json!(b)), "merge_key": id("blur")}));
            }
            let mut dist = offset_x.hypot(*offset_y);
            if ui.add(egui::Slider::new(&mut dist, 0.0..=60.0).text(tr("Distance"))).changed() {
                let d = dist / std::f64::consts::SQRT_2;
                let mut e = with("offset_x", json!(d));
                if let Some(o) = e.as_object_mut() {
                    o.insert("offset_y".into(), json!(d));
                }
                acts.run("text.setEffect", json!({"effect": e, "merge_key": id("dist")}));
            }
        }
        TextEffect::Outline { color, width } => {
            label(ui, t, tr("Outline colour"));
            if let Some(c) = color_row(ui, *color, brand) {
                acts.run("text.setEffect", json!({"effect": with("color", json!(c.to_hex())), "merge_key": id("color")}));
            }
            let mut w = *width;
            if ui.add(egui::Slider::new(&mut w, 0.5..=(st.font_size / 4.0).max(2.0)).text(tr("Thickness"))).changed() {
                acts.run("text.setEffect", json!({"effect": with("width", json!(w)), "merge_key": id("width")}));
            }
        }
        TextEffect::Highlight { color, padding, corner_radius } => {
            label(ui, t, tr("Highlight colour"));
            if let Some(c) = color_row(ui, *color, brand) {
                acts.run("text.setEffect", json!({"effect": with("color", json!(c.to_hex())), "merge_key": id("color")}));
            }
            let (mut p, mut r) = (*padding, *corner_radius);
            if ui.add(egui::Slider::new(&mut p, 0.0..=(st.font_size).max(4.0)).text(tr("Padding"))).changed() {
                acts.run("text.setEffect", json!({"effect": with("padding", json!(p)), "merge_key": id("padding")}));
            }
            if ui.add(egui::Slider::new(&mut r, 0.0..=(st.font_size).max(4.0)).text(tr("Corners"))).changed() {
                acts.run("text.setEffect", json!({"effect": with("corner_radius", json!(r)), "merge_key": id("radius")}));
            }
        }
    }
}

/// A sample "Ab" with effect `i` (0 none, 1 shadow, 2 outline, 3 highlight), drawn with the
/// painter (a sketch of the effect, the same for every text).
fn effect_picture(painter: &egui::Painter, rect: egui::Rect, t: &Tokens, i: usize) {
    let font = egui::FontId::proportional(26.0);
    let c = rect.center();
    let ink = t.text;
    match i {
        1 => {
            painter.text(c + egui::vec2(2.5, 2.5), egui::Align2::CENTER_CENTER, "Ab", font.clone(), t.text_dim.gamma_multiply(0.6));
        }
        2 => {
            for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)] {
                painter.text(c + egui::vec2(dx, dy), egui::Align2::CENTER_CENTER, "Ab", font.clone(), t.accent);
            }
            painter.text(c, egui::Align2::CENTER_CENTER, "Ab", font, t.panel);
            return;
        }
        3 => {
            painter.rect_filled(egui::Rect::from_center_size(c, egui::vec2(46.0, 28.0)), 6, egui::Color32::from_rgb(0xfa, 0xd8, 0x3c));
            painter.text(c, egui::Align2::CENTER_CENTER, "Ab", font, egui::Color32::from_rgb(0x1c, 0x1b, 0x29));
            return;
        }
        _ => {}
    }
    painter.text(c, egui::Align2::CENTER_CENTER, "Ab", font, ink);
}

/// The current colour (click for the full picker) and the brand's colours on one row,
/// [`SWATCHES`] on the next.
pub(super) fn swatches(ui: &mut egui::Ui, t: &Tokens, current: genha_color::Color, brand: &[genha_color::Color]) -> Option<genha_color::Color> {
    let mut out = None;
    let same = |a: genha_color::Color, b: genha_color::Color| a.r == b.r && a.g == b.g && a.b == b.b;
    // A round swatch; whether it was clicked.
    let dot = |ui: &mut egui::Ui, col: genha_color::Color| -> bool {
        let (rect, r) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
        let center = rect.center();
        ui.painter().circle_filled(center, 10.5, to_egui(col));
        ui.painter().circle_stroke(center, 10.5, egui::Stroke::new(1.0, t.border));
        if same(col, current) {
            ui.painter().circle_stroke(center, 13.0, egui::Stroke::new(2.0, t.accent));
        } else if r.hovered() {
            ui.painter().circle_stroke(center, 13.0, egui::Stroke::new(1.5, t.text_dim));
        }
        r.on_hover_text(col.to_hex()).clicked()
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 6.0);
        // The current colour, large; a click opens the full picker.
        let (rect, r) = ui.allocate_exact_size(egui::vec2(40.0, 28.0), egui::Sense::click());
        let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(rect, 8, to_egui(current));
        ui.painter().rect_stroke(rect, 8, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let r = r.on_hover_text(tr("Pick any colour"));
        egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            let mut c = to_egui(current);
            if egui::widgets::color_picker::color_picker_color32(ui, &mut c, egui::widgets::color_picker::Alpha::Opaque) {
                out = Some(crate::theme::from_egui(c));
            }
        });
        if !brand.is_empty() {
            ui.add_space(6.0);
            for b in brand.iter().take(7) {
                if dot(ui, *b) {
                    out = Some(*b);
                }
            }
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 6.0);
        for h in SWATCHES {
            let col = genha_color::Color::rgb((h >> 16) as u8, (h >> 8) as u8, h as u8);
            if dot(ui, col) {
                out = Some(col);
            }
        }
    });
    out
}

/// A tile's background: accent when on, soft when hovered.
pub(super) fn tile_back(ui: &egui::Ui, t: &Tokens, rect: egui::Rect, on: bool, hovered: bool) {
    let k = if on { 1.0 } else { crate::theme::hover_fade(ui.ctx(), rect, hovered) };
    ui.painter().rect_filled(rect, 8, t.panel_alt.lerp_to_gamma(t.accent_soft, k));
    if on {
        ui.painter().rect_stroke(rect, 8, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    }
}

/// A round-cornered square button with a symbol.
pub(super) fn pill_button(ui: &mut egui::Ui, t: &Tokens, symbol: &str, side: f32) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(side, 30.0), egui::Sense::click());
    let r = r.on_hover_cursor(egui::CursorIcon::PointingHand);
    tile_back(ui, t, rect, false, r.hovered());
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, symbol, egui::FontId::proportional(17.0), t.text);
    r
}

/// Three or four lines aligned like the text.
fn align_icon(painter: &egui::Painter, rect: egui::Rect, a: TextAlign, ink: egui::Color32) {
    let c = rect.center();
    for (i, w) in [16.0_f32, 10.0, 14.0, 8.0].into_iter().enumerate() {
        let y = c.y - 6.0 + i as f32 * 4.0;
        let (x0, x1) = match a {
            TextAlign::Left => (c.x - 8.0, c.x - 8.0 + w),
            TextAlign::Right => (c.x + 8.0 - w, c.x + 8.0),
            _ => (c.x - w / 2.0, c.x + w / 2.0),
        };
        painter.line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], egui::Stroke::new(1.6, ink));
    }
}
