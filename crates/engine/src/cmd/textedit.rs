//! Editing text in place: `text.beginEdit` enters text editing on a text box, `text.edit` types,
//! deletes or replaces text and moves the selection, `text.endEdit` leaves. The session keeps the
//! element being edited and its selection ([`TextEditing`]), so the canvas, the control channel
//! and MCP all see and drive the same state. Every change made while editing one box merges into a
//! single undo step.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use super::{bad, cmd, has_doc, ok, str_param};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Element, ElementKind, NodeId, SpanStyle, TextEl};
use genha_geom::{Frame, Point};
use genha_text::edit::{self, MAX_EDIT_BYTES};

/// The text box being edited in place and its selection.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEditing {
    pub id: NodeId,
    /// Selection in characters: where it started, and where the caret is.
    pub anchor: usize,
    pub caret: usize,
    /// Merges this session's changes into one undo step.
    pub merge_key: String,
    /// The document being edited (`DocState::recovery_id`).
    pub doc: u64,
    /// Style changes made with nothing selected (Bold with the caret in a word): the next text
    /// typed takes them. Cleared when the caret moves.
    pub typing_style: Option<SpanStyle>,
}

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "text.beginEdit",
            "Edit Text in Place",
            ["Edit"],
            Some("Enter"),
            "{id?, selection?: [anchor, caret] in characters (default: all)} — edit a text box on the canvas (default: the selected one)",
            has_doc,
            begin_edit
        ),
        cmd!(
            "text.edit",
            "Type Text",
            [],
            None,
            "{id?, text?: the whole new text, insert?: text replacing `range` (default: the selection while editing, else the end), range?: [start, end] in characters, selection?: [anchor, caret]} — one undo step per editing session; the box height follows the text",
            has_doc,
            edit_text
        ),
        cmd!("text.endEdit", "Finish Editing Text", [], None, "{} — leave text editing (an empty text box is removed)", has_doc, end_edit),
    ]
}

/// A `[a, b]` pair of character indices.
pub(crate) fn pair(p: &Value, key: &str, cmd: &str) -> Result<Option<(usize, usize)>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 2 => {
            let n = |v: &Value| v.as_u64().and_then(|v| usize::try_from(v).ok());
            match (a.first().and_then(n), a.get(1).and_then(n)) {
                (Some(x), Some(y)) => Ok(Some((x, y))),
                _ => Err(bad(cmd, format!("`{key}` must be two character indices"))),
            }
        }
        Some(_) => Err(bad(cmd, format!("`{key}` must be [start, end] character indices"))),
    }
}

/// The text box a command acts on: `id`, else the one being edited, else the selected text box.
pub(crate) fn target(s: &Session, p: &Value, cmd: &str) -> Result<NodeId> {
    let d = s.doc()?;
    let id = match p.get("id") {
        Some(Value::Null) | None => match &s.text_editing {
            Some(e) => e.id,
            None => {
                let texts: Vec<NodeId> = d.selected_elements().iter().filter(|e| matches!(e.kind, ElementKind::Text(_))).map(|e| e.id).collect();
                match texts.as_slice() {
                    [one] => *one,
                    [] => return Err(bad(cmd, "select a text box (or pass `id`)")),
                    _ => return Err(bad(cmd, "several text boxes are selected: pass `id`")),
                }
            }
        },
        Some(v) => NodeId(v.as_u64().ok_or_else(|| bad(cmd, "`id` must be an object id"))?),
    };
    let (_, el) = d.doc.find(id).ok_or(EngineError::NoNode(id))?;
    if !matches!(el.kind, ElementKind::Text(_)) {
        return Err(bad(cmd, format!("object {id} is not a text box")));
    }
    if el.locked {
        return Err(EngineError::Other("the text box is locked".into()));
    }
    Ok(id)
}

fn text_of(s: &Session, id: NodeId) -> Result<String> {
    match s.doc()?.doc.find(id) {
        Some((_, Element { kind: ElementKind::Text(t), .. })) => Ok(t.text.clone()),
        _ => Err(EngineError::NoNode(id)),
    }
}

/// The text element `id` (None when it is gone or not text).
pub(crate) fn text_el(s: &Session, id: NodeId) -> Option<TextEl> {
    match s.doc().ok()?.doc.find(id) {
        Some((_, Element { kind: ElementKind::Text(t), .. })) => Some(t.clone()),
        _ => None,
    }
}

