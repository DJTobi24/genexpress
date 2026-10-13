//! Recent designs: the designs in My designs ([`super::designs`]) opened or changed last, newest
//! first. `recent.remove` only takes a design off this list (it stays in My designs).
//!
//! Before My designs, recent designs were kept on their own (`recent/list.json` with copies and
//! thumbnails under `recent/…`, and before that `recent.json`, a list of paths); [`legacy_entries`]
//! reads them so [`super::designs::migrate`] can move them in.

use serde::Deserialize;
use serde_json::{Value, json};

use super::designs::{DesignEntry, library};
use super::{always, bad, cmd, ok, str_req};
use crate::{CommandSpec, EngineError, Result, Session};

const LIST: &str = "recent/list.json";
/// What the desktop app kept before recent designs moved into the store: a JSON list of paths.
const LEGACY_LIST: &str = "recent.json";
/// Designs listed.
pub const MAX_RECENT: usize = 12;
/// Largest list file read.
const MAX_LIST_BYTES: usize = 1 << 20;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "recent.list",
            "Recent Designs",
            [],
            None,
            "{} — newest first: [{id, title, name, path?, pages, edited, folder?, thumbnail?: store key}] (ids are My designs' ids, see design.*)",
            always,
            list
        ),
        cmd!("recent.open", "Open Recent Design", [], None, "{id: from recent.list}", always, open),
        cmd!(query "recent.remove", "Remove from Recent Designs", [], None, "{id: from recent.list} — it stays in My designs", always, remove),
    ]
}

/// The recent designs, newest first (empty without a store).
pub fn entries(s: &Session) -> Vec<DesignEntry> {
    let mut list: Vec<DesignEntry> = library(s).designs.into_iter().filter(|e| e.recent).collect();
    list.sort_by_key(|e| std::cmp::Reverse(e.touched()));
    list.truncate(MAX_RECENT);
    list
}

/// The thumbnail (PNG) of design `id`, if one was kept.
pub fn thumbnail(s: &Session, id: &str) -> Option<Vec<u8>> {
    super::designs::thumbnail(s, id)
}

/// One entry of the older recent-designs list.
#[derive(Deserialize)]
struct OldEntry {
    id: String,
    title: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    pages: usize,
}

/// A stable id for a file path (FNV-1a), as the older list made them.
fn path_id(path: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("f{h:016x}")
}

pub(crate) fn legacy_copy_key(id: &str) -> String {
    format!("recent/{id}.{}", genha_format::EXTENSION)
}

pub(crate) fn legacy_thumbnail_key(id: &str) -> String {
    format!("recent/{id}.png")
}

/// Whether an older recent-designs list is kept.
pub(crate) fn has_legacy(s: &Session) -> bool {
    s.store.as_ref().is_some_and(|st| st.read(LIST).is_some() || st.read(LEGACY_LIST).is_some())
}

/// The older recent designs as My designs entries (newest first, with times that keep that order).
pub(crate) fn legacy_entries(s: &Session) -> Vec<DesignEntry> {
    let Some(store) = s.store.as_ref() else { return vec![] };
    let old: Vec<OldEntry> = if let Some(bytes) = store.read(LIST).filter(|b| b.len() <= MAX_LIST_BYTES) {
        serde_json::from_slice(&bytes).unwrap_or_default()
    } else if let Some(bytes) = store.read(LEGACY_LIST).filter(|b| b.len() <= MAX_LIST_BYTES) {
        let paths: Vec<String> = serde_json::from_slice(&bytes).unwrap_or_default();
        paths
            .into_iter()
            .map(|p| {
                let path = std::path::Path::new(&p);
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let title = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                OldEntry { id: path_id(&p), title, name, path: Some(p), pages: 0 }
            })
            .collect()
    } else {
        return vec![];
    };
    let kept = store.list("recent");
    let now = super::versions::now_secs();
    old.into_iter()
        .take(super::designs::MAX_DESIGNS)
        .enumerate()
        .map(|(i, e)| {
            let thumb = legacy_thumbnail_key(&e.id);
            let written = kept.iter().find(|(k, _)| *k == thumb).map(|(_, t)| *t).filter(|t| *t > 0);
            let t = written.unwrap_or_else(|| now.saturating_sub(i as u64 * 60));
            DesignEntry {
                id: e.id,
                title: e.title,
                name: e.name,
                path: e.path,
                pages: e.pages,
                width: 0.0,
                height: 0.0,
                created: t,
                edited: t,
                opened: t,
                folder: None,
                recent: true,
                version: 1,
            }
        })
        .collect()
}

/// Removes the older list and its files (after they were moved into My designs).
pub(crate) fn remove_legacy(s: &mut Session, moved: &[DesignEntry]) {
    let Some(store) = s.store.as_mut() else { return };
    for e in moved {
        store.remove(&legacy_copy_key(&e.id));
        store.remove(&legacy_thumbnail_key(&e.id));
    }
    store.remove(LIST);
    store.remove(LEGACY_LIST);
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let kept: Vec<(String, u64)> = s.store.as_ref().map(|st| st.list("designs")).unwrap_or_default();
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
                "edited": e.edited,
                "folder": e.folder,
                "thumbnail": if has_thumb { Some(thumb) } else { None },
            })
        })
        .collect();
    Ok(json!(out))
}

fn open(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "recent.open")?.to_string();
    super::designs::open_entry(s, &id, "recent.open")
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "recent.remove")?.to_string();
    if s.store.is_none() {
        return Err(EngineError::Other("there are no recent designs in this app".into()));
    }
    super::designs::migrate(s);
    let mut lib = library(s);
    let e = lib.designs.iter_mut().find(|e| e.id == id).ok_or_else(|| bad("recent.remove", format!("no recent design `{id}`")))?;
    e.recent = false;
    super::designs::store_library(s, &lib)?;
    Ok(ok(json!({"left": entries(s).len()})))
}
