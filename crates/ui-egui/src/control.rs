//! The control channel: JSON lines on a loopback TCP port, so agents and scripts can drive the
//! running app (see docs/control-protocol.md). Requests are answered on the UI thread, between
//! frames, through the same commands the UI uses.

use std::sync::mpsc::{Receiver, Sender};

use serde_json::{Value, json};

use crate::App;

pub struct Request {
    pub method: String,
    pub params: Value,
    pub reply: Sender<Result<Value, String>>,
}

pub struct ControlServer {
    rx: Receiver<Request>,
}

impl ControlServer {
    /// Listens on `127.0.0.1:port`. Each connection gets its own thread; requests are queued for
    /// the UI thread, which `ctx` wakes up.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start(port: u16, ctx: egui::Context) -> std::io::Result<ControlServer> {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind(("127.0.0.1", port))?;
        let (tx, rx) = std::sync::mpsc::channel::<Request>();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let tx = tx.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let Ok(read) = stream.try_clone() else { return };
                    let mut out = stream;
                    for line in BufReader::new(read).lines() {
                        let Ok(line) = line else { break };
                        if line.trim().is_empty() {
                            continue;
                        }
                        let reply = match serde_json::from_str::<Value>(&line) {
                            Ok(msg) => {
                                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                                let method = msg.get("method").and_then(Value::as_str).unwrap_or("").to_string();
                                let params = msg.get("params").cloned().unwrap_or(json!({}));
                                let (rtx, rrx) = std::sync::mpsc::channel();
                                if tx.send(Request { method, params, reply: rtx }).is_err() {
                                    break;
                                }
                                ctx.request_repaint();
                                match rrx.recv() {
                                    Ok(Ok(result)) => json!({"id": id, "ok": true, "result": result}),
                                    Ok(Err(e)) => json!({"id": id, "ok": false, "error": e}),
                                    Err(_) => break,
                                }
                            }
                            Err(e) => json!({"id": null, "ok": false, "error": format!("bad JSON: {e}")}),
                        };
                        if writeln!(out, "{reply}").is_err() {
                            break;
                        }
                    }
                });
            }
        });
        Ok(ControlServer { rx })
    }
}

/// Answers queued requests (called once per frame).
pub fn process(app: &mut App, ctx: &egui::Context) {
    let mut reqs = vec![];
    if let Some(c) = &app.control {
        while let Ok(r) = c.rx.try_recv() {
            reqs.push(r);
        }
        // Keep polling while a client may be waiting.
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    for r in reqs {
        let result = handle(app, ctx, &r.method, &r.params);
        // The client may have gone away; nothing to do then.
        let _ = r.reply.send(result);
    }
}

pub(crate) fn handle(app: &mut App, ctx: &egui::Context, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "engine.execute" => {
            let cmd = params.get("command").and_then(Value::as_str).ok_or("missing `command`")?;
            let p = params.get("params").cloned().unwrap_or(json!({}));
            let r = app.session.execute(cmd, &p).map_err(|e| e.to_string());
            if r.is_ok() && !app.session.docs.is_empty() {
                app.screen = crate::Screen::Editor;
                if cmd.starts_with("file.") || cmd.starts_with("template.") || cmd.starts_with("page.") {
                    app.view.fit_pending = true;
                }
            }
            r
        }
        "engine.commands" | "document.inspect" | "session.inspect" => app.session.execute(method, params).map_err(|e| e.to_string()),
        "ui.render" => {
            let d = app.session.active().ok_or("no document open")?;
            let page = params.get("page").and_then(Value::as_u64).map(|p| p as usize).unwrap_or(d.page);
            let scale = params.get("scale").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(1.0).clamp(0.05, 8.0);
            let opts = genha_render::RenderOptions { scale, transparent_background: false };
            let doc = d.doc.clone();
            let r = app.renderer.render_page(&doc, page, &app.session.fonts, &opts).map_err(|e| e.to_string())?;
            let png = genha_render::encode_png(&r).map_err(|e| e.to_string())?;
            match params.get("path").and_then(Value::as_str) {
                Some(path) => write(path, &png).map(|_| json!({"path": path, "width": r.width, "height": r.height})),
                None => {
                    use base64::Engine as _;
                    Ok(json!({"png_base64": base64::engine::general_purpose::STANDARD.encode(&png), "width": r.width, "height": r.height}))
                }
            }
        }
        "ui.state" => Ok(json!({
            "screen": match app.screen { crate::Screen::Home => "home", crate::Screen::Editor => "editor" },
            "zoom": app.view.zoom,
            "left_tab": app.left_tab.label(),
            "status": app.status.as_ref().map(|s| s.0.clone()),
            "playing": app.motion.playing.is_some(),
            "presenting": app.motion.present.as_ref().map(|p| p.page),
        })),
        "ui.play" => {
            app.session.active().ok_or("no document open")?;
            if params.get("stop").and_then(Value::as_bool).unwrap_or(false) {
                crate::motion::stop(app);
            } else {
                crate::motion::play(app, ctx);
            }
            Ok(json!({"playing": app.motion.playing.is_some()}))
        }
        "ui.present" => {
            let pages = app.session.active().ok_or("no document open")?.doc.pages.len();
            if params.get("stop").and_then(Value::as_bool).unwrap_or(false) {
                crate::motion::stop_presenting(app, ctx);
                return Ok(json!({"presenting": null}));
            }
            if app.motion.present.is_none() {
                crate::motion::start_presenting(app, ctx);
            }
            if let Some(i) = params.get("page").and_then(Value::as_u64) {
                let i = usize::try_from(i).ok().filter(|i| *i < pages).ok_or("no such page")?;
                crate::motion::present_page(app, ctx, i);
            }
            Ok(json!({"presenting": app.motion.present.as_ref().map(|p| p.page)}))
        }
        "ui.home" => {
            app.screen = crate::Screen::Home;
            Ok(json!({}))
        }
        other => Err(format!("unknown method `{other}`")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(target_arch = "wasm32")]
fn write(_: &str, _: &[u8]) -> Result<(), String> {
    Err("no file system in the browser".into())
}
