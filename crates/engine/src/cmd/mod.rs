//! The command registry. Every user-visible action is a [`CommandSpec`] here.

pub mod brand;
pub mod clipboard;
pub mod content;
pub mod create;
pub mod data;
pub mod edit;
pub mod export;
pub mod file;
pub mod fonts;
pub mod models;
pub mod motion;
pub mod object;
pub mod page;
pub mod query;
pub mod recent;
pub mod recovery;
pub mod richtext;
pub mod style;
pub mod textedit;
pub mod versions;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{EngineError, Result, Session};
use genha_color::Color;
use genha_doc::NodeId;

pub type Run = fn(&mut Session, &Value) -> Result<Value>;
pub type Enabled = fn(&Session) -> std::result::Result<(), String>;

/// Metadata + implementation for one command.
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    /// Menu placement, e.g. `["Arrange"]`. Empty = not in menus.
    pub menu: &'static [&'static str],
    /// Default shortcut (`Cmd+Shift+]`), mapped per platform by the UI.
    pub shortcut: Option<&'static str>,
    /// Human/agent-readable parameter description.
    pub params: &'static str,
    pub enabled: Enabled,
    pub run: Run,
    /// False for queries (they never change the document).
    pub edits: bool,
}

/// Serializable command metadata.
#[derive(Clone, Debug, Serialize)]
pub struct CommandInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub menu: Vec<&'static str>,
    pub shortcut: Option<&'static str>,
    pub params: &'static str,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

impl CommandSpec {
    pub fn info(&self, s: &Session) -> CommandInfo {
        let e = (self.enabled)(s);
        CommandInfo {
            id: self.id,
            label: self.label,
            menu: self.menu.to_vec(),
            shortcut: self.shortcut,
            params: self.params,
            enabled: e.is_ok(),
            disabled_reason: e.err(),
        }
    }
}

// ---------- enablement predicates ----------

pub fn always(_: &Session) -> std::result::Result<(), String> {
    Ok(())
}
pub fn has_doc(s: &Session) -> std::result::Result<(), String> {
    s.active().map(|_| ()).ok_or_else(|| "no document open".into())
}
pub fn has_selection(s: &Session) -> std::result::Result<(), String> {
    let d = s.active().ok_or("no document open")?;
    if d.selected_elements().is_empty() { Err("nothing selected".into()) } else { Ok(()) }
}
pub fn has_multi(s: &Session) -> std::result::Result<(), String> {
    let d = s.active().ok_or("no document open")?;
    if d.selected_elements().len() < 2 { Err("select at least two objects".into()) } else { Ok(()) }
}
pub fn can_undo(s: &Session) -> std::result::Result<(), String> {
    s.active().filter(|d| !d.history.undo.is_empty()).map(|_| ()).ok_or_else(|| "nothing to undo".into())
}
pub fn can_redo(s: &Session) -> std::result::Result<(), String> {
    s.active().filter(|d| !d.history.redo.is_empty()).map(|_| ()).ok_or_else(|| "nothing to redo".into())
}

macro_rules! cmd {
    ($id:literal, $label:literal, [$($m:literal),*], $sc:expr, $params:literal, $en:expr, $run:expr) => {
        $crate::cmd::CommandSpec { id: $id, label: $label, menu: &[$($m),*], shortcut: $sc, params: $params, enabled: $en, run: $run, edits: true }
    };
    (query $id:literal, $label:literal, [$($m:literal),*], $sc:expr, $params:literal, $en:expr, $run:expr) => {
        $crate::cmd::CommandSpec { id: $id, label: $label, menu: &[$($m),*], shortcut: $sc, params: $params, enabled: $en, run: $run, edits: false }
    };
}
pub(crate) use cmd;

pub fn command_specs() -> &'static [CommandSpec] {
    static SPECS: std::sync::OnceLock<Vec<CommandSpec>> = std::sync::OnceLock::new();
    SPECS.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(file::specs());
        v.extend(edit::specs());
        v.extend(page::specs());
        v.extend(create::specs());
        v.extend(data::specs());
        v.extend(object::specs());
        v.extend(style::specs());
        v.extend(textedit::specs());
        v.extend(richtext::specs());
        v.extend(brand::specs());
        v.extend(export::specs());
        v.extend(motion::specs());
        v.extend(content::specs());
        v.extend(versions::specs());
        v.extend(recovery::specs());
        v.extend(recent::specs());
        v.extend(fonts::specs());
        v.extend(models::specs());
        v.extend(clipboard::specs());
        v.extend(query::specs());
        v
    })
}

