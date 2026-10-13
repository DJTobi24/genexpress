//! Optional machine-learning models (background removal on busy photos): which are installed,
//! downloading and removing them, and the switch that keeps GenHa off the network.
//!
//! No model ships with GenHa. `model.download` fetches one over HTTPS (desktop, CLI, MCP; the
//! browser app fetches it from its own prompt and hands the bytes to [`install`]) into the
//! session's model store — `models/<file>` in the app's data folder — after checking its pinned
//! SHA-256 ([`genha_segment::verify`]). Every load checks it again, so a damaged file is reported,
//! never run.

use serde_json::{Value, json};

use super::{always, bad, cmd, str_param};
use crate::store::Store;
use crate::{CommandSpec, EngineError, Result, Session};
use genha_segment::{MODELS, ModelSpec, Segmenter};

/// The store folder models are kept in.
pub const DIR: &str = "models";
/// Where the network switch is kept.
const NETWORK_KEY: &str = "settings/network";

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "model.status",
            "Background Removal Models",
            [],
            None,
            "{} — {models: [{name, label, summary, bytes, installed, licence, source, url}], network: bool, downloads: bool (this host can download), available: bool (this build can run models), folder?} for the ML background removal",
            always,
            status
        ),
        cmd!(
            query "model.download",
            "Download Background Removal Model",
            [],
            None,
            "{name?: u2netp (4.6 MB, default) | u2net (176 MB), url?: an https mirror serving the same file} — downloads over HTTPS into the app's data folder and checks its SHA-256; needs `settings.network` on (the browser app downloads from its own prompt)",
            always,
            download
        ),
        cmd!(query "model.remove", "Remove Background Removal Model", [], None, "{name}", always, remove),
        cmd!(
            query "settings.network",
            "Allow Network Use",
            [],
            None,
            "{on?: bool} — whether GenHa may use the network (model downloads; toggles without `on`); kept between runs. GENHA_OFFLINE=1 in the environment turns it off",
            always,
            network
        ),
    ]
}

fn store(s: &Session) -> Option<&dyn Store> {
    s.models.as_deref().or(s.store.as_deref())
}

fn store_mut(s: &mut Session) -> Option<&mut (dyn Store + 'static)> {
    match s.models.as_deref_mut() {
        Some(m) => Some(m),
        None => s.store.as_deref_mut(),
    }
}

/// The store key of `spec`'s file.
pub fn key(spec: &ModelSpec) -> String {
    format!("{DIR}/{}", spec.file)
}

/// Whether `spec`'s file is in the model store (checked when it is loaded).
pub fn installed(s: &Session, spec: &ModelSpec) -> bool {
    let k = key(spec);
    store(s).is_some_and(|st| st.list(DIR).iter().any(|(name, _)| *name == k))
}

/// The installed model to use when none is named: the larger (better) one when both are there.
pub fn preferred(s: &Session) -> Option<&'static ModelSpec> {
    MODELS.iter().rev().find(|m| installed(s, m))
}

/// The model called `name`.
pub fn spec(name: &str, cmd: &str) -> Result<&'static ModelSpec> {
    genha_segment::model(name).map_err(|e| bad(cmd, e.to_string()))
}

/// Whether the environment forbids network use (`GENHA_OFFLINE` set to anything but `0`).
pub fn offline_env() -> bool {
    std::env::var_os("GENHA_OFFLINE").is_some_and(|v| !v.is_empty() && v != "0")
}

/// Whether GenHa may use the network now.
pub fn network_allowed(s: &Session) -> bool {
    s.network && !offline_env()
}

/// Where to download `spec` from: `GENHA_MODEL_BASE_URL/<file>` when that is set (a mirror; the
/// file must still match the pinned checksum), else the model's own address.
pub fn download_url(spec: &ModelSpec) -> String {
    match std::env::var("GENHA_MODEL_BASE_URL") {
        Ok(base) if !base.trim().is_empty() => format!("{}/{}", base.trim().trim_end_matches('/'), spec.file),
        _ => spec.url.to_string(),
    }
}

/// Checks `bytes` against `spec` and keeps them in the model store.
pub fn install(s: &mut Session, spec: &'static ModelSpec, bytes: &[u8]) -> Result<Value> {
    genha_segment::verify(spec, bytes)?;
    let st = store_mut(s).ok_or_else(|| EngineError::Other("there is nowhere to keep downloaded models in this app".into()))?;
    st.write(&key(spec), bytes).map_err(|e| EngineError::Other(format!("could not save the {} model: {e}", spec.name)))?;
    let place = st.locate(&key(spec));
    genha_segment::forget_cached();
    s.post(format!("Installed the {} background removal model", spec.label));
    Ok(json!({"ok": true, "name": spec.name, "bytes": bytes.len(), "path": place}))
}

