//! Version history: save, list, restore, copy, delete, rename snapshots kept in the file.

use serde_json::{Value, json};

use super::{bad, cmd, has_doc, ok, str_param, str_req};
use crate::{CommandSpec, Result, Session};
use genha_doc::Version;
use genha_doc::version::MAX_AUTO_VERSIONS;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "version.save",
            "Save Version",
            ["File", "Version History"],
            None,
            "{name?} — a bookmarked snapshot of the whole design, kept in the file",
            has_doc,
            save
        ),
        cmd!(query "version.list", "Version History", ["File", "Version History"], None, "{}", has_doc, list),
        cmd!("version.restore", "Restore Version", [], None, "{id} — the current state is kept as a version first; undoable", has_doc, restore),
        cmd!("version.copy", "Open Version as New Design", [], None, "{id}", has_doc, copy),
        cmd!("version.rename", "Rename Version", [], None, "{id, name, bookmarked?: bool}", has_doc, rename),
        cmd!("version.delete", "Delete Version", [], None, "{id}", has_doc, delete),
    ]
}

/// Seconds since 1970: the host's clock when it set one ([`crate::set_clock`]: the browser,
/// where `SystemTime` is unavailable), else the system's (0 in the browser without one).
pub fn now_secs() -> u64 {
    if let Some(clock) = crate::CLOCK.get() {
        return clock();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

/// A snapshot of the document's current content.
fn snapshot(doc: &mut genha_doc::Document, name: String, bookmarked: bool) -> u64 {
    let id = doc.alloc_id().0;
    doc.versions.push(Version {
        id,
        name,
        created: now_secs(),
        bookmarked,
        title: doc.title.clone(),
        pages: doc.pages.clone(),
        brand: doc.brand.clone(),
    });
    prune(doc);
    id
}

/// Keeps the newest automatic versions and every bookmarked one (up to the document limit).
fn prune(doc: &mut genha_doc::Document) {
    let autos = doc.versions.iter().filter(|v| !v.bookmarked).count();
    if autos > MAX_AUTO_VERSIONS {
        let mut drop = autos - MAX_AUTO_VERSIONS;
        doc.versions.retain(|v| {
            if !v.bookmarked && drop > 0 {
                drop -= 1;
                false
            } else {
                true
            }
        });
    }
    let max = genha_doc::version::MAX_VERSIONS;
    if doc.versions.len() > max {
        let n = doc.versions.len() - max;
        doc.versions.drain(..n);
    }
}

/// Adds an automatic version when the content differs from the newest version (called on save).
/// Not an undo step: it only records history.
pub fn auto_version(s: &mut Session) {
    let Some(d) = s.active_mut() else { return };
    let same = d.doc.versions.last().is_some_and(|v| v.pages == d.doc.pages && v.brand == d.doc.brand && v.title == d.doc.title);
    if same {
        return;
    }
    let doc = std::sync::Arc::make_mut(&mut d.doc);
    snapshot(doc, "Saved".into(), false);
}

fn save(s: &mut Session, p: &Value) -> Result<Value> {
    let name = str_param(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Version").to_string();
    let id = s.edit("Save Version", |doc, _| Ok(snapshot(doc, name, true)))?;
    Ok(ok(json!({"id": id})))
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let d = s.doc()?;
    Ok(json!(
        d.doc
            .versions
            .iter()
            .rev()
            .map(|v| json!({"id": v.id, "name": v.name, "created": v.created, "bookmarked": v.bookmarked, "pages": v.pages.len(), "title": v.title}))
            .collect::<Vec<_>>()
    ))
}

fn version_id(p: &Value, cmd: &str) -> Result<u64> {
    p.get("id").and_then(Value::as_u64).ok_or_else(|| bad(cmd, "missing version `id` (see version.list)"))
}

fn find(s: &Session, id: u64, cmd: &str) -> Result<Version> {
    s.doc()?.doc.versions.iter().find(|v| v.id == id).cloned().ok_or_else(|| bad(cmd, format!("no version {id}")))
}

fn restore(s: &mut Session, p: &Value) -> Result<Value> {
    let id = version_id(p, "version.restore")?;
    let v = find(s, id, "version.restore")?;
    s.edit("Restore Version", |doc, cx| {
        snapshot(doc, "Before restoring".into(), false);
        doc.pages = if v.pages.is_empty() { doc.pages.clone() } else { v.pages.clone() };
        doc.brand = v.brand.clone();
        doc.title = v.title.clone();
        cx.page = 0;
        cx.selection.clear();
        Ok(())
    })?;
    Ok(ok(json!({"restored": id})))
}

fn copy(s: &mut Session, p: &Value) -> Result<Value> {
    let id = version_id(p, "version.copy")?;
    let v = find(s, id, "version.copy")?;
    let mut doc = (*s.doc()?.doc).clone();
    doc.pages = v.pages;
    doc.brand = v.brand;
    doc.title = format!("{} ({})", v.title, v.name);
    doc.versions.clear();
    for a in doc.unused_assets() {
        doc.assets.remove(&a);
    }
    doc.repair();
    let index = s.open_document(doc);
    if let Some(d) = s.active_mut() {
        d.dirty = true;
    }
    Ok(ok(json!({"document": index})))
}

fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let id = version_id(p, "version.rename")?;
    let name = str_req(p, "name", "version.rename")?.to_string();
    let bookmarked = p.get("bookmarked").and_then(Value::as_bool);
    find(s, id, "version.rename")?;
    s.edit("Rename Version", |doc, _| {
        if let Some(v) = doc.versions.iter_mut().find(|v| v.id == id) {
            v.name = name;
            if let Some(b) = bookmarked {
                v.bookmarked = b;
            }
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}

fn delete(s: &mut Session, p: &Value) -> Result<Value> {
    let id = version_id(p, "version.delete")?;
    find(s, id, "version.delete")?;
    s.edit("Delete Version", |doc, _| {
        doc.versions.retain(|v| v.id != id);
        for a in doc.unused_assets() {
            doc.assets.remove(&a);
        }
        Ok(())
    })?;
    Ok(ok(json!({})))
}
