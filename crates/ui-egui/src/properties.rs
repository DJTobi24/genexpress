//! The right-hand panel: properties of the selection, or of the page when nothing is selected.

use serde_json::{Value, json};

#[path = "data_props.rs"]
mod data_props;

use crate::App;
use crate::theme::{Tokens, from_egui, to_egui};
use genha_doc::{AnimationKind, Element, ElementKind, Fill, Page, RangeStyle, ShapeKind, TextAlign, TextEffect, TextFit};
use genha_i18n::{N_, tr, tr_n};

/// Commands to run after drawing (the panel reads a snapshot, then acts).
#[derive(Default)]
struct Actions {
    commands: Vec<(&'static str, Value)>,
    /// Play the page's animation on the canvas afterwards (UI only).
    preview: bool,
    /// Background removal: method and progress.
    ml: crate::models::Panel,
}

impl Actions {
    fn run(&mut self, id: &'static str, p: Value) {
        self.commands.push((id, p));
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let Some(d) = app.session.active() else { return };
    let selected: Vec<Element> = d.selected_elements().into_iter().cloned().collect();
    let page = d.doc.pages.get(d.page).cloned();
    let brand: Vec<genha_color::Color> = d.doc.brand.colors.iter().map(|c| c.color).collect();
    let families = app.font_families().to_vec();
    // The text being edited in place: the style at its selection.
    let editing = genha_engine::text_selection_style(&app.session);
    let editing_caret = app.session.text_editing.as_ref().is_some_and(|e| e.anchor == e.caret);
    let mut acts = Actions { ml: crate::models::panel(app), ..Default::default() };
    let mut choices = app.motion.panel;
    match selected.as_slice() {
        [] => {
            if let Some(p) = &page {
                page_props(ui, t, p, &brand, &mut acts);
                page_animation(ui, t, p, &mut choices, &mut acts);
            }
        }
        [el] => {
            let sel = editing.as_ref().filter(|(id, _)| *id == el.id).map(|(_, r)| r);
            // The scale shrinking text is shown at (as the canvas draws it).
            let fit = match &el.kind {
                ElementKind::Text(tx) if tx.fit == TextFit::Shrink => {
                    Some(genha_render::text_fit(&app.session.fonts, tx, el.frame.width, el.frame.height))
                }
                _ => None,
            };
            element_props(ui, t, el, sel, editing_caret, &brand, &families, fit, &mut acts);
        }
        many => multi_props(ui, t, many, &mut acts),
    }
    app.motion.panel = choices;
    crate::models::apply(app, acts.ml);
    for (id, p) in acts.commands {
        if crate::models::is_background(id) {
            crate::models::start(app, ui.ctx(), id, p);
        } else {
            app.run(id, &p);
        }
    }
    if acts.preview {
        crate::motion::play(app, ui.ctx());
    }
}

/// An entrance picker; a non-empty `none` adds that choice for "no animation". Returns the new
/// choice when changed.
fn kind_combo(ui: &mut egui::Ui, id: &str, current: Option<AnimationKind>, none: &str) -> Option<Option<AnimationKind>> {
    let mut out = None;
    let text = tr(current.map(genha_motion::kind_label).unwrap_or(none));
    egui::ComboBox::from_id_salt(id).selected_text(text).width(150.0).show_ui(ui, |ui| {
        if !none.is_empty() && ui.selectable_label(current.is_none(), tr(none)).clicked() {
            out = Some(None);
        }
        for (k, _, label) in genha_motion::ENTRANCES {
            if ui.selectable_label(current == Some(*k), tr(label)).clicked() {
                out = Some(Some(*k));
            }
        }
    });
    out
}

/// The selected object's animation: entrance, timing, loop, exit and a preview.
fn element_animation(ui: &mut egui::Ui, t: &Tokens, el: &Element, acts: &mut Actions) {
    let id = el.id.0;
    section(ui, t, tr("ANIMATION"));
    let a = el.animation.as_ref();
    ui.horizontal(|ui| {
        ui.label(tr("Entrance"));
        if let Some(choice) = kind_combo(ui, &format!("anim_in_{id}"), a.map(|a| a.kind), N_("None")) {
            let kind = choice.map(genha_motion::kind_id).unwrap_or("none");
            acts.run("animation.set", json!({"kind": kind}));
        }
    });
    let Some(a) = a else {
        ui.label(egui::RichText::new(tr("Pick an entrance to animate this object.")).size(11.0).color(t.text_dim));
        return;
    };
    let (mut duration, mut delay) = (a.duration, a.delay);
    if ui.add(egui::Slider::new(&mut duration, 0.1..=5.0).text(tr("Duration (s)")).max_decimals(1)).changed() {
        acts.run("animation.set", json!({"duration": duration, "merge_key": format!("animdur:{id}")}));
    }
    if ui.add(egui::Slider::new(&mut delay, 0.0..=10.0).text(tr("Delay (s)")).max_decimals(1)).changed() {
        acts.run("animation.set", json!({"delay": delay, "merge_key": format!("animdelay:{id}")}));
    }
    ui.horizontal(|ui| {
        ui.label(tr("Loop"));
        let text = tr(a.looping.map(genha_motion::loop_label).unwrap_or(N_("None")));
        egui::ComboBox::from_id_salt(format!("anim_loop_{id}")).selected_text(text).width(150.0).show_ui(ui, |ui| {
            if ui.selectable_label(a.looping.is_none(), tr("None")).clicked() {
                acts.run("animation.set", json!({"loop": "none"}));
            }
            for (k, kid, label) in genha_motion::LOOPS {
                if ui.selectable_label(a.looping == Some(*k), tr(label)).clicked() {
                    acts.run("animation.set", json!({"loop": kid}));
                }
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label(tr("Exit"));
        if let Some(choice) = kind_combo(ui, &format!("anim_out_{id}"), a.exit, N_("None")) {
            acts.run("animation.set", json!({"exit": choice.map(genha_motion::kind_id).unwrap_or("none")}));
        }
    });
    if ui.button(tr("Preview")).on_hover_text(tr("Play this page's animations on the canvas")).clicked() {
        acts.preview = true;
    }
}

/// The page's animation: animate everything at once, and how long the page plays.
fn page_animation(ui: &mut egui::Ui, t: &Tokens, p: &Page, choices: &mut crate::motion::PanelChoices, acts: &mut Actions) {
    section(ui, t, tr("ANIMATION"));
    ui.horizontal(|ui| {
        ui.label(tr("Style"));
        if let Some(Some(k)) = kind_combo(ui, "anim_all_kind", Some(choices.kind), "") {
            choices.kind = k;
        }
    });
    ui.add(egui::Slider::new(&mut choices.stagger, 0.0..=1.5).text(tr("Stagger (s)")).max_decimals(2));
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Animate all elements")).on_hover_text(tr("One after another; backgrounds stay still")).clicked() {
            acts.run("animation.applyToPage", json!({"kind": genha_motion::kind_id(choices.kind), "stagger": choices.stagger}));
            acts.preview = true;
        }
        if genha_motion::is_animated(p) {
            if ui.button(tr("Remove all")).clicked() {
                acts.run("animation.applyToPage", json!({"kind": "none", "include_background": true}));
            }
            if ui.button(tr("Preview")).clicked() {
                acts.preview = true;
            }
        }
    });
    let mut fixed = p.duration.is_some();
    let mut seconds = genha_motion::page_duration(p);
    ui.horizontal(|ui| {
        if ui.checkbox(&mut fixed, tr("Fixed length")).on_hover_text(tr("Off: the page plays until its animations finish")).changed() {
            let value = if fixed { json!((seconds * 10.0).round() / 10.0) } else { Value::Null };
            acts.run("page.setDuration", json!({"seconds": value}));
        }
        if fixed {
            let r = ui.add(egui::DragValue::new(&mut seconds).range(0.5..=genha_motion::MAX_PAGE_DURATION).speed(0.1).suffix(" s"));
            if r.changed() {
                acts.run("page.setDuration", json!({"seconds": seconds, "merge_key": format!("pagedur:{}", p.id.0)}));
            }
        } else {
            ui.label(egui::RichText::new(tr!("Plays for {seconds} s", seconds = format!("{seconds:.1}"))).color(t.text_dim));
        }
    });
}

fn title(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(16.0).strong());
    ui.add_space(2.0);
}

fn section(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(text).size(12.0).color(t.text_dim).strong());
}

