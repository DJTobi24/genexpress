//! Formatting part of a text box: `text.setStyle` on a character range or the text selected
//! while editing, Bold / Italic / Underline / Strikethrough toggles, and `text.inspect` (the
//! box's style runs and the style at the selection).
//!
//! Where a style change goes: an explicit `range` of characters; else, while a box is edited in
//! place, its selection (with nothing selected, the change waits for the next text typed: it
//! becomes [`TextEditing::typing_style`](crate::TextEditing::typing_style)); else the whole of
//! every selected text box (its style, with the runs' overrides of the changed values removed).
//! Paragraph-wide values (alignment, line height, letter spacing, upper case) always go to the
//! whole box.

use std::ops::Range;
use std::sync::Arc;

use serde_json::{Map, Value, json};

use super::textedit::{fitted_frame, pair, text_el};
use super::{bad, cmd, has_doc, merge, ok, targets};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Element, ElementKind, NodeId, RangeStyle, SPAN_FIELDS, SpanStyle, TextEl, TextStyle};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "text.toggleBold",
            "Bold",
            ["Text"],
            Some("Cmd+B"),
            "{ids?, range?: [start, end]} — the selected text while editing, else the whole box",
            has_text,
            toggle_bold
        ),
        cmd!(
            "text.toggleItalic",
            "Italic",
            ["Text"],
            Some("Cmd+I"),
            "{ids?, range?: [start, end]} — the selected text while editing, else the whole box",
            has_text,
            toggle_italic
        ),
        cmd!(
            "text.toggleUnderline",
            "Underline",
            ["Text"],
            Some("Cmd+U"),
            "{ids?, range?: [start, end]} — the selected text while editing, else the whole box",
            has_text,
            toggle_underline
        ),
        cmd!(
            "text.toggleStrikethrough",
            "Strikethrough",
            ["Text"],
            Some("Cmd+Shift+X"),
            "{ids?, range?: [start, end]} — the selected text while editing, else the whole box",
            has_text,
            toggle_strikethrough
        ),
        cmd!(
            query "text.inspect",
            "Inspect Text",
            [],
            None,
            "{id?} — a text box's style, its style runs ({start, end, text, style overrides}) and, while editing, the style at the selection (`mixed` lists values that vary)",
            has_doc,
            inspect
        ),
    ]
}

/// A text box is selected or being edited.
fn has_text(s: &Session) -> std::result::Result<(), String> {
    let d = s.active().ok_or("no document open")?;
    if s.text_editing.is_some() || d.selected_elements().iter().any(|e| is_text(e)) { Ok(()) } else { Err("select a text box".into()) }
}

fn is_text(e: &Element) -> bool {
    match &e.kind {
        ElementKind::Text(_) => true,
        ElementKind::Group(g) => g.children.iter().any(is_text),
        _ => false,
    }
}

/// Where a style change goes.
enum Target {
    /// Characters of one box.
    Range(NodeId, Range<usize>),
    /// The caret of the box being edited, nothing selected.
    Caret(NodeId, usize),
    /// Whole boxes (or groups holding them).
    Boxes(Vec<NodeId>),
}

fn target(s: &Session, p: &Value, cmd: &str) -> Result<Target> {
    if let Some((a, b)) = pair(p, "range", cmd)? {
        let ids = match p.get("ids") {
            Some(Value::Array(a)) if a.len() == 1 => a.first().and_then(Value::as_u64).map(NodeId),
            Some(Value::Array(_)) => return Err(bad(cmd, "`range` needs a single text box")),
            _ => None,
        };
        let id = match ids {
            Some(id) => super::textedit::target(s, &json!({"id": id.0}), cmd)?,
            None => super::textedit::target(s, p, cmd)?,
        };
        return Ok(Target::Range(id, a.min(b)..a.max(b)));
    }
    if let Some(e) = &s.text_editing {
        let named: Vec<u64> = match (p.get("ids"), p.get("id")) {
            (Some(Value::Array(a)), _) => a.iter().filter_map(Value::as_u64).collect(),
            (_, Some(v)) => v.as_u64().into_iter().collect(),
            _ => vec![],
        };
        if named.is_empty() || named == [e.id.0] {
            return Ok(if e.anchor == e.caret {
                Target::Caret(e.id, e.caret)
            } else {
                Target::Range(e.id, e.anchor.min(e.caret)..e.anchor.max(e.caret))
            });
        }
    }
    Ok(Target::Boxes(targets(s, p, cmd)?))
}

