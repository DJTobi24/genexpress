//! The right-hand panel: properties of the selection, or of the page when nothing is selected.

use serde_json::{Value, json};

use crate::App;
use crate::theme::{Tokens, from_egui, to_egui};
use genha_doc::{Element, ElementKind, Fill, Page, ShapeKind, TextAlign, TextEffect};

/// Commands to run after drawing (the panel reads a snapshot, then acts).
#[derive(Default)]
struct Actions(Vec<(&'static str, Value)>);

impl Actions {
    fn run(&mut self, id: &'static str, p: Value) {
        self.0.push((id, p));
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let Some(d) = app.session.active() else { return };
    let selected: Vec<Element> = d.selected_elements().into_iter().cloned().collect();
    let page = d.doc.pages.get(d.page).cloned();
    let brand: Vec<genha_color::Color> = d.doc.brand.colors.iter().map(|c| c.color).collect();
    let families = app.font_families().to_vec();
    let mut acts = Actions::default();
    match selected.as_slice() {
        [] => {
            if let Some(p) = &page {
                page_props(ui, t, p, &brand, &mut acts);
            }
        }
        [el] => element_props(ui, t, el, &brand, &families, &mut acts),
        many => multi_props(ui, t, many, &mut acts),
    }
    for (id, p) in acts.0 {
        app.run(id, &p);
    }
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
    title(ui, "Page");
    let mut name = p.name.clone();
    if ui.text_edit_singleline(&mut name).changed() {
        acts.run("page.rename", json!({"name": name, "merge_key": format!("pagename:{}", p.id.0)}));
    }
    section(ui, t, "SIZE");
    let (mut w, mut h) = (p.width, p.height);
    ui.horizontal(|ui| {
        let a = ui.add(egui::DragValue::new(&mut w).range(1.0..=16384.0).prefix("W ").suffix(" px"));
        let b = ui.add(egui::DragValue::new(&mut h).range(1.0..=16384.0).prefix("H ").suffix(" px"));
        if a.changed() || b.changed() {
            acts.run("page.setSize", json!({"width": w, "height": h, "merge_key": format!("pagesize:{}", p.id.0)}));
        }
    });
    ui.label(egui::RichText::new("Use ⇲ Resize in the top bar to re-layout for another size.").size(11.0).color(t.text_dim));
    section(ui, t, "BACKGROUND");
    let current = p.background.primary_color().unwrap_or(genha_color::Color::WHITE);
    if let Some(c) = color_row(ui, current, brand) {
        acts.run("page.setBackground", json!({"color": c.to_hex(), "merge_key": format!("pagebg:{}", p.id.0)}));
    }
    ui.horizontal(|ui| {
        if ui.button("Gradient").clicked() {
            let second = brand.get(1).copied().unwrap_or(genha_color::Color::rgb(0xff, 0x6b, 0x5b));
            acts.run("page.setBackground", json!({"fill": Fill::linear(90.0, current, second)}));
        }
        if ui.button("Transparent").clicked() {
            acts.run("page.setBackground", json!({"fill": Fill::None}));
        }
    });
    section(ui, t, "NOTES");
    let mut notes = p.notes.clone();
    if ui.add(egui::TextEdit::multiline(&mut notes).desired_rows(3).hint_text("Speaker notes")).changed() {
        acts.run("page.setNotes", json!({"notes": notes, "merge_key": format!("notes:{}", p.id.0)}));
    }
    section(ui, t, "PAGE");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Add page").clicked() {
            acts.run("page.add", json!({}));
        }
        if ui.button("Duplicate page").clicked() {
            acts.run("page.duplicate", json!({}));
        }
    });
}

