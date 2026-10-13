//! Read-only views of the session for agents, the CLI and the UI.

use serde_json::{Value, json};

use crate::{DocState, Session};
use genha_doc::{Element, ElementKind, Fill, Frame, ShapeKind, TextEffect, TextEl, TextFit};
use genha_text::FontBook;

/// How a text box fits its text: `{fit: none|shrink, scale, fits}` — the scale its sizes are
/// laid out at (1: as set) and whether the text fits the box at that scale (shrinking text that
/// overflows even at the smallest scale allowed does not).
pub fn fit_json(fonts: &FontBook, t: &TextEl, frame: Frame) -> Value {
    let f = frame.sanitized();
    let r = genha_render::text_fit(fonts, t, f.width, f.height);
    let fits = match t.fit {
        TextFit::Shrink => r.fits,
        TextFit::None if t.curve_degrees() != 0.0 => {
            genha_render::curved_text_size(fonts, t, f.width).is_none_or(|(w, h)| w <= f.width + 0.5 && h <= f.height + 0.5)
        }
        TextFit::None => genha_text::measure_wrapped_el(fonts, t, f.width).1 <= f.height + 0.5,
    };
    json!({"fit": t.fit.id(), "scale": (r.scale * 1000.0).round() / 1000.0, "fits": fits})
}

/// One element as a compact JSON summary; text boxes say how they fit their text ([`fit_json`],
/// measured with `fonts`).
pub fn element_summary(e: &Element, fonts: &FontBook) -> Value {
    let mut v = json!({
        "id": e.id.0,
        "type": e.kind.type_name().to_lowercase(),
        "name": e.display_name(),
        "frame": e.frame,
    });
    let Some(o) = v.as_object_mut() else { return v };
    if e.opacity != 1.0 {
        o.insert("opacity".into(), json!(e.opacity));
    }
    if e.locked {
        o.insert("locked".into(), json!(true));
    }
    if e.hidden {
        o.insert("hidden".into(), json!(true));
    }
    match &e.kind {
        ElementKind::Shape(s) => {
            let shape = match &s.shape {
                ShapeKind::Rect { corner_radius } if *corner_radius > 0.0 => "rounded",
                ShapeKind::Rect { .. } => "rect",
                ShapeKind::Ellipse => "ellipse",
                ShapeKind::Triangle => "triangle",
                ShapeKind::Polygon { .. } => "polygon",
                ShapeKind::Star { .. } => "star",
                ShapeKind::Line => "line",
                ShapeKind::Path { .. } => "path",
            };
            o.insert("shape".into(), json!(shape));
            o.insert("fill".into(), fill_summary(&s.fill));
            if let Some(st) = &s.stroke {
                o.insert("stroke".into(), json!({"color": st.color, "width": st.width}));
            }
        }
        ElementKind::Text(t) => {
            o.insert("text".into(), json!(t.text));
            o.insert("font".into(), json!(format!("{} {}{}", t.style.font_family, t.style.font_size, if t.style.bold { " bold" } else { "" })));
            o.insert("color".into(), json!(t.style.color));
            if t.effect != TextEffect::None {
                o.insert("effect".into(), json!(t.effect));
            }
            if t.curve != 0.0 {
                o.insert("curve".into(), json!(t.curve));
            }
            if !t.spans.is_empty() {
                // Formatted parts: {start, end (characters), and the values that differ}.
                o.insert("spans".into(), json!(t.spans));
            }
            if t.fit != TextFit::None {
                o.insert("fit".into(), fit_json(fonts, t, e.frame));
            }
        }
        ElementKind::Image(i) => {
            o.insert("asset".into(), json!(i.asset.0));
            if !i.adjustments.is_identity() {
                o.insert("adjustments".into(), json!(i.adjustments));
            }
            if let Some(f) = &i.filter {
                o.insert("filter".into(), json!(f));
            }
        }
        ElementKind::Group(g) => {
            o.insert("children".into(), Value::Array(g.children.iter().map(|c| element_summary(c, fonts)).collect()));
        }
        ElementKind::Chart(c) => {
            o.insert("chart".into(), json!(c.kind.id()));
            o.insert("data".into(), json!(c.points()));
            o.insert("categories".into(), json!(c.categories));
            o.insert("series".into(), json!(c.series));
            if c.stacked {
                o.insert("stacked".into(), json!(true));
            }
            if !c.colors.is_empty() {
                o.insert("colors".into(), json!(c.colors));
            }
            o.insert("value_format".into(), json!(c.value_format));
        }
        ElementKind::Table(t) => {
            o.insert("rows".into(), json!(t.rows));
            o.insert("size".into(), json!([t.rows.len(), t.columns()]));
            o.insert("header_row".into(), json!(t.header_row));
            if !t.merges.is_empty() {
                o.insert("merges".into(), json!(t.merges));
            }
            if !t.cell_styles.is_empty() {
                o.insert("cell_styles".into(), json!(t.cell_styles));
            }
        }
    }
    v
}

fn fill_summary(f: &Fill) -> Value {
    match f {
        Fill::None => json!("none"),
        Fill::Solid { color } => json!(color),
        other => json!(other),
    }
}

/// The open document: pages (current page in full), selection, brand.
pub fn document_summary(d: &DocState, all_pages: bool, fonts: &FontBook) -> Value {
    let pages: Vec<Value> = d
        .doc
        .pages
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut v = json!({"index": i, "id": p.id.0, "name": p.name, "width": p.width, "height": p.height, "background": fill_summary(&p.background), "elements": p.elements.len()});
            if (all_pages || i == d.page)
                && let Some(o) = v.as_object_mut()
            {
                o.insert("elements".into(), Value::Array(p.elements.iter().map(|e| element_summary(e, fonts)).collect()));
            }
            v
        })
        .collect();
    json!({
        "title": d.doc.title,
        "path": d.path.as_ref().map(|p| p.display().to_string()),
        "dirty": d.dirty,
        "current_page": d.page,
        "selection": d.selection.iter().map(|i| i.0).collect::<Vec<_>>(),
        "pages": pages,
        "brand": d.doc.brand,
        "assets": d.doc.assets.iter().map(|(id, a)| json!({"id": id.0, "name": a.name, "mime": a.mime, "width": a.width, "height": a.height, "bytes": a.data.len()})).collect::<Vec<_>>(),
        "undo": d.history.undo.last().map(|e| e.label.clone()),
        "redo": d.history.redo.last().map(|e| e.label.clone()),
    })
}

/// Open documents, for `session.inspect`.
pub fn session_summary(s: &Session) -> Value {
    json!({
        "active": s.active,
        "documents": s.docs.iter().enumerate().map(|(i, d)| json!({"index": i, "name": d.display_name(), "pages": d.doc.pages.len(), "dirty": d.dirty})).collect::<Vec<_>>(),
        "tool": s.tools.tool.id(),
        "text_editing": s.text_editing.as_ref().map(|e| json!({"id": e.id, "selection": [e.anchor, e.caret]})),
    })
}
