//! My designs: every design made or opened in GenHa, in project folders, kept in the session's
//! [`Store`](crate::store::Store) under `designs/…` (desktop: the data folder; browser: IndexedDB).
//!
//! - `designs/index.json` lists the designs ([`DesignEntry`]) and folders ([`Folder`]).
//! - `designs/<id>.genha` is the copy of a design that lives only in the app (never saved to a
//!   file; in the browser every design), so it is never lost; `designs/<id>.png` its thumbnail.
//! - A design saved to a file points at the file (`path`) and keeps no copy.
//!
//! Designs go in by themselves: every design a command creates or opens is added
//! ([`adopt`], after each command), saving records its file ([`record`]), and `design.sync`
//! (the app runs it with Data Recovery and when going home) writes the copies of designs that
//! changed. The recent designs (`recent.*`) are the designs opened or changed last.
//!
//! Ids from parameters are only ever compared with the ids in the index, and every id in the
//! index is a safe store key part ([`crate::store::valid_key`]), so no parameter can reach
//! outside `designs/`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{always, bad, bool_or, cmd, has_doc, ok, str_param, str_req, usize_param};
use crate::store::{key_part, valid_key};
use crate::{CommandSpec, EngineError, Result, Session};
use genha_doc::Document;

const DIR: &str = "designs";
const INDEX: &str = "designs/index.json";
/// Most designs and folders kept in the index.
pub const MAX_DESIGNS: usize = 5000;
pub const MAX_FOLDERS: usize = 200;
/// Longest design or folder name, in characters.
pub const MAX_NAME: usize = 120;
/// Longest side of a thumbnail, in pixels.
pub const THUMBNAIL_SIDE: u32 = 320;
/// Largest index file read.
const MAX_INDEX_BYTES: usize = 16 << 20;
/// Most ids one `design.move` / `design.remove` takes.
const MAX_IDS: usize = MAX_DESIGNS;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "design.list",
            "My Designs",
            [],
            None,
            "{folder?: folder id | \"unfiled\" (default: all), query?: words in the title or file name, sort?: edited (default) | opened | created | name, limit?} — {designs: [{id, title, name, path?, pages, width, height, created, edited, opened, folder?, thumbnail?: store key, stored: bool (a copy kept by GenHa), open: bool}], folders: [{id, name, color?, count}], total, unfiled}",
            always,
            list
        ),
        cmd!("design.open", "Open from My Designs", [], None, "{id: from design.list} — switches to it when it is open already", always, open),
        cmd!(query "design.rename", "Rename in My Designs", [], None, "{id, title} — also renames the open design and the title kept in its file", always, rename),
        cmd!(query "design.duplicate", "Duplicate in My Designs", [], None, "{id, title?: default \"<title> (copy)\"} — a copy kept by GenHa, in the same folder: {id}", always, duplicate),
        cmd!(query "design.move", "Move to Folder", [], None, "{ids: [id…] (or id), folder: folder id | null (no folder)}", always, move_to),
        cmd!(
            query "design.remove",
            "Remove from My Designs",
            [],
            None,
            "{ids: [id…] (or id)} — copies kept by GenHa are deleted; files saved elsewhere are kept (only forgotten): {removed, deleted}",
            always,
            remove
        ),
        cmd!(query "design.add", "Add to My Designs", [], None, "{} — the active design (again, after it was removed): {id}", has_doc, add),
        cmd!(query "design.sync", "Update My Designs", [], None, "{} — writes the copies and thumbnails of open designs that changed: {updated}", always, sync_cmd),
        cmd!(query "folder.list", "Project Folders", [], None, "{} — [{id, name, color?, count}] in their order", always, folder_list),
        cmd!(query "folder.create", "New Folder", [], None, "{name, color?: #rrggbb} — {id}", always, folder_create),
        cmd!(query "folder.rename", "Rename Folder", [], None, "{id, name}", always, folder_rename),
        cmd!(query "folder.setColor", "Folder Colour", [], None, "{id, color: #rrggbb | null}", always, folder_set_color),
        cmd!(query "folder.move", "Move Folder", [], None, "{id, index: new position}", always, folder_move),
        cmd!(
            query "folder.delete",
            "Delete Folder",
            [],
            None,
            "{id, keep_designs?: bool = true} — its designs move out of the folder; with keep_designs false they are removed as by design.remove",
            always,
            folder_delete
        ),
    ]
}

/// One design in My designs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesignEntry {
    pub id: String,
    pub title: String,
    /// The file name it was opened or saved as (empty when it never had one).
    #[serde(default)]
    pub name: String,
    /// Its file, when it was saved to one (desktop).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub pages: usize,
    /// The first page's size (for the card's shape).
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    /// Seconds since 1970 (0: unknown).
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub edited: u64,
    #[serde(default)]
    pub opened: u64,
    /// Its project folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Listed among the recent designs (`recent.remove` hides it there).
    #[serde(default = "yes")]
    pub recent: bool,
    /// Bumped whenever its thumbnail is written again.
    #[serde(default)]
    pub version: u64,
}

