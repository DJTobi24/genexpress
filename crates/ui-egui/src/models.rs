//! Background removal with the AI model: the method picker in the picture panel, the prompt that
//! offers to download the model the first time, and the work that runs off the UI thread on the
//! desktop (the download with progress and Cancel, then finding the subject), like the MP4 export.
//!
//! The engine does the work (`genha_engine::cmd::models` and `content::background_job`); this is
//! only how it looks. In the browser there are no threads: the model is fetched with the
//! browser's `fetch` (from `models/<file>` next to the app, or `?models=<address>` in the page's
//! address) and pictures are processed in place.

use serde_json::{Value, json};

use crate::App;
use genha_engine::cmd::models;
use genha_i18n::tr;
use genha_segment::ModelSpec;

/// The commands that find a picture's subject (and so take a `method`).
pub fn is_background(cmd: &str) -> bool {
    matches!(cmd, "image.removeBackground" | "image.changeBackground" | "image.blurBackground")
}

/// How the subject is found, as offered in the picture panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Auto,
    Ml,
    Color,
}

impl Method {
    const ALL: [Method; 3] = [Method::Auto, Method::Ml, Method::Color];

    pub fn id(self) -> &'static str {
        match self {
            Method::Auto => "auto",
            Method::Ml => "ml",
            Method::Color => "color",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Method::Auto => tr("Automatic"),
            Method::Ml => tr("AI model"),
            Method::Color => tr("Colour-based"),
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Method::Auto => tr("The AI model when it is downloaded, else colour-based"),
            Method::Ml => tr("Finds the subject on busy photos (the model is downloaded once)"),
            Method::Color => tr("Removes the backdrop connected to the picture's edges: best on plain backgrounds"),
        }
    }
}

/// Background removal state kept by the app.
#[derive(Default)]
pub struct MlState {
    pub method: Method,
    /// A background command waiting for the user to decide about downloading the model.
    prompt: Option<(String, Value)>,
    /// Download the larger, high-quality model from the prompt.
    high_quality: bool,
    /// The user was asked this run already (Automatic asks once).
    asked: bool,
    #[cfg(not(target_arch = "wasm32"))]
    download: Option<Download>,
    #[cfg(not(target_arch = "wasm32"))]
    work: Option<Work>,
    /// The model being fetched by the browser.
    #[cfg(target_arch = "wasm32")]
    fetching: Option<&'static ModelSpec>,
}

/// What the picture panel shows (read before the panel is drawn, applied after).
#[derive(Clone, Debug, Default)]
pub struct Panel {
    pub method: Method,
    /// The installed model's label.
    installed: Option<&'static str>,
    /// Work in progress: what, and how far (0..1, or none when unknown).
    busy: Option<(String, Option<f32>)>,
    cancel: bool,
}

/// The panel's view of the state.
pub fn panel(app: &App) -> Panel {
    Panel { method: app.ml.method, installed: models::preferred(&app.session).map(|m| m.label), busy: busy(app), cancel: false }
}

