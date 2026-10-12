//! Data Recovery: copies of unsaved designs written in the background, offered again after a crash.
//!
//! The app sets [`Session::recovery_dir`] and runs `recovery.save` every minute. Copies are named
//! `<process id>-<document>.genha`; files from other (crashed) processes are what `recovery.list`
//! offers. Saving or closing a design removes its copy.

use super::{always, cmd};
use crate::CommandSpec;
#[cfg(target_arch = "wasm32")]
use crate::{EngineError, Result, Session};
#[cfg(target_arch = "wasm32")]
use serde_json::{Value, json};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "recovery.save", "Save Recovery Copies", [], None, "{} — writes a copy of every design with unsaved changes to the recovery folder", always, save),
        cmd!(query "recovery.list", "Recovered Designs", [], None, "{} — copies left by a previous session that ended unexpectedly", always, list),
        cmd!("recovery.open", "Open Recovered Design", [], None, "{path: from recovery.list}", always, open),
        cmd!(query "recovery.discard", "Discard Recovered Design", [], None, "{path: from recovery.list}", always, discard),
    ]
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::forget;
#[cfg(not(target_arch = "wasm32"))]
use native::{discard, list, open, save};

/// The browser has no folder to keep copies in.
#[cfg(target_arch = "wasm32")]
pub(crate) fn forget(_: &Session, _: usize) {}

#[cfg(target_arch = "wasm32")]
fn unavailable(_: &mut Session, _: &Value) -> Result<Value> {
    Err(EngineError::Other("Data Recovery is not available in the browser".into()))
}
#[cfg(target_arch = "wasm32")]
use unavailable as save;
#[cfg(target_arch = "wasm32")]
use unavailable as open;
#[cfg(target_arch = "wasm32")]
use unavailable as discard;
#[cfg(target_arch = "wasm32")]
fn list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!([]))
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::path::{Path, PathBuf};

    use serde_json::{Value, json};

    use super::super::{bad, ok, str_req};
    use crate::{EngineError, Result, Session};

    fn dir(s: &Session) -> Result<PathBuf> {
        s.recovery_dir.clone().ok_or_else(|| EngineError::Other("Data Recovery is off (no recovery folder)".into()))
    }

    fn own_prefix() -> String {
        format!("{}-", std::process::id())
    }

    fn copy_path(dir: &Path, recovery_id: u64) -> PathBuf {
        dir.join(format!("{}{recovery_id}.{}", own_prefix(), genha_format::EXTENSION))
    }

    /// Removes document `index`'s recovery copy (after it was saved or closed).
    pub(crate) fn forget(s: &Session, index: usize) {
        if let (Some(dir), Some(d)) = (&s.recovery_dir, s.docs.get(index)) {
            // Missing copies are normal (nothing was unsaved long enough).
            let _ = std::fs::remove_file(copy_path(dir, d.recovery_id));
        }
    }

    /// A path the caller gave, accepted only inside the recovery folder.
    fn checked(s: &Session, p: &Value, cmd: &str) -> Result<PathBuf> {
        let dir = dir(s)?;
        let path = PathBuf::from(str_req(p, "path", cmd)?);
        let inside = path.parent().and_then(|p| p.canonicalize().ok()) == dir.canonicalize().ok();
        if !inside || path.extension().and_then(|e| e.to_str()) != Some(genha_format::EXTENSION) {
            return Err(bad(cmd, "not a file in the recovery folder"));
        }
        Ok(path)
    }

    pub(super) fn save(s: &mut Session, _: &Value) -> Result<Value> {
        let dir = dir(s)?;
        std::fs::create_dir_all(&dir).map_err(|e| EngineError::Other(format!("cannot create {}: {e}", dir.display())))?;
        let mut written = vec![];
        for d in &s.docs {
            let path = copy_path(&dir, d.recovery_id);
            if d.dirty {
                let bytes = genha_format::to_bytes(&d.doc, true)?;
                genha_format::write_atomic(&path, &bytes)?;
                written.push(path.display().to_string());
            } else {
                // A design saved since the last copy needs none.
                let _ = std::fs::remove_file(&path);
            }
        }
        Ok(ok(json!({"written": written})))
    }

    pub(super) fn list(s: &mut Session, _: &Value) -> Result<Value> {
        let Ok(dir) = dir(s) else { return Ok(json!([])) };
        let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(json!([])) };
        let own = own_prefix();
        let mut out = vec![];
        for e in entries.flatten().take(1000) {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(&own) || path.extension().and_then(|x| x.to_str()) != Some(genha_format::EXTENSION) {
                continue;
            }
            let modified = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let title = std::fs::read(&path).ok().and_then(|b| genha_format::from_bytes(&b).ok()).map(|(d, _)| d.title);
            out.push(json!({"path": path.display().to_string(), "title": title.unwrap_or_else(|| "(damaged copy)".into()), "modified": modified}));
        }
        Ok(json!(out))
    }

    pub(super) fn open(s: &mut Session, p: &Value) -> Result<Value> {
        let path = checked(s, p, "recovery.open")?;
        let bytes = std::fs::read(&path).map_err(|e| EngineError::Other(format!("cannot read {}: {e}", path.display())))?;
        let (doc, notes) = genha_format::from_bytes(&bytes)?;
        let index = s.open_document(doc);
        if let Some(d) = s.active_mut() {
            d.dirty = true;
        }
        // Write this session's copy before removing the old one, so the design is never unprotected.
        save(s, &json!({}))?;
        let _ = std::fs::remove_file(&path); // already replaced by this session's copy
        Ok(ok(json!({"document": index, "repaired": notes})))
    }

    pub(super) fn discard(s: &mut Session, p: &Value) -> Result<Value> {
        let path = checked(s, p, "recovery.discard")?;
        std::fs::remove_file(&path).map_err(|e| EngineError::Other(format!("cannot remove {}: {e}", path.display())))?;
        Ok(ok(json!({})))
    }
}