fn element_props(ui: &mut egui::Ui, t: &Tokens, el: &Element, brand: &[genha_color::Color], families: &[String], acts: &mut Actions) {
    let id = el.id.0;
    let key = |what: &str| format!("{what}:{id}");
    title(ui, &el.display_name());
    match &el.kind {
        ElementKind::Text(text) => {
            section(ui, t, "TEXT");
            let mut s = text.text.clone();
            if ui.add(egui::TextEdit::multiline(&mut s).desired_rows(3).desired_width(f32::INFINITY)).changed() {
                acts.run("text.setText", json!({"text": s, "merge_key": key("text")}));
            }
            let st = &text.style;
            let mut family = st.font_family.clone();
            crate::editor::font_combo(ui, &format!("font_{id}"), "Font", &mut family, families);
            if family != st.font_family {
                acts.run("text.setStyle", json!({"font_family": family}));
            }
            ui.horizontal(|ui| {
                let mut size = st.font_size;
                if ui.add(egui::DragValue::new(&mut size).range(1.0..=2000.0).speed(0.5).prefix("Size ")).changed() {
                    acts.run("text.setStyle", json!({"font_size": size, "merge_key": key("size")}));
                }
                for (label, field, on) in
                    [("B", "bold", st.bold), ("I", "italic", st.italic), ("U", "underline", st.underline), ("AA", "uppercase", st.uppercase)]
                {
                    if ui.add(egui::Button::new(label).selected(on)).clicked() {
                        acts.run("text.setStyle", json!({field: !on}));
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Align");
                for (label, a) in [("Left", TextAlign::Left), ("Center", TextAlign::Center), ("Right", TextAlign::Right)] {
                    if ui.add(egui::Button::new(label).selected(st.align == a)).clicked() {
                        acts.run("text.setStyle", json!({"align": a}));
                    }
                }
            });
            if let Some(c) = color_row(ui, st.color, brand) {
                acts.run("text.setStyle", json!({"color": c.to_hex(), "merge_key": key("color")}));
            }
            let (mut lh, mut ls) = (st.line_height, st.letter_spacing);
            if ui.add(egui::Slider::new(&mut lh, 0.6..=3.0).text("Line height")).changed() {
                acts.run("text.setStyle", json!({"line_height": lh, "merge_key": key("lh")}));
            }
            if ui.add(egui::Slider::new(&mut ls, -200.0..=800.0).text("Letter spacing")).changed() {
                acts.run("text.setStyle", json!({"letter_spacing": ls, "merge_key": key("ls")}));
            }
            section(ui, t, "EFFECT");
            ui.horizontal_wrapped(|ui| {
                let effects: [(&str, TextEffect); 4] = [
                    ("None", TextEffect::None),
                    ("Shadow", TextEffect::Shadow { color: genha_color::Color::rgba(0, 0, 0, 110), offset_x: 4.0, offset_y: 4.0, blur: 6.0 }),
                    ("Outline", TextEffect::Outline { color: st.color.readable_text(), width: (st.font_size / 18.0).max(1.0) }),
                    (
                        "Highlight",
                        TextEffect::Highlight {
                            color: brand.get(1).copied().unwrap_or(genha_color::Color::rgb(255, 230, 0)),
                            padding: st.font_size * 0.2,
                            corner_radius: st.font_size * 0.15,
                        },
                    ),
                ];
                for (label, e) in effects {
                    let on = std::mem::discriminant(&text.effect) == std::mem::discriminant(&e);
                    if ui.add(egui::Button::new(label).selected(on)).clicked() {
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
            section(ui, t, "FILL");
            let fill = sh.fill.primary_color().unwrap_or(genha_color::Color::TRANSPARENT);
            if let Some(c) = color_row(ui, fill, brand) {
                acts.run("style.fill", json!({"color": c.to_hex(), "merge_key": key("fill")}));
            }
            ui.horizontal(|ui| {
                if ui.button("Gradient").clicked() {
                    let second = brand.iter().find(|b| **b != fill).copied().unwrap_or(genha_color::Color::WHITE);
                    acts.run("style.fill", json!({"fill": Fill::linear(45.0, fill, second)}));
                }
                if ui.button("No fill").clicked() {
                    acts.run("style.fill", json!({"color": "none"}));
                }
            });
            if let ShapeKind::Rect { corner_radius } = &sh.shape {
                let mut r = *corner_radius;
                if ui.add(egui::Slider::new(&mut r, 0.0..=(el.frame.width.min(el.frame.height) / 2.0).max(1.0)).text("Corners")).changed() {
                    acts.run("style.cornerRadius", json!({"radius": r, "merge_key": key("radius")}));
                }
            }
            section(ui, t, "BORDER");
            let stroke = sh.stroke.clone();
            ui.horizontal(|ui| {
                let mut on = stroke.is_some();
                if ui.checkbox(&mut on, "Border").changed() {
                    acts.run("style.stroke", if on { json!({"width": 4.0}) } else { json!({"none": true}) });
                }
            });
            if let Some(st) = stroke {
                if let Some(c) = color_row(ui, st.color, brand) {
                    acts.run("style.stroke", json!({"color": c.to_hex(), "merge_key": key("strokecolor")}));
                }
                let mut w = st.width;
                if ui.add(egui::Slider::new(&mut w, 0.0..=100.0).text("Width")).changed() {
                    acts.run("style.stroke", json!({"width": w, "merge_key": key("strokewidth")}));
                }
                let mut dashed = !st.dash.is_empty();
                if ui.checkbox(&mut dashed, "Dashed").changed() {
                    let dash = if dashed { vec![w * 3.0, w * 2.0] } else { vec![] };
                    acts.run("style.stroke", json!({"dash": dash}));
                }
            }
            section(ui, t, "SHADOW");
            let mut on = sh.shadow.is_some();
            if ui.checkbox(&mut on, "Drop shadow").changed() {
                acts.run("style.shadow", if on { json!({}) } else { json!({"none": true}) });
            }
            if let Some(s) = &sh.shadow {
                let mut blur = s.blur;
                if ui.add(egui::Slider::new(&mut blur, 0.0..=100.0).text("Blur")).changed() {
                    acts.run("style.shadow", json!({"blur": blur, "merge_key": key("shadowblur")}));
                }
                let (mut ox, mut oy) = (s.offset_x, s.offset_y);
                ui.horizontal(|ui| {
                    let a = ui.add(egui::DragValue::new(&mut ox).prefix("X "));
                    let b = ui.add(egui::DragValue::new(&mut oy).prefix("Y "));
                    if a.changed() || b.changed() {
                        acts.run("style.shadow", json!({"offset_x": ox, "offset_y": oy, "merge_key": key("shadowoff")}));
                    }
                });
            }
        }
        ElementKind::Image(img) => {
            section(ui, t, "FILTERS");
            let current = img.filter.clone();
            ui.horizontal_wrapped(|ui| {
                for f in genha_effects::filter_presets() {
                    if ui.add(egui::Button::new(f.label).selected(current.as_deref() == Some(f.id))).clicked() {
                        acts.run("image.filter", json!({"preset": f.id, "intensity": 100}));
                    }
                }
            });
            section(ui, t, "ADJUST");
            let a = &img.adjustments;
            for (label, field, v, lo, hi) in [
                ("Brightness", "brightness", a.brightness, -100.0, 100.0),
                ("Contrast", "contrast", a.contrast, -100.0, 100.0),
                ("Saturation", "saturation", a.saturation, -100.0, 100.0),
                ("Warmth", "warmth", a.warmth, -100.0, 100.0),
                ("Sharpen", "sharpen", a.sharpen, 0.0, 100.0),
                ("Blur", "blur", a.blur, 0.0, 100.0),
            ] {
                let mut x = v;
                if ui.add(egui::Slider::new(&mut x, lo..=hi).text(label)).changed() {
                    acts.run("image.adjust", json!({field: x, "merge_key": key(field)}));
                }
            }
            if ui.button("Reset adjustments").clicked() {
                acts.run("image.adjust", json!({"reset": true}));
            }
            section(ui, t, "CROP & SHAPE");
            let c = img.crop;
            let (mut cx, mut cy, mut cw, mut ch) = (c.x, c.y, c.width, c.height);
            let mut changed = false;
            changed |= ui.add(egui::Slider::new(&mut cx, 0.0..=0.95).text("Left")).changed();
            changed |= ui.add(egui::Slider::new(&mut cy, 0.0..=0.95).text("Top")).changed();
            changed |= ui.add(egui::Slider::new(&mut cw, 0.05..=1.0).text("Width")).changed();
            changed |= ui.add(egui::Slider::new(&mut ch, 0.05..=1.0).text("Height")).changed();
            if changed {
                acts.run("image.crop", json!({"x": cx, "y": cy, "width": cw, "height": ch, "merge_key": key("crop")}));
            }
            ui.horizontal_wrapped(|ui| {
                for (label, shape) in
                    [("None", ""), ("Circle", "circle"), ("Rounded", "rounded"), ("Star", "star"), ("Hexagon", "polygon"), ("Triangle", "triangle")]
                {
                    if ui.button(label).clicked() {
                        acts.run("image.mask", if shape.is_empty() { json!({"none": true}) } else { json!({"shape": shape}) });
                    }
                }
            });
            let mut r = img.corner_radius;
            if ui.add(egui::Slider::new(&mut r, 0.0..=(el.frame.width.min(el.frame.height) / 2.0).max(1.0)).text("Corners")).changed() {
                acts.run("style.cornerRadius", json!({"radius": r, "merge_key": key("radius")}));
            }
            section(ui, t, "QUICK ACTIONS");
            ui.horizontal_wrapped(|ui| {
                if ui.button("Remove background").on_hover_text("Colour-based: works best on plain backdrops").clicked() {
                    acts.run("image.removeBackground", json!({}));
                }
                if ui.button("Fill frame").clicked() {
                    acts.run("image.setFit", json!({"fit": "cover"}));
                }
                if ui.button("Fit inside").clicked() {
                    acts.run("image.setFit", json!({"fit": "contain"}));
                }
                if ui.button("Colours → brand").clicked() {
                    acts.run("brand.fromPicture", json!({}));
                }
            });
        }
        ElementKind::Group(g) => {
            ui.label(format!("Group of {} objects", g.children.len()));
            if ui.button("Ungroup").clicked() {
                acts.run("object.ungroup", json!({}));
            }
        }
    }
    common(ui, t, el, acts);
}

fn common(ui: &mut egui::Ui, t: &Tokens, el: &Element, acts: &mut Actions) {
    let id = el.id.0;
    section(ui, t, "POSITION");
    let f = el.frame;
    let (mut x, mut y, mut w, mut h, mut r) = (f.x, f.y, f.width, f.height, f.rotation);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.add(egui::DragValue::new(&mut x).prefix("X ").speed(1.0)).changed();
        changed |= ui.add(egui::DragValue::new(&mut y).prefix("Y ").speed(1.0)).changed();
    });
    ui.horizontal(|ui| {
        changed |= ui.add(egui::DragValue::new(&mut w).prefix("W ").range(1.0..=100_000.0).speed(1.0)).changed();
        changed |= ui.add(egui::DragValue::new(&mut h).prefix("H ").range(1.0..=100_000.0).speed(1.0)).changed();
        changed |= ui.add(egui::DragValue::new(&mut r).prefix("Rot ").suffix("°").speed(0.5)).changed();
    });
    if changed {
        acts.run("object.setFrame", json!({"x": x, "y": y, "width": w, "height": h, "rotation": r, "merge_key": format!("frame:{id}")}));
    }
    let mut o = el.opacity;
    if ui.add(egui::Slider::new(&mut o, 0.0..=1.0).text("Opacity")).changed() {
        acts.run("object.setOpacity", json!({"opacity": o, "merge_key": format!("opacity:{id}")}));
    }
    section(ui, t, "ALIGN TO PAGE");
    align_buttons(ui, acts, "page");
    section(ui, t, "ARRANGE");
    ui.horizontal_wrapped(|ui| {
        for (label, cmd) in [
            ("Forward", "object.bringForward"),
            ("Backward", "object.sendBackward"),
            ("Flip H", "object.flipHorizontal"),
            ("Flip V", "object.flipVertical"),
            ("Duplicate", "edit.duplicate"),
            ("Copy style", "style.copy"),
            ("Paste style", "style.paste"),
        ] {
            if ui.button(label).clicked() {
                acts.run(cmd, json!({}));
            }
        }
        if ui.button(if el.locked { "Unlock" } else { "Lock" }).clicked() {
            acts.run(if el.locked { "object.unlock" } else { "object.lock" }, json!({}));
        }
        if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
            acts.run("edit.delete", json!({}));
        }
    });
}

fn align_buttons(ui: &mut egui::Ui, acts: &mut Actions, to: &str) {
    ui.horizontal_wrapped(|ui| {
        for (label, how) in
            [("Left", "left"), ("Center", "center-h"), ("Right", "right"), ("Top", "top"), ("Middle", "center-v"), ("Bottom", "bottom")]
        {
            if ui.button(label).on_hover_text(format!("Align {how}")).clicked() {
                acts.run("object.align", json!({"how": how, "to": to}));
            }
        }
    });
}

fn multi_props(ui: &mut egui::Ui, t: &Tokens, els: &[Element], acts: &mut Actions) {
    title(ui, &format!("{} objects", els.len()));
    section(ui, t, "ALIGN");
    align_buttons(ui, acts, "selection");
    section(ui, t, "DISTRIBUTE");
    ui.horizontal(|ui| {
        if ui.button("Horizontally").clicked() {
            acts.run("object.distribute", json!({"axis": "horizontal"}));
        }
        if ui.button("Vertically").clicked() {
            acts.run("object.distribute", json!({"axis": "vertical"}));
        }
    });
    section(ui, t, "ARRANGE");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Group").clicked() {
            acts.run("object.group", json!({}));
        }
        if ui.button("Duplicate").clicked() {
            acts.run("edit.duplicate", json!({}));
        }
        if ui.button("Paste style").clicked() {
            acts.run("style.paste", json!({}));
        }
        if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
            acts.run("edit.delete", json!({}));
        }
    });
    if els.iter().all(|e| matches!(e.kind, ElementKind::Image(_))) {
        section(ui, t, "COLLAGE");
        ui.label(egui::RichText::new("Pick a layout in Elements › Collage.").color(t.text_dim));
    }
}