/// Splits a `text.setStyle` patch into the values a run can override and the paragraph-wide
/// rest, both validated.
fn split_patch(p: &Value, cmd: &str) -> Result<(SpanStyle, Map<String, Value>)> {
    let Some(obj) = p.as_object() else { return Err(bad(cmd, "expected an object of fields to change")) };
    let mut span = Map::new();
    let mut rest = Map::new();
    for (k, v) in obj {
        match k.as_str() {
            "ids" | "id" | "range" | "merge_key" => {}
            k if SPAN_FIELDS.contains(&k) => {
                span.insert(k.to_string(), v.clone());
            }
            _ => {
                rest.insert(k.clone(), v.clone());
            }
        }
    }
    let span: SpanStyle = serde_json::from_value(Value::Object(span)).map_err(|e| bad(cmd, e.to_string()))?;
    if span.font_size.is_some_and(|v| !v.is_finite() || !(1.0..=2000.0).contains(&v)) {
        return Err(bad(cmd, "font_size must be 1–2000"));
    }
    if span.font_family.as_ref().is_some_and(|f| f.trim().is_empty() || f.len() > 200) {
        return Err(bad(cmd, "font_family must be a family name"));
    }
    // The rest must make a valid style.
    let _: TextStyle = merge(&TextStyle::default(), &Value::Object(rest.clone()), cmd)?;
    Ok((span, rest))
}

/// `text.setStyle`: see the module docs.
pub(crate) fn set_style(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "text.setStyle";
    let (span, rest) = split_patch(p, cmd)?;
    apply(s, p, cmd, "Text Style", &span, &rest)
}

fn apply(s: &mut Session, p: &Value, cmd: &str, label: &str, span: &SpanStyle, rest: &Map<String, Value>) -> Result<Value> {
    let rest_v = Value::Object(rest.clone());
    let fonts = Arc::clone(&s.fonts);
    match target(s, p, cmd)? {
        Target::Range(id, range) => {
            s.edit(label, |doc, _| {
                let el = doc.element_mut(id)?;
                let ElementKind::Text(t) = &mut el.kind else { return Err(EngineError::NoNode(id)) };
                t.set_style_range(range.clone(), span);
                if !rest.is_empty() {
                    t.style = merge(&t.style, &rest_v, cmd)?;
                    t.normalize_spans();
                }
                let t = t.clone();
                el.frame = fitted_frame(&fonts, &t, el.frame);
                Ok(())
            })?;
            Ok(ok(json!({"changed": 1, "id": id, "range": [range.start, range.end]})))
        }
        Target::Caret(id, caret) => {
            if !rest.is_empty() {
                s.edit(label, |doc, _| {
                    let el = doc.element_mut(id)?;
                    let ElementKind::Text(t) = &mut el.kind else { return Err(EngineError::NoNode(id)) };
                    t.style = merge(&t.style, &rest_v, cmd)?;
                    t.normalize_spans();
                    let t = t.clone();
                    el.frame = fitted_frame(&fonts, &t, el.frame);
                    Ok(())
                })?;
            }
            // The next text typed takes the run values.
            if let Some(e) = s.text_editing.as_mut().filter(|_| !span.is_empty()) {
                e.typing_style = Some(e.typing_style.clone().unwrap_or_default().merged(span));
            }
            let typing = s.text_editing.as_ref().and_then(|e| e.typing_style.clone());
            Ok(ok(json!({"changed": usize::from(!rest.is_empty()), "id": id, "caret": caret, "typing_style": typing})))
        }
        Target::Boxes(ids) => {
            let n = s.edit(label, |doc, _| {
                let mut n = 0;
                for id in &ids {
                    let el = doc.element_mut(*id)?;
                    if !el.locked && style_deep(el, span, &rest_v, &fonts, cmd) {
                        n += 1;
                    }
                }
                if n == 0 {
                    return Err(EngineError::Other(format!("{label} does not apply to the selected objects")));
                }
                Ok(n)
            })?;
            Ok(ok(json!({"changed": n})))
        }
    }
}

