//! The GenHa command line.

use std::sync::Arc;

use serde_json::{Value, json};

use genha_engine::Session;

const USAGE: &str = "usage: genha-cli <command> …

  new (--preset ID | --template ID | --width W --height H) -o OUT.genha
  info FILE                          the design as JSON (pages, objects, brand)
  export FILE -o OUT.png|jpg|svg|pdf|gif|mp4 [--page N] [--scale S] [--all-pages] [--transparent]
                                     [--fps F] [--quality low|medium|high] (gif, mp4: animated)
  run SCRIPT.json [--open FILE] [--save OUT.genha] [--export OUT.png|jpg|svg|pdf|gif|mp4]
                                     SCRIPT: [{\"command\": \"shape.add\", \"params\": {…}}, …]
  quick ACTION IN -o OUT [--scale S] [--width W] [--height H] [--preset P] [--tolerance T]
                    [--method auto|ml|color] [--model NAME] [--color #RRGGBB] [--amount PX]
                                     ACTION: remove-background, change-background, blur-background,
                                     resize, convert, auto-crop, filter, pixelate, rotate, flip
  model [status | download [NAME] | remove NAME]
                                     the AI background removal models (kept in the app's data folder;
                                     GENHA_OFFLINE=1 forbids downloads)
  exec COMMAND [JSON-PARAMS] [--open FILE]   run one command and print its result
  commands [PREFIX]                  every command with its parameters
  presets | templates                page sizes and templates
  mcp [--attach HOST:PORT]           MCP server on stdio (headless, or driving a running app)";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// `--name value` from the arguments.
fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn num(args: &[String], name: &str) -> Result<Option<f64>, String> {
    opt(args, name).map(|v| v.parse::<f64>().map_err(|_| format!("{name} needs a number"))).transpose()
}

fn session() -> Session {
    with_models(Session::with_fonts(Arc::new(genha_text::FontBook::system())))
}

/// `s` with the models the app downloaded (the data folder's `models/`), for the background
/// commands' AI method and `model.*`.
fn with_models(mut s: Session) -> Session {
    s.models = genha_engine::store::user_data_dir().map(|d| Box::new(genha_engine::store::DirStore::new(d)) as Box<dyn genha_engine::store::Store>);
    s
}

fn exec(s: &mut Session, cmd: &str, params: Value) -> Result<Value, String> {
    s.execute(cmd, &params).map_err(|e| e.to_string())
}

fn print(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn open(s: &mut Session, path: &str) -> Result<(), String> {
    exec(s, "file.open", json!({"path": path})).map(|_| ())
}

/// Exports the active design to `out`, picking the command from the extension.
fn export(s: &mut Session, out: &str, args: &[String]) -> Result<Value, String> {
    let ext = std::path::Path::new(out).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let cmd = match ext.as_str() {
        "png" => "export.png",
        "jpg" | "jpeg" => "export.jpeg",
        "svg" => "export.svg",
        "pdf" => "export.pdf",
        "gif" => "export.gif",
        "mp4" => "export.mp4",
        other => return Err(format!("unknown export type `.{other}` (png, jpg, svg, pdf, gif, mp4)")),
    };
    let mut p = json!({"path": out, "transparent": flag(args, "--transparent")});
    if let Some(o) = p.as_object_mut() {
        // Without --all-pages each command keeps its own default (a video: every page).
        if flag(args, "--all-pages") {
            o.insert("all_pages".into(), json!(true));
        }
        if let Some(n) = num(args, "--page")? {
            o.insert("page".into(), json!(n.max(0.0) as u64));
        }
        if let Some(sc) = num(args, "--scale")? {
            o.insert("scale".into(), json!(sc));
        }
        if let Some(dpi) = num(args, "--dpi")? {
            o.insert("dpi".into(), json!(dpi));
        }
        if let Some(fps) = num(args, "--fps")? {
            o.insert("fps".into(), json!(fps));
        }
        if let Some(q) = opt(args, "--quality") {
            o.insert("quality".into(), json!(q));
        }
    }
    exec(s, cmd, p)
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(cmd) = args.first() else {
        println!("{USAGE}");
        return Ok(());
    };
    let rest = args.get(1..).unwrap_or_default();
    match cmd.as_str() {
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "commands" => {
            let mut s = Session::new();
            let list = exec(&mut s, "engine.commands", json!({"prefix": rest.first().map(String::as_str).unwrap_or("")}))?;
            for c in list.as_array().into_iter().flatten() {
                println!(
                    "{:<26} {}\n{:<26} {}",
                    c["id"].as_str().unwrap_or(""),
                    c["label"].as_str().unwrap_or(""),
                    "",
                    c["params"].as_str().unwrap_or("")
                );
            }
            Ok(())
        }
        "presets" => {
            let mut s = Session::new();
            print(&exec(&mut s, "presets.list", json!({}))?);
            Ok(())
        }
        "templates" => {
            let mut s = Session::new();
            print(&exec(&mut s, "template.list", json!({}))?);
            Ok(())
        }
        "new" => {
            let out = opt(rest, "-o").ok_or("new needs -o OUT.genha")?;
            let mut s = session();
            if let Some(t) = opt(rest, "--template") {
                exec(&mut s, "template.use", json!({"id": t}))?;
            } else {
                let mut p = json!({});
                if let Some(o) = p.as_object_mut() {
                    if let Some(pr) = opt(rest, "--preset") {
                        o.insert("preset".into(), json!(pr));
                    }
                    if let Some(w) = num(rest, "--width")? {
                        o.insert("width".into(), json!(w));
                    }
                    if let Some(h) = num(rest, "--height")? {
                        o.insert("height".into(), json!(h));
                    }
                }
                exec(&mut s, "file.new", p)?;
            }
            print(&exec(&mut s, "file.saveAs", json!({"path": out}))?);
            Ok(())
        }
        "info" => {
            let file = rest.first().ok_or("info needs a file")?;
            let mut s = Session::new();
            open(&mut s, file)?;
            print(&exec(&mut s, "document.inspect", json!({"all_pages": true}))?);
            Ok(())
        }
        "export" => {
            let file = rest.first().ok_or("export needs a file")?;
            let out = opt(rest, "-o").ok_or("export needs -o OUT")?;
            let mut s = session();
            open(&mut s, file)?;
            print(&export(&mut s, out, rest)?);
            Ok(())
        }
        "run" => {
            let script = rest.first().ok_or("run needs a script file")?;
            let text = std::fs::read_to_string(script).map_err(|e| format!("cannot read {script}: {e}"))?;
            let list: Value = serde_json::from_str(&text).map_err(|e| format!("{script}: {e}"))?;
            let list = list.as_array().ok_or("the script must be a JSON array of {command, params}")?;
            let mut s = session();
            if let Some(f) = opt(rest, "--open") {
                open(&mut s, f)?;
            }
            for (i, step) in list.iter().enumerate() {
                let c = step.get("command").and_then(Value::as_str).ok_or(format!("step {i}: missing `command`"))?;
                let r = exec(&mut s, c, step.get("params").cloned().unwrap_or(json!({}))).map_err(|e| format!("step {i} ({c}): {e}"))?;
                eprintln!("{c}: {r}");
            }
            if let Some(out) = opt(rest, "--save") {
                print(&exec(&mut s, "file.saveAs", json!({"path": out}))?);
            }
            if let Some(out) = opt(rest, "--export") {
                print(&export(&mut s, out, rest)?);
            }
            Ok(())
        }
        "exec" => {
            let c = rest.first().ok_or("exec needs a command id")?;
            let params: Value = match rest.get(1).filter(|a| !a.starts_with("--")) {
                Some(j) => serde_json::from_str(j).map_err(|e| format!("params: {e}"))?,
                None => json!({}),
            };
            let mut s = session();
            if let Some(f) = opt(rest, "--open") {
                open(&mut s, f)?;
            }
            print(&exec(&mut s, c, params)?);
            Ok(())
        }
        "quick" => {
            let action = rest.first().ok_or("quick needs an action")?;
            let input = rest.get(1).ok_or("quick needs an input picture")?;
            let out = opt(rest, "-o").ok_or("quick needs -o OUT")?;
            let mut p = json!({"action": action, "input": input, "output": out});
            if let Some(o) = p.as_object_mut() {
                for key in ["scale", "width", "height", "tolerance", "intensity", "quality", "quarter_turns", "amount", "block"] {
                    if let Some(v) = num(rest, &format!("--{key}"))? {
                        o.insert(key.into(), json!(v));
                    }
                }
                for key in ["preset", "fit", "method", "model", "color"] {
                    if let Some(v) = opt(rest, &format!("--{key}")) {
                        o.insert(key.into(), json!(v));
                    }
                }
            }
            let mut s = with_models(Session::new());
            print(&exec(&mut s, "quick.image", p)?);
            Ok(())
        }
        "model" => {
            let mut s = with_models(Session::new());
            let r = match (rest.first().map(String::as_str), rest.get(1)) {
                (None | Some("status"), _) => exec(&mut s, "model.status", json!({}))?,
                (Some("download"), name) => {
                    let spec = genha_segment::model(name.map(String::as_str).unwrap_or(genha_segment::DEFAULT_MODEL)).map_err(|e| e.to_string())?;
                    eprintln!(
                        "downloading {} ({:.1} MB) from {}",
                        spec.label,
                        spec.bytes as f64 / 1e6,
                        genha_engine::cmd::models::download_url(spec)
                    );
                    exec(&mut s, "model.download", json!({"name": spec.name}))?
                }
                (Some("remove"), Some(name)) => exec(&mut s, "model.remove", json!({"name": name}))?,
                (Some(other), _) => return Err(format!("unknown model command `{other}` (status, download [NAME], remove NAME)")),
            };
            print(&r);
            Ok(())
        }
        "mcp" => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            match opt(rest, "--attach") {
                Some(addr) => genha_mcp::Server::new(genha_mcp::Remote::new(addr)).serve(stdin.lock(), stdout.lock()),
                None => {
                    let hook: genha_mcp::RenderHook = Box::new(|s: &Session, page: usize, scale: f64| {
                        let d = s.active().ok_or("no document open")?;
                        let opts = genha_render::RenderOptions { scale, transparent_background: false };
                        let r = genha_render::render_page(&d.doc, page, &s.fonts, &opts).map_err(|e| e.to_string())?;
                        genha_render::encode_png(&r).map_err(|e| e.to_string())
                    });
                    let mut backend = genha_mcp::Headless::new(Some(hook));
                    backend.session.fonts = Arc::new(genha_text::FontBook::system());
                    backend.session.models = with_models(Session::new()).models;
                    genha_mcp::Server::new(backend).serve(stdin.lock(), stdout.lock())
                }
            }
            .map_err(|e| e.to_string())
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    }
}