/// The loaded model `spec` (from the cache, or read from the store and checked), `None` when it
/// is not installed.
pub fn segmenter(s: &Session, spec: &'static ModelSpec) -> Result<Option<Segmenter>> {
    if !installed(s, spec) {
        return Ok(None);
    }
    let st = store(s);
    Ok(genha_segment::cached(spec, || st.and_then(|st| st.read(&key(spec))))?)
}

/// The model file's bytes when it is installed but not loaded yet, so a worker thread can load
/// it ([`genha_segment::cached`]); `None` when it is already loaded or not installed.
pub fn unloaded_bytes(s: &Session, spec: &'static ModelSpec) -> Option<Vec<u8>> {
    if genha_segment::peek(spec).is_some() || !installed(s, spec) {
        return None;
    }
    store(s)?.read(&key(spec))
}

/// Downloads `spec` (from [`download_url`]) and checks it, without keeping it: the slow part of
/// `model.download`, for a worker thread. `progress(done, total)` returns false to cancel.
#[cfg(not(target_arch = "wasm32"))]
pub fn fetch(spec: &'static ModelSpec, url: &str, progress: &mut dyn FnMut(u64, u64) -> bool) -> Result<Vec<u8>> {
    if offline_env() {
        return Err(EngineError::Other("network use is turned off (GENHA_OFFLINE)".into()));
    }
    Ok(genha_segment::download::fetch(spec, url, progress)?)
}

fn status(s: &mut Session, _: &Value) -> Result<Value> {
    let models: Vec<Value> = MODELS
        .iter()
        .map(|m| {
            json!({
                "name": m.name, "label": m.label, "summary": m.summary, "bytes": m.bytes,
                "installed": installed(s, m), "licence": m.licence, "source": m.source, "url": download_url(m),
            })
        })
        .collect();
    let mut out = json!({
        "models": models,
        "network": network_allowed(s),
        "downloads": cfg!(not(target_arch = "wasm32")) && store(s).is_some(),
        "available": genha_segment::INFERENCE,
        "default": genha_segment::DEFAULT_MODEL,
    });
    if let (Some(st), Some(o)) = (store(s), out.as_object_mut()) {
        o.insert("folder".into(), json!(st.locate(DIR)));
    }
    Ok(out)
}