/// Styles a whole text box, or every text box in a group. True when one was changed.
fn style_deep(el: &mut Element, span: &SpanStyle, rest: &Value, fonts: &genha_text::FontBook, cmd: &str) -> bool {
    match &mut el.kind {
        ElementKind::Group(g) => {
            let mut any = false;
            for c in &mut g.children {
                any |= !c.locked && style_deep(c, span, rest, fonts, cmd);
            }
            any
        }
        ElementKind::Text(t) => {
            let Ok(style) = merge(&t.style, rest, cmd) else { return false };
            let before = t.style.clone();
            t.style = style;
            t.set_style_all(span);
            // Boxes follow their text's height when its size changes.
            if t.style.font_size != before.font_size || !span.is_empty() {
                let t = t.clone();
                el.frame = fitted_frame(fonts, &t, el.frame);
            }
            true
        }
        _ => false,
    }
}

/// Which on/off value a toggle flips.
#[derive(Clone, Copy)]
enum Flag {
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

impl Flag {
    fn name(self) -> &'static str {
        match self {
            Flag::Bold => "bold",
            Flag::Italic => "italic",
            Flag::Underline => "underline",
            Flag::Strikethrough => "strikethrough",
        }
    }

    fn of(self, st: &TextStyle) -> bool {
        match self {
            Flag::Bold => st.bold,
            Flag::Italic => st.italic,
            Flag::Underline => st.underline,
            Flag::Strikethrough => st.strikethrough,
        }
    }

    fn of_span(self, st: &SpanStyle) -> Option<bool> {
        match self {
            Flag::Bold => st.bold,
            Flag::Italic => st.italic,
            Flag::Underline => st.underline,
            Flag::Strikethrough => st.strikethrough,
        }
    }

    fn span(self, on: bool) -> SpanStyle {
        let v = Some(on);
        match self {
            Flag::Bold => SpanStyle { bold: v, ..SpanStyle::default() },
            Flag::Italic => SpanStyle { italic: v, ..SpanStyle::default() },
            Flag::Underline => SpanStyle { underline: v, ..SpanStyle::default() },
            Flag::Strikethrough => SpanStyle { strikethrough: v, ..SpanStyle::default() },
        }
    }

    /// On for all of `r` (a range's style).
    fn all_on(self, r: &RangeStyle) -> bool {
        !r.is_mixed(self.name()) && self.of(&r.style)
    }
}

/// Every text element of `el` (itself, or a group's).
fn texts_of<'e>(el: &'e Element, out: &mut Vec<&'e TextEl>) {
    match &el.kind {
        ElementKind::Text(t) => out.push(t),
        ElementKind::Group(g) => g.children.iter().for_each(|c| texts_of(c, out)),
        _ => {}
    }
}

