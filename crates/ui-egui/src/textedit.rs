//! Text edited in place on the canvas.
//!
//! The engine holds the editing state (`Session::text_editing`: which text box, the selection)
//! and the text; this module turns pointer, keyboard, clipboard and IME input into `text.edit`
//! commands and draws the caret and selection over the rendered page. Caret positions come from
//! `genha_text::caret::CaretMap`, built from the same layout the renderer draws, and are mapped
//! through the element's transform, so they follow zoom, rotation, wrapping and alignment.
//! Curved text is shown straight while it is edited (`Session::display_document`).

use std::ops::Range;
use std::sync::Arc;

use serde_json::json;

use crate::App;
use crate::theme::Tokens;
use genha_doc::{Document, ElementKind, NodeId, TextEl};
use genha_geom::{Affine, Point, Rect};
use genha_text::caret::CaretMap;
use genha_text::edit::{self, EditBuffer, EditKind, UndoStack};

/// Presses closer than this in time (seconds) and space (points) are double / triple clicks.
const MULTI_CLICK_TIME: f64 = 0.45;
const MULTI_CLICK_DIST: f32 = 6.0;
/// How long the caret stays shown, then hidden (seconds).
const BLINK: f64 = 0.53;
/// How far outside its box a press still edits the text (points).
const GRAB: f64 = 6.0;

/// UI-only state of the editing session the engine runs.
#[derive(Default)]
pub struct TextEditUi {
    /// The editing session (its merge key) this state belongs to.
    session: Option<String>,
    /// The caret is shown at the end of its line where a wrapped line ends.
    upstream: bool,
    /// The x the caret keeps while moving up and down (box coordinates).
    column: Option<f64>,
    undo: UndoStack,
    /// IME composition shown in the text (characters).
    preedit: Option<Range<usize>>,
    /// A drag selecting text: its unit (1 letters, 2 words, 3 paragraphs) and starting range.
    drag: Option<(u8, Range<usize>)>,
    /// The last press: time, screen position, click count.
    last_press: Option<(f64, egui::Pos2, u8)>,
    /// When the caret last moved (it shows, then blinks).
    blink_from: f64,
    /// The caret map and the document and element it was made for.
    map: Option<(Arc<Document>, NodeId, Arc<CaretMap>)>,
    /// The canvas took keyboard focus for editing.
    focused: bool,
}

/// How the canvas maps page points to the screen and back.
pub struct View<'a> {
    pub to_screen: &'a dyn Fn(Point) -> egui::Pos2,
    pub to_page: &'a dyn Fn(egui::Pos2) -> Point,
    pub zoom: f32,
}

/// The text element `id` on `page` of `doc`: its transform (box → page), box width and height,
/// and the text element.
fn element(doc: &Document, page: usize, id: NodeId) -> Option<(Affine, f64, f64, TextEl)> {
    let el = doc.pages.get(page)?.find(id)?;
    let ElementKind::Text(t) = &el.kind else { return None };
    let f = el.frame.sanitized();
    Some((genha_render::element_transform(el), f.width, f.height, t.clone()))
}

fn caret_map(app: &mut App, doc: &Arc<Document>, page: usize, id: NodeId) -> Option<Arc<CaretMap>> {
    if let Some((d, i, m)) = &app.canvas.text.map
        && Arc::ptr_eq(d, doc)
        && *i == id
    {
        return Some(Arc::clone(m));
    }
    let (_, w, _, t) = element(doc, page, id)?;
    let m = Arc::new(CaretMap::for_el(&app.session.fonts, &t, w));
    app.canvas.text.map = Some((Arc::clone(doc), id, Arc::clone(&m)));
    Some(m)
}

/// Starts editing text box `id` after a double-click at `at` (page) / `screen`: the word there
/// is selected, and a third click selects its paragraph.
pub fn begin_at(app: &mut App, doc: &Arc<Document>, page: usize, id: NodeId, at: Point, screen: egui::Pos2, now: f64) {
    let Some((m, _, _, t)) = element(doc, page, id) else { return };
    let Some(map) = caret_map(app, doc, page, id) else { return };
    let local = m.inverse() * at;
    let (i, _) = map.hit(local.x, local.y);
    let w = edit::word_at(&t.text, i);
    if app.run("text.beginEdit", &json!({"id": id.0, "selection": [w.start, w.end]})).is_some() {
        app.canvas.text.last_press = Some((now, screen, 2));
    }
}

