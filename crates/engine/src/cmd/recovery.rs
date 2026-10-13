//! Data Recovery: copies of unsaved designs written in the background, offered again after a crash.
//!
//! The app gives the session a [`Store`](crate::store::Store) and runs `recovery.save` every
//! minute. Copies are kept as `recovery/<instance>-<document>.genha` (the instance names this run:
//! the process id on the desktop, the page load in the browser); copies from other runs (that
//! ended unexpectedly, or a browser tab that was closed or reloaded) are what `recovery.list`
//! offers. Saving or closing a design removes its copy. The same code runs on the desktop (a
//! folder) and in the browser (IndexedDB).

use serde_json::{Value, json};

use super::{always, bad, cmd, ok, str_param};
use crate::{CommandSpec, EngineError, Result, Session};

/// The store folder copies are kept in.
const DIR: &str = "recovery";

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "recovery.save", "Save Recovery Copies", [], None, "{} — writes a copy of every design with unsaved changes to the recovery folder", always, save),
        cmd!(query "recovery.list", "Recovered Designs", [], None, "{} — copies left by a previous session that ended unexpectedly: [{path, key, title, modified}]", always, list),
        cmd!("recovery.open", "Open Recovered Design", [], None, "{path or key: from recovery.list}", always, open),
        cmd!(query "recovery.discard", "Discard Recovered Design", [], None, "{path or key: from recovery.list}", always, discard),
    ]
}

fn own_prefix(s: &Session) -> String {
    format!("{DIR}/{}-", crate::store::key_part(&s.instance))
}

fn copy_key(s: &Session, recovery_id: u64) -> String {
    format!("{}{recovery_id}.{}", own_prefix(s), genha_format::EXTENSION)
}

/// Removes document `index`'s recovery copy (after it was saved or closed).
pub(crate) fn forget(s: &mut Session, index: usize) {
    let Some(id) = s.docs.get(index).map(|d| d.recovery_id) else { return };
    let key = copy_key(s, id);
    if let Some(store) = s.store.as_mut() {
        store.remove(&key);
    }
}

fn no_store() -> EngineError {
    EngineError::Other("Data Recovery is off (no place to keep copies)".into())
}

/// Copies from other runs: (key, modified).
fn others(s: &Session) -> Vec<(String, u64)> {
    let Some(store) = s.store.as_ref() else { return vec![] };
    let own = own_prefix(s);
    let ext = format!(".{}", genha_format::EXTENSION);
    store.list(DIR).into_iter().filter(|(k, _)| !k.starts_with(&own) && k.ends_with(&ext)).collect()
}

/// The copy a caller named by `key` or `path` (as `recovery.list` gave them); only listed copies
/// are accepted, so nothing outside the recovery folder can be read or removed.
fn named(s: &Session, p: &Value, cmd: &str) -> Result<String> {
    let store = s.store.as_ref().ok_or_else(no_store)?;
    let wanted = str_param(p, "key").or_else(|| str_param(p, "path")).ok_or_else(|| bad(cmd, "missing `key` (or `path`) from recovery.list"))?;
    others(s)
        .into_iter()
        .map(|(k, _)| k)
        .find(|k| k == wanted || store.locate(k) == wanted)
        .ok_or_else(|| bad(cmd, "not a copy in the recovery folder"))
}

fn save(s: &mut Session, _: &Value) -> Result<Value> {
    if s.store.is_none() {
        return Err(no_store());
    }
    let mut copies = vec![];
    for d in &s.docs {
        let key = copy_key(s, d.recovery_id);
        if d.dirty {
            copies.push((key, Some(genha_format::to_bytes(&d.doc, true)?)));
        } else {
            // A design saved since the last copy needs none.
            copies.push((key, None));
        }
    }
    let store = s.store.as_mut().ok_or_else(no_store)?;
    let mut written = vec![];
    for (key, bytes) in copies {
        match bytes {
            Some(b) => {
                store.write(&key, &b).map_err(EngineError::Other)?;
                written.push(store.locate(&key));
            }
            None => store.remove(&key),
        }
    }
    Ok(ok(json!({"written": written})))
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let Some(store) = s.store.as_ref() else { return Ok(json!([])) };
    let mut out = vec![];
    for (key, modified) in others(s) {
        let title = store.read(&key).and_then(|b| genha_format::from_bytes(&b).ok()).map(|(d, _)| d.title);
        out.push(json!({
            "path": store.locate(&key),
            "key": key,
            "title": title.unwrap_or_else(|| "(damaged copy)".into()),
            "modified": modified,
        }));
    }
    Ok(json!(out))
}

fn open(s: &mut Session, p: &Value) -> Result<Value> {
    let key = named(s, p, "recovery.open")?;
    let bytes = s.store.as_ref().and_then(|st| st.read(&key)).ok_or_else(|| EngineError::Other("the recovered copy can no longer be read".into()))?;
    let (doc, notes) = genha_format::from_bytes(&bytes)?;
    let index = s.open_document(doc);
    if let Some(d) = s.active_mut() {
        d.dirty = true;
    }
    // Write this run's copy before removing the old one, so the design is never unprotected.
    save(s, &json!({}))?;
    if let Some(store) = s.store.as_mut() {
        store.remove(&key);
    }
    Ok(ok(json!({"document": index, "repaired": notes})))
}

fn discard(s: &mut Session, p: &Value) -> Result<Value> {
    let key = named(s, p, "recovery.discard")?;
    if let Some(store) = s.store.as_mut() {
        store.remove(&key);
    }
    Ok(ok(json!({})))
}
