//! Pages: add, duplicate, delete, reorder, size, background.

use serde_json::{Value, json};

use super::{bad, cmd, color_param, f64_or, has_doc, ok, parse, str_param, str_req, usize_param};
use crate::{CommandSpec, Result, Session};
use genha_doc::{Fill, MAX_PAGE_SIDE, MAX_PAGES, Page, preset};

fn has_pages(s: &Session) -> std::result::Result<(), String> {
    let d = s.active().ok_or("no document open")?;
    if d.doc.pages.len() < 2 { Err("the design has only one page".into()) } else { Ok(()) }
}

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "page.add",
            "Add Page",
            ["Page"],
            Some("Cmd+Enter"),
            "{preset?, width?, height?, name?} — inserted after the current page, same size by default",
            has_doc,
            add
        ),
        cmd!("page.duplicate", "Duplicate Page", ["Page"], None, "{index?}", has_doc, duplicate),
        cmd!("page.delete", "Delete Page", ["Page"], None, "{index?}", has_pages, delete),
        cmd!("page.goto", "Go to Page", [], None, "{index}", has_doc, goto),
        cmd!("page.next", "Next Page", ["Page"], Some("PageDown"), "{}", has_doc, next),
        cmd!("page.previous", "Previous Page", ["Page"], Some("PageUp"), "{}", has_doc, previous),
        cmd!("page.move", "Move Page", [], None, "{from?, to}", has_pages, move_page),
        cmd!("page.rename", "Rename Page", ["Page"], None, "{index?, name}", has_doc, rename),
        cmd!(
            "page.setSize",
            "Page Size…",
            ["Page"],
            None,
            "{preset? | width, height, index?} — changes the size without moving content (see `page.resize` for a smart resize)",
            has_doc,
            set_size
        ),
        cmd!("page.setBackground", "Page Background", ["Page"], None, "{color?: #rrggbb | fill?: Fill JSON, index?}", has_doc, set_background),
        cmd!("page.setNotes", "Page Notes", [], None, "{notes, index?}", has_doc, set_notes),
        cmd!("page.setDuration", "Page Duration", [], None, "{seconds | null, index?}", has_doc, set_duration),
    ]
}

fn index_or_current(s: &Session, p: &Value, cmd: &str) -> Result<usize> {
    let d = s.doc()?;
    let i = usize_param(p, "index").unwrap_or(d.page);
    if i >= d.doc.pages.len() {
        return Err(bad(cmd, format!("no page {i} (the design has {})", d.doc.pages.len())));
    }
    Ok(i)
}

pub(crate) fn size_params(p: &Value, cmd: &str, default: (f64, f64)) -> Result<(f64, f64)> {
    let (mut w, mut h) = default;
    if let Some(id) = str_param(p, "preset") {
        let pr = preset::find(id).ok_or_else(|| bad(cmd, format!("unknown preset `{id}`")))?;
        (w, h) = (pr.width, pr.height);
    }
    w = f64_or(p, "width", w);
    h = f64_or(p, "height", h);
    if !(1.0..=MAX_PAGE_SIDE).contains(&w) || !(1.0..=MAX_PAGE_SIDE).contains(&h) {
        return Err(bad(cmd, format!("page size must be 1–{MAX_PAGE_SIDE} px")));
    }
    Ok((w, h))
}

fn add(s: &mut Session, p: &Value) -> Result<Value> {
    let d = s.doc()?;
    if d.doc.pages.len() >= MAX_PAGES {
        return Err(bad("page.add", format!("a design can have at most {MAX_PAGES} pages")));
    }
    let cur = d.doc.pages.get(d.page).map(|p| (p.width, p.height)).unwrap_or((1080.0, 1080.0));
    let (w, h) = size_params(p, "page.add", cur)?;
    let name = str_param(p, "name").map(str::to_string);
    let index = s.edit("Add Page", |doc, cx| {
        let id = doc.alloc_id();
        let n = doc.pages.len() + 1;
        let page = Page::new(id, &name.unwrap_or_else(|| format!("Page {n}")), w, h);
        let at = (cx.page + 1).min(doc.pages.len());
        doc.pages.insert(at, page);
        cx.page = at;
        cx.selection.clear();
        Ok(at)
    })?;
    Ok(ok(json!({"index": index})))
}