/// Draws and drives in-place editing. True when it took this frame's pointer input (the tools
/// must not see it).
pub fn show(app: &mut App, ui: &egui::Ui, resp: &egui::Response, doc: &Arc<Document>, page: usize, view: &View, t: &Tokens) -> bool {
    let Some(e) = app.session.text_editing.clone() else {
        if app.canvas.text.focused {
            resp.surrender_focus();
        }
        app.canvas.text = TextEditUi { last_press: app.canvas.text.last_press, ..TextEditUi::default() };
        return false;
    };
    let now = ui.input(|i| i.time);
    let st = &mut app.canvas.text;
    if st.session.as_deref() != Some(e.merge_key.as_str()) {
        *st = TextEditUi { session: Some(e.merge_key.clone()), blink_from: now, last_press: st.last_press, ..TextEditUi::default() };
    }
    let Some((m, w, h, el)) = element(doc, page, e.id) else { return false };
    let text = el.text.clone();
    let Some(map) = caret_map(app, doc, page, e.id) else { return false };

    // Keyboard focus: the canvas takes it; when another widget takes it, editing ends.
    if !resp.has_focus() {
        let other = ui.memory(|mem| mem.focused().is_some_and(|f| f != resp.id));
        if app.canvas.text.focused && other {
            app.run("text.endEdit", &json!({}));
            app.canvas.text.focused = false;
            return false;
        }
        resp.request_focus();
        app.canvas.text.focused = true;
    }
    let filter = egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true };
    ui.memory_mut(|mem| mem.set_focus_lock_filter(resp.id, filter));

    let mut buf = EditBuffer::new(text.clone(), e.anchor, e.caret);
    let start = buf.clone();
    let inv = m.inverse();
    let consumed = pointer(app, ui, resp, &map, &mut buf, inv, (w, h), view, now);
    if app.session.text_editing.is_none() {
        return consumed;
    }
    let mut end = false;
    let mut toggles: Vec<(&'static str, EditBuffer)> = vec![];
    if resp.has_focus() {
        end = keyboard(app, ui, &map, &el, w, &mut buf, &mut toggles);
    }
    // The edits, in order with the formatting shortcuts (Cmd/Ctrl+B, I, U) pressed between them:
    // each shortcut acts on the text and selection as they were when it was pressed.
    let mut sent = start.clone();
    let steps = toggles.into_iter().map(|(cmd, at)| (Some(cmd), at)).chain(std::iter::once((None, buf.clone())));
    for (cmd, at) in steps {
        if at != sent {
            app.canvas.text.blink_from = now;
            let mut p = json!({"id": e.id.0, "selection": [at.anchor, at.caret]});
            if at.text != sent.text {
                p["text"] = json!(at.text);
            }
            app.run("text.edit", &p);
            ui.ctx().request_repaint();
            sent = at;
        }
        if let Some(cmd) = cmd {
            app.run(cmd, &json!({}));
            ui.ctx().request_repaint();
        }
    }
    if end {
        app.canvas.text.preedit = None;
        app.run("text.endEdit", &json!({}));
        resp.surrender_focus();
        app.canvas.text.focused = false;
        return consumed;
    }
    // Drawn from the state this frame's page picture shows (changes show next frame).
    draw(app, ui, &map, &start, m, view, t, now);
    consumed
}

