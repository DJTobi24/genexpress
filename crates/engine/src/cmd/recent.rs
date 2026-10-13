//! Recent designs: the designs opened or saved lately, newest first, with a thumbnail of their
//! first page, kept in the session's [`Store`](crate::store::Store) (`recent/…`).
//!
//! On the desktop an entry points at the design's file. In the browser designs have no path, so
//! the entry keeps a copy of the design (as it was when opened or downloaded) next to its
//! thumbnail. Opening, saving (and downloading) a design records it ([`remember`]); the home
//! screen lists `recent.list` and opens entries with `recent.open`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{always, bad, cmd, ok, str_req};
use crate::store::key_part;
use crate::{CommandSpec, EngineError, Result, Session};

const LIST: &str = "recent/list.json";
/// What the desktop app kept before recent designs moved into the store: a JSON list of paths.
const LEGACY_LIST: &str = "recent.json";
/// Designs listed.
pub const MAX_RECENT: usize = 12;
/// Longest side of a thumbnail, in pixels.
pub const THUMBNAIL_SIDE: u32 = 320;
/// Largest list file read.
const MAX_LIST_BYTES: usize = 1 << 20;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "recent.list", "Recent Designs", [], None, "{} — newest first: [{id, title, name, path?, pages, thumbnail?: store key}]", always, list),
        cmd!("recent.open", "Open Recent Design", [], None, "{id: from recent.list}", always, open),
        cmd!(query "recent.remove", "Remove from Recent Designs", [], None, "{id: from recent.list}", always, remove),
    ]
}

/// One recent design.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentEntry {
    pub id: String,
    pub title: String,
    /// The file name (or download name) it was opened or saved as.
    #[serde(default)]
    pub name: String,
    /// Its file, on the desktop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub pages: usize,
}

impl RecentEntry {
    /// Store key of the design copy (entries without a path).
    pub fn copy_key(&self) -> String {
        format!("recent/{}.{}", self.id, genha_format::EXTENSION)
    }
    /// Store key of the thumbnail (PNG).
    pub fn thumbnail_key(&self) -> String {
        format!("recent/{}.png", self.id)
    }
}

/// The recent designs, newest first (empty without a store).
pub fn entries(s: &Session) -> Vec<RecentEntry> {
    let Some(store) = s.store.as_ref() else { return vec![] };
    if let Some(bytes) = store.read(LIST) {
        if bytes.len() > MAX_LIST_BYTES {
            return vec![];
        }
        let mut list: Vec<RecentEntry> = serde_json::from_slice(&bytes).unwrap_or_default();
        list.retain(|e| crate::store::valid_key(&e.copy_key()));
        list.truncate(MAX_RECENT);
        return list;
    }
    // Before the store: a list of file paths.
    let Some(bytes) = store.read(LEGACY_LIST).filter(|b| b.len() <= MAX_LIST_BYTES) else { return vec![] };
    let paths: Vec<String> = serde_json::from_slice(&bytes).unwrap_or_default();
    paths
        .into_iter()
        .take(MAX_RECENT)
        .map(|p| {
            let path = std::path::Path::new(&p);
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let title = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            RecentEntry { id: path_id(&p), title, name, path: Some(p), pages: 0 }
        })
        .collect()
}

/// The thumbnail (PNG) of entry `id`, if one was kept.
pub fn thumbnail(s: &Session, id: &str) -> Option<Vec<u8>> {
    let e = entries(s).into_iter().find(|e| e.id == id)?;
    s.store.as_ref()?.read(&e.thumbnail_key())
}

/// A stable id for a file path (FNV-1a).
fn path_id(path: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("f{h:016x}")
}

fn store_list(s: &mut Session, list: &[RecentEntry]) -> std::result::Result<(), String> {
    let bytes = serde_json::to_vec(list).map_err(|e| e.to_string())?;
    let store = s.store.as_mut().ok_or("no store")?;
    store.write(LIST, &bytes)?;
    // The old list has been carried over.
    store.remove(LEGACY_LIST);
    Ok(())
}

/// Records document `index` as the most recent design (with a thumbnail, and in the browser a
/// copy of it). Failing to is logged, never an error: recent designs are a convenience.
pub(crate) fn remember(s: &mut Session, index: usize) {
    if let Err(e) = try_remember(s, index) {
        log::warn!("recent designs: {e}");
    }
}