fn busy(app: &App) -> Option<(String, Option<f32>)> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(d) = &app.ml.download {
            return Some((tr!("Downloading the {name} model…", name = d.spec.name), Some(d.fraction())));
        }
        if let Some(w) = &app.ml.work {
            return Some((w.what.clone(), w.fraction()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(spec) = app.ml.fetching {
        return Some((tr!("Downloading the {name} model…", name = spec.name), None));
    }
    None
}

/// The method picker and progress, under the picture's quick actions.
pub fn draw(ui: &mut egui::Ui, p: &mut Panel) {
    if !genha_segment::INFERENCE {
        return;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Find the subject"));
        egui::ComboBox::from_id_salt("bg-method").selected_text(p.method.label()).show_ui(ui, |ui| {
            for m in Method::ALL {
                ui.selectable_value(&mut p.method, m, m.label()).on_hover_text(m.hint());
            }
        });
    });
    match (p.method, p.installed) {
        (Method::Color, _) => {}
        (_, Some(label)) => {
            ui.weak(tr!("AI model: {label}", label = label));
        }
        (_, None) => {
            ui.weak(tr("The AI model is not downloaded yet"));
        }
    }
    if let Some((what, fraction)) = &p.busy {
        // Wrapped, so a long message never widens the panel.
        ui.horizontal_wrapped(|ui| {
            if fraction.is_none() {
                ui.spinner();
            }
            ui.label(what);
        });
        if let Some(f) = fraction {
            ui.add(egui::ProgressBar::new(*f).desired_width(ui.available_width().min(240.0)).show_percentage());
        }
        if ui.button(tr("Cancel")).clicked() {
            p.cancel = true;
        }
    }
}

/// Applies what the panel changed.
pub fn apply(app: &mut App, p: Panel) {
    app.ml.method = p.method;
    if p.cancel {
        cancel(app);
    }
}

fn cancel(app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(d) = &app.ml.download {
            d.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(w) = &app.ml.work {
            w.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        // The browser's fetch cannot be stopped from here; its result is dropped.
        app.ml.fetching = None;
        app.ml.prompt = None;
    }
}

/// The model the prompt offers.
fn offered(app: &App) -> &'static ModelSpec {
    let name = if app.ml.high_quality { "u2net" } else { genha_segment::DEFAULT_MODEL };
    genha_segment::model(name).unwrap_or(&genha_segment::MODELS[0])
}

/// Runs background command `cmd` with the panel's method, asking about the model first when it
/// would be used but is not downloaded.
pub fn start(app: &mut App, ctx: &egui::Context, cmd: &str, mut p: Value) {
    // A build without the model runtime (the default browser build) only has the colour method.
    let method = if genha_segment::INFERENCE { app.ml.method } else { Method::Color };
    if let Some(o) = p.as_object_mut() {
        o.entry("method").or_insert(json!(method.id()));
    }
    if busy(app).is_some() {
        app.set_status(tr("Background removal is busy: wait for it to finish or cancel it"));
        return;
    }
    let installed = models::preferred(&app.session).is_some();
    let ask = !installed
        && match p.get("method").and_then(Value::as_str) {
            Some("ml") => true,
            Some("auto") => !app.ml.asked,
            _ => false,
        };
    if ask {
        app.ml.prompt = Some((cmd.to_string(), p));
        return;
    }
    run(app, ctx, cmd, p);
}

/// Runs the command: the colour method (and the browser) in place, the model on a worker thread.
fn run(app: &mut App, ctx: &egui::Context, cmd: &str, p: Value) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let job = match genha_engine::cmd::content::background_job(&app.session, cmd, &p) {
            Ok(job) => job,
            Err(e) => {
                app.set_status(e.to_string());
                return;
            }
        };
        if job.finder.method() == "ml" {
            app.ml.work = Work::start(ctx, job);
            if app.ml.work.is_none() {
                app.set_status(tr("Could not start background removal"));
            }
            return;
        }
    }
    let _ = ctx; // only the desktop's worker threads need it
    app.run(cmd, &p);
}

/// The download prompt and finished work; call once per frame.
pub fn show(app: &mut App, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    poll(app, ctx);
    #[cfg(target_arch = "wasm32")]
    web_poll(app, ctx);
    prompt(app, ctx);
}

/// What the user chose in the prompt.
enum Choice {
    Download,
    Colour,
    Close,
    AllowNetwork,
}