/// Presses, drags and multi-clicks while editing. Returns whether it took the pointer.
#[allow(clippy::too_many_arguments)]
fn pointer(
    app: &mut App,
    ui: &egui::Ui,
    resp: &egui::Response,
    map: &CaretMap,
    buf: &mut EditBuffer,
    inv: Affine,
    size: (f64, f64),
    view: &View,
    now: f64,
) -> bool {
    let (pos, pressed, down, shift) =
        ui.input(|i| (i.pointer.interact_pos(), i.pointer.primary_pressed(), i.pointer.primary_down(), i.modifiers.shift));
    let local = |p: egui::Pos2| inv * (view.to_page)(p);
    let grab = GRAB / f64::from(view.zoom.max(0.01));
    let inside = |l: Point| {
        let r = Rect::new(0.0, 0.0, size.0, size.1).inflate(grab, grab);
        r.contains(l) || map.bounds().is_some_and(|b| b.inflate(grab, grab).contains(l))
    };
    if let Some(p) = resp.hover_pos()
        && inside(local(p))
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    let st = &mut app.canvas.text;
    if pressed && let Some(p) = pos {
        if !resp.hovered() {
            return false;
        }
        let l = local(p);
        if !inside(l) {
            // A press elsewhere ends editing; the tools then handle it as usual.
            st.drag = None;
            app.run("text.endEdit", &json!({}));
            return false;
        }
        let count = match st.last_press {
            Some((t, at, n)) if now - t < MULTI_CLICK_TIME && at.distance(p) < MULTI_CLICK_DIST => n % 3 + 1,
            _ => 1,
        };
        st.last_press = Some((now, p, count));
        let (i, up) = map.hit(l.x, l.y);
        st.upstream = up;
        st.column = None;
        st.preedit = None;
        st.undo.break_run();
        let unit = match count {
            2 => edit::word_at(&buf.text, i),
            3 => edit::paragraph_at(&buf.text, i),
            _ if shift => buf.anchor..buf.anchor,
            _ => i..i,
        };
        if count == 1 {
            buf.move_to(i, shift);
        } else {
            buf.select(unit.start, unit.end);
        }
        st.drag = Some((count, unit));
        return true;
    }
    let Some((unit, origin)) = st.drag.clone() else { return false };
    if !down {
        st.drag = None;
        return true;
    }
    if let Some(p) = pos {
        let l = local(p);
        let (i, up) = map.hit(l.x, l.y);
        let r = match unit {
            2 => edit::word_at(&buf.text, i),
            3 => edit::paragraph_at(&buf.text, i),
            _ => i..i,
        };
        if r.start < origin.start {
            buf.select(origin.end, r.start);
        } else {
            buf.select(origin.start, r.end.max(origin.end));
        }
        st.upstream = up;
    }
    true
}

