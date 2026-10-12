//! Queries: they read, never change.

use serde_json::{Value, json};

use super::{always, cmd, has_doc, str_param};
use crate::{CommandSpec, EngineError, Result, Session, inspect};
use genexpress_doc::preset;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "engine.commands", "List Commands", [], None, "{prefix?: only ids starting with it}", always, commands),
        cmd!(query "session.inspect", "Inspect Session", [], None, "{}", always, session),
        cmd!(query "document.inspect", "Inspect Design", [], None, "{all_pages?: bool}", has_doc, document),
        cmd!(query "document.json", "Design as JSON", [], None, "{} — the full document model (picture data left out)", has_doc, document_json),
        cmd!(query "element.get", "Inspect Object", [], None, "{id} — the element's full JSON", has_doc, element_get),
        cmd!(query "presets.list", "Page Size Presets", [], None, "{category?}", always, presets),
        cmd!(query "history.list", "History", ["Edit"], None, "{}", has_doc, history),
    ]
}

fn commands(s: &mut Session, p: &Value) -> Result<Value> {
    let prefix = str_param(p, "prefix").unwrap_or("");
    let list: Vec<_> = crate::command_specs().iter().filter(|c| c.id.starts_with(prefix)).map(|c| c.info(s)).collect();
    Ok(json!(list))
}

fn session(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(inspect::session_summary(s))
}

fn document(s: &mut Session, p: &Value) -> Result<Value> {
    Ok(inspect::document_summary(s.doc()?, super::bool_or(p, "all_pages", false)))
}

fn document_json(s: &mut Session, _: &Value) -> Result<Value> {
    let mut v = serde_json::to_value(&*s.doc()?.doc).map_err(|e| EngineError::Other(e.to_string()))?;
    if let Some(assets) = v.get_mut("assets").and_then(Value::as_object_mut) {
        for a in assets.values_mut() {
            if let Some(o) = a.as_object_mut() {
                o.insert("data".into(), json!("…"));
            }
        }
    }
    Ok(v)
}

fn element_get(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p.get("id").and_then(Value::as_u64).map(genexpress_doc::NodeId).ok_or_else(|| super::bad("element.get", "missing `id`"))?;
    let el = s.doc()?.doc.element(id)?;
    serde_json::to_value(el).map_err(|e| EngineError::Other(e.to_string()))
}

fn presets(_: &mut Session, p: &Value) -> Result<Value> {
    let cat = str_param(p, "category");
    Ok(json!(preset::PRESETS.iter().filter(|x| cat.is_none_or(|c| c.eq_ignore_ascii_case(x.category))).collect::<Vec<_>>()))
}

fn history(s: &mut Session, _: &Value) -> Result<Value> {
    let d = s.doc()?;
    Ok(json!({
        "undo": d.history.undo.iter().map(|e| e.label.clone()).collect::<Vec<_>>(),
        "redo": d.history.redo.iter().rev().map(|e| e.label.clone()).collect::<Vec<_>>(),
    }))
}