fn prompt(app: &mut App, ctx: &egui::Context) {
    if app.ml.prompt.is_none() {
        return;
    }
    let spec = offered(app);
    let network = models::network_allowed(&app.session);
    let downloading = busy(app);
    let mut high = app.ml.high_quality;
    let mut choice = None;
    #[cfg(not(target_arch = "wasm32"))]
    let source = models::download_url(spec);
    #[cfg(target_arch = "wasm32")]
    let source = web_url(spec);
    let host = source.split('/').nth(2).unwrap_or("the internet").to_string();
    egui::Window::new(tr("Remove background with the AI model"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(tr("GenHa can find the subject of a photo with an AI model, so backgrounds come off busy photos too, not only plain ones."));
            ui.add_space(4.0);
            ui.label(tr!(
                "The model ({model}, licence {licence}) is downloaded once from {host} and kept on this device. Nothing is uploaded: pictures stay on your device.",
                model = spec.label,
                licence = spec.licence,
                host = host
            ));
            ui.add_enabled_ui(downloading.is_none(), |ui| {
                crate::theme::switch(ui, &mut high, tr("High quality model instead (U²-Net, 176 MB, slower)"));
            });
            ui.add_space(6.0);
            if let Some((what, fraction)) = &downloading {
                ui.horizontal(|ui| {
                    match fraction {
                        Some(f) => {
                            ui.add(egui::ProgressBar::new(*f).desired_width(200.0).show_percentage());
                        }
                        None => {
                            ui.spinner();
                        }
                    }
                    ui.label(what);
                });
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(Choice::Close);
                }
                return;
            }
            if !network {
                ui.colored_label(ui.visuals().warn_fg_color, tr("Network use is turned off."));
            }
            ui.horizontal(|ui| {
                if network {
                    if ui.button(tr!("Download ({size} MB)", size = format!("{:.1}", spec.bytes as f64 / 1e6))).clicked() {
                        choice = Some(Choice::Download);
                    }
                } else if ui.button(tr("Allow network use")).clicked() {
                    choice = Some(Choice::AllowNetwork);
                }
                if ui.button(tr("Use colour-based")).on_hover_text(Method::Color.hint()).clicked() {
                    choice = Some(Choice::Colour);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(Choice::Close);
                }
            });
        });
    app.ml.high_quality = high;
    match choice {
        Some(Choice::Download) => {
            app.ml.asked = true;
            begin_download(app, ctx, spec);
        }
        Some(Choice::Colour) => {
            app.ml.asked = true;
            if let Some((cmd, mut p)) = app.ml.prompt.take() {
                if let Some(o) = p.as_object_mut() {
                    o.insert("method".into(), json!("color"));
                }
                run(app, ctx, &cmd, p);
            }
        }
        Some(Choice::Close) => {
            app.ml.asked = true;
            cancel(app);
            app.ml.prompt = None;
        }
        Some(Choice::AllowNetwork) => {
            app.run("settings.network", &json!({"on": true}));
        }
        None => {}
    }
}

/// The model arrived: keep it, then run the command that was waiting for it.
fn installed(app: &mut App, ctx: &egui::Context, spec: &'static ModelSpec, bytes: &[u8]) {
    match models::install(&mut app.session, spec, bytes) {
        Ok(_) => {
            app.set_status(tr!("Downloaded the {model} model", model = spec.label));
            if let Some((cmd, p)) = app.ml.prompt.take() {
                run(app, ctx, &cmd, p);
            }
        }
        Err(e) => {
            app.ml.prompt = None;
            app.set_status(e.to_string());
        }
    }
}