fn editing_json(e: Option<&TextEditing>) -> Value {
    match e {
        Some(e) => json!({"id": e.id, "selection": [e.anchor, e.caret]}),
        None => Value::Null,
    }
}

fn begin_edit(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "text.beginEdit";
    let id = target(s, p, cmd)?;
    let on_page = s.doc()?.doc.pages.get(s.doc()?.page).is_some_and(|pg| pg.find(id).is_some());
    if !on_page {
        return Err(bad(cmd, format!("object {id} is not on the current page")));
    }
    if s.text_editing.as_ref().is_some_and(|e| e.id != id) {
        end_edit(s, &json!({}))?;
    }
    let text = text_of(s, id)?;
    let n = edit::char_len(&text);
    let (anchor, caret) = pair(p, "selection", cmd)?.unwrap_or((0, n));
    static SERIAL: AtomicU64 = AtomicU64::new(1);
    let merge_key = match &s.text_editing {
        Some(e) if e.id == id => e.merge_key.clone(),
        _ => format!("text-edit:{}:{}", id.0, SERIAL.fetch_add(1, Ordering::Relaxed)),
    };
    let doc = s.doc()?.recovery_id;
    s.set_selection(vec![id])?;
    s.tools.cancel();
    s.preview = None;
    s.text_editing = Some(TextEditing { id, anchor: anchor.min(n), caret: caret.min(n), merge_key, doc, typing_style: None });
    Ok(ok(json!({"id": id, "text": text, "editing": editing_json(s.text_editing.as_ref())})))
}

fn edit_text(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "text.edit";
    let id = target(s, p, cmd)?;
    let old = text_of(s, id)?;
    let editing = s.text_editing.clone().filter(|e| e.id == id);
    let n_old = edit::char_len(&old);
    let (sel_a, sel_c) = editing.as_ref().map_or((n_old, n_old), |e| (e.anchor.min(n_old), e.caret.min(n_old)));
    // The characters replaced and their replacement, when known (else the change is found by
    // comparing the old and new text).
    let mut replaced: Option<(std::ops::Range<usize>, String)> = None;
    // The new text and where the caret lands.
    let (new, caret) = match (str_param(p, "text"), str_param(p, "insert")) {
        (Some(_), Some(_)) => return Err(bad(cmd, "pass `text` or `insert`, not both")),
        (Some(t), None) => (t.to_string(), None),
        (None, Some(ins)) => {
            let (a, b) = pair(p, "range", cmd)?.unwrap_or((sel_a, sel_c));
            let mut buf = edit::EditBuffer::new(old.clone(), a, b);
            let range = buf.range();
            if !buf.insert(ins) && !ins.is_empty() {
                return Err(bad(cmd, format!("text longer than {MAX_EDIT_BYTES} bytes")));
            }
            replaced = Some((range, edit::clean_input(ins)));
            let c = buf.caret;
            (buf.text, Some(c))
        }
        (None, None) => {
            if p.get("selection").is_none_or(Value::is_null) {
                return Err(bad(cmd, "pass `text`, `insert` or `selection`"));
            }
            (old.clone(), None)
        }
    };
    if new.len() > MAX_EDIT_BYTES {
        return Err(bad(cmd, format!("text longer than {MAX_EDIT_BYTES} bytes")));
    }
    let n = edit::char_len(&new);
    let selection = match pair(p, "selection", cmd)? {
        Some((a, c)) => (a.min(n), c.min(n)),
        None => {
            let c = caret.unwrap_or(if new == old { sel_c.min(n) } else { n });
            (c, c)
        }
    };
    let changed = new != old;
    // Text typed after a style change with nothing selected takes that style.
    let typing = editing.as_ref().and_then(|e| e.typing_style.clone()).map(|t| {
        let at = replaced.as_ref().map_or(sel_c.min(sel_a), |(r, _)| r.start);
        text_el(s, id).map_or(t.clone(), |el| el.caret_style(at).merged(&t))
    });
    if changed {
        // While editing, every change joins the session's undo step.
        if let Some(e) = &editing
            && s.merge_key.is_none()
        {
            s.merge_key = Some(e.merge_key.clone());
        }
        let fonts = Arc::clone(&s.fonts);
        let hint = pair(p, "selection", cmd)?.map(|(_, c)| c).or(caret);
        s.edit("Edit Text", |doc, _| {
            let el = doc.element_mut(id)?;
            let ElementKind::Text(t) = &mut el.kind else { return Err(EngineError::NoNode(id)) };
            // The style runs move with the text.
            match &replaced {
                Some((range, ins)) => t.replace_range(range.clone(), ins, typing.clone()),
                None => t.set_text_keeping_styles(&new, hint, typing.clone()),
            }
            if t.text != new {
                t.text = new.clone();
                t.normalize_spans();
            }
            let t = t.clone();
            el.frame = fitted_frame(&fonts, &t, el.frame);
            Ok(())
        })?;
    }
    if let Some(e) = s.text_editing.as_mut().filter(|e| e.id == id) {
        // Typing used the pending style; moving the caret drops it.
        if changed || (e.anchor, e.caret) != selection {
            e.typing_style = None;
        }
        e.anchor = selection.0;
        e.caret = selection.1;
    }
    Ok(ok(json!({"id": id, "text": new, "changed": changed, "editing": editing_json(s.text_editing.as_ref())})))
}