/// A colour button plus brand swatches; returns the new colour when changed.
fn color_row(ui: &mut egui::Ui, color: genha_color::Color, brand: &[genha_color::Color]) -> Option<genha_color::Color> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        let mut c = to_egui(color);
        if ui.color_edit_button_srgba(&mut c).changed() {
            out = Some(from_egui(c));
        }
        for b in brand.iter().take(8) {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 10, to_egui(*b));
            ui.painter().circle_stroke(rect.center(), 10.0, egui::Stroke::new(1.0, egui::Color32::from_black_alpha(40)));
            if r.on_hover_text(b.to_hex()).clicked() {
                out = Some(*b);
            }
        }
    });
    out
}

fn page_props(ui: &mut egui::Ui, t: &Tokens, p: &Page, brand: &[genha_color::Color], acts: &mut Actions) {
    title(ui, tr("Page"));
    let mut name = crate::shown_name(&p.name);
    if ui.text_edit_singleline(&mut name).changed() {
        acts.run("page.rename", json!({"name": name, "merge_key": format!("pagename:{}", p.id.0)}));
    }
    section(ui, t, tr("SIZE"));
    let (mut w, mut h) = (p.width, p.height);
    ui.horizontal(|ui| {
        let a = ui.add(egui::DragValue::new(&mut w).range(1.0..=16384.0).prefix(tr("W ")).suffix(" px"));
        let b = ui.add(egui::DragValue::new(&mut h).range(1.0..=16384.0).prefix(tr("H ")).suffix(" px"));
        if a.changed() || b.changed() {
            acts.run("page.setSize", json!({"width": w, "height": h, "merge_key": format!("pagesize:{}", p.id.0)}));
        }
    });
    ui.label(egui::RichText::new(tr("Use ⇲ Resize in the top bar to re-layout for another size.")).size(11.0).color(t.text_dim));
    section(ui, t, tr("BACKGROUND"));
    let current = p.background.primary_color().unwrap_or(genha_color::Color::WHITE);
    if let Some(c) = color_row(ui, current, brand) {
        acts.run("page.setBackground", json!({"color": c.to_hex(), "merge_key": format!("pagebg:{}", p.id.0)}));
    }
    ui.horizontal(|ui| {
        if ui.button(tr("Gradient")).clicked() {
            let second = brand.get(1).copied().unwrap_or(genha_color::Color::rgb(0xff, 0x6b, 0x5b));
            acts.run("page.setBackground", json!({"fill": Fill::linear(90.0, current, second)}));
        }
        if ui.button(tr("Transparent")).clicked() {
            acts.run("page.setBackground", json!({"fill": Fill::None}));
        }
    });
    section(ui, t, tr("NOTES"));
    let mut notes = p.notes.clone();
    if ui.add(egui::TextEdit::multiline(&mut notes).desired_rows(3).hint_text(tr("Speaker notes"))).changed() {
        acts.run("page.setNotes", json!({"notes": notes, "merge_key": format!("notes:{}", p.id.0)}));
    }
    section(ui, t, tr("PAGE"));
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Add page")).clicked() {
            acts.run("page.add", json!({}));
        }
        if ui.button(tr("Duplicate page")).clicked() {
            acts.run("page.duplicate", json!({}));
        }
        if ui.button(tr("Crop to content")).on_hover_text(tr("Shrink the page to its objects")).clicked() {
            acts.run("page.cropToContent", json!({"padding": 0}));
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn element_props(
    ui: &mut egui::Ui,
    t: &Tokens,
    el: &Element,
    sel: Option<&RangeStyle>,
    editing_caret: bool,
    brand: &[genha_color::Color],
    families: &[String],
    fit: Option<genha_text::FitScale>,
    acts: &mut Actions,
) {
    let id = el.id.0;
    let key = |what: &str| format!("{what}:{id}");
    title(ui, &crate::element_name(el));
    match &el.kind {
        ElementKind::Text(text) => {
            section(ui, t, tr("TEXT"));
            let mut s = text.text.clone();
            if ui.add(egui::TextEdit::multiline(&mut s).desired_rows(3).desired_width(f32::INFINITY)).changed() {
                acts.run("text.setText", json!({"text": s, "merge_key": key("text")}));
            }
            // The style shown: at the selection while the box is edited, else its first
            // character's; values that vary over it show as mixed.
            let whole;
            let shown = match sel {
                Some(r) => r,
                None => {
                    whole = text.range_style(0..text.text.chars().count());
                    &whole
                }
            };
            let st = &shown.style;
            let mixed = |f: &str| shown.is_mixed(f);
            if sel.is_some() {
                let hint = if editing_caret { tr("Changes apply to the next text you type") } else { tr("Changes apply to the selected text") };
                ui.label(egui::RichText::new(hint).size(11.0).color(t.text_dim));
            }
            let mut family = st.font_family.clone();
            crate::editor::font_combo(
                ui,
                &format!("font_{id}"),
                if mixed("font_family") { tr("Font (mixed)") } else { tr("Font") },
                &mut family,
                families,
            );
            if family != st.font_family {
                acts.run("text.setStyle", json!({"font_family": family}));
            }
            ui.horizontal(|ui| {
                let mut size = st.font_size;
                let prefix = if mixed("font_size") { tr("Size (mixed) ") } else { tr("Size ") };
                if ui.add(egui::DragValue::new(&mut size).range(1.0..=2000.0).speed(0.5).prefix(prefix)).changed() {
                    acts.run("text.setStyle", json!({"font_size": size, "merge_key": key("size")}));
                }
                for (label, field, cmd, on) in [
                    ("B", N_("bold"), "text.toggleBold", st.bold),
                    ("I", N_("italic"), "text.toggleItalic", st.italic),
                    ("U", N_("underline"), "text.toggleUnderline", st.underline),
                    ("S", N_("strikethrough"), "text.toggleStrikethrough", st.strikethrough),
                ] {
                    let label = if mixed(field) { format!("{label}~") } else { label.to_string() };
                    let b = ui.add(egui::Button::new(label).selected(on && !mixed(field)));
                    if b.on_hover_text(if mixed(field) { tr!("{style}: mixed", style = tr(field)) } else { tr(field).to_string() }).clicked() {
                        acts.run(cmd, json!({}));
                    }
                }
                if ui.add(egui::Button::new("AA").selected(text.style.uppercase)).on_hover_text(tr("uppercase")).clicked() {
                    acts.run("text.setStyle", json!({"uppercase": !text.style.uppercase}));
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Align"));
                for (label, a) in [(N_("Left"), TextAlign::Left), (N_("Center"), TextAlign::Center), (N_("Right"), TextAlign::Right)] {
                    if ui.add(egui::Button::new(tr(label)).selected(st.align == a)).clicked() {
                        acts.run("text.setStyle", json!({"align": a}));
                    }
                }
            });
            if mixed("color") {
                ui.label(egui::RichText::new(tr("Colour: mixed")).size(11.0).color(t.text_dim));
            }
            if let Some(c) = color_row(ui, st.color, brand) {
                acts.run("text.setStyle", json!({"color": c.to_hex(), "merge_key": key("color")}));
            }
            let st = &text.style;
            let (mut lh, mut ls) = (st.line_height, st.letter_spacing);
            if ui.add(egui::Slider::new(&mut lh, 0.6..=3.0).text(tr("Line height"))).changed() {
                acts.run("text.setStyle", json!({"line_height": lh, "merge_key": key("lh")}));
            }
            if ui.add(egui::Slider::new(&mut ls, -200.0..=800.0).text(tr("Letter spacing"))).changed() {
                acts.run("text.setStyle", json!({"letter_spacing": ls, "merge_key": key("ls")}));
            }
            let mut shrink = text.fit == TextFit::Shrink;
            let tip = tr("When the text does not fit its box, set it smaller (down to 40%) instead of growing the box");
            if ui.checkbox(&mut shrink, tr("Shrink text to fit")).on_hover_text(tip).changed() {
                acts.run("text.setFit", json!({"fit": if shrink { "shrink" } else { "none" }}));
            }
            if let Some(f) = fit.filter(|f| f.scale < 1.0 || !f.fits) {
                let percent = format!("{:.0}", f.scale * 100.0);
                let note = if f.fits {
                    tr!("Shown at {percent}% to fit", percent = percent)
                } else {
                    tr!("Too long to fit, even at {percent}%", percent = percent)
                };
                ui.label(egui::RichText::new(note).size(11.0).color(t.text_dim));
            }
            data_props::curve(ui, t, id, text, acts);
            section(ui, t, tr("EFFECT"));
            ui.horizontal_wrapped(|ui| {
                let effects: [(&str, TextEffect); 4] = [
                    (N_("None"), TextEffect::None),
                    (N_("Shadow"), TextEffect::Shadow { color: genha_color::Color::rgba(0, 0, 0, 110), offset_x: 4.0, offset_y: 4.0, blur: 6.0 }),
                    (N_("Outline"), TextEffect::Outline { color: st.color.readable_text(), width: (st.font_size / 18.0).max(1.0) }),
                    (
                        N_("Highlight"),
                        TextEffect::Highlight {
                            color: brand.get(1).copied().unwrap_or(genha_color::Color::rgb(255, 230, 0)),
                            padding: st.font_size * 0.2,
                            corner_radius: st.font_size * 0.15,
                        },
                    ),
                ];
                for (label, e) in effects {
                    let on = std::mem::discriminant(&text.effect) == std::mem::discriminant(&e);
                    if ui.add(egui::Button::new(tr(label)).selected(on)).clicked() {
                        acts.run("text.setEffect", json!({"effect": e}));
                    }
                }
            });
            match &text.effect {
                TextEffect::Shadow { color, .. } | TextEffect::Outline { color, .. } | TextEffect::Highlight { color, .. } => {
                    if let Some(c) = color_row(ui, *color, brand) {
                        let mut e = serde_json::to_value(&text.effect).unwrap_or(Value::Null);
                        if let Some(o) = e.as_object_mut() {
                            o.insert("color".into(), json!(c.to_hex()));
                        }
                        acts.run("text.setEffect", json!({"effect": e, "merge_key": key("effectcolor")}));
                    }
                }
                TextEffect::None => {}
            }
        }
        ElementKind::Shape(sh) => {
            section(ui, t, tr("FILL"));
            let fill = sh.fill.primary_color().unwrap_or(genha_color::Color::TRANSPARENT);
            if let Some(c) = color_row(ui, fill, brand) {
                acts.run("style.fill", json!({"color": c.to_hex(), "merge_key": key("fill")}));
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Gradient")).clicked() {
                    let second = brand.iter().find(|b| **b != fill).copied().unwrap_or(genha_color::Color::WHITE);
                    acts.run("style.fill", json!({"fill": Fill::linear(45.0, fill, second)}));
                }
                if ui.button(tr("No fill")).clicked() {
                    acts.run("style.fill", json!({"color": "none"}));
                }
            });
            if let ShapeKind::Rect { corner_radius } = &sh.shape {
                let mut r = *corner_radius;
                if ui.add(egui::Slider::new(&mut r, 0.0..=(el.frame.width.min(el.frame.height) / 2.0).max(1.0)).text(tr("Corners"))).changed() {
                    acts.run("style.cornerRadius", json!({"radius": r, "merge_key": key("radius")}));
                }
            }
            section(ui, t, tr("BORDER"));
            let stroke = sh.stroke.clone();
            ui.horizontal(|ui| {
                let mut on = stroke.is_some();
                if ui.checkbox(&mut on, tr("Border")).changed() {
                    acts.run("style.stroke", if on { json!({"width": 4.0}) } else { json!({"none": true}) });
                }
            });
            if let Some(st) = stroke {
                if let Some(c) = color_row(ui, st.color, brand) {
                    acts.run("style.stroke", json!({"color": c.to_hex(), "merge_key": key("strokecolor")}));
                }
                let mut w = st.width;
                if ui.add(egui::Slider::new(&mut w, 0.0..=100.0).text(tr("Width"))).changed() {
                    acts.run("style.stroke", json!({"width": w, "merge_key": key("strokewidth")}));
                }
                let mut dashed = !st.dash.is_empty();
                if ui.checkbox(&mut dashed, tr("Dashed")).changed() {
                    let dash = if dashed { vec![w * 3.0, w * 2.0] } else { vec![] };
                    acts.run("style.stroke", json!({"dash": dash}));
                }
            }
            section(ui, t, tr("SHADOW"));
            let mut on = sh.shadow.is_some();
            if ui.checkbox(&mut on, tr("Drop shadow")).changed() {
                acts.run("style.shadow", if on { json!({}) } else { json!({"none": true}) });
            }
            if let Some(s) = &sh.shadow {
                let mut blur = s.blur;
                if ui.add(egui::Slider::new(&mut blur, 0.0..=100.0).text(tr("Blur"))).changed() {
                    acts.run("style.shadow", json!({"blur": blur, "merge_key": key("shadowblur")}));
                }
                let (mut ox, mut oy) = (s.offset_x, s.offset_y);
                ui.horizontal(|ui| {
                    let a = ui.add(egui::DragValue::new(&mut ox).prefix(tr("X ")));
                    let b = ui.add(egui::DragValue::new(&mut oy).prefix(tr("Y ")));
                    if a.changed() || b.changed() {
                        acts.run("style.shadow", json!({"offset_x": ox, "offset_y": oy, "merge_key": key("shadowoff")}));
                    }
                });
            }
        }
        ElementKind::Image(img) => {
            section(ui, t, tr("FILTERS"));
            let current = img.filter.clone();
            ui.horizontal_wrapped(|ui| {
                for f in genha_effects::filter_presets() {
                    if ui.add(egui::Button::new(genha_i18n::trc("filter", f.label)).selected(current.as_deref() == Some(f.id))).clicked() {
                        acts.run("image.filter", json!({"preset": f.id, "intensity": 100}));
                    }
                }
            });
            section(ui, t, tr("ADJUST"));
            let a = &img.adjustments;
            for (label, field, v, lo, hi) in [
                (N_("Brightness"), "brightness", a.brightness, -100.0, 100.0),
                (N_("Contrast"), "contrast", a.contrast, -100.0, 100.0),
                (N_("Saturation"), "saturation", a.saturation, -100.0, 100.0),
                (N_("Warmth"), "warmth", a.warmth, -100.0, 100.0),
                (N_("Sharpen"), "sharpen", a.sharpen, 0.0, 100.0),
                (N_("Blur"), "blur", a.blur, 0.0, 100.0),
            ] {
                let mut x = v;
                if ui.add(egui::Slider::new(&mut x, lo..=hi).text(tr(label))).changed() {
                    acts.run("image.adjust", json!({field: x, "merge_key": key(field)}));
                }
            }
            if ui.button(tr("Reset adjustments")).clicked() {
                acts.run("image.adjust", json!({"reset": true}));
            }
            section(ui, t, tr("CROP & SHAPE"));
            let c = img.crop;
            let (mut cx, mut cy, mut cw, mut ch) = (c.x, c.y, c.width, c.height);
            let mut changed = false;
            changed |= ui.add(egui::Slider::new(&mut cx, 0.0..=0.95).text(tr("Left"))).changed();
            changed |= ui.add(egui::Slider::new(&mut cy, 0.0..=0.95).text(tr("Top"))).changed();
            changed |= ui.add(egui::Slider::new(&mut cw, 0.05..=1.0).text(tr("Width"))).changed();
            changed |= ui.add(egui::Slider::new(&mut ch, 0.05..=1.0).text(tr("Height"))).changed();
            if changed {
                acts.run("image.crop", json!({"x": cx, "y": cy, "width": cw, "height": ch, "merge_key": key("crop")}));
            }
            ui.horizontal_wrapped(|ui| {
                for (label, shape) in [
                    (N_("None"), ""),
                    (N_("Circle"), "circle"),
                    (N_("Rounded"), "rounded"),
                    (N_("Star"), "star"),
                    (N_("Hexagon"), "polygon"),
                    (N_("Triangle"), "triangle"),
                ] {
                    if ui.button(tr(label)).clicked() {
                        acts.run("image.mask", if shape.is_empty() { json!({"none": true}) } else { json!({"shape": shape}) });
                    }
                }
            });
            let mut r = img.corner_radius;
            if ui.add(egui::Slider::new(&mut r, 0.0..=(el.frame.width.min(el.frame.height) / 2.0).max(1.0)).text(tr("Corners"))).changed() {
                acts.run("style.cornerRadius", json!({"radius": r, "merge_key": key("radius")}));
            }
            section(ui, t, tr("QUICK ACTIONS"));
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("Remove background")).on_hover_text(tr("Finds the subject and removes everything else")).clicked() {
                    acts.run("image.removeBackground", json!({}));
                }
                if ui.button(tr("Blur background")).clicked() {
                    acts.run("image.blurBackground", json!({}));
                }
                if ui.button(tr("White background")).clicked() {
                    acts.run("image.changeBackground", json!({"color": "#ffffff"}));
                }
                if let Some(b) = brand.first()
                    && ui.button(tr("Brand background")).clicked()
                {
                    acts.run("image.changeBackground", json!({"color": b.to_hex()}));
                }
                if ui.button(tr("Fill frame")).clicked() {
                    acts.run("image.setFit", json!({"fit": "cover"}));
                }
                if ui.button(tr("Fit inside")).clicked() {
                    acts.run("image.setFit", json!({"fit": "contain"}));
                }
                if ui.button(tr("Colours → brand")).clicked() {
                    acts.run("brand.fromPicture", json!({}));
                }
            });
            crate::models::draw(ui, &mut acts.ml);
        }
        ElementKind::Chart(c) => data_props::chart(ui, t, id, c, brand, acts),
        ElementKind::Table(tb) => data_props::table(ui, t, id, tb, el.frame.width, brand, acts),
        ElementKind::Group(g) => {
            ui.label(tr_n!("Group of {n} object", "Group of {n} objects", g.children.len()));
            if ui.button(tr("Ungroup")).clicked() {
                acts.run("object.ungroup", json!({}));
            }
        }
    }
    element_animation(ui, t, el, acts);
    common(ui, t, el, acts);
}

