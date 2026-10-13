//! Edit: undo/redo, selection, clipboard, duplicate, delete, batches.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use super::{always, bad, can_redo, can_undo, cmd, f64_or, has_doc, has_selection, ok, str_param, targets};
use crate::{CommandSpec, EngineError, HistoryEntry, Result, Session};
use genha_doc::{Asset, AssetId, Element, ElementKind, NodeId};
use genha_tools::Invocation;

/// Copied elements and the pictures they use (so pasting into another design works).
#[derive(Clone, Debug, Default)]
pub struct Clipboard {
    pub elements: Vec<Element>,
    pub assets: BTreeMap<AssetId, Asset>,
}

impl Clipboard {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

fn has_clipboard(s: &Session) -> std::result::Result<(), String> {
    has_doc(s)?;
    if s.clipboard.is_empty() { Err("the clipboard is empty".into()) } else { Ok(()) }
}

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!("edit.undo", "Undo", ["Edit"], Some("Cmd+Z"), "{}", can_undo, undo),
        cmd!("edit.redo", "Redo", ["Edit"], Some("Cmd+Shift+Z"), "{}", can_redo, redo),
        cmd!("edit.cut", "Cut", ["Edit"], Some("Cmd+X"), "{ids?}", has_selection, cut),
        cmd!("edit.copy", "Copy", ["Edit"], Some("Cmd+C"), "{ids?}", has_selection, copy),
        cmd!("edit.paste", "Paste", ["Edit"], Some("Cmd+V"), "{in_place?: bool}", has_clipboard, paste),
        cmd!("edit.duplicate", "Duplicate", ["Edit"], Some("Cmd+D"), "{ids?, offset?: px = 20}", has_selection, duplicate),
        cmd!("edit.delete", "Delete", ["Edit"], Some("Delete"), "{ids?}", has_selection, delete),
        cmd!("select.set", "Select", [], None, "{ids: [object ids on the current page]}", has_doc, select_set),
        cmd!("select.all", "Select All", ["Edit"], Some("Cmd+A"), "{}", has_doc, select_all),
        cmd!("select.none", "Deselect", ["Edit"], Some("Escape"), "{}", has_doc, select_none),
        cmd!("tool.set", "Choose Tool", [], None, "{tool: select|rect|ellipse|line|text|hand}", always, tool_set),
        cmd!(
            query "view.snapping",
            "Smart Guides",
            ["View"],
            None,
            "{on?: bool} — snap moved and resized objects to the page, other objects, equal gaps and matching sizes (toggles without `on`; hold Ctrl/Cmd while dragging to skip snapping once)",
            always,
            view_snapping
        ),
        cmd!(
            "command.batch",
            "Run Commands",
            [],
            None,
            "{label?, commands: [{command, params}]} — runs them as one undo step; stops (and rolls back) at the first error",
            always,
            batch
        ),
    ]
}

fn undo(s: &mut Session, _: &Value) -> Result<Value> {
    let d = s.doc_mut()?;
    let e = d.history.undo.pop().ok_or_else(|| EngineError::Other("nothing to undo".into()))?;
    let label = e.label.clone();
    d.history.redo.push(HistoryEntry { label: e.label, merge_key: None, doc: Arc::clone(&d.doc), selection: d.selection.clone(), page: d.page });
    d.doc = e.doc;
    d.selection = e.selection;
    d.page = e.page.min(d.doc.pages.len().saturating_sub(1));
    d.dirty = true;
    d.revision = d.revision.wrapping_add(1);
    Ok(ok(json!({"undone": label})))
}

fn redo(s: &mut Session, _: &Value) -> Result<Value> {
    let d = s.doc_mut()?;
    let e = d.history.redo.pop().ok_or_else(|| EngineError::Other("nothing to redo".into()))?;
    let label = e.label.clone();
    d.history.undo.push(HistoryEntry { label: e.label, merge_key: None, doc: Arc::clone(&d.doc), selection: d.selection.clone(), page: d.page });
    d.doc = e.doc;
    d.selection = e.selection;
    d.page = e.page.min(d.doc.pages.len().saturating_sub(1));
    d.dirty = true;
    d.revision = d.revision.wrapping_add(1);
    Ok(ok(json!({"redone": label})))
}

