//! File: new, open, save, close.

use serde_json::{Value, json};

use super::{always, bad, cmd, f64_or, has_doc, ok, str_param, str_req, usize_param};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::{Document, MAX_PAGE_SIDE, preset};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "file.new",
            "New Design…",
            ["File"],
            Some("Cmd+N"),
            "{preset?: preset id (see `presets.list`), width?: px, height?: px, title?}",
            always,
            file_new
        ),
        cmd!("file.open", "Open…", ["File"], Some("Cmd+O"), "{path: .genha file, or a picture to start a design from}", always, file_open),
        cmd!("file.save", "Save", ["File"], Some("Cmd+S"), "{path?: defaults to the document's file}", has_doc, file_save),
        cmd!("file.saveAs", "Save As…", ["File"], Some("Cmd+Shift+S"), "{path, compress?: bool = true}", has_doc, file_save_as),
        cmd!("file.close", "Close", ["File"], Some("Cmd+W"), "{index?: document index, default active}", has_doc, file_close),
        cmd!("document.switch", "Switch Document", [], None, "{index}", has_doc, doc_switch),
        cmd!("document.rename", "Rename Design", ["File"], None, "{title}", has_doc, doc_rename),
    ]
}

fn file_new(s: &mut Session, p: &Value) -> Result<Value> {
    let (mut w, mut h) = (1080.0, 1080.0);
    let mut title = "Untitled design".to_string();
    if let Some(id) = str_param(p, "preset") {
        let pr = preset::find(id).ok_or_else(|| bad("file.new", format!("unknown preset `{id}`")))?;
        (w, h) = (pr.width, pr.height);
        title = pr.label.to_string();
    }
    w = f64_or(p, "width", w);
    h = f64_or(p, "height", h);
    if !(1.0..=MAX_PAGE_SIDE).contains(&w) || !(1.0..=MAX_PAGE_SIDE).contains(&h) {
        return Err(bad("file.new", format!("page size must be 1–{MAX_PAGE_SIDE} px")));
    }
    if let Some(t) = str_param(p, "title") {
        title = t.to_string();
    }
    let index = s.open_document(Document::new(&title, w, h));
    Ok(ok(json!({"document": index, "width": w, "height": h})))
}

/// Opens document bytes (a `.genha` file, or a picture), as from a file or a drop. It goes into
/// My designs (and the recent designs).
pub fn open_bytes(s: &mut Session, name: &str, bytes: &[u8]) -> Result<Value> {
    let r = open_impl(s, name, bytes, None)?;
    // Not through `Session::execute`, which adds new designs after commands.
    super::designs::adopt(s);
    Ok(r)
}

/// [`open_bytes`] for a design read from `path` (kept as the design's file).
fn open_impl(s: &mut Session, name: &str, bytes: &[u8], path: Option<std::path::PathBuf>) -> Result<Value> {
    if genha_svg::import::is_svg(bytes) {
        let (mut el, w, h) = super::create::svg_element(name, bytes)?;
        let (w, h) = (w.clamp(1.0, MAX_PAGE_SIDE), h.clamp(1.0, MAX_PAGE_SIDE));
        let mut doc = Document::new(name, w, h);
        doc.reassign_ids(&mut el);
        if let Some(page) = doc.pages.first_mut() {
            page.background = genha_doc::Fill::None;
            page.elements.push(el);
        }
        let index = s.open_document(doc);
        return Ok(ok(json!({"document": index, "svg": true})));
    }
    if genha_effects::probe(bytes).is_ok() {
        let info = genha_effects::probe(bytes)?;
        let (w, h) = (f64::from(info.width).clamp(1.0, MAX_PAGE_SIDE), f64::from(info.height).clamp(1.0, MAX_PAGE_SIDE));
        let mut doc = Document::new(name, w, h);
        let asset = super::create::asset_from_bytes(name, bytes)?;
        let aid = doc.add_asset(asset);
        let id = doc.alloc_id();
        if let Some(page) = doc.pages.first_mut() {
            let mut el =
                genha_doc::Element::new(id, genha_doc::Frame::new(0.0, 0.0, w, h), genha_doc::ElementKind::Image(genha_doc::ImageEl::new(aid)));
            el.name = name.to_string();
            page.elements.push(el);
        }
        let index = s.open_document(doc);
        return Ok(ok(json!({"document": index, "picture": true})));
    }
    let (doc, notes) = genha_format::from_bytes(bytes)?;
    for n in &notes {
        s.post(format!("{name}: {n}"));
    }
    let index = s.open_document(doc);
    if let Some(d) = s.docs.get_mut(index) {
        d.path = path;
    }
    super::designs::record(s, index, super::designs::Why::Opened);
    Ok(ok(json!({"document": index, "repaired": notes})))
}