fn duplicate(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.duplicate")?;
    if s.doc()?.doc.pages.len() >= MAX_PAGES {
        return Err(bad("page.duplicate", format!("a design can have at most {MAX_PAGES} pages")));
    }
    let index = s.edit("Duplicate Page", |doc, cx| {
        let mut page = doc.page(i)?.clone();
        doc.reassign_page_ids(&mut page);
        page.name = format!("{} copy", page.name);
        doc.pages.insert(i + 1, page);
        cx.page = i + 1;
        cx.selection.clear();
        Ok(i + 1)
    })?;
    Ok(ok(json!({"index": index})))
}

fn delete(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.delete")?;
    s.edit("Delete Page", |doc, cx| {
        doc.pages.remove(i);
        cx.page = cx.page.min(doc.pages.len().saturating_sub(1));
        cx.selection.clear();
        Ok(())
    })?;
    Ok(ok(json!({"pages": s.doc()?.doc.pages.len()})))
}

pub(crate) fn go(s: &mut Session, index: usize) -> Result<Value> {
    let d = s.doc_mut()?;
    if index >= d.doc.pages.len() {
        return Err(bad("page.goto", format!("no page {index}")));
    }
    if d.page != index {
        d.page = index;
        d.selection.clear();
        d.revision = d.revision.wrapping_add(1);
    }
    s.preview = None;
    Ok(ok(json!({"index": index})))
}

fn goto(s: &mut Session, p: &Value) -> Result<Value> {
    let i = usize_param(p, "index").ok_or_else(|| bad("page.goto", "missing `index`"))?;
    go(s, i)
}

fn next(s: &mut Session, _: &Value) -> Result<Value> {
    let d = s.doc()?;
    let i = (d.page + 1).min(d.doc.pages.len().saturating_sub(1));
    go(s, i)
}

fn previous(s: &mut Session, _: &Value) -> Result<Value> {
    let i = s.doc()?.page.saturating_sub(1);
    go(s, i)
}

fn move_page(s: &mut Session, p: &Value) -> Result<Value> {
    let from = index_or_current(s, &json!({"index": p.get("from")}), "page.move")?;
    let to = usize_param(p, "to").ok_or_else(|| bad("page.move", "missing `to`"))?;
    let n = s.doc()?.doc.pages.len();
    if to >= n {
        return Err(bad("page.move", format!("`to` must be below {n}")));
    }
    s.edit("Move Page", |doc, cx| {
        let page = doc.pages.remove(from);
        doc.pages.insert(to, page);
        if cx.page == from {
            cx.page = to;
        }
        Ok(())
    })?;
    Ok(ok(json!({"index": to})))
}

fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.rename")?;
    let name = str_req(p, "name", "page.rename")?.to_string();
    s.edit("Rename Page", |doc, _| {
        doc.page_mut(i)?.name = name;
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn set_size(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.setSize")?;
    let cur = s.doc()?.doc.pages.get(i).map(|p| (p.width, p.height)).unwrap_or((1080.0, 1080.0));
    let (w, h) = size_params(p, "page.setSize", cur)?;
    s.edit("Page Size", |doc, _| {
        let page = doc.page_mut(i)?;
        page.width = w;
        page.height = h;
        Ok(())
    })?;
    Ok(ok(json!({"width": w, "height": h})))
}

fn set_background(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.setBackground")?;
    let fill = match color_param(p, "color", "page.setBackground")? {
        Some(c) => Fill::solid(c),
        None => parse::<Fill>(p, "fill", "page.setBackground")?,
    };
    s.edit("Page Background", |doc, _| {
        doc.page_mut(i)?.background = fill;
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn set_notes(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.setNotes")?;
    let notes = str_req(p, "notes", "page.setNotes")?.to_string();
    s.edit("Page Notes", |doc, _| {
        doc.page_mut(i)?.notes = notes;
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn set_duration(s: &mut Session, p: &Value) -> Result<Value> {
    let i = index_or_current(s, p, "page.setDuration")?;
    let secs = match p.get("seconds") {
        Some(Value::Null) | None => None,
        Some(v) => Some(
            v.as_f64()
                .filter(|v| v.is_finite() && (0.1..=3600.0).contains(v))
                .ok_or_else(|| bad("page.setDuration", "`seconds` must be 0.1–3600"))?,
        ),
    };
    s.edit("Page Duration", |doc, _| {
        doc.page_mut(i)?.duration = secs;
        Ok(())
    })?;
    Ok(ok(json!({})))
}