fn try_remember(s: &mut Session, index: usize) -> std::result::Result<(), String> {
    if s.store.is_none() {
        return Ok(());
    }
    let Some(d) = s.docs.get(index) else { return Ok(()) };
    let mut list = entries(s);
    let path = d.path.as_ref().map(|p| p.display().to_string());
    let name = d.display_name();
    let id = match (&path, &d.recent_id) {
        (Some(p), _) => path_id(p),
        (None, Some(id)) => id.clone(),
        // The same download opened again is the same entry.
        (None, None) => list
            .iter()
            .find(|e| e.path.is_none() && e.name == name)
            .map(|e| e.id.clone())
            .unwrap_or_else(|| format!("d{}-{}", key_part(&s.instance), d.recovery_id)),
    };
    let entry = RecentEntry { id: key_part(&id), title: d.doc.title.clone(), name, path: path.clone(), pages: d.doc.pages.len() };
    let copy = if path.is_none() { Some(genha_format::to_bytes(&d.doc, true).map_err(|e| e.to_string())?) } else { None };
    let thumb = genha_render::render_thumbnail(&d.doc, 0, &s.fonts, THUMBNAIL_SIDE).ok().and_then(|r| genha_render::encode_png(&r).ok());
    if let Some(d) = s.docs.get_mut(index) {
        d.recent_id = Some(entry.id.clone());
    }
    list.retain(|e| e.id != entry.id);
    list.insert(0, entry.clone());
    let dropped: Vec<RecentEntry> = if list.len() > MAX_RECENT { list.split_off(MAX_RECENT) } else { vec![] };
    let store = s.store.as_mut().ok_or("no store")?;
    match copy {
        Some(bytes) => store.write(&entry.copy_key(), &bytes)?,
        None => store.remove(&entry.copy_key()),
    }
    match thumb {
        Some(png) => store.write(&entry.thumbnail_key(), &png)?,
        None => store.remove(&entry.thumbnail_key()),
    }
    for e in &dropped {
        store.remove(&e.copy_key());
        store.remove(&e.thumbnail_key());
    }
    store_list(s, &list)
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let kept: Vec<(String, u64)> = s.store.as_ref().map(|st| st.list("recent")).unwrap_or_default();
    let out: Vec<Value> = entries(s)
        .into_iter()
        .map(|e| {
            let thumb = e.thumbnail_key();
            let has_thumb = kept.iter().any(|(k, _)| *k == thumb);
            json!({
                "id": e.id,
                "title": e.title,
                "name": e.name,
                "path": e.path,
                "pages": e.pages,
                "thumbnail": if has_thumb { Some(thumb) } else { None },
            })
        })
        .collect();
    Ok(json!(out))
}

fn find(s: &Session, p: &Value, cmd: &str) -> Result<RecentEntry> {
    let id = str_req(p, "id", cmd)?;
    entries(s).into_iter().find(|e| e.id == id).ok_or_else(|| bad(cmd, format!("no recent design `{id}`")))
}

fn open(s: &mut Session, p: &Value) -> Result<Value> {
    let e = find(s, p, "recent.open")?;
    if let Some(path) = &e.path {
        return super::file::open_path(s, path);
    }
    let bytes =
        s.store.as_ref().and_then(|st| st.read(&e.copy_key())).ok_or_else(|| EngineError::Other(format!("the copy of {} is gone", e.title)))?;
    let (doc, notes) = genha_format::from_bytes(&bytes)?;
    let index = s.open_document(doc);
    if let Some(d) = s.docs.get_mut(index) {
        d.recent_id = Some(e.id.clone());
    }
    remember(s, index);
    Ok(ok(json!({"document": index, "repaired": notes})))
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let e = find(s, p, "recent.remove")?;
    let mut list = entries(s);
    list.retain(|x| x.id != e.id);
    if let Some(store) = s.store.as_mut() {
        store.remove(&e.copy_key());
        store.remove(&e.thumbnail_key());
    }
    store_list(s, &list).map_err(EngineError::Other)?;
    Ok(ok(json!({"left": list.len()})))
}