/// The active design as `.genha` bytes (compressed), for hosts without a file system (the web
/// build downloads them). Like a save: records a version and marks the design saved.
pub fn design_bytes(s: &mut Session) -> Result<Vec<u8>> {
    super::versions::auto_version(s);
    let bytes = genha_format::to_bytes(&s.doc()?.doc, true)?;
    let active = s.active;
    super::recovery::forget(s, active);
    s.doc_mut()?.dirty = false;
    super::designs::record(s, active, super::designs::Why::Saved);
    Ok(bytes)
}

fn file_open(s: &mut Session, p: &Value) -> Result<Value> {
    open_path(s, str_req(p, "path", "file.open")?)
}

/// Opens the design (or picture) at `path`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn open_path(s: &mut Session, path: &str) -> Result<Value> {
    let path = std::path::PathBuf::from(path);
    let meta = std::fs::metadata(&path).map_err(|e| EngineError::Other(format!("cannot open {}: {e}", path.display())))?;
    if meta.len() > genha_format::MAX_FILE_BYTES {
        return Err(EngineError::Other(format!("{} is too large", path.display())));
    }
    let bytes = std::fs::read(&path).map_err(|e| EngineError::Other(format!("cannot read {}: {e}", path.display())))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let is_native =
        path.extension().is_some_and(|e| e.eq_ignore_ascii_case(genha_format::EXTENSION) || e.eq_ignore_ascii_case(genha_format::LEGACY_EXTENSION));
    open_impl(s, &name, &bytes, is_native.then_some(path))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn open_path(_: &mut Session, _: &str) -> Result<Value> {
    Err(EngineError::Other("opening files by path is not available in the browser; drop the file on the window".into()))
}

fn file_save(s: &mut Session, p: &Value) -> Result<Value> {
    let path = match str_param(p, "path") {
        Some(path) => std::path::PathBuf::from(path),
        None => s.doc()?.path.clone().ok_or_else(|| bad("file.save", "the design has no file yet: use `file.saveAs` with a path"))?,
    };
    save_to(s, &path, true)
}

fn file_save_as(s: &mut Session, p: &Value) -> Result<Value> {
    let mut path = std::path::PathBuf::from(str_req(p, "path", "file.saveAs")?);
    if path.extension().is_none() {
        path.set_extension(genha_format::EXTENSION);
    }
    save_to(s, &path, super::bool_or(p, "compress", true))
}

#[cfg(not(target_arch = "wasm32"))]
fn save_to(s: &mut Session, path: &std::path::Path, compress: bool) -> Result<Value> {
    super::versions::auto_version(s);
    let bytes = genha_format::to_bytes(&s.doc()?.doc, compress)?;
    genha_format::write_atomic(path, &bytes)?;
    let active = s.active;
    super::recovery::forget(s, active);
    let d = s.doc_mut()?;
    d.path = Some(path.to_path_buf());
    d.dirty = false;
    super::designs::record(s, active, super::designs::Why::Saved);
    Ok(ok(json!({"path": path.display().to_string(), "bytes": bytes.len()})))
}

#[cfg(target_arch = "wasm32")]
fn save_to(_: &mut Session, _: &std::path::Path, _: bool) -> Result<Value> {
    Err(EngineError::Other("saving to a path is not available in the browser; use Download".into()))
}

fn file_close(s: &mut Session, p: &Value) -> Result<Value> {
    let i = usize_param(p, "index").unwrap_or(s.active);
    if i >= s.docs.len() {
        return Err(bad("file.close", format!("no document {i}")));
    }
    // A design kept by GenHa keeps its last state in My designs.
    super::designs::sync_one(s, i);
    super::recovery::forget(s, i);
    s.docs.remove(i);
    if s.active >= s.docs.len() {
        s.active = s.docs.len().saturating_sub(1);
    } else if s.active > i {
        s.active -= 1;
    }
    s.preview = None;
    Ok(ok(json!({"open": s.docs.len()})))
}

fn doc_switch(s: &mut Session, p: &Value) -> Result<Value> {
    let i = usize_param(p, "index").ok_or_else(|| bad("document.switch", "missing `index`"))?;
    if i >= s.docs.len() {
        return Err(bad("document.switch", format!("no document {i}")));
    }
    s.active = i;
    s.preview = None;
    Ok(ok(json!({})))
}

fn doc_rename(s: &mut Session, p: &Value) -> Result<Value> {
    let title = str_req(p, "title", "document.rename")?.trim().to_string();
    if title.is_empty() {
        return Err(bad("document.rename", "the title is empty"));
    }
    s.edit("Rename Design", |doc, _| {
        doc.title = title;
        Ok(())
    })?;
    Ok(ok(json!({})))
}
