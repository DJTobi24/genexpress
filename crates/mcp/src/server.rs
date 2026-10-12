//! The JSON-RPC loop.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::Backend;
use crate::tools::{call_tool, tool_definitions};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Longest request line accepted.
const MAX_LINE: usize = 64 << 20;

pub struct Server<B: Backend> {
    pub backend: B,
}

impl<B: Backend> Server<B> {
    pub fn new(backend: B) -> Self {
        Server { backend }
    }

    /// Serves until stdin closes.
    pub fn serve(&mut self, input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reply) = self.handle_line(&line) {
                output.write_all(reply.as_bytes())?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    /// Handles one message; `None` for notifications.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        if line.len() > MAX_LINE {
            return Some(error(Value::Null, -32600, "request too large").to_string());
        }
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}")).to_string()),
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let id = id?;
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "genha", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "GenHa is an open-source design app (social posts, flyers, presentations…). Every action is a command: list them with list_commands, run them with run_command, check your work with look."
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tool_definitions()})),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let (content, is_error) = call_tool(&mut self.backend, name, &args);
                Ok(json!({"content": content, "isError": is_error}))
            }
            other => Err((-32601, format!("method not found: {other}"))),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}).to_string(),
            Err((code, msg)) => error(id, code, &msg).to_string(),
        })
    }
}

fn error(id: Value, code: i64, msg: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg}})
}