fn copy_to_clipboard(s: &mut Session, ids: &[NodeId]) -> Result<usize> {
    let d = s.doc()?;
    let mut clip = Clipboard::default();
    // Keep page order (bottom first), not selection order.
    if let Some(page) = d.doc.pages.get(d.page) {
        for e in &page.elements {
            if ids.contains(&e.id) {
                clip.elements.push(e.clone());
            }
        }
    }
    for id in ids {
        if !clip.elements.iter().any(|e| e.id == *id)
            && let Some((_, e)) = d.doc.find(*id)
        {
            clip.elements.push(e.clone());
        }
    }
    for e in &clip.elements {
        collect_assets(e, &mut |a| {
            if let Some(asset) = d.doc.assets.get(&a) {
                clip.assets.insert(a, asset.clone());
            }
        });
    }
    let n = clip.elements.len();
    s.clipboard = clip;
    Ok(n)
}

fn collect_assets(e: &Element, f: &mut dyn FnMut(AssetId)) {
    match &e.kind {
        ElementKind::Image(i) => f(i.asset),
        ElementKind::Group(g) => g.children.iter().for_each(|c| collect_assets(c, f)),
        _ => {}
    }
}

fn copy(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "edit.copy")?;
    let n = copy_to_clipboard(s, &ids)?;
    Ok(ok(json!({"copied": n})))
}

fn cut(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "edit.cut")?;
    copy_to_clipboard(s, &ids)?;
    remove(s, &ids, "Cut")
}

fn delete(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "edit.delete")?;
    remove(s, &ids, "Delete")
}

fn remove(s: &mut Session, ids: &[NodeId], label: &str) -> Result<Value> {
    let locked: Vec<NodeId> = ids.iter().filter(|id| s.doc().ok().and_then(|d| d.doc.find(**id)).is_some_and(|(_, e)| e.locked)).copied().collect();
    let n = s.edit(label, |doc, cx| {
        let mut n = 0;
        for id in ids {
            if !locked.contains(id) && doc.remove(*id).is_some() {
                n += 1;
            }
        }
        cx.selection.retain(|id| !ids.contains(id) || locked.contains(id));
        Ok(n)
    })?;
    Ok(ok(json!({"removed": n, "locked_kept": locked.len()})))
}

/// Inserts copies of `elements` (with fresh ids) on the current page, offset by `(dx, dy)`, above
/// the topmost of `above` (or at the top). Selects them; returns the new ids.
fn insert_copies(s: &mut Session, label: &str, elements: Vec<Element>, assets: BTreeMap<AssetId, Asset>, dx: f64, dy: f64) -> Result<Vec<NodeId>> {
    s.edit(label, |doc, cx| {
        // Pictures from another design come with their data; reuse identical ones.
        let mut remap: BTreeMap<AssetId, AssetId> = BTreeMap::new();
        for (old, asset) in assets {
            let existing = doc.assets.iter().find(|(_, a)| a.data == asset.data).map(|(id, _)| *id);
            let new = match existing {
                Some(id) => id,
                None => doc.add_asset(asset),
            };
            remap.insert(old, new);
        }
        let page_index = cx.page;
        let mut ids = vec![];
        let mut copies = vec![];
        for mut e in elements {
            doc.reassign_ids(&mut e);
            shift(&mut e, dx, dy);
            remap_assets(&mut e, &remap);
            e.locked = false;
            ids.push(e.id);
            copies.push(e);
        }
        let page = doc.page_mut(page_index)?;
        page.elements.extend(copies);
        cx.selection = ids.clone();
        Ok(ids)
    })
}

fn remap_assets(e: &mut Element, map: &BTreeMap<AssetId, AssetId>) {
    match &mut e.kind {
        ElementKind::Image(i) => {
            if let Some(n) = map.get(&i.asset) {
                i.asset = *n;
            }
        }
        ElementKind::Group(g) => g.children.iter_mut().for_each(|c| remap_assets(c, map)),
        _ => {}
    }
}

/// Moves an element (and a group's children) by `(dx, dy)`.
pub(crate) fn shift(e: &mut Element, dx: f64, dy: f64) {
    e.frame = e.frame.translated(dx, dy);
    if let ElementKind::Group(g) = &mut e.kind {
        for c in &mut g.children {
            shift(c, dx, dy);
        }
    }
}