/// Typing, deleting, moving, clipboard and IME; formatting shortcuts are collected in `toggles`,
/// each with the text and selection it applies to. Returns true when editing should end.
fn keyboard(
    app: &mut App,
    ui: &egui::Ui,
    map: &CaretMap,
    el: &TextEl,
    width: f64,
    buf: &mut EditBuffer,
    toggles: &mut Vec<(&'static str, EditBuffer)>,
) -> bool {
    let events = ui.input(|i| i.events.clone());
    let fonts = Arc::clone(&app.session.fonts);
    let st = &mut app.canvas.text;
    // The text laid out again when keys arrive after it changed within one frame.
    let mut relaid: Option<(String, CaretMap)> = None;
    let original = buf.text.clone();
    for ev in &events {
        match ev {
            egui::Event::Text(s) => {
                if st.preedit.is_some() || s == "\n" || s == "\r" {
                    continue;
                }
                type_text(st, buf, s);
            }
            egui::Event::Paste(s) => {
                st.undo.record(buf, EditKind::Other);
                buf.insert(s);
                after_edit(st);
            }
            egui::Event::Copy => {
                if buf.has_selection() {
                    ui.ctx().copy_text(buf.selected().to_string());
                }
            }
            egui::Event::Cut => {
                if buf.has_selection() {
                    ui.ctx().copy_text(buf.selected().to_string());
                    st.undo.record(buf, EditKind::Other);
                    buf.insert("");
                    after_edit(st);
                }
            }
            egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                if (text.is_empty() && st.preedit.is_none()) || text == "\n" || text == "\r" {
                    continue;
                }
                let r = match st.preedit.take() {
                    Some(r) => r,
                    None => {
                        st.undo.record(buf, EditKind::Other);
                        buf.range()
                    }
                };
                buf.select(r.start, r.end);
                buf.insert(text);
                if !text.is_empty() {
                    st.preedit = Some(r.start..r.start + edit::char_len(text));
                }
            }
            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                if (text.is_empty() && st.preedit.is_none()) || text == "\n" || text == "\r" {
                    continue;
                }
                let r = match st.preedit.take() {
                    Some(r) => r,
                    None => {
                        st.undo.record(buf, EditKind::Typing);
                        buf.range()
                    }
                };
                buf.select(r.start, r.end);
                buf.insert(text);
                after_edit(st);
            }
            egui::Event::Ime(egui::ImeEvent::DeleteSurrounding { before_chars, after_chars }) => {
                let c = buf.caret;
                st.undo.record(buf, EditKind::Deleting);
                buf.select(c.saturating_sub(*before_chars), c.saturating_add(*after_chars));
                buf.insert("");
            }
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                if st.preedit.is_some() {
                    continue;
                }
                let m = if buf.text == original {
                    map
                } else {
                    if relaid.as_ref().is_none_or(|(t, _)| *t != buf.text) {
                        // The style runs moved with the text, as the engine will move them.
                        let mut t = el.clone();
                        t.set_text_keeping_styles(&buf.text, Some(buf.caret), None);
                        relaid = Some((buf.text.clone(), CaretMap::for_el(&fonts, &t, width)));
                    }
                    relaid.as_ref().map_or(map, |(_, m)| m)
                };
                if modifiers.command && !modifiers.shift && !modifiers.alt {
                    let toggle = match key {
                        egui::Key::B => Some("text.toggleBold"),
                        egui::Key::I => Some("text.toggleItalic"),
                        egui::Key::U => Some("text.toggleUnderline"),
                        _ => None,
                    };
                    if let Some(cmd) = toggle {
                        toggles.push((cmd, buf.clone()));
                        continue;
                    }
                }
                if key_press(ui, st, buf, m, *key, *modifiers) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn after_edit(st: &mut TextEditUi) {
    st.upstream = false;
    st.column = None;
}

fn type_text(st: &mut TextEditUi, buf: &mut EditBuffer, s: &str) {
    st.undo.record(buf, EditKind::Typing);
    if buf.insert(s) && s.chars().any(char::is_whitespace) {
        // Each word is its own undo step.
        st.undo.break_run();
    }
    after_edit(st);
}

/// One key press. Returns true when it ends editing.
fn key_press(ui: &egui::Ui, st: &mut TextEditUi, buf: &mut EditBuffer, map: &CaretMap, key: egui::Key, mods: egui::Modifiers) -> bool {
    use egui::Key;
    // Ctrl (Option on a Mac) moves and deletes by words; Cmd on a Mac jumps to line ends.
    let word = mods.ctrl || mods.alt;
    let line = mods.mac_cmd;
    let shift = mods.shift;
    let moved = |st: &mut TextEditUi| {
        st.undo.break_run();
        st.column = None;
    };
    match key {
        Key::Escape => return true,
        Key::Enter if mods.command => return true,
        Key::Enter => {
            st.undo.record(buf, EditKind::Other);
            buf.insert("\n");
            st.undo.break_run();
            after_edit(st);
        }
        Key::Backspace => {
            st.undo.record(buf, EditKind::Deleting);
            buf.backspace(word);
            after_edit(st);
        }
        Key::Delete => {
            st.undo.record(buf, EditKind::Deleting);
            buf.delete_forward(word);
            after_edit(st);
        }
        Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End if line || matches!(key, Key::Home | Key::End) => {
            moved(st);
            let to_end = matches!(key, Key::ArrowRight | Key::End);
            if mods.ctrl && !line {
                buf.move_to(if to_end { buf.len() } else { 0 }, shift);
                st.upstream = false;
            } else if to_end {
                let (i, up) = map.line_end(buf.caret, st.upstream);
                buf.move_to(i, shift);
                st.upstream = up;
            } else {
                buf.move_to(map.line_start(buf.caret, st.upstream), shift);
                st.upstream = false;
            }
        }
        Key::ArrowLeft => {
            moved(st);
            buf.left(word, shift);
            st.upstream = false;
        }
        Key::ArrowRight => {
            moved(st);
            buf.right(word, shift);
            st.upstream = false;
        }
        Key::ArrowUp | Key::ArrowDown => {
            st.undo.break_run();
            let down = key == Key::ArrowDown;
            if line {
                buf.move_to(if down { buf.len() } else { 0 }, shift);
                st.upstream = false;
                st.column = None;
            } else {
                let x = st.column.or_else(|| map.caret(buf.caret, st.upstream).map(|c| c.x)).unwrap_or(0.0);
                let (i, up) = map.vertical(buf.caret, st.upstream, if down { 1 } else { -1 }, x);
                buf.move_to(i, shift);
                st.upstream = up;
                st.column = Some(x);
            }
        }
        Key::A if mods.command => {
            moved(st);
            buf.select_all();
        }
        Key::Z if mods.command && shift => redo(st, buf),
        Key::Y if mods.command => redo(st, buf),
        Key::Z if mods.command => {
            if let Some(prev) = st.undo.undo(buf) {
                *buf = prev;
                after_edit(st);
            }
        }
        _ => {}
    }
    ui.ctx().request_repaint();
    false
}

fn redo(st: &mut TextEditUi, buf: &mut EditBuffer) {
    if let Some(next) = st.undo.redo(buf) {
        *buf = next;
        after_edit(st);
    }
}

/// Selection boxes, IME underline and the blinking caret; tells the platform where the caret is
/// (IME candidate windows).
#[allow(clippy::too_many_arguments)]
fn draw(app: &App, ui: &egui::Ui, map: &CaretMap, buf: &EditBuffer, m: Affine, view: &View, t: &Tokens, now: f64) {
    let painter = ui.painter();
    let quad = |r: Rect| -> Vec<egui::Pos2> {
        [Point::new(r.x0, r.y0), Point::new(r.x1, r.y0), Point::new(r.x1, r.y1), Point::new(r.x0, r.y1)].map(|p| (view.to_screen)(m * p)).to_vec()
    };
    let fill = t.selection.gamma_multiply(0.35);
    for r in map.selection_rects(buf.anchor, buf.caret) {
        painter.add(egui::Shape::convex_polygon(quad(r), fill, egui::Stroke::NONE));
    }
    let st = &app.canvas.text;
    if let Some(pre) = &st.preedit {
        for r in map.selection_rects(pre.start, pre.end) {
            let q = quad(Rect::new(r.x0, r.y1 - r.height() * 0.08, r.x1, r.y1));
            painter.add(egui::Shape::convex_polygon(q, t.selection, egui::Stroke::NONE));
        }
    }
    let Some(c) = map.caret(buf.caret, st.upstream) else { return };
    let (top, bottom) = ((view.to_screen)(m * Point::new(c.x, c.top)), (view.to_screen)(m * Point::new(c.x, c.top + c.height)));
    let phase = ((now - st.blink_from).max(0.0) / BLINK).floor();
    if !buf.has_selection() && phase % 2.0 == 0.0 {
        painter.line_segment([top, bottom], egui::Stroke::new(2.0, t.selection));
    }
    if !buf.has_selection() {
        let next = BLINK - (now - st.blink_from).max(0.0) % BLINK;
        ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(next.clamp(0.01, BLINK)));
    }
    let frame = egui::Rect::from_points(&quad(map.bounds().unwrap_or_default()));
    let cursor = egui::Rect::from_two_pos(top, bottom).expand2(egui::vec2(1.0, 0.0));
    ui.ctx().output_mut(|o| {
        o.ime = Some(egui::output::IMEOutput {
            purpose: egui::IMEPurpose::Normal,
            rect: frame.union(cursor),
            cursor_rect: cursor,
            should_interrupt_composition: false,
        });
    });
}