fn common(ui: &mut egui::Ui, t: &Tokens, el: &Element, acts: &mut Actions) {
    let id = el.id.0;
    section(ui, t, tr("POSITION"));
    let f = el.frame;
    let (mut x, mut y, mut w, mut h, mut r) = (f.x, f.y, f.width, f.height, f.rotation);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.add(egui::DragValue::new(&mut x).prefix(tr("X ")).speed(1.0)).changed();
        changed |= ui.add(egui::DragValue::new(&mut y).prefix(tr("Y ")).speed(1.0)).changed();
    });
    ui.horizontal(|ui| {
        changed |= ui.add(egui::DragValue::new(&mut w).prefix(tr("W ")).range(1.0..=100_000.0).speed(1.0)).changed();
        changed |= ui.add(egui::DragValue::new(&mut h).prefix(tr("H ")).range(1.0..=100_000.0).speed(1.0)).changed();
        changed |= ui.add(egui::DragValue::new(&mut r).prefix(tr("Rot ")).suffix("°").speed(0.5)).changed();
    });
    if changed {
        acts.run("object.setFrame", json!({"x": x, "y": y, "width": w, "height": h, "rotation": r, "merge_key": format!("frame:{id}")}));
    }
    let mut o = el.opacity;
    if ui.add(egui::Slider::new(&mut o, 0.0..=1.0).text(tr("Opacity"))).changed() {
        acts.run("object.setOpacity", json!({"opacity": o, "merge_key": format!("opacity:{id}")}));
    }
    section(ui, t, tr("ALIGN TO PAGE"));
    align_buttons(ui, acts, "page");
    section(ui, t, tr("ARRANGE"));
    ui.horizontal_wrapped(|ui| {
        for (label, cmd) in [
            (N_("Forward"), "object.bringForward"),
            (N_("Backward"), "object.sendBackward"),
            (N_("Flip H"), "object.flipHorizontal"),
            (N_("Flip V"), "object.flipVertical"),
            (N_("Duplicate"), "edit.duplicate"),
            (N_("Copy style"), "style.copy"),
            (N_("Paste style"), "style.paste"),
        ] {
            if ui.button(tr(label)).clicked() {
                acts.run(cmd, json!({}));
            }
        }
        if ui.button(if el.locked { tr("Unlock") } else { tr("Lock") }).clicked() {
            acts.run(if el.locked { "object.unlock" } else { "object.lock" }, json!({}));
        }
        if ui.add(egui::Button::new(egui::RichText::new(tr("Delete")).color(t.danger))).clicked() {
            acts.run("edit.delete", json!({}));
        }
    });
}