fn paste(s: &mut Session, p: &Value) -> Result<Value> {
    let clip = s.clipboard.clone();
    let d = if super::bool_or(p, "in_place", false) { 0.0 } else { 20.0 };
    let ids = insert_copies(s, "Paste", clip.elements, clip.assets, d, d)?;
    Ok(ok(json!({"ids": ids})))
}

fn duplicate(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = targets(s, p, "edit.duplicate")?;
    let off = f64_or(p, "offset", 20.0);
    let d = s.doc()?;
    let mut els: Vec<Element> = vec![];
    if let Some(page) = d.doc.pages.get(d.page) {
        els.extend(page.elements.iter().filter(|e| ids.contains(&e.id)).cloned());
    }
    if els.is_empty() {
        return Err(bad("edit.duplicate", "select objects on the current page"));
    }
    let new = insert_copies(s, "Duplicate", els, BTreeMap::new(), off, off)?;
    Ok(ok(json!({"ids": new})))
}

fn select_set(s: &mut Session, p: &Value) -> Result<Value> {
    let ids: Vec<NodeId> = match p.get("ids") {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_u64).map(NodeId).collect(),
        _ => return Err(bad("select.set", "`ids` must be an array of object ids")),
    };
    s.set_selection(ids)?;
    let n = s.doc()?.selection.len();
    Ok(ok(json!({"selected": n})))
}

fn select_all(s: &mut Session, _: &Value) -> Result<Value> {
    let d = s.doc()?;
    let ids: Vec<NodeId> =
        d.doc.pages.get(d.page).map(|p| p.elements.iter().filter(|e| !e.locked && !e.hidden).map(|e| e.id).collect()).unwrap_or_default();
    s.set_selection(ids)?;
    Ok(ok(json!({"selected": s.doc()?.selection.len()})))
}

fn select_none(s: &mut Session, _: &Value) -> Result<Value> {
    s.set_selection(vec![])?;
    Ok(ok(json!({})))
}

fn tool_set(s: &mut Session, p: &Value) -> Result<Value> {
    let id = str_param(p, "tool").ok_or_else(|| bad("tool.set", "missing `tool`"))?;
    let t = genha_tools::ToolId::from_id(id).ok_or_else(|| bad("tool.set", format!("unknown tool `{id}`")))?;
    s.tools.set_tool(t);
    s.preview = None;
    Ok(ok(json!({"tool": id})))
}

/// Where the smart guides preference is kept.
const SNAPPING_KEY: &str = "settings/snapping";

fn view_snapping(s: &mut Session, p: &Value) -> Result<Value> {
    let on = match p.get("on") {
        None | Some(Value::Null) => !s.snapping,
        Some(v) => v.as_bool().ok_or_else(|| bad("view.snapping", "`on` must be true or false"))?,
    };
    s.snapping = on;
    s.preview = None;
    if let Some(store) = s.store.as_mut() {
        // Losing the preference only means it is back on next run: not worth failing the toggle.
        if let Err(e) = store.write(SNAPPING_KEY, if on { b"on" } else { b"off" }) {
            s.post(format!("Could not keep the smart guides setting: {e}"));
        }
    }
    Ok(ok(json!({"on": on})))
}

/// Reads the preferences kept in the session's store (smart guides on or off).
pub fn load_preferences(s: &mut Session) {
    if let Some(v) = s.store.as_ref().and_then(|st| st.read(SNAPPING_KEY)) {
        s.snapping = v.as_slice() != b"off";
    }
}

fn batch(s: &mut Session, p: &Value) -> Result<Value> {
    let list: Vec<Value> = p.get("commands").and_then(Value::as_array).cloned().ok_or_else(|| bad("command.batch", "missing `commands` array"))?;
    if list.len() > 10_000 {
        return Err(bad("command.batch", "at most 10000 commands per batch"));
    }
    let invs: Vec<Invocation> = list
        .iter()
        .map(|c| {
            let id = c.get("command").and_then(Value::as_str).ok_or_else(|| bad("command.batch", "each entry needs a `command`"))?;
            if id == "command.batch" {
                return Err(bad("command.batch", "batches cannot nest"));
            }
            Ok(Invocation::new(id, c.get("params").cloned().unwrap_or(json!({}))))
        })
        .collect::<Result<_>>()?;
    let label = str_param(p, "label").unwrap_or("Commands").to_string();
    let results = s.execute_batch(&label, &invs)?;
    Ok(ok(json!({"results": results})))
}