fn yes() -> bool {
    true
}

impl DesignEntry {
    /// Store key of the copy kept by GenHa (designs without a file).
    pub fn copy_key(&self) -> String {
        format!("{DIR}/{}.{}", self.id, genha_format::EXTENSION)
    }
    /// Store key of the thumbnail (PNG).
    pub fn thumbnail_key(&self) -> String {
        format!("{DIR}/{}.png", self.id)
    }
    /// When it was last opened or changed (the recent designs' order).
    pub fn touched(&self) -> u64 {
        self.opened.max(self.edited)
    }
}

/// A project folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    /// `#rrggbb`, when one was chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub created: u64,
}

/// Everything in My designs: designs (newest first as added) and folders (in their order).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub designs: Vec<DesignEntry>,
    #[serde(default)]
    pub folders: Vec<Folder>,
}

impl Library {
    pub fn design(&self, id: &str) -> Option<&DesignEntry> {
        self.designs.iter().find(|e| e.id == id)
    }
    pub fn folder(&self, id: &str) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }
    /// Designs in folder `id`.
    pub fn count(&self, id: &str) -> usize {
        self.designs.iter().filter(|e| e.folder.as_deref() == Some(id)).count()
    }
}

/// Why a design is recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Why {
    /// Created or opened.
    Opened,
    /// Saved to a file (or downloaded).
    Saved,
    /// Its content changed in the app.
    Changed,
}

fn now() -> u64 {
    super::versions::now_secs()
}

/// `name` trimmed and cut to [`MAX_NAME`] characters.
fn clean_name(name: &str) -> String {
    name.trim().chars().filter(|c| !c.is_control()).take(MAX_NAME).collect::<String>().trim().to_string()
}

/// A safe, valid index: unique ids that are store keys, known folders, names cut to size.
fn sanitize(mut lib: Library) -> Library {
    let mut seen = std::collections::HashSet::new();
    lib.folders.retain(|f| !f.id.contains('/') && valid_key(&format!("{DIR}/{}", f.id)) && f.id.len() <= 80 && seen.insert(f.id.clone()));
    lib.folders.truncate(MAX_FOLDERS);
    for f in &mut lib.folders {
        f.name = clean_name(&f.name);
        if f.name.is_empty() {
            f.name = "Folder".into();
        }
        f.color = f.color.as_deref().and_then(genha_color::Color::parse).map(|c| c.to_hex());
    }
    let folders: std::collections::HashSet<String> = lib.folders.iter().map(|f| f.id.clone()).collect();
    let mut seen = std::collections::HashSet::new();
    lib.designs.retain(|e| !e.id.contains('/') && e.id.len() <= 80 && valid_key(&e.copy_key()) && seen.insert(e.id.clone()));
    lib.designs.truncate(MAX_DESIGNS);
    for e in &mut lib.designs {
        e.title = clean_name(&e.title);
        e.name = e.name.chars().take(260).collect();
        if e.folder.as_ref().is_some_and(|f| !folders.contains(f)) {
            e.folder = None;
        }
        if !(e.width.is_finite() && e.width >= 0.0) || !(e.height.is_finite() && e.height >= 0.0) {
            (e.width, e.height) = (0.0, 0.0);
        }
    }
    lib
}

/// My designs as kept in the store (empty without a store). Before the first change after an
/// update, the recent designs kept by older versions are listed (see [`migrate`]).
pub fn library(s: &Session) -> Library {
    let Some(store) = s.store.as_ref() else { return Library::default() };
    match store.read(INDEX) {
        Some(bytes) if bytes.len() <= MAX_INDEX_BYTES => match serde_json::from_slice::<Library>(&bytes) {
            Ok(lib) => sanitize(lib),
            Err(e) => {
                log::warn!("My designs: the index is damaged ({e}); listing the kept copies");
                rebuild(s)
            }
        },
        Some(_) => rebuild(s),
        None => sanitize(Library { designs: super::recent::legacy_entries(s), folders: vec![] }),
    }
}