fn toggle(s: &mut Session, p: &Value, flag: Flag, label: &str) -> Result<Value> {
    let cmd = match flag {
        Flag::Bold => "text.toggleBold",
        Flag::Italic => "text.toggleItalic",
        Flag::Underline => "text.toggleUnderline",
        Flag::Strikethrough => "text.toggleStrikethrough",
    };
    // On everywhere it applies: turn it off; otherwise on.
    let on = match target(s, p, cmd)? {
        Target::Range(id, r) => text_el(s, id).is_some_and(|t| flag.all_on(&t.range_style(r))),
        Target::Caret(id, caret) => {
            let pending = s.text_editing.as_ref().and_then(|e| e.typing_style.as_ref()).and_then(|t| flag.of_span(t));
            pending.unwrap_or_else(|| text_el(s, id).is_some_and(|t| flag.of(&t.caret_style(caret).apply(&t.style))))
        }
        Target::Boxes(ids) => {
            let d = s.doc()?;
            let mut all: Vec<&TextEl> = vec![];
            for id in &ids {
                if let Ok(el) = d.doc.element(*id) {
                    texts_of(el, &mut all);
                }
            }
            !all.is_empty() && all.iter().all(|t| flag.all_on(&t.range_style(0..genha_text::edit::char_len(&t.text))))
        }
    };
    let r = apply(s, p, cmd, label, &flag.span(!on), &Map::new())?;
    let mut r = r;
    r[flag.name()] = json!(!on);
    Ok(r)
}

fn toggle_bold(s: &mut Session, p: &Value) -> Result<Value> {
    toggle(s, p, Flag::Bold, "Bold")
}
fn toggle_italic(s: &mut Session, p: &Value) -> Result<Value> {
    toggle(s, p, Flag::Italic, "Italic")
}
fn toggle_underline(s: &mut Session, p: &Value) -> Result<Value> {
    toggle(s, p, Flag::Underline, "Underline")
}
fn toggle_strikethrough(s: &mut Session, p: &Value) -> Result<Value> {
    toggle(s, p, Flag::Strikethrough, "Strikethrough")
}

/// The style the UI shows for the text box being edited: at its selection (or caret, with any
/// pending typing style), with the values that vary over the selection. None when no box is
/// being edited.
pub fn selection_style(s: &Session) -> Option<(NodeId, RangeStyle)> {
    let e = s.text_editing.as_ref()?;
    let t = text_el(s, e.id)?;
    let mut r = t.range_style(e.anchor..e.caret);
    if e.anchor == e.caret
        && let Some(typing) = &e.typing_style
    {
        r.style = typing.apply(&r.style);
    }
    Some((e.id, r))
}

/// A text element's style runs: `{start, end, text, style}` by character, with only the values
/// that differ from the box's style.
pub fn runs_json(t: &TextEl) -> Value {
    let mut out = vec![];
    let mut at_char = 0usize;
    let mut at_byte = 0usize;
    for (r, st) in t.style_runs() {
        let text = t.text.get(r.clone()).unwrap_or("");
        let skipped = t.text.get(at_byte..r.start).map_or(0, |g| g.chars().count());
        let start = at_char + skipped;
        let end = start + text.chars().count();
        out.push(json!({"start": start, "end": end, "text": text, "style": SpanStyle::of(&st).reduced(&t.style)}));
        at_char = end;
        at_byte = r.end;
    }
    Value::Array(out)
}

