//! Fonts the user adds: font files (TrueType, OpenType, collections) put into the session's
//! [`FontBook`](genha_text::FontBook) and, with a [`Store`](crate::store::Store), kept as
//! `fonts/<name>` so they come back next time ([`load_stored_fonts`]). In the browser, where
//! pages can't read installed fonts, this is how text gets more than the built-in faces.

use std::sync::Arc;

use base64::Engine as _;
use serde_json::{Value, json};

use super::{always, bad, bool_or, cmd, ok, str_param, str_req};
use crate::store::key_part;
use crate::{CommandSpec, EngineError, Result, Session};

const DIR: &str = "fonts";

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "font.add",
            "Add Font",
            [],
            None,
            "{data: base64 font file (.ttf, .otf, .ttc), name?: file name, keep?: bool = true (keep it for next time)} → {faces, families}",
            always,
            add
        ),
        cmd!(query "font.list", "Fonts", [], None, "{} → {families: every family text can be set in, added: [{key, name}] fonts kept for next time}", always, list),
        cmd!(query "font.remove", "Remove Added Font", [], None, "{key: from font.list `added`} — no longer loaded from next time", always, remove),
    ]
}

/// Adds the faces in `bytes` to the session's fonts; with `keep` (and a store) the file is kept
/// for next time under its `name`. Returns (faces added, families that are new).
pub fn add_font_bytes(s: &mut Session, name: &str, bytes: Vec<u8>, keep: bool) -> Result<(usize, Vec<String>)> {
    let before = s.fonts.families();
    let kept = keep.then(|| bytes.clone());
    let n = Arc::make_mut(&mut s.fonts).add_font(bytes).map_err(|e| EngineError::Other(format!("{name}: {e}")))?;
    let new: Vec<String> = s.fonts.families().into_iter().filter(|f| !before.iter().any(|b| b.eq_ignore_ascii_case(f))).collect();
    if let (Some(data), Some(store)) = (kept, s.store.as_mut()) {
        let key = format!("{DIR}/{}", key_part(name));
        // The font is in use either way; it just won't come back next time.
        if let Err(e) = store.write(&key, &data) {
            s.messages.push(format!("{name} could not be kept for next time: {e}"));
        }
    }
    Ok((n, new))
}

/// Loads the fonts kept in the store (at start); returns how many faces were added. Files that
/// don't load are skipped.
pub fn load_stored_fonts(s: &mut Session) -> usize {
    let Some(store) = s.store.as_ref() else { return 0 };
    let files: Vec<(String, Vec<u8>)> = store.list(DIR).into_iter().filter_map(|(k, _)| store.read(&k).map(|b| (k, b))).collect();
    let book = Arc::make_mut(&mut s.fonts);
    let mut n = 0;
    for (key, bytes) in files {
        match book.add_font(bytes) {
            Ok(k) => n += k,
            Err(e) => log::warn!("{key}: {e}"),
        }
    }
    n
}

fn add(s: &mut Session, p: &Value) -> Result<Value> {
    let data = str_req(p, "data", "font.add")?;
    // Base64 is 4/3 of the bytes: refuse what can't be a font the book accepts.
    if data.len() / 4 * 3 > genha_text::MAX_FONT_FILE + 3 {
        return Err(bad("font.add", "the font file is too large"));
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(|e| bad("font.add", format!("`data` is not base64: {e}")))?;
    let name = str_param(p, "name").unwrap_or("font.ttf");
    let (faces, families) = add_font_bytes(s, name, bytes, bool_or(p, "keep", true))?;
    Ok(ok(json!({"faces": faces, "families": families})))
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let added: Vec<Value> = s
        .store
        .as_ref()
        .map(|st| st.list(DIR))
        .unwrap_or_default()
        .into_iter()
        .map(|(k, _)| {
            let name = k.strip_prefix("fonts/").unwrap_or(&k).to_string();
            json!({"key": k, "name": name})
        })
        .collect();
    Ok(json!({"families": s.fonts.families(), "added": added}))
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let key = str_req(p, "key", "font.remove")?;
    let store = s.store.as_mut().ok_or_else(|| EngineError::Other("no fonts are kept here".into()))?;
    if !store.list(DIR).iter().any(|(k, _)| k == key) {
        return Err(bad("font.remove", format!("no added font `{key}`")));
    }
    store.remove(key);
    Ok(ok(json!({})))
}
