//! The home screen's data: the quick actions, one search over templates, sizes, quick actions and
//! My designs (`home.search`), and the personal settings the home screen shows (`settings.*`).

use serde::Serialize;
use serde_json::{Value, json};

use super::designs::{DesignEntry, library, matches};
use super::{always, bad, cmd, ok, str_req, usize_param};
use crate::{CommandSpec, Result, Session};
use genha_i18n::{N_, tr};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "home.search",
            "Search Everything",
            [],
            None,
            "{query, limit?: per group = 8} — words matched in English and in the UI's language: {designs: [{id, title}], templates: [{id, name, category}], sizes: [{id, label, category, width, height}], actions: [{id, label}]}",
            always,
            search_cmd
        ),
        cmd!(query "settings.get", "Settings", [], None, "{} — {name, dark, language, network, snapping, layer_strip}", always, settings_get),
        cmd!(
            query "settings.set",
            "Change Settings",
            [],
            None,
            "{name?: the name the home screen greets (\"\" for none), dark?: bool | null (the app's default)} — kept between runs; returns settings.get",
            always,
            settings_set
        ),
    ]
}

/// A quick action on the home screen: id, label, what it does, search words (English; shown
/// translated). Picture actions run `quick.image` (file in, file out); `qr-code` and `collage`
/// start a design.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct QuickAction {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub keywords: &'static [&'static str],
}

const fn qa(id: &'static str, label: &'static str, hint: &'static str, keywords: &'static [&'static str]) -> QuickAction {
    QuickAction { id, label, hint, keywords }
}

pub const QUICK_ACTIONS: &[QuickAction] = &[
    qa("remove-background", N_("Remove background"), N_("Cut out the subject of a photo"), &["cut out", "transparent", "subject", "png"]),
    qa("resize", N_("Resize picture"), N_("Scale a picture to 50%"), &["scale", "smaller", "shrink"]),
    qa("convert-png", N_("Convert to PNG"), N_("Any picture to PNG"), &["convert", "format", "png"]),
    qa("convert-jpeg", N_("Convert to JPEG"), N_("Any picture to JPEG"), &["convert", "format", "jpg", "jpeg"]),
    qa("convert-webp", N_("Convert to WebP"), N_("Any picture to WebP"), &["convert", "format", "webp"]),
    qa("auto-crop", N_("Trim borders"), N_("Remove empty or uniform edges"), &["crop", "trim", "edges"]),
    qa("blur-background", N_("Blur background"), N_("Keep the subject sharp, blur what is behind"), &["blur", "bokeh", "portrait"]),
    qa("change-background", N_("White background"), N_("Replace a plain backdrop with white"), &["background", "white", "product"]),
    qa("pixelate", N_("Pixelate"), N_("Big blocky pixels"), &["pixel", "mosaic", "censor"]),
    qa("grayscale", N_("Black & white"), N_("Turn a photo black and white"), &["grey", "gray", "mono", "black and white"]),
    qa("qr-code", N_("QR code"), N_("A design with a QR code for a link or text"), &["qr", "link", "url", "barcode"]),
    qa("collage", N_("Photo collage"), N_("Pictures arranged in a grid"), &["collage", "grid", "photos", "layout"]),
];

pub fn quick_action(id: &str) -> Option<&'static QuickAction> {
    QUICK_ACTIONS.iter().find(|a| a.id == id)
}

/// What `home.search` found, best first in each group.
#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    pub designs: Vec<DesignEntry>,
    pub templates: Vec<genha_templates::TemplateInfo>,
    pub sizes: Vec<&'static genha_doc::preset::PagePreset>,
    pub actions: Vec<&'static QuickAction>,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.designs.is_empty() && self.templates.is_empty() && self.sizes.is_empty() && self.actions.is_empty()
    }
}

/// `text` and its translation into the UI's language.
fn both(text: &str) -> [String; 2] {
    [text.to_string(), tr(text).to_string()]
}