/// The designs whose copies are in the store, when the index can't be read (their folders and
/// the designs saved to files are lost; the designs themselves are not).
fn rebuild(s: &Session) -> Library {
    let Some(store) = s.store.as_ref() else { return Library::default() };
    let ext = format!(".{}", genha_format::EXTENSION);
    let mut designs = vec![];
    for (key, modified) in store.list(DIR) {
        let Some(id) = key.strip_prefix("designs/").and_then(|k| k.strip_suffix(&ext)) else { continue };
        let doc = store.read(&key).and_then(|b| genha_format::from_bytes(&b).ok()).map(|(d, _)| d);
        let (title, pages, (width, height)) = match &doc {
            Some(d) => (d.title.clone(), d.pages.len(), first_size(d)),
            None => ("(damaged copy)".to_string(), 0, (0.0, 0.0)),
        };
        designs.push(DesignEntry {
            id: id.to_string(),
            title,
            name: String::new(),
            path: None,
            pages,
            width,
            height,
            created: modified,
            edited: modified,
            opened: 0,
            folder: None,
            recent: true,
            version: 1,
        });
    }
    designs.sort_by_key(|e| std::cmp::Reverse(e.edited));
    sanitize(Library { designs, folders: vec![] })
}

fn first_size(d: &Document) -> (f64, f64) {
    d.pages.first().map_or((0.0, 0.0), |p| (p.width, p.height))
}

/// Moves the recent designs kept by older versions (`recent/…`) into My designs, once.
pub fn migrate(s: &mut Session) {
    let Some(store) = s.store.as_ref() else { return };
    if store.read(INDEX).is_some() {
        return;
    }
    let old = super::recent::legacy_entries(s);
    if old.is_empty() && !super::recent::has_legacy(s) {
        return;
    }
    let lib = sanitize(Library { designs: old, folders: vec![] });
    let Some(store) = s.store.as_mut() else { return };
    for e in &lib.designs {
        for (from, to) in [(super::recent::legacy_copy_key(&e.id), e.copy_key()), (super::recent::legacy_thumbnail_key(&e.id), e.thumbnail_key())] {
            if let Some(bytes) = store.read(&from)
                && let Err(err) = store.write(&to, &bytes)
            {
                log::warn!("My designs: could not move {from}: {err}");
                return;
            }
        }
    }
    if let Err(e) = write_index(s, &lib) {
        log::warn!("My designs: {e}");
        return;
    }
    // Carried over: the old list and files go.
    super::recent::remove_legacy(s, &lib.designs);
}

fn write_index(s: &mut Session, lib: &Library) -> std::result::Result<(), String> {
    let bytes = serde_json::to_vec(lib).map_err(|e| e.to_string())?;
    let store = s.store.as_mut().ok_or("there is nowhere to keep My designs in this app")?;
    store.write(INDEX, &bytes)?;
    s.library_rev = s.library_rev.wrapping_add(1);
    Ok(())
}

/// The library for a change (older recent designs moved in first).
fn load_mut(s: &mut Session) -> Result<Library> {
    if s.store.is_none() {
        return Err(no_store());
    }
    migrate(s);
    Ok(library(s))
}

fn save(s: &mut Session, lib: &Library) -> Result<()> {
    write_index(s, lib).map_err(EngineError::Other)
}

fn no_store() -> EngineError {
    EngineError::Other("My designs is off (no place to keep designs)".into())
}

/// The thumbnail (PNG) of design `id`, if one was kept.
pub fn thumbnail(s: &Session, id: &str) -> Option<Vec<u8>> {
    let lib = library(s);
    let e = lib.design(id)?;
    s.store.as_ref()?.read(&e.thumbnail_key())
}

fn render_thumbnail(s: &Session, doc: &Document) -> Option<Vec<u8>> {
    genha_render::render_thumbnail(doc, 0, &s.fonts, THUMBNAIL_SIDE).ok().and_then(|r| genha_render::encode_png(&r).ok())
}