fn requested<'a>(p: &'a Value, cmd: &str) -> Result<&'a str> {
    match p.get("name") {
        None | Some(Value::Null) => Ok(genha_segment::DEFAULT_MODEL),
        Some(Value::String(n)) => Ok(n),
        Some(_) => Err(bad(cmd, "`name` must be a model name (see model.status)")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn download(s: &mut Session, p: &Value) -> Result<Value> {
    let spec = spec(requested(p, "model.download")?, "model.download")?;
    if !network_allowed(s) {
        return Err(EngineError::Other("network use is turned off (settings.network, GENHA_OFFLINE): turn it on to download models".into()));
    }
    if store(s).is_none() {
        return Err(EngineError::Other("there is nowhere to keep downloaded models in this app".into()));
    }
    let url = match p.get("url") {
        None | Some(Value::Null) => download_url(spec),
        Some(Value::String(u)) => u.clone(),
        Some(_) => return Err(bad("model.download", "`url` must be an https address")),
    };
    let bytes = fetch(spec, &url, &mut |_, _| true)?;
    install(s, spec, &bytes)
}

#[cfg(target_arch = "wasm32")]
fn download(_: &mut Session, p: &Value) -> Result<Value> {
    spec(requested(p, "model.download")?, "model.download")?;
    Err(EngineError::Other("in the browser, models are downloaded from the Remove background prompt".into()))
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let name = str_param(p, "name").ok_or_else(|| bad("model.remove", "missing string `name`"))?;
    let spec = spec(name, "model.remove")?;
    let was = installed(s, spec);
    if let Some(st) = store_mut(s) {
        st.remove(&key(spec));
    }
    genha_segment::forget_cached();
    Ok(json!({"ok": true, "removed": was}))
}

fn network(s: &mut Session, p: &Value) -> Result<Value> {
    let on = match p.get("on") {
        None | Some(Value::Null) => !s.network,
        Some(v) => v.as_bool().ok_or_else(|| bad("settings.network", "`on` must be true or false"))?,
    };
    s.network = on;
    if let Some(st) = s.store.as_mut() {
        // Losing the setting only means it is back to its default next run.
        if let Err(e) = st.write(NETWORK_KEY, if on { b"on" } else { b"off" }) {
            s.post(format!("Could not keep the network setting: {e}"));
        }
    }
    Ok(json!({"on": on, "allowed": network_allowed(s)}))
}

/// Reads the network setting kept in the session's store.
pub fn load_preferences(s: &mut Session) {
    if let Some(v) = s.store.as_ref().and_then(|st| st.read(NETWORK_KEY)) {
        s.network = v.as_slice() != b"off";
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineError;
    use crate::store::MemStore;
    use base64::Engine as _;

    /// A session with a design holding one busy picture (selected), and a model store.
    fn session() -> (Session, u64) {
        let mut s = Session::new();
        s.store = Some(Box::new(MemStore::default()));
        s.execute("file.new", &json!({"preset": "instagram-post"})).unwrap();
        let img = image::RgbaImage::from_fn(96, 64, |x, y| {
            if (32..64).contains(&x) && (16..48).contains(&y) {
                image::Rgba([200, 40, 40, 255])
            } else {
                image::Rgba([((x * 37) % 256) as u8, ((y * 53) % 256) as u8, ((x * y) % 256) as u8, 255])
            }
        });
        let mut png = vec![];
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let data = base64::engine::general_purpose::STANDARD.encode(&png);
        let id = s.execute("image.place", &json!({"data": data, "name": "busy.png"})).unwrap()["id"].as_u64().unwrap();
        (s, id)
    }

    #[test]
    fn method_selection_and_missing_model() {
        let (mut s, _) = session();
        let st = s.execute("model.status", &json!({})).unwrap();
        assert_eq!(st["models"].as_array().unwrap().len(), MODELS.len());
        assert_eq!(st["models"][0]["installed"], false);
        assert_eq!(st["network"], true);
        // Auto without a model: the colour method.
        let r = s.execute("image.removeBackground", &json!({})).unwrap();
        assert_eq!((r["method"].as_str(), r["changed"].as_u64()), (Some("color"), Some(1)));
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.execute("image.blurBackground", &json!({"method": "color", "amount": 4})).unwrap()["method"], "color");
        assert_eq!(s.execute("image.changeBackground", &json!({"method": "auto", "color": "#ffffff"})).unwrap()["method"], "color");
        // ML without a model: a message that says how to get one (or, in a build without the
        // runtime, that this build cannot).
        for cmd in ["image.removeBackground", "image.blurBackground"] {
            let e = s.execute(cmd, &json!({"method": "ml"})).unwrap_err().to_string();
            assert!(if genha_segment::INFERENCE { e.contains("model.download") && e.contains("u2netp") } else { e.contains("colour-based") }, "{e}");
        }
        if genha_segment::INFERENCE {
            let e = s.execute("image.removeBackground", &json!({"method": "ml", "model": "u2net"})).unwrap_err().to_string();
            assert!(e.contains("\"u2net\"") && e.contains("176"), "{e}");
        }
        // Bad params.
        for p in [json!({"method": "magic"}), json!({"method": 3}), json!({"model": "rmbg-2"}), json!({"model": []})] {
            assert!(matches!(s.execute("image.removeBackground", &p), Err(EngineError::BadParams { .. })), "{p}");
        }
        // The quick action on bytes (the browser's) has only the colour method.
        let png = s.doc().unwrap().doc.assets.values().next().unwrap().data.to_vec();
        let p = json!({"action": "remove-background"});
        assert!(crate::cmd::content::quick_image_bytes(&png, "png", &p).is_ok());
        assert!(crate::cmd::content::quick_image_bytes(&png, "png", &json!({"action": "remove-background", "method": "ml"})).is_err());
    }

    #[test]
    fn checksum_mismatch_and_damaged_files() {
        let (mut s, _) = session();
        let spec = &MODELS[0];
        // A fake download is refused before anything is kept.
        let fake = vec![1u8; spec.bytes as usize];
        let e = install(&mut s, spec, &fake).unwrap_err().to_string();
        assert!(e.contains("checksum"), "{e}");
        assert!(install(&mut s, spec, b"short").unwrap_err().to_string().contains("size"));
        assert!(!installed(&s, spec));
        // A damaged file already in the folder is reported, never run. (The large model here:
        // another test may have the real small one loaded in this process's cache.)
        let big = &MODELS[1];
        s.store.as_mut().unwrap().write(&key(big), b"not a model").unwrap();
        assert!(installed(&s, big));
        assert_eq!(s.execute("model.status", &json!({})).unwrap()["models"][1]["installed"], true);
        for method in ["ml", "auto"] {
            let r = s.execute("image.removeBackground", &json!({"method": method}));
            if genha_segment::INFERENCE {
                let e = r.unwrap_err().to_string();
                assert!(e.contains("u2net ") && e.contains("download it again"), "{e}");
            } else {
                // Without the runtime, auto never tries the model.
                assert_eq!(r.is_ok(), method == "auto");
            }
        }
        // The colour method still works, and removing the model brings back auto = colour.
        assert_eq!(s.execute("image.removeBackground", &json!({"method": "color"})).unwrap()["method"], "color");
        assert_eq!(s.execute("model.remove", &json!({"name": "u2net"})).unwrap()["removed"], true);
        assert!(!installed(&s, big));
        assert_eq!(s.execute("model.remove", &json!({"name": "u2net"})).unwrap()["removed"], false);
        assert!(s.execute("model.remove", &json!({"name": "nope"})).is_err());
        assert_eq!(s.execute("image.removeBackground", &json!({})).unwrap()["method"], "color");
    }

    #[test]
    fn downloads_respect_the_network_switch_and_fail_cleanly() {
        let (mut s, _) = session();
        assert_eq!(s.execute("settings.network", &json!({"on": false})).unwrap()["on"], false);
        let e = s.execute("model.download", &json!({})).unwrap_err().to_string();
        assert!(e.contains("network use is turned off"), "{e}");
        // Kept between runs.
        let mut next = Session::new();
        next.store = s.store.take();
        load_preferences(&mut next);
        assert!(!next.network);
        assert_eq!(next.execute("settings.network", &json!({})).unwrap()["on"], true);
        assert!(next.execute("settings.network", &json!({"on": "yes"})).is_err());
        // Bad names and addresses.
        assert!(matches!(next.execute("model.download", &json!({"name": "rmbg"})), Err(EngineError::BadParams { .. })));
        assert!(matches!(next.execute("model.download", &json!({"name": 5})), Err(EngineError::BadParams { .. })));
        assert!(next.execute("model.download", &json!({"url": "http://example.com/u2netp.onnx"})).unwrap_err().to_string().contains("https"));
        assert!(next.execute("model.download", &json!({"url": 7})).is_err());
        #[cfg(not(target_arch = "wasm32"))]
        if !offline_env() {
            // A host that cannot exist (.invalid is reserved): an error message, nothing installed.
            let r = next.execute("model.download", &json!({"url": "https://genha-model.invalid/u2netp.onnx"}));
            assert!(r.is_err());
            assert!(!installed(&next, &MODELS[0]));
        }
        // Without anywhere to keep models.
        let mut bare = Session::new();
        assert!(bare.execute("model.download", &json!({})).unwrap_err().to_string().contains("nowhere"));
        assert_eq!(bare.execute("model.status", &json!({})).unwrap()["downloads"], false);
    }

    /// Runs the real model through the commands when it is on this machine (see
    /// `crates/segment/tests/real_model.rs`); skipped otherwise.
    #[test]
    fn real_model_through_the_commands() {
        if !genha_segment::INFERENCE {
            return;
        }
        let spec = &MODELS[0];
        let Some(dir) =
            std::env::var_os("GENHA_MODEL_DIR").map(std::path::PathBuf::from).or_else(|| crate::store::user_data_dir().map(|d| d.join(DIR)))
        else {
            return;
        };
        let Ok(bytes) = std::fs::read(dir.join(spec.file)) else {
            eprintln!("skipped: no {} model in {}", spec.name, dir.display());
            return;
        };
        let (mut s, pic) = session();
        s.models = Some(Box::new(MemStore::default()));
        install(&mut s, spec, &bytes).unwrap();
        assert!(s.store.as_ref().unwrap().list(DIR).is_empty(), "models go to the model store when there is one");
        let r = s.execute("image.removeBackground", &json!({})).unwrap();
        assert_eq!((r["method"].as_str(), r["model"].as_str()), (Some("ml"), Some("u2netp")));
        let d = s.doc().unwrap();
        let crate::doc::ElementKind::Image(im) = &d.doc.element(genha_doc::NodeId(pic)).unwrap().kind else { panic!() };
        let cut = genha_effects::decode(&d.doc.asset(im.asset).unwrap().data).unwrap();
        assert_eq!(cut.dimensions(), (96, 64));
        assert!(cut.pixels().any(|p| p[3] == 0) && cut.pixels().any(|p| p[3] == 255));
        let r = s.execute("image.blurBackground", &json!({"method": "ml"})).unwrap();
        assert_eq!(r["method"], "ml");
    }
}