/// `frame` with its height fitted to `t`: straight text keeps its top edge (its top-left corner
/// on the page when turned), curved text its centre.
pub(crate) fn fitted_frame(fonts: &genha_text::FontBook, t: &TextEl, frame: Frame) -> Frame {
    let f = frame.sanitized();
    if t.curve_degrees() != 0.0 {
        return match genha_render::curved_text_size(fonts, t, f.width) {
            Some((_, h)) if h.is_finite() && h > 0.0 => {
                let h = h.ceil();
                Frame { y: f.y + (f.height - h) / 2.0, height: h, ..f }
            }
            _ => f,
        };
    }
    let h = if fonts.is_empty() && t.spans.is_empty() {
        super::create::estimate_text_size(&t.text, &t.style, f.width).1
    } else {
        genha_text::layout_el(fonts, t, f.width).height.max(t.style.font_size)
    };
    if !h.is_finite() || h <= 0.0 || (h - f.height).abs() < 1e-9 {
        return f;
    }
    let corner = f.to_page() * Point::ORIGIN;
    let mut g = Frame { height: h, ..f };
    let moved = g.to_page() * Point::ORIGIN;
    g.x += corner.x - moved.x;
    g.y += corner.y - moved.y;
    g
}

fn end_edit(s: &mut Session, _: &Value) -> Result<Value> {
    let Some(e) = s.text_editing.take() else { return Ok(ok(json!({"editing": null}))) };
    let empty = text_of(s, e.id).is_ok_and(|t| t.trim().is_empty());
    let mut removed = false;
    if empty {
        // Removing the empty box joins the editing session's undo step.
        let saved = s.merge_key.replace(e.merge_key.clone());
        let r = s.edit("Edit Text", |doc, cx| {
            doc.remove(e.id).ok_or(EngineError::NoNode(e.id))?;
            cx.selection.retain(|i| *i != e.id);
            Ok(())
        });
        s.merge_key = saved;
        removed = r.is_ok();
    }
    Ok(ok(json!({"id": e.id, "removed": removed, "editing": null})))
}