/// A new id, unique in `lib`.
fn new_id(lib: &Library, prefix: char, seed: &str) -> String {
    let base = key_part(&format!("{prefix}{:x}-{seed}", now()));
    let mut id = base.clone();
    let mut n = 2;
    while lib.design(&id).is_some() || lib.folder(&id).is_some() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Records document `index` in My designs (adding it when new) and writes its copy and thumbnail
/// when they changed. Failing to is logged, never an error for the command that triggered it.
pub(crate) fn record(s: &mut Session, index: usize, why: Why) {
    if s.store.is_none() {
        return;
    }
    if let Err(e) = try_record(s, index, why) {
        log::warn!("My designs: {e}");
    }
}

fn try_record(s: &mut Session, index: usize, why: Why) -> Result<()> {
    let Some(d) = s.docs.get(index) else { return Ok(()) };
    if d.library_removed {
        return Ok(());
    }
    let doc = Arc::clone(&d.doc);
    let path = d.path.as_ref().map(|p| p.display().to_string());
    let name = d.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let changed = d.library_synced.as_ref().is_none_or(|old| !Arc::ptr_eq(old, &doc));
    let known = d.library_id.clone();
    let seed = format!("{}-{}", s.instance, d.recovery_id);
    let mut lib = load_mut(s)?;
    let pos = known
        .as_ref()
        .and_then(|id| lib.designs.iter().position(|e| &e.id == id))
        .or_else(|| path.as_ref().and_then(|p| lib.designs.iter().position(|e| e.path.as_ref() == Some(p))));
    let t = now();
    let mut entry = match pos {
        Some(i) => lib.designs.remove(i),
        None => {
            if lib.designs.len() >= MAX_DESIGNS {
                return Err(EngineError::Other(format!("My designs is full ({MAX_DESIGNS} designs)")));
            }
            DesignEntry {
                id: known.filter(|id| lib.design(id).is_none() && valid_key(&format!("{DIR}/{id}"))).unwrap_or_else(|| new_id(&lib, 'd', &seed)),
                title: String::new(),
                name: String::new(),
                path: None,
                pages: 0,
                width: 0.0,
                height: 0.0,
                created: t,
                edited: t,
                opened: t,
                folder: None,
                recent: true,
                version: 0,
            }
        }
    };
    let is_new = pos.is_none();
    entry.title = clean_name(&doc.title);
    if !name.is_empty() {
        entry.name = name;
    }
    entry.path = path.clone();
    entry.pages = doc.pages.len();
    (entry.width, entry.height) = first_size(&doc);
    entry.recent = true;
    match why {
        Why::Opened => entry.opened = t,
        Why::Saved | Why::Changed => entry.edited = t,
    }
    let write_content = is_new || changed || why == Why::Saved;
    let copy = if path.is_none() && write_content { Some(genha_format::to_bytes(&doc, true)?) } else { None };
    let thumb = if write_content { render_thumbnail(s, &doc) } else { None };
    if write_content {
        entry.version = entry.version.wrapping_add(1);
    }
    let store = s.store.as_mut().ok_or_else(no_store)?;
    if let Some(bytes) = &copy {
        store.write(&entry.copy_key(), bytes).map_err(EngineError::Other)?;
    }
    if path.is_some() {
        // Saved to a file: the file is the design now.
        store.remove(&entry.copy_key());
    }
    if write_content {
        match &thumb {
            Some(png) => store.write(&entry.thumbnail_key(), png).map_err(EngineError::Other)?,
            None => store.remove(&entry.thumbnail_key()),
        }
    }
    let id = entry.id.clone();
    lib.designs.insert(0, entry);
    save(s, &lib)?;
    if let Some(d) = s.docs.get_mut(index) {
        d.library_id = Some(id);
        d.library_synced = Some(doc);
    }
    Ok(())
}

/// Adds the open designs that are not in My designs yet (after every command).
pub(crate) fn adopt(s: &mut Session) {
    if s.store.is_none() {
        return;
    }
    for i in 0..s.docs.len() {
        let needs = s.docs.get(i).is_some_and(|d| d.library_id.is_none() && !d.library_removed);
        if needs {
            record(s, i, Why::Opened);
            // Tried once: a failure is retried by `design.sync`, not after every command.
            if let Some(d) = s.docs.get_mut(i)
                && d.library_id.is_none()
            {
                d.library_id = Some(String::new());
            }
        }
    }
}

/// Writes the copy and thumbnail of document `index` when it changed (designs kept by GenHa;
/// designs saved to files are recorded when saved). Whether anything was written.
pub(crate) fn sync_one(s: &mut Session, index: usize) -> bool {
    let Some(d) = s.docs.get(index) else { return false };
    if d.library_removed || s.store.is_none() {
        return false;
    }
    let listed = d.library_id.as_ref().is_some_and(|id| !id.is_empty() && library(s).design(id).is_some());
    let changed = d.library_synced.as_ref().is_none_or(|old| !Arc::ptr_eq(old, &d.doc));
    if listed && (d.path.is_some() || !changed) {
        return false;
    }
    if !listed && let Some(d) = s.docs.get_mut(index) {
        // Gone from the index (or never got in): added again.
        d.library_id = None;
    }
    record(s, index, if listed { Why::Changed } else { Why::Opened });
    true
}

fn sync_cmd(s: &mut Session, _: &Value) -> Result<Value> {
    if s.store.is_none() {
        return Err(no_store());
    }
    let mut updated = 0;
    for i in 0..s.docs.len() {
        if sync_one(s, i) {
            updated += 1;
        }
    }
    Ok(ok(json!({"updated": updated})))
}

fn entry_json(s: &Session, e: &DesignEntry, kept: &[(String, u64)]) -> Value {
    let thumb = e.thumbnail_key();
    // A full listing may leave some out: then one was written when it has a version.
    let has_thumb = kept.iter().any(|(k, _)| *k == thumb) || (kept.len() >= crate::store::MAX_LIST && e.version > 0);
    let open = s.docs.iter().any(|d| d.library_id.as_deref() == Some(e.id.as_str()));
    json!({
        "id": e.id,
        "title": e.title,
        "name": e.name,
        "path": e.path,
        "pages": e.pages,
        "width": e.width,
        "height": e.height,
        "created": e.created,
        "edited": e.edited,
        "opened": e.opened,
        "folder": e.folder,
        "thumbnail": if has_thumb { Some(thumb) } else { None },
        "stored": e.path.is_none(),
        "open": open,
    })
}

/// Which designs `design.list` shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FolderFilter {
    #[default]
    All,
    Unfiled,
    Folder(String),
}

