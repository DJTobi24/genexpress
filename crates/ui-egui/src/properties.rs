//! The object panels: when an object is selected the left panel edits it (text, pictures,
//! shapes, charts, tables, groups, several objects), and the Page item of the rail edits the
//! page. Each panel reads a snapshot of the design, then acts through commands.

use serde_json::{Value, json};

#[path = "data_props.rs"]
mod data_props;
#[path = "image_panel.rs"]
mod image_panel;
#[path = "object_panel.rs"]
mod object_panel;
#[path = "text_panel.rs"]
mod text_panel;

pub use text_panel::PanelTab;
pub(crate) use text_panel::preset_picture;

/// What the left panel edits in place of its tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Text,
    Picture,
    Shape,
    Chart,
    Table,
    Group,
    /// Several objects (the id is the first one's).
    Several,
}

/// The object the left panel edits: the selection (or the text box edited in place), unless the
/// panel was closed for it.
pub fn context_target(app: &App) -> Option<(genha_doc::NodeId, Context)> {
    let d = app.session.active()?;
    let (id, kind) = match &app.session.text_editing {
        Some(e) => (e.id, Context::Text),
        None => match d.selected_elements().as_slice() {
            [] => return None,
            [el] => (
                el.id,
                match el.kind {
                    ElementKind::Text(_) => Context::Text,
                    ElementKind::Image(_) => Context::Picture,
                    ElementKind::Shape(_) => Context::Shape,
                    ElementKind::Chart(_) => Context::Chart,
                    ElementKind::Table(_) => Context::Table,
                    ElementKind::Group(_) => Context::Group,
                },
            ),
            [first, ..] => (first.id, Context::Several),
        },
    };
    (app.editor.panel_closed != Some(id)).then_some((id, kind))
}

/// Draws the left panel's editor for `id`.
pub fn context_panel(app: &mut App, ui: &mut egui::Ui, t: &Tokens, id: genha_doc::NodeId, kind: Context) {
    match kind {
        Context::Text => text_panel::show(app, ui, t, id),
        Context::Picture => image_panel::show(app, ui, t, id),
        Context::Several => object_panel::several(app, ui, t, id),
        _ => object_panel::show(app, ui, t, id),
    }
}

/// The Page item of the rail: the page's name, size, background, notes and animation.
pub fn page_panel(app: &mut App, ui: &mut egui::Ui, t: &Tokens) {
    let Some(d) = app.session.active() else { return };
    let Some(page) = d.doc.pages.get(d.page).cloned() else { return };
    let brand: Vec<genha_color::Color> = d.doc.brand.colors.iter().map(|c| c.color).collect();
    let mut acts = Actions::default();
    let mut choices = app.motion.panel;
    page_props(ui, t, &page, &brand, &mut acts);
    page_animation(ui, t, &page, &mut choices, &mut acts);
    app.motion.panel = choices;
    for (cmd, p) in acts.commands {
        app.run(cmd, &p);
    }
    if acts.preview {
        crate::motion::play(app, ui.ctx());
    }
}

use crate::App;
use crate::theme::{Tokens, from_egui, to_egui};
use genha_doc::{AnimationKind, Element, ElementKind, Fill, Page};
use genha_i18n::{N_, tr};

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
    section(ui, t, tr("Animation"));
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
    section(ui, t, tr("Animation"));
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
        if crate::theme::switch(ui, &mut fixed, tr("Fixed length")).on_hover_text(tr("Off: the page plays until its animations finish")).changed() {
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
    ui.add_space(14.0);
    ui.label(egui::RichText::new(text).size(12.5).strong().color(t.text_dim));
    ui.add_space(2.0);
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
    section(ui, t, tr("Size"));
    let (mut w, mut h) = (p.width, p.height);
    ui.horizontal(|ui| {
        let a = ui.add(egui::DragValue::new(&mut w).range(1.0..=16384.0).prefix(tr("W ")).suffix(" px"));
        let b = ui.add(egui::DragValue::new(&mut h).range(1.0..=16384.0).prefix(tr("H ")).suffix(" px"));
        if a.changed() || b.changed() {
            acts.run("page.setSize", json!({"width": w, "height": h, "merge_key": format!("pagesize:{}", p.id.0)}));
        }
    });
    ui.label(egui::RichText::new(tr("Use ⇲ Resize in the top bar to re-layout for another size.")).size(11.0).color(t.text_dim));
    section(ui, t, tr("Background"));
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
    section(ui, t, tr("Notes"));
    let mut notes = p.notes.clone();
    if ui.add(egui::TextEdit::multiline(&mut notes).desired_rows(3).hint_text(tr("Speaker notes"))).changed() {
        acts.run("page.setNotes", json!({"notes": notes, "merge_key": format!("notes:{}", p.id.0)}));
    }
    section(ui, t, tr("Page"));
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

/// Position, size and rotation as numbers.
fn common_frame(ui: &mut egui::Ui, el: &Element, acts: &mut Actions) {
    let id = el.id.0;
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