fn inspect(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "text.inspect";
    let d = s.doc()?;
    let id = match (p.get("id").and_then(Value::as_u64), &s.text_editing) {
        (Some(id), _) => NodeId(id),
        (None, Some(e)) => e.id,
        (None, None) => {
            let texts: Vec<NodeId> = d.selected_elements().iter().filter(|e| matches!(e.kind, ElementKind::Text(_))).map(|e| e.id).collect();
            match texts.as_slice() {
                [one] => *one,
                [] => return Err(bad(cmd, "select a text box (or pass `id`)")),
                _ => return Err(bad(cmd, "several text boxes are selected: pass `id`")),
            }
        }
    };
    let t = text_el(s, id).ok_or_else(|| bad(cmd, format!("object {id} is not a text box")))?;
    let editing = selection_style(s).filter(|(i, _)| *i == id).map(|(_, r)| {
        let e = s.text_editing.as_ref();
        json!({
            "selection": e.map(|e| [e.anchor, e.caret]),
            "style": r.style,
            "mixed": r.mixed,
            "typing_style": e.and_then(|e| e.typing_style.clone()),
        })
    });
    Ok(json!({"id": id, "text": t.text, "style": t.style, "spans": t.spans, "runs": runs_json(&t), "editing": editing}))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Session;
    use genha_doc::{ElementKind, NodeId, TextEl};

    fn session() -> Session {
        let mut s = Session::new();
        s.execute("file.new", &json!({"preset": "instagram-post"})).unwrap();
        s
    }

    fn text(s: &Session, id: u64) -> TextEl {
        match &s.doc().unwrap().doc.element(NodeId(id)).unwrap().kind {
            ElementKind::Text(t) => t.clone(),
            _ => panic!("not text"),
        }
    }

    fn add(s: &mut Session, t: &str) -> u64 {
        s.execute("text.add", &json!({"text": t})).unwrap()["id"].as_u64().unwrap()
    }

    #[test]
    fn style_the_selection_while_editing() {
        let mut s = session();
        let id = add(&mut s, "Grand opening today");
        s.execute("text.beginEdit", &json!({"id": id, "selection": [6, 13]})).unwrap();
        s.execute("text.setStyle", &json!({"bold": true, "color": "#ff0000"})).unwrap();
        let t = text(&s, id);
        assert_eq!(t.spans.len(), 1);
        assert_eq!((t.spans[0].start, t.spans[0].end), (6, 13));
        assert!(t.style_at(6).bold && !t.style_at(0).bold && !t.style.bold);
        // Paragraph values go to the whole box.
        s.execute("text.setStyle", &json!({"align": "left"})).unwrap();
        assert_eq!(text(&s, id).style.align, genha_doc::TextAlign::Left);
        assert_eq!(text(&s, id).spans.len(), 1);
        // Inspecting shows the runs and the selection's style.
        let r = s.execute("text.inspect", &json!({})).unwrap();
        assert_eq!(r["runs"].as_array().unwrap().len(), 3);
        assert_eq!(r["runs"][1]["text"], "opening");
        assert_eq!(r["runs"][1]["style"]["bold"], true);
        assert_eq!(r["editing"]["style"]["bold"], true);
        s.execute("text.edit", &json!({"selection": [0, 13]})).unwrap();
        let r = s.execute("text.inspect", &json!({})).unwrap();
        let mixed: Vec<&str> = r["editing"]["mixed"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert!(mixed.contains(&"bold") && mixed.contains(&"color"), "{mixed:?}");
        // The summary lists the runs too.
        let doc = s.execute("document.inspect", &json!({})).unwrap();
        assert!(doc.to_string().contains("\"spans\""));
        // Undo takes the formatting back, one step each.
        s.execute("text.endEdit", &json!({})).unwrap();
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(text(&s, id).style.align, genha_doc::TextAlign::Center);
        s.execute("edit.undo", &json!({})).unwrap();
        assert!(text(&s, id).spans.is_empty());
    }

    #[test]
    fn toggles_flip_the_selection_and_whole_boxes() {
        let mut s = session();
        let id = add(&mut s, "one two");
        s.execute("text.beginEdit", &json!({"id": id, "selection": [0, 3]})).unwrap();
        let r = s.execute("text.toggleBold", &json!({})).unwrap();
        assert_eq!(r["bold"], true);
        assert!(text(&s, id).style_at(0).bold && !text(&s, id).style_at(4).bold);
        // Mixed: the whole text turns bold, then off.
        s.execute("text.edit", &json!({"selection": [0, 7]})).unwrap();
        assert_eq!(s.execute("text.toggleBold", &json!({})).unwrap()["bold"], true);
        assert!(text(&s, id).spans.is_empty() || (0..7).all(|i| text(&s, id).style_at(i).bold));
        assert_eq!(s.execute("text.toggleBold", &json!({})).unwrap()["bold"], false);
        assert!((0..7).all(|i| !text(&s, id).style_at(i).bold));
        s.execute("text.endEdit", &json!({})).unwrap();
        // Not editing: the whole box.
        s.execute("text.toggleItalic", &json!({"ids": [id]})).unwrap();
        assert!(text(&s, id).style.italic);
        s.execute("text.toggleUnderline", &json!({"ids": [id], "range": [4, 7]})).unwrap();
        let t = text(&s, id);
        assert!(t.style_at(5).underline && !t.style_at(1).underline);
        s.execute("text.toggleStrikethrough", &json!({"ids": [id]})).unwrap();
        assert!(text(&s, id).style.strikethrough);
    }

    #[test]
    fn typing_takes_the_style_at_the_caret_and_pending_styles() {
        let mut s = session();
        let id = add(&mut s, "Hello world");
        s.execute("text.setStyle", &json!({"ids": [id], "range": [0, 5], "color": "#00aa00"})).unwrap();
        s.execute("text.beginEdit", &json!({"id": id, "selection": [5, 5]})).unwrap();
        s.execute("text.edit", &json!({"insert": "!!"})).unwrap();
        let t = text(&s, id);
        assert_eq!(t.text, "Hello!! world");
        assert_eq!((t.spans[0].start, t.spans[0].end), (0, 7));
        // Whole-text edits from the canvas keep the runs in place.
        s.execute("text.edit", &json!({"text": "Hello!!! world", "selection": [8, 8]})).unwrap();
        assert_eq!(text(&s, id).spans[0].end, 8);
        s.execute("text.edit", &json!({"text": "Hi!!! world", "selection": [2, 2]})).unwrap();
        assert_eq!(text(&s, id).spans[0].end, 5);
        // Bold with nothing selected: the next text typed is bold (and keeps the colour).
        s.execute("text.edit", &json!({"selection": [11, 11]})).unwrap();
        s.execute("text.toggleBold", &json!({})).unwrap();
        assert!(text(&s, id).spans.iter().all(|sp| sp.style.bold.is_none()), "nothing changed yet");
        s.execute("text.edit", &json!({"insert": " now"})).unwrap();
        let t = text(&s, id);
        assert!(t.style_at(12).bold && !t.style_at(10).bold, "{:?}", t.spans);
        // Moving the caret drops a pending style.
        s.execute("text.toggleItalic", &json!({})).unwrap();
        s.execute("text.edit", &json!({"selection": [0, 0]})).unwrap();
        s.execute("text.edit", &json!({"insert": "X"})).unwrap();
        assert!(!text(&s, id).style_at(0).italic);
    }

    #[test]
    fn whole_box_styles_clear_run_overrides_and_resize() {
        let mut s = session();
        let id = add(&mut s, "Big sale");
        s.execute("text.setStyle", &json!({"ids": [id], "range": [0, 3], "color": "#ff0000", "font_size": 120})).unwrap();
        let h1 = s.doc().unwrap().doc.element(NodeId(id)).unwrap().frame.height;
        s.execute("text.setStyle", &json!({"ids": [id], "color": "#0000ff"})).unwrap();
        let t = text(&s, id);
        assert_eq!(t.spans.len(), 1);
        assert_eq!(t.spans[0].style.color, None);
        assert_eq!(t.spans[0].style.font_size, Some(120.0));
        s.execute("text.setStyle", &json!({"ids": [id], "font_size": 20})).unwrap();
        assert!(text(&s, id).spans.is_empty());
        let h2 = s.doc().unwrap().doc.element(NodeId(id)).unwrap().frame.height;
        assert!(h2 < h1, "{h1} → {h2}");
    }

    #[test]
    fn bad_style_params_are_refused() {
        let mut s = session();
        let id = add(&mut s, "abc");
        for p in [
            json!({"ids": [id], "font_size": 0}),
            json!({"ids": [id], "font_size": "big"}),
            json!({"ids": [id], "color": "nope"}),
            json!({"ids": [id], "range": [0]}),
            json!({"ids": [id, id], "range": [0, 1], "bold": true}),
            json!({"ids": [id], "font_family": ""}),
        ] {
            assert!(s.execute("text.setStyle", &p).is_err(), "{p}");
        }
        // Ranges past the end are clamped.
        s.execute("text.setStyle", &json!({"ids": [id], "range": [2, 99], "bold": true})).unwrap();
        assert_eq!(text(&s, id).spans.len(), 1);
    }
}