/// How `design.list` sorts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Edited,
    Opened,
    Created,
    Name,
}

impl Sort {
    pub fn parse(s: &str) -> Option<Sort> {
        Some(match s {
            "edited" => Sort::Edited,
            "opened" => Sort::Opened,
            "created" => Sort::Created,
            "name" => Sort::Name,
            _ => return None,
        })
    }
    pub fn id(self) -> &'static str {
        match self {
            Sort::Edited => "edited",
            Sort::Opened => "opened",
            Sort::Created => "created",
            Sort::Name => "name",
        }
    }
}

/// Whether `text` contains every word of `query` (case-insensitive; an empty query matches).
pub fn matches(query: &str, texts: &[&str]) -> bool {
    let hay: Vec<String> = texts.iter().map(|t| t.to_lowercase()).collect();
    query.to_lowercase().split_whitespace().all(|w| hay.iter().any(|h| h.contains(w)))
}

/// The designs of `lib` in `folder` matching `query`, sorted.
pub fn filtered<'a>(lib: &'a Library, folder: &FolderFilter, query: &str, sort: Sort) -> Vec<&'a DesignEntry> {
    let mut v: Vec<&DesignEntry> = lib
        .designs
        .iter()
        .filter(|e| match folder {
            FolderFilter::All => true,
            FolderFilter::Unfiled => e.folder.is_none(),
            FolderFilter::Folder(id) => e.folder.as_deref() == Some(id.as_str()),
        })
        .filter(|e| matches(query, &[&e.title, &e.name]))
        .collect();
    match sort {
        Sort::Edited => v.sort_by_key(|e| std::cmp::Reverse(e.edited.max(e.created))),
        Sort::Opened => v.sort_by_key(|e| std::cmp::Reverse(e.touched())),
        Sort::Created => v.sort_by_key(|e| std::cmp::Reverse(e.created)),
        Sort::Name => v.sort_by_key(|e| e.title.to_lowercase()),
    }
    v
}

fn folders_json(lib: &Library) -> Vec<Value> {
    lib.folders.iter().map(|f| json!({"id": f.id, "name": f.name, "color": f.color, "count": lib.count(&f.id)})).collect()
}

fn list(s: &mut Session, p: &Value) -> Result<Value> {
    let lib = library(s);
    let folder = match p.get("folder") {
        None | Some(Value::Null) => FolderFilter::All,
        Some(Value::String(f)) if f == "unfiled" => FolderFilter::Unfiled,
        Some(Value::String(f)) if f == "all" => FolderFilter::All,
        Some(Value::String(f)) => {
            lib.folder(f).ok_or_else(|| bad("design.list", format!("no folder `{f}` (see folder.list)")))?;
            FolderFilter::Folder(f.clone())
        }
        Some(_) => return Err(bad("design.list", "`folder` must be a folder id, \"unfiled\" or null")),
    };
    let sort = match str_param(p, "sort") {
        None => Sort::Edited,
        Some(x) => Sort::parse(x).ok_or_else(|| bad("design.list", format!("unknown sort `{x}` (edited, opened, created, name)")))?,
    };
    let query = str_param(p, "query").unwrap_or("");
    let limit = usize_param(p, "limit").unwrap_or(MAX_DESIGNS);
    let kept: Vec<(String, u64)> = s.store.as_ref().map(|st| st.list(DIR)).unwrap_or_default();
    let found = filtered(&lib, &folder, query, sort);
    let total = found.len();
    let designs: Vec<Value> = found.into_iter().take(limit).map(|e| entry_json(s, e, &kept)).collect();
    let unfiled = lib.designs.iter().filter(|e| e.folder.is_none()).count();
    Ok(json!({"designs": designs, "folders": folders_json(&lib), "total": total, "unfiled": unfiled}))
}

fn find(s: &Session, p: &Value, cmd: &str) -> Result<DesignEntry> {
    let id = str_req(p, "id", cmd)?;
    library(s).design(id).cloned().ok_or_else(|| bad(cmd, format!("no design `{id}` in My designs (see design.list)")))
}