fn align_buttons(ui: &mut egui::Ui, acts: &mut Actions, to: &str) {
    ui.horizontal_wrapped(|ui| {
        for (label, how, hint) in [
            (N_("Left"), "left", N_("Align left edges")),
            (N_("Center"), "center-h", N_("Centre horizontally")),
            (N_("Right"), "right", N_("Align right edges")),
            (N_("Top"), "top", N_("Align top edges")),
            (N_("Middle"), "center-v", N_("Centre vertically")),
            (N_("Bottom"), "bottom", N_("Align bottom edges")),
        ] {
            if ui.button(tr(label)).on_hover_text(tr(hint)).clicked() {
                acts.run("object.align", json!({"how": how, "to": to}));
            }
        }
    });
}

fn multi_props(ui: &mut egui::Ui, t: &Tokens, els: &[Element], acts: &mut Actions) {
    title(ui, &tr_n!("{n} object", "{n} objects", els.len()));
    section(ui, t, tr("ALIGN"));
    align_buttons(ui, acts, "selection");
    section(ui, t, tr("DISTRIBUTE"));
    ui.horizontal(|ui| {
        if ui.button(tr("Horizontally")).clicked() {
            acts.run("object.distribute", json!({"axis": "horizontal"}));
        }
        if ui.button(tr("Vertically")).clicked() {
            acts.run("object.distribute", json!({"axis": "vertical"}));
        }
    });
    section(ui, t, tr("ARRANGE"));
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Group")).clicked() {
            acts.run("object.group", json!({}));
        }
        if ui.button(tr("Duplicate")).clicked() {
            acts.run("edit.duplicate", json!({}));
        }
        if ui.button(tr("Paste style")).clicked() {
            acts.run("style.paste", json!({}));
        }
        if ui.add(egui::Button::new(egui::RichText::new(tr("Delete")).color(t.danger))).clicked() {
            acts.run("edit.delete", json!({}));
        }
    });
    if els.iter().all(|e| matches!(e.kind, ElementKind::Image(_))) {
        section(ui, t, tr("COLLAGE"));
        ui.label(egui::RichText::new(tr("Pick a layout in Elements › Collage.")).color(t.text_dim));
    }
}