/// Searches templates (name, tags, category), size presets (name, category), quick actions and
/// My designs for every word of `query`, in English and in the UI's language; at most `limit`
/// per group. An empty query finds nothing.
pub fn search(s: &Session, query: &str, limit: usize) -> SearchResults {
    let mut out = SearchResults::default();
    if query.trim().is_empty() {
        return out;
    }
    let q = query.trim();
    let hit = |texts: Vec<String>| {
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        matches(q, &refs)
    };
    let lib = library(s);
    let mut designs: Vec<&DesignEntry> = lib.designs.iter().filter(|e| matches(q, &[&e.title, &e.name])).collect();
    designs.sort_by_key(|e| std::cmp::Reverse(e.touched()));
    out.designs = designs.into_iter().take(limit).cloned().collect();
    out.templates = genha_templates::catalog()
        .into_iter()
        .filter(|t| {
            let mut texts: Vec<String> = vec![t.id.to_string()];
            texts.extend(both(t.name));
            texts.extend(both(t.category));
            texts.extend(t.tags.iter().map(|g| g.to_string()));
            hit(texts)
        })
        .take(limit)
        .collect();
    out.sizes = genha_doc::preset::PRESETS
        .iter()
        .filter(|p| {
            let mut texts = vec![p.id.to_string(), format!("{:.0}x{:.0}", p.width, p.height)];
            texts.extend(both(p.label));
            texts.extend(both(p.category));
            hit(texts)
        })
        .take(limit)
        .collect();
    out.actions = QUICK_ACTIONS
        .iter()
        .filter(|a| {
            let mut texts = vec![a.id.to_string()];
            texts.extend(both(a.label));
            texts.extend(both(a.hint));
            texts.extend(a.keywords.iter().map(|k| k.to_string()));
            hit(texts)
        })
        .take(limit)
        .collect();
    out
}

fn search_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let query = str_req(p, "query", "home.search")?;
    if query.len() > 1000 {
        return Err(bad("home.search", "the query is too long"));
    }
    let limit = usize_param(p, "limit").unwrap_or(8).clamp(1, 200);
    let r = search(s, query, limit);
    Ok(json!({
        "designs": r.designs.iter().map(|e| json!({"id": e.id, "title": e.title})).collect::<Vec<_>>(),
        "templates": r.templates.iter().map(|t| json!({"id": t.id, "name": t.name, "category": t.category})).collect::<Vec<_>>(),
        "sizes": r.sizes.iter().map(|p| json!({"id": p.id, "label": p.label, "category": p.category, "width": p.width, "height": p.height})).collect::<Vec<_>>(),
        "actions": r.actions.iter().map(|a| json!({"id": a.id, "label": a.label})).collect::<Vec<_>>(),
    }))
}

// ---------- settings ----------

/// Where the greeting's name and dark mode are kept.
const NAME_KEY: &str = "settings/name";
const DARK_KEY: &str = "settings/dark";
/// Longest name kept, in characters.
pub const MAX_USER_NAME: usize = 60;

/// Reads the name and dark mode kept in the store.
pub(crate) fn load_preferences(s: &mut Session) {
    if let Some(v) = s.store.as_ref().and_then(|st| st.read(NAME_KEY)) {
        s.user_name = String::from_utf8_lossy(&v).chars().filter(|c| !c.is_control()).take(MAX_USER_NAME).collect::<String>().trim().to_string();
    }
    if let Some(v) = s.store.as_ref().and_then(|st| st.read(DARK_KEY)) {
        s.dark = match v.as_slice() {
            b"dark" => Some(true),
            b"light" => Some(false),
            _ => None,
        };
    }
}

fn settings_get(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({
        "name": s.user_name,
        "dark": s.dark,
        "language": s.language.id(),
        "network": s.network,
        "snapping": s.snapping,
        "layer_strip": s.layer_strip,
    }))
}

fn settings_set(s: &mut Session, p: &Value) -> Result<Value> {
    let name = match p.get("name") {
        None => None,
        Some(Value::Null) => Some(String::new()),
        Some(Value::String(n)) => Some(n.chars().filter(|c| !c.is_control()).take(MAX_USER_NAME).collect::<String>().trim().to_string()),
        Some(_) => return Err(bad("settings.set", "`name` must be a string")),
    };
    let dark = match p.get("dark") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::Bool(b)) => Some(Some(*b)),
        Some(_) => return Err(bad("settings.set", "`dark` must be true, false or null")),
    };
    let mut notes = vec![];
    if let Some(n) = name {
        s.user_name = n;
        if let Some(st) = s.store.as_mut()
            && let Err(e) = st.write(NAME_KEY, s.user_name.as_bytes())
        {
            notes.push(format!("Could not keep the name: {e}"));
        }
    }
    if let Some(d) = dark {
        s.dark = d;
        let value: &[u8] = match d {
            Some(true) => b"dark",
            Some(false) => b"light",
            None => b"default",
        };
        if let Some(st) = s.store.as_mut()
            && let Err(e) = st.write(DARK_KEY, value)
        {
            notes.push(format!("Could not keep the dark mode setting: {e}"));
        }
    }
    // Losing a setting only means its default next run: reported, not an error.
    for n in notes {
        s.post(n);
    }
    Ok(ok(settings_get(s, &json!({}))?))
}