pub fn find_command(id: &str) -> Option<&'static CommandSpec> {
    command_specs().iter().find(|c| c.id == id)
}

// ---------- param helpers ----------

pub(crate) fn bad(cmd: &str, msg: impl Into<String>) -> EngineError {
    EngineError::BadParams { cmd: cmd.into(), msg: msg.into() }
}
pub(crate) fn f64_or(p: &Value, key: &str, default: f64) -> f64 {
    p.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(default)
}
pub(crate) fn f64_req(p: &Value, key: &str, cmd: &str) -> Result<f64> {
    p.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()).ok_or_else(|| bad(cmd, format!("missing number `{key}`")))
}
pub(crate) fn bool_or(p: &Value, key: &str, default: bool) -> bool {
    p.get(key).and_then(Value::as_bool).unwrap_or(default)
}
pub(crate) fn str_param<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    p.get(key).and_then(Value::as_str)
}
pub(crate) fn str_req<'a>(p: &'a Value, key: &str, cmd: &str) -> Result<&'a str> {
    str_param(p, key).ok_or_else(|| bad(cmd, format!("missing string `{key}`")))
}
pub(crate) fn usize_param(p: &Value, key: &str) -> Option<usize> {
    p.get(key).and_then(Value::as_u64).and_then(|v| usize::try_from(v).ok())
}
pub(crate) fn color_param(p: &Value, key: &str, cmd: &str) -> Result<Option<Color>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Color::parse(s).map(Some).ok_or_else(|| bad(cmd, format!("`{key}`: invalid colour `{s}` (use #rrggbb)"))),
        Some(_) => Err(bad(cmd, format!("`{key}` must be a colour string like \"#ff8800\""))),
    }
}
pub(crate) fn color_req(p: &Value, key: &str, cmd: &str) -> Result<Color> {
    color_param(p, key, cmd)?.ok_or_else(|| bad(cmd, format!("missing colour `{key}`")))
}

/// Deserializes `p[key]` (or the whole `p` when `key` is empty) into `T`.
pub(crate) fn parse<T: DeserializeOwned>(p: &Value, key: &str, cmd: &str) -> Result<T> {
    let v = if key.is_empty() { p } else { p.get(key).ok_or_else(|| bad(cmd, format!("missing `{key}`")))? };
    serde_json::from_value(v.clone()).map_err(|e| bad(cmd, format!("`{key}`: {e}")))
}

/// `current` with the fields in `patch` (a JSON object) replaced: partial updates for style
/// structs (`{"font_size": 64}` changes only the size).
pub(crate) fn merge<T: Serialize + DeserializeOwned>(current: &T, patch: &Value, cmd: &str) -> Result<T> {
    let mut v = serde_json::to_value(current).map_err(|e| EngineError::Other(e.to_string()))?;
    let (Some(obj), Some(patch)) = (v.as_object_mut(), patch.as_object()) else {
        return Err(bad(cmd, "expected an object of fields to change"));
    };
    for (k, val) in patch {
        obj.insert(k.clone(), val.clone());
    }
    serde_json::from_value(v).map_err(|e| bad(cmd, e.to_string()))
}

/// The objects a command acts on: `ids` when given (all must exist), else the selection.
pub(crate) fn targets(s: &Session, p: &Value, cmd: &str) -> Result<Vec<NodeId>> {
    let d = s.doc()?;
    match p.get("ids") {
        Some(Value::Array(a)) => {
            let ids: Vec<NodeId> =
                a.iter().map(|v| v.as_u64().map(NodeId).ok_or_else(|| bad(cmd, "`ids` must be object ids"))).collect::<Result<_>>()?;
            for id in &ids {
                if d.doc.find(*id).is_none() {
                    return Err(EngineError::NoNode(*id));
                }
            }
            Ok(ids)
        }
        Some(Value::Null) | None => {
            if let Some(id) = p.get("id").and_then(Value::as_u64) {
                let id = NodeId(id);
                return if d.doc.find(id).is_some() { Ok(vec![id]) } else { Err(EngineError::NoNode(id)) };
            }
            let sel: Vec<NodeId> = d.selected_elements().iter().map(|e| e.id).collect();
            if sel.is_empty() { Err(bad(cmd, "nothing selected (select objects or pass `ids`)")) } else { Ok(sel) }
        }
        Some(_) => Err(bad(cmd, "`ids` must be an array of object ids")),
    }
}

/// `{"ok": true}` plus extra fields.
pub(crate) fn ok(extra: Value) -> Value {
    let mut v = serde_json::json!({"ok": true});
    if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        for (k, val) in e {
            o.insert(k.clone(), val.clone());
        }
    }
    v
}
