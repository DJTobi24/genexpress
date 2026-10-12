//! The MCP tools GenHa offers.

use serde_json::{Value, json};

use crate::Backend;

pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "list_commands",
            "description": "List GenHa commands (id, label, parameter description, whether enabled now). Every action in the app is a command.",
            "inputSchema": {"type": "object", "properties": {"prefix": {"type": "string", "description": "Only ids starting with this, e.g. `text.` or `page.`"}}}
        },
        {
            "name": "run_command",
            "description": "Run one GenHa command, e.g. {command: \"shape.add\", params: {shape: \"star\", fill: \"#ffcc00\"}}. Start with `file.new` (params {preset: \"instagram-post\"}) or `template.use`. Returns the command's result (new object ids etc.).",
            "inputSchema": {"type": "object", "properties": {"command": {"type": "string"}, "params": {"type": "object"}}, "required": ["command"]}
        },
        {
            "name": "run_commands",
            "description": "Run several commands as one undo step; stops and rolls back at the first error.",
            "inputSchema": {"type": "object", "properties": {"label": {"type": "string"}, "commands": {"type": "array", "items": {"type": "object", "properties": {"command": {"type": "string"}, "params": {"type": "object"}}, "required": ["command"]}}}, "required": ["commands"]}
        },
        {
            "name": "inspect_design",
            "description": "The open design: pages, the current page's objects (ids, frames, text, colours), selection, brand kit.",
            "inputSchema": {"type": "object", "properties": {"all_pages": {"type": "boolean"}}}
        },
        {
            "name": "look",
            "description": "Render a page to a PNG image so you can see the design. Use it after changes to check the result.",
            "inputSchema": {"type": "object", "properties": {"page": {"type": "integer"}, "scale": {"type": "number", "description": "0.05–4, default 0.5"}}}
        }
    ])
}

/// Runs tool `name`; returns MCP `content` and whether it is an error.
pub fn call_tool(backend: &mut dyn Backend, name: &str, args: &Value) -> (Vec<Value>, bool) {
    let r = match name {
        "list_commands" => backend.call("engine.commands", args),
        "run_command" => {
            let command = args.get("command").cloned().unwrap_or(Value::Null);
            backend.call("engine.execute", &json!({"command": command, "params": args.get("params").cloned().unwrap_or(json!({}))}))
        }
        "run_commands" => backend.call("engine.execute", &json!({"command": "command.batch", "params": args})),
        "inspect_design" => backend.call("document.inspect", args),
        "look" => match backend.call("ui.render", args) {
            Ok(v) => match v.get("png_base64").and_then(Value::as_str) {
                Some(data) => return (vec![json!({"type": "image", "data": data, "mimeType": "image/png"})], false),
                None => Ok(v),
            },
            Err(e) => Err(e),
        },
        other => Err(format!("unknown tool `{other}`")),
    };
    match r {
        Ok(v) => (vec![json!({"type": "text", "text": v.to_string()})], false),
        Err(e) => (vec![json!({"type": "text", "text": e})], true),
    }
}