/// Drops the editing state when its text box is gone, not on the current page, no longer
/// selected or locked, or another document is active; keeps the selection inside the text.
pub(crate) fn check(s: &mut Session) {
    let Some(e) = s.text_editing.clone() else { return };
    let len = s.active().filter(|d| d.recovery_id == e.doc && d.selection.contains(&e.id)).and_then(|d| {
        let el = d.doc.pages.get(d.page)?.find(e.id)?;
        match &el.kind {
            ElementKind::Text(t) if !el.locked => Some(edit::char_len(&t.text)),
            _ => None,
        }
    });
    match len {
        Some(n) => {
            if let Some(cur) = s.text_editing.as_mut() {
                cur.anchor = cur.anchor.min(n);
                cur.caret = cur.caret.min(n);
            }
        }
        None => s.text_editing = None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Session;
    use genha_doc::{ElementKind, NodeId};
    use genha_geom::Point;
    use genha_tools::{PointerEvent, PointerKind, ToolId};

    fn session() -> Session {
        let mut s = Session::new();
        s.execute("file.new", &json!({"preset": "instagram-post"})).unwrap();
        s
    }

    fn text(s: &Session, id: u64) -> String {
        match &s.doc().unwrap().doc.element(NodeId(id)).unwrap().kind {
            ElementKind::Text(t) => t.text.clone(),
            _ => panic!("not text"),
        }
    }

    fn selection(s: &Session) -> (usize, usize) {
        let e = s.text_editing.as_ref().unwrap();
        (e.anchor, e.caret)
    }

    #[test]
    fn an_editing_session_is_one_undo_step() {
        let mut s = session();
        let id = s.execute("text.add", &json!({"text": "Hello"})).unwrap()["id"].as_u64().unwrap();
        let steps = s.doc().unwrap().history.undo.len();
        let r = s.execute("text.beginEdit", &json!({})).unwrap();
        assert_eq!(r["editing"]["selection"], json!([0, 5]));
        assert_eq!(selection(&s), (0, 5));
        // Typing replaces the selection, then goes on at the caret.
        s.execute("text.edit", &json!({"insert": "Hi"})).unwrap();
        s.execute("text.edit", &json!({"insert": " there"})).unwrap();
        assert_eq!(text(&s, id), "Hi there");
        assert_eq!(selection(&s), (8, 8));
        // Moving the selection is not an undo step.
        s.execute("text.edit", &json!({"selection": [0, 2]})).unwrap();
        s.execute("text.edit", &json!({"insert": "Yo"})).unwrap();
        s.execute("text.edit", &json!({"text": "Yo there!", "selection": [9, 9]})).unwrap();
        assert_eq!(text(&s, id), "Yo there!");
        assert_eq!(s.doc().unwrap().history.undo.len(), steps + 1);
        let r = s.execute("session.inspect", &json!({})).unwrap();
        assert_eq!(r["text_editing"]["selection"], json!([9, 9]));
        s.execute("text.endEdit", &json!({})).unwrap();
        assert!(s.text_editing.is_none());
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(text(&s, id), "Hello");
        // A second session is a second step.
        s.execute("edit.redo", &json!({})).unwrap();
        s.execute("text.beginEdit", &json!({"id": id, "selection": [99, 0]})).unwrap();
        assert_eq!(selection(&s), (9, 0));
        s.execute("text.edit", &json!({"insert": ">"})).unwrap();
        assert_eq!(text(&s, id), ">");
        assert_eq!(s.doc().unwrap().history.undo.len(), steps + 2);
    }

    #[test]
    fn agents_edit_ranges_without_a_session() {
        let mut s = session();
        let id = s.execute("text.add", &json!({"text": "Grand opening"})).unwrap()["id"].as_u64().unwrap();
        s.execute("text.edit", &json!({"id": id, "insert": "Big", "range": [0, 5]})).unwrap();
        assert_eq!(text(&s, id), "Big opening");
        s.execute("text.edit", &json!({"id": id, "insert": "!"})).unwrap();
        assert_eq!(text(&s, id), "Big opening!");
        assert!(s.text_editing.is_none());
        // Bad requests are refused.
        assert!(s.execute("text.edit", &json!({"id": id})).is_err());
        assert!(s.execute("text.edit", &json!({"id": id, "text": "a", "insert": "b"})).is_err());
        assert!(s.execute("text.edit", &json!({"id": id, "insert": "x", "range": [1]})).is_err());
        assert!(s.execute("text.edit", &json!({"id": id, "text": "x".repeat(100_001)})).is_err());
        let shape = s.execute("shape.add", &json!({})).unwrap()["id"].as_u64().unwrap();
        assert!(s.execute("text.edit", &json!({"id": shape, "text": "x"})).is_err());
        assert!(s.execute("text.beginEdit", &json!({})).is_err(), "a shape is selected");
        s.execute("object.lock", &json!({"ids": [id]})).unwrap();
        assert!(s.execute("text.beginEdit", &json!({"id": id})).is_err(), "locked");
    }

    #[test]
    fn the_box_grows_with_the_text_and_keeps_its_corner() {
        let mut s = session();
        let id = s.execute("text.add", &json!({"text": "One", "x": 100, "y": 100, "width": 400})).unwrap()["id"].as_u64().unwrap();
        s.execute("object.setFrame", &json!({"id": id, "rotation": 30})).unwrap();
        let before = s.doc().unwrap().doc.element(NodeId(id)).unwrap().frame;
        let corner = before.to_page() * Point::ORIGIN;
        s.execute("text.beginEdit", &json!({"id": id})).unwrap();
        s.execute("text.edit", &json!({"text": "One\nTwo\nThree"})).unwrap();
        let after = s.doc().unwrap().doc.element(NodeId(id)).unwrap().frame;
        assert!(after.height > before.height * 2.5, "{} → {}", before.height, after.height);
        assert_eq!(after.width, before.width);
        let moved = after.to_page() * Point::ORIGIN;
        assert!((moved - corner).hypot() < 1e-6);
    }

    #[test]
    fn leaving_an_empty_box_removes_it() {
        let mut s = session();
        let id = s.execute("text.add", &json!({"text": "Gone"})).unwrap()["id"].as_u64().unwrap();
        let steps = s.doc().unwrap().history.undo.len();
        s.execute("text.beginEdit", &json!({})).unwrap();
        s.execute("text.edit", &json!({"insert": ""})).unwrap();
        let r = s.execute("text.endEdit", &json!({})).unwrap();
        assert_eq!(r["removed"], json!(true));
        assert!(s.doc().unwrap().doc.find(NodeId(id)).is_none());
        assert_eq!(s.doc().unwrap().history.undo.len(), steps + 1);
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(text(&s, id), "Gone");
        // Leaving when not editing is harmless.
        assert!(s.execute("text.endEdit", &json!({})).is_ok());
    }

    #[test]
    fn editing_ends_when_its_box_goes_away() {
        let mut s = session();
        let id = s.execute("text.add", &json!({"text": "Here"})).unwrap()["id"].as_u64().unwrap();
        s.execute("text.beginEdit", &json!({})).unwrap();
        s.execute("page.add", &json!({})).unwrap();
        assert!(s.text_editing.is_none(), "another page");
        s.execute("page.goto", &json!({"index": 0})).unwrap();
        s.execute("text.beginEdit", &json!({"id": id})).unwrap();
        let other = s.execute("shape.add", &json!({})).unwrap()["id"].as_u64().unwrap();
        assert!(s.text_editing.is_none(), "another object selected");
        s.execute("text.beginEdit", &json!({"id": id})).unwrap();
        s.execute("select.set", &json!({"ids": [other]})).unwrap();
        assert!(s.text_editing.is_none());
        s.execute("text.beginEdit", &json!({"id": id})).unwrap();
        s.execute("edit.delete", &json!({"ids": [id]})).unwrap();
        assert!(s.text_editing.is_none());
    }

    #[test]
    fn the_text_tool_starts_editing_and_curves_show_straight() {
        let mut s = session();
        s.tools.set_tool(ToolId::Text);
        let ev = PointerEvent { kind: PointerKind::Down, pos: Point::new(300.0, 300.0), shift: false, alt: false };
        s.tool_event(ev, 1.0, (&[], &[])).unwrap();
        let e = s.text_editing.clone().unwrap();
        let placeholder = text(&s, e.id.0);
        assert_eq!((e.anchor, e.caret), (0, placeholder.chars().count()));
        s.execute("text.endEdit", &json!({})).unwrap();
        // Curved text is drawn straight while it is edited; the design keeps its curve.
        s.execute("text.setCurve", &json!({"ids": [e.id.0], "degrees": 120})).unwrap();
        s.execute("text.beginEdit", &json!({"id": e.id.0})).unwrap();
        let shown = s.display_document().unwrap();
        let curve = |d: &genha_doc::Document| match &d.element(e.id).unwrap().kind {
            ElementKind::Text(t) => t.curve,
            _ => f64::NAN,
        };
        assert_eq!(curve(&shown), 0.0);
        assert_eq!(curve(&s.doc().unwrap().doc), 120.0);
        assert!(std::sync::Arc::ptr_eq(&shown, &s.display_document().unwrap()), "cached");
        s.execute("text.endEdit", &json!({})).unwrap();
        assert_eq!(curve(&s.display_document().unwrap()), 120.0);
    }
}