/// Opens design `id` from My designs (or switches to it when open).
pub(crate) fn open_entry(s: &mut Session, id: &str, cmd: &str) -> Result<Value> {
    if s.store.is_none() {
        return Err(no_store());
    }
    if id.is_empty() {
        return Err(bad(cmd, "missing `id`"));
    }
    if let Some(i) = s.docs.iter().position(|d| d.library_id.as_deref() == Some(id)) {
        s.active = i;
        s.preview = None;
        return Ok(ok(json!({"document": i, "already_open": true})));
    }
    let e = library(s).design(id).cloned().ok_or_else(|| bad(cmd, format!("no design `{id}` in My designs (see design.list)")))?;
    if let Some(path) = &e.path {
        return super::file::open_path(s, path);
    }
    let bytes =
        s.store.as_ref().and_then(|st| st.read(&e.copy_key())).ok_or_else(|| EngineError::Other(format!("the copy of {} is gone", e.title)))?;
    let (doc, notes) = genha_format::from_bytes(&bytes)?;
    let index = s.open_document(doc);
    if let Some(d) = s.docs.get_mut(index) {
        d.library_id = Some(e.id.clone());
        d.library_synced = Some(Arc::clone(&d.doc));
    }
    record(s, index, Why::Opened);
    Ok(ok(json!({"document": index, "repaired": notes})))
}

fn open(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_req(p, "id", "design.open")?.to_string();
    open_entry(s, &id, "design.open")
}

/// `doc` with a new title, as `.genha` bytes.
fn retitled(bytes: &[u8], title: &str) -> Result<(Document, Vec<u8>)> {
    let (mut doc, _) = genha_format::from_bytes(bytes)?;
    doc.title = title.to_string();
    let out = genha_format::to_bytes(&doc, true)?;
    Ok((doc, out))
}

fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let e = find(s, p, "design.rename")?;
    let title = clean_name(str_req(p, "title", "design.rename")?);
    if title.is_empty() {
        return Err(bad("design.rename", "the title is empty"));
    }
    let open: Vec<usize> = s.docs.iter().enumerate().filter(|(_, d)| d.library_id.as_deref() == Some(e.id.as_str())).map(|(i, _)| i).collect();
    // The kept content first (nothing changes when that fails).
    if open.is_empty() {
        match &e.path {
            None => {
                if let Some(bytes) = s.store.as_ref().and_then(|st| st.read(&e.copy_key())) {
                    let (_, out) = retitled(&bytes, &title)?;
                    s.store.as_mut().ok_or_else(no_store)?.write(&e.copy_key(), &out).map_err(EngineError::Other)?;
                }
            }
            Some(path) => rename_file(path, &title)?,
        }
    }
    for i in open {
        if let Some(d) = s.docs.get_mut(i) {
            let mut doc = (*d.doc).clone();
            doc.title = title.clone();
            d.doc = Arc::new(doc);
            d.dirty = true;
            d.revision = d.revision.wrapping_add(1);
        }
    }
    let mut lib = load_mut(s)?;
    if let Some(x) = lib.designs.iter_mut().find(|x| x.id == e.id) {
        x.title = title.clone();
    }
    save(s, &lib)?;
    Ok(ok(json!({"title": title})))
}

