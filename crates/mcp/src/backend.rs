//! Where MCP tool calls go: an in-process session or the running app.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{Value, json};

use genexpress_engine::{Session, guard, inspect};

/// A control-channel endpoint: `method` + `params` in, a result or an error message out.
pub trait Backend {
    fn call(&mut self, method: &str, params: &Value) -> Result<Value, String>;
}

/// Renders page `page` of the active document to PNG bytes (supplied by the app/CLI, which own the
/// renderer and fonts; layer rules keep the renderer out of this crate's required deps).
pub type RenderHook = Box<dyn FnMut(&Session, usize, f64) -> Result<Vec<u8>, String> + Send>;

/// An in-process session.
pub struct Headless {
    pub session: Session,
    pub render: Option<RenderHook>,
}

impl Default for Headless {
    fn default() -> Self {
        Headless { session: Session::new(), render: None }
    }
}

impl Headless {
    pub fn new(render: Option<RenderHook>) -> Self {
        Headless { session: Session::new(), render }
    }
}

impl Backend for Headless {
    fn call(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        match guard::catch_panic(|| self.call_inner(method, params)) {
            Ok(r) => r,
            Err(msg) => Err(format!("internal error: {msg}")),
        }
    }
}

impl Headless {
    fn call_inner(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        let s = &mut self.session;
        match method {
            "engine.execute" => {
                let cmd = params.get("command").and_then(Value::as_str).ok_or("missing `command`")?;
                let p = params.get("params").cloned().unwrap_or(json!({}));
                s.execute(cmd, &p).map_err(|e| e.to_string())
            }
            "engine.commands" => s.execute("engine.commands", params).map_err(|e| e.to_string()),
            "document.inspect" => s.execute("document.inspect", params).map_err(|e| e.to_string()),
            "session.inspect" => Ok(inspect::session_summary(s)),
            "ui.render" => {
                let page = params
                    .get("page")
                    .and_then(Value::as_u64)
                    .map(|p| p as usize)
                    .or_else(|| s.active().map(|d| d.page))
                    .ok_or("no document open")?;
                let scale = params.get("scale").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(0.5).clamp(0.05, 4.0);
                let hook = self.render.as_mut().ok_or("rendering is not available in this build")?;
                let png = hook(s, page, scale)?;
                if let Some(path) = params.get("path").and_then(Value::as_str) {
                    write_file(path, &png)?;
                    return Ok(json!({"path": path, "bytes": png.len()}));
                }
                use base64::Engine as _;
                Ok(json!({"png_base64": base64::engine::general_purpose::STANDARD.encode(&png)}))
            }
            other => Err(format!("unknown method `{other}`")),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(target_arch = "wasm32")]
fn write_file(_: &str, _: &[u8]) -> Result<(), String> {
    Err("no file system".into())
}

/// The running desktop app's control channel.
pub struct Remote {
    addr: String,
    conn: Option<(BufReader<TcpStream>, TcpStream)>,
    next_id: u64,
}

impl Remote {
    pub fn new(addr: &str) -> Self {
        Remote { addr: addr.to_string(), conn: None, next_id: 1 }
    }

    fn connect(&mut self) -> Result<&mut (BufReader<TcpStream>, TcpStream), String> {
        if self.conn.is_none() {
            let stream =
                TcpStream::connect(&self.addr).map_err(|e| format!("cannot reach GenExpress at {} (start it with --control): {e}", self.addr))?;
            stream.set_read_timeout(Some(Duration::from_secs(120))).map_err(|e| e.to_string())?;
            let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
            self.conn = Some((reader, stream));
        }
        self.conn.as_mut().ok_or_else(|| "not connected".to_string())
    }
}

impl Backend for Remote {
    fn call(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let line = json!({"id": id, "method": method, "params": params}).to_string();
        let result = (|| {
            let (reader, writer) = self.connect()?;
            writer.write_all(line.as_bytes()).and_then(|_| writer.write_all(b"\n")).map_err(|e| e.to_string())?;
            let mut buf = String::new();
            reader.read_line(&mut buf).map_err(|e| e.to_string())?;
            if buf.is_empty() {
                return Err("the app closed the connection".to_string());
            }
            serde_json::from_str::<Value>(&buf).map_err(|e| format!("bad reply: {e}"))
        })();
        let reply = match result {
            Ok(r) => r,
            Err(e) => {
                self.conn = None;
                return Err(e);
            }
        };
        if reply.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(reply.get("result").cloned().unwrap_or(Value::Null))
        } else {
            Err(reply.get("error").and_then(Value::as_str).unwrap_or("unknown error").to_string())
        }
    }
}