// ---------- desktop: worker threads ----------

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// A model download on its own thread.
#[cfg(not(target_arch = "wasm32"))]
struct Download {
    spec: &'static ModelSpec,
    progress: Arc<(AtomicU64, AtomicU64)>,
    cancel: Arc<AtomicBool>,
    done: std::sync::mpsc::Receiver<Result<Vec<u8>, String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Download {
    fn fraction(&self) -> f32 {
        let (n, total) = (self.progress.0.load(Ordering::Relaxed), self.progress.1.load(Ordering::Relaxed));
        if total == 0 { 0.0 } else { (n as f64 / total as f64).clamp(0.0, 1.0) as f32 }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn begin_download(app: &mut App, ctx: &egui::Context, spec: &'static ModelSpec) {
    let url = models::download_url(spec);
    let progress = Arc::new((AtomicU64::new(0), AtomicU64::new(spec.bytes)));
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, done) = std::sync::mpsc::channel();
    let (p, c, ctx2) = (progress.clone(), cancel.clone(), ctx.clone());
    let spawned = std::thread::Builder::new().name("model download".into()).spawn(move || {
        let r = models::fetch(spec, &url, &mut |n, total| {
            p.0.store(n, Ordering::Relaxed);
            p.1.store(total, Ordering::Relaxed);
            ctx2.request_repaint();
            !c.load(Ordering::Relaxed)
        });
        // Nothing to do if the app has gone away: it no longer waits for the result.
        tx.send(r.map_err(|e| e.to_string())).ok();
        ctx2.request_repaint();
    });
    match spawned {
        Ok(_) => app.ml.download = Some(Download { spec, progress, cancel, done }),
        Err(e) => app.set_status(tr!("Could not start the download: {error}", error = e)),
    }
}

/// Background removal with the model on its own thread.
#[cfg(not(target_arch = "wasm32"))]
struct Work {
    what: String,
    progress: Arc<(AtomicUsize, AtomicUsize)>,
    cancel: Arc<AtomicBool>,
    done: std::sync::mpsc::Receiver<Result<genha_engine::cmd::content::BackgroundDone, String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Work {
    fn start(ctx: &egui::Context, job: genha_engine::cmd::content::BackgroundJob) -> Option<Work> {
        let n = job.len();
        let what = if n > 1 { tr!("Finding the subjects of {n} pictures…", n = n) } else { tr!("Finding the subject…") };
        let progress = Arc::new((AtomicUsize::new(0), AtomicUsize::new(n)));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, done) = std::sync::mpsc::channel();
        let (p, c, ctx) = (progress.clone(), cancel.clone(), ctx.clone());
        let spawned = std::thread::Builder::new().name("background removal".into()).spawn(move || {
            let r = job.run(&mut |i, total| {
                p.0.store(i, Ordering::Relaxed);
                p.1.store(total, Ordering::Relaxed);
                ctx.request_repaint();
                !c.load(Ordering::Relaxed)
            });
            tx.send(r.map_err(|e| e.to_string())).ok();
            ctx.request_repaint();
        });
        spawned.ok().map(|_| Work { what, progress, cancel, done })
    }

    /// Done share; unknown for a single picture (the model runs in one go).
    fn fraction(&self) -> Option<f32> {
        let (i, n) = (self.progress.0.load(Ordering::Relaxed), self.progress.1.load(Ordering::Relaxed));
        (n > 1).then(|| (i as f32 / n as f32).clamp(0.0, 1.0))
    }
}

/// Picks up finished downloads and work.
#[cfg(not(target_arch = "wasm32"))]
fn poll(app: &mut App, ctx: &egui::Context) {
    use std::sync::mpsc::TryRecvError;
    if let Some(d) = &app.ml.download {
        let spec = d.spec;
        match d.done.try_recv() {
            Err(TryRecvError::Empty) => {}
            Ok(Ok(bytes)) => {
                app.ml.download = None;
                installed(app, ctx, spec, &bytes);
            }
            Ok(Err(e)) => {
                app.ml.download = None;
                app.ml.prompt = None;
                app.set_status(e);
            }
            Err(TryRecvError::Disconnected) => {
                app.ml.download = None;
                app.ml.prompt = None;
                app.set_status(tr("The model download stopped unexpectedly"));
            }
        }
    }
    if let Some(w) = &app.ml.work {
        match w.done.try_recv() {
            Err(TryRecvError::Empty) => {}
            Ok(Ok(done)) => {
                app.ml.work = None;
                if let Err(e) = genha_engine::cmd::content::apply_background(&mut app.session, done) {
                    app.set_status(e.to_string());
                }
            }
            Ok(Err(e)) => {
                app.ml.work = None;
                app.set_status(if e == "cancelled" { tr!("Background removal cancelled") } else { e });
            }
            Err(TryRecvError::Disconnected) => {
                app.ml.work = None;
                app.set_status(tr("Background removal stopped unexpectedly"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppOptions;
    use base64::Engine as _;

    fn frames(app: &mut App, ctx: &egui::Context, n: usize) {
        for _ in 0..n {
            let input =
                egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| app.ui(ui));
            out.textures_delta.clear();
        }
    }

    #[test]
    fn prompt_then_colour_fallback() {
        if !genha_segment::INFERENCE {
            return;
        }
        let ctx = egui::Context::default();
        let mut app = App::new(std::sync::Arc::new(genha_text::FontBook::new()), AppOptions::default());
        app.session.store = Some(Box::new(genha_engine::store::MemStore::default()));
        app.run("file.new", &json!({"preset": "instagram-post"}));
        let img = image_png();
        let id = app.run("image.place", &json!({"data": img})).unwrap()["id"].as_u64().unwrap();
        app.run("select.set", &json!({"ids": [id]}));
        frames(&mut app, &ctx, 2);
        // The AI method without a model asks first, and nothing changes yet.
        app.ml.method = Method::Ml;
        let undo = app.session.active().unwrap().history.undo.len();
        start(&mut app, &ctx, "image.removeBackground", json!({}));
        assert!(app.ml.prompt.is_some());
        assert_eq!(app.session.active().unwrap().history.undo.len(), undo);
        frames(&mut app, &ctx, 2);
        // Automatic asks once per run; after the answer it falls back to the colour method.
        app.ml.prompt = None;
        app.ml.asked = true;
        app.ml.method = Method::Auto;
        let before = app.session.active().unwrap().history.undo.len();
        start(&mut app, &ctx, "image.removeBackground", json!({}));
        assert!(app.ml.prompt.is_none());
        assert_eq!(app.session.active().unwrap().history.undo.len(), before + 1);
        // Colour-based never asks.
        app.ml.asked = false;
        app.ml.method = Method::Color;
        start(&mut app, &ctx, "image.blurBackground", json!({"amount": 4}));
        assert!(app.ml.prompt.is_none());
        assert_eq!(app.session.active().unwrap().history.undo.len(), before + 2);
        frames(&mut app, &ctx, 2);
        assert!(is_background("image.changeBackground") && !is_background("image.filter"));
        let p = panel(&app);
        assert_eq!((p.method, p.installed, p.busy.is_none()), (Method::Color, None, true));
    }

    fn image_png() -> String {
        let img = genha_effects::RgbaImage::from_fn(40, 30, |x, _| [if x < 20 { 250 } else { 30 }, 120, 60, 255].into());
        let png = genha_effects::encode(&img, genha_effects::OutputFormat::Png).unwrap();
        base64::engine::general_purpose::STANDARD.encode(&png)
    }
}

// ---------- browser: fetch ----------

/// A fetched model's bytes (or why the fetch failed).
#[cfg(target_arch = "wasm32")]
type Fetched = (&'static ModelSpec, Result<Vec<u8>, String>);

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// A fetched model, waiting for the next frame.
    static FETCHED: std::cell::RefCell<Option<Fetched>> = const { std::cell::RefCell::new(None) };
}

/// Where the browser fetches `spec` from: `<models>/<file>` with `?models=<address>` in the page's
/// address, else `models/<file>` next to the app (the browser cannot download from the model's
/// own site, which does not allow other sites to fetch it).
#[cfg(target_arch = "wasm32")]
fn web_url(spec: &ModelSpec) -> String {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    let base = search
        .trim_start_matches('?')
        .split('&')
        .find_map(|kv| kv.strip_prefix("models="))
        .and_then(|v| js_sys::decode_uri_component(v).ok())
        .and_then(|v| v.as_string())
        .filter(|v| v.starts_with("https://") || v.starts_with('/') || !v.contains(':'));
    match base {
        Some(b) => format!("{}/{}", b.trim_end_matches('/'), spec.file),
        None => format!("models/{}", spec.file),
    }
}

#[cfg(target_arch = "wasm32")]
fn begin_download(app: &mut App, ctx: &egui::Context, spec: &'static ModelSpec) {
    app.ml.fetching = Some(spec);
    let url = web_url(spec);
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let r = fetch_bytes(&url, spec.bytes).await;
        FETCHED.with(|f| *f.borrow_mut() = Some((spec, r)));
        ctx.request_repaint();
    });
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes(url: &str, max: u64) -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast as _;
    let js = |e: wasm_bindgen::JsValue| e.as_string().unwrap_or_else(|| format!("{e:?}"));
    let window = web_sys::window().ok_or("no window")?;
    let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await.map_err(|e| format!("cannot reach {url}: {}", js(e)))?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "not a response".to_string())?;
    if !resp.ok() {
        return Err(format!("{url}: the server answered {}", resp.status()));
    }
    let buf = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(js)?).await.map_err(js)?;
    let arr = js_sys::Uint8Array::new(&buf);
    if u64::from(arr.length()) > max {
        return Err(format!("{url}: the file is larger than the model"));
    }
    Ok(arr.to_vec())
}

#[cfg(target_arch = "wasm32")]
fn web_poll(app: &mut App, ctx: &egui::Context) {
    let Some((spec, r)) = FETCHED.with(|f| f.borrow_mut().take()) else { return };
    // Cancelled meanwhile: drop it.
    if app.ml.fetching.take().is_none() {
        return;
    }
    match r {
        Ok(bytes) => installed(app, ctx, spec, &bytes),
        Err(e) => {
            app.ml.prompt = None;
            app.set_status(e);
        }
    }
}