/// Changes the title kept in a design file (GenHa files only).
#[cfg(not(target_arch = "wasm32"))]
fn rename_file(path: &str, title: &str) -> Result<()> {
    let p = std::path::Path::new(path);
    let native =
        p.extension().is_some_and(|x| x.eq_ignore_ascii_case(genha_format::EXTENSION) || x.eq_ignore_ascii_case(genha_format::LEGACY_EXTENSION));
    if !native {
        return Ok(());
    }
    let meta = std::fs::metadata(p).map_err(|e| EngineError::Other(format!("cannot read {path}: {e}")))?;
    if meta.len() > genha_format::MAX_FILE_BYTES {
        return Err(EngineError::Other(format!("{path} is too large")));
    }
    let bytes = std::fs::read(p).map_err(|e| EngineError::Other(format!("cannot read {path}: {e}")))?;
    let (_, out) = retitled(&bytes, title)?;
    genha_format::write_atomic(p, &out)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn rename_file(_: &str, _: &str) -> Result<()> {
    Ok(())
}

/// The design's content: the open design, its copy, or its file.
fn content(s: &Session, e: &DesignEntry) -> Result<Vec<u8>> {
    if let Some(d) = s.docs.iter().find(|d| d.library_id.as_deref() == Some(e.id.as_str())) {
        return Ok(genha_format::to_bytes(&d.doc, true)?);
    }
    match &e.path {
        None => s.store.as_ref().and_then(|st| st.read(&e.copy_key())).ok_or_else(|| EngineError::Other(format!("the copy of {} is gone", e.title))),
        Some(path) => read_file(path),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &str) -> Result<Vec<u8>> {
    let meta = std::fs::metadata(path).map_err(|e| EngineError::Other(format!("cannot open {path}: {e}")))?;
    if meta.len() > genha_format::MAX_FILE_BYTES {
        return Err(EngineError::Other(format!("{path} is too large")));
    }
    std::fs::read(path).map_err(|e| EngineError::Other(format!("cannot read {path}: {e}")))
}

#[cfg(target_arch = "wasm32")]
fn read_file(path: &str) -> Result<Vec<u8>> {
    Err(EngineError::Other(format!("cannot open {path} in the browser")))
}

fn duplicate(s: &mut Session, p: &Value) -> Result<Value> {
    let e = find(s, p, "design.duplicate")?;
    let title = match str_param(p, "title").map(clean_name).filter(|t| !t.is_empty()) {
        Some(t) => t,
        None => clean_name(&genha_i18n::tr!("{title} (copy)", title = e.title)),
    };
    let bytes = content(s, &e)?;
    let (doc, out) = retitled(&bytes, &title)?;
    let mut lib = load_mut(s)?;
    if lib.designs.len() >= MAX_DESIGNS {
        return Err(EngineError::Other(format!("My designs is full ({MAX_DESIGNS} designs)")));
    }
    let t = now();
    let (width, height) = first_size(&doc);
    let copy = DesignEntry {
        id: new_id(&lib, 'd', &format!("{}-copy", s.instance)),
        title,
        name: String::new(),
        path: None,
        pages: doc.pages.len(),
        width,
        height,
        created: t,
        edited: t,
        opened: 0,
        folder: e.folder.clone(),
        recent: true,
        version: 1,
    };
    let thumb = s.store.as_ref().and_then(|st| st.read(&e.thumbnail_key())).or_else(|| render_thumbnail(s, &doc));
    let store = s.store.as_mut().ok_or_else(no_store)?;
    store.write(&copy.copy_key(), &out).map_err(EngineError::Other)?;
    if let Some(png) = thumb {
        store.write(&copy.thumbnail_key(), &png).map_err(EngineError::Other)?;
    }
    let id = copy.id.clone();
    lib.designs.insert(0, copy);
    save(s, &lib)?;
    Ok(ok(json!({"id": id})))
}

/// The ids named by `ids` (an array) or `id`.
fn ids_param(p: &Value, cmd: &str) -> Result<Vec<String>> {
    match (p.get("ids"), p.get("id")) {
        (Some(Value::Array(a)), _) => {
            if a.len() > MAX_IDS {
                return Err(bad(cmd, format!("at most {MAX_IDS} ids")));
            }
            a.iter().map(|v| v.as_str().map(str::to_string).ok_or_else(|| bad(cmd, "`ids` must be design ids (strings)"))).collect()
        }
        (Some(_), _) => Err(bad(cmd, "`ids` must be an array of design ids")),
        (None, Some(Value::String(id))) => Ok(vec![id.clone()]),
        _ => Err(bad(cmd, "missing `ids` (or `id`)")),
    }
}

fn move_to(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = ids_param(p, "design.move")?;
    let mut lib = load_mut(s)?;
    let folder = match p.get("folder") {
        Some(Value::Null) => None,
        Some(Value::String(f)) if f == "unfiled" => None,
        Some(Value::String(f)) => Some(lib.folder(f).ok_or_else(|| bad("design.move", format!("no folder `{f}` (see folder.list)")))?.id.clone()),
        _ => return Err(bad("design.move", "missing `folder` (a folder id, or null for no folder)")),
    };
    for id in &ids {
        if lib.design(id).is_none() {
            return Err(bad("design.move", format!("no design `{id}` in My designs")));
        }
    }
    let mut moved = 0;
    for e in lib.designs.iter_mut().filter(|e| ids.contains(&e.id)) {
        e.folder = folder.clone();
        moved += 1;
    }
    save(s, &lib)?;
    Ok(ok(json!({"moved": moved})))
}

/// Removes designs `ids` from `lib` (deleting their copies); (removed, copies deleted).
fn remove_ids(s: &mut Session, lib: &mut Library, ids: &[String]) -> (usize, usize) {
    let gone: Vec<DesignEntry> = lib.designs.iter().filter(|e| ids.contains(&e.id)).cloned().collect();
    lib.designs.retain(|e| !ids.contains(&e.id));
    let mut deleted = 0;
    if let Some(store) = s.store.as_mut() {
        for e in &gone {
            if e.path.is_none() {
                deleted += 1;
            }
            store.remove(&e.copy_key());
            store.remove(&e.thumbnail_key());
        }
    }
    for d in s.docs.iter_mut().filter(|d| d.library_id.as_ref().is_some_and(|id| ids.contains(id))) {
        d.library_id = None;
        d.library_synced = None;
        d.library_removed = true;
    }
    (gone.len(), deleted)
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = ids_param(p, "design.remove")?;
    let mut lib = load_mut(s)?;
    for id in &ids {
        if lib.design(id).is_none() {
            return Err(bad("design.remove", format!("no design `{id}` in My designs")));
        }
    }
    let (removed, deleted) = remove_ids(s, &mut lib, &ids);
    save(s, &lib)?;
    Ok(ok(json!({"removed": removed, "deleted": deleted})))
}

fn add(s: &mut Session, _: &Value) -> Result<Value> {
    if s.store.is_none() {
        return Err(no_store());
    }
    let i = s.active;
    if let Some(d) = s.docs.get_mut(i) {
        d.library_removed = false;
        if d.library_id.as_deref() == Some("") {
            d.library_id = None;
        }
    }
    try_record(s, i, Why::Changed)?;
    let id = s.docs.get(i).and_then(|d| d.library_id.clone()).unwrap_or_default();
    Ok(ok(json!({"id": id})))
}

// ---------- folders ----------

fn folder_list(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!(folders_json(&library(s))))
}

fn color_value(p: &Value, cmd: &str) -> Result<Option<String>> {
    Ok(super::color_param(p, "color", cmd)?.map(|c| c.to_hex()))
}

fn folder_create(s: &mut Session, p: &Value) -> Result<Value> {
    let name = clean_name(str_req(p, "name", "folder.create")?);
    if name.is_empty() {
        return Err(bad("folder.create", "the name is empty"));
    }
    let color = color_value(p, "folder.create")?;
    let mut lib = load_mut(s)?;
    if lib.folders.len() >= MAX_FOLDERS {
        return Err(EngineError::Other(format!("at most {MAX_FOLDERS} folders")));
    }
    let id = new_id(&lib, 'p', &key_part(&name.to_lowercase()).chars().take(24).collect::<String>());
    lib.folders.push(Folder { id: id.clone(), name, color, created: now() });
    save(s, &lib)?;
    Ok(ok(json!({"id": id})))
}

fn find_folder(lib: &Library, p: &Value, cmd: &str) -> Result<usize> {
    let id = str_req(p, "id", cmd)?;
    lib.folders.iter().position(|f| f.id == id).ok_or_else(|| bad(cmd, format!("no folder `{id}` (see folder.list)")))
}

fn folder_rename(s: &mut Session, p: &Value) -> Result<Value> {
    let name = clean_name(str_req(p, "name", "folder.rename")?);
    if name.is_empty() {
        return Err(bad("folder.rename", "the name is empty"));
    }
    let mut lib = load_mut(s)?;
    let i = find_folder(&lib, p, "folder.rename")?;
    if let Some(f) = lib.folders.get_mut(i) {
        f.name = name;
    }
    save(s, &lib)?;
    Ok(ok(json!({})))
}

fn folder_set_color(s: &mut Session, p: &Value) -> Result<Value> {
    let color = color_value(p, "folder.setColor")?;
    let mut lib = load_mut(s)?;
    let i = find_folder(&lib, p, "folder.setColor")?;
    if let Some(f) = lib.folders.get_mut(i) {
        f.color = color;
    }
    save(s, &lib)?;
    Ok(ok(json!({})))
}

fn folder_move(s: &mut Session, p: &Value) -> Result<Value> {
    let to = usize_param(p, "index").ok_or_else(|| bad("folder.move", "missing `index`"))?;
    let mut lib = load_mut(s)?;
    let i = find_folder(&lib, p, "folder.move")?;
    let f = lib.folders.remove(i);
    let to = to.min(lib.folders.len());
    lib.folders.insert(to, f);
    save(s, &lib)?;
    Ok(ok(json!({"index": to})))
}

fn folder_delete(s: &mut Session, p: &Value) -> Result<Value> {
    let keep = bool_or(p, "keep_designs", true);
    let mut lib = load_mut(s)?;
    let i = find_folder(&lib, p, "folder.delete")?;
    let f = lib.folders.remove(i);
    let inside: Vec<String> = lib.designs.iter().filter(|e| e.folder.as_deref() == Some(f.id.as_str())).map(|e| e.id.clone()).collect();
    let (mut removed, mut deleted) = (0, 0);
    if keep {
        for e in lib.designs.iter_mut().filter(|e| e.folder.as_deref() == Some(f.id.as_str())) {
            e.folder = None;
        }
    } else {
        (removed, deleted) = remove_ids(s, &mut lib, &inside);
    }
    save(s, &lib)?;
    Ok(ok(json!({"moved_out": if keep { inside.len() } else { 0 }, "removed": removed, "deleted": deleted})))
}

/// Writes `lib` as My designs' index.
pub(crate) fn store_library(s: &mut Session, lib: &Library) -> Result<()> {
    save(s, lib)
}
