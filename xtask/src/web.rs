//! `cargo xtask web`: the browser build in `dist/web` (docs/development.md › Web).
//!
//! 1. `cargo build -p genha-web --target wasm32-unknown-unknown --release`;
//! 2. `wasm-bindgen --target web` (the CLI at the same version as the `wasm-bindgen` crate in
//!    Cargo.lock, installed with `cargo install` when missing) writes the JavaScript glue;
//! 3. `wasm-opt -Oz` shrinks the module when Binaryen is installed (optional);
//! 4. `apps/genha-web/index.html` is copied with its loader line (an ES-module import of the glue,
//!    as Trunk would inject) in place of the `<!-- genha:loader -->` marker.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::run;

const MARKER: &str = "<!-- genha:loader -->";
const LOADER: &str = r#"<script type="module">import init from "./genha_web.js"; init();</script>"#;

pub fn build(root: &Path) -> Result<(), String> {
    run("cargo", &["build", "-p", "genha-web", "--target", "wasm32-unknown-unknown", "--release"])?;
    let wasm = target_dir(root)?.join("wasm32-unknown-unknown").join("release").join("genha-web.wasm");
    if !wasm.exists() {
        return Err(format!("{} was not built", wasm.display()));
    }
    let version = lock_version(root, "wasm-bindgen")?;
    ensure_bindgen(&version)?;
    let out = root.join("dist").join("web");
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let (wasm_s, out_s) = (wasm.display().to_string(), out.display().to_string());
    run("wasm-bindgen", &["--target", "web", "--no-typescript", "--out-dir", &out_s, "--out-name", "genha_web", &wasm_s])?;
    let module = out.join("genha_web_bg.wasm");
    if Command::new("wasm-opt").arg("--version").output().is_ok_and(|o| o.status.success()) {
        let m = module.display().to_string();
        run("wasm-opt", &["-Oz", "--enable-bulk-memory", "--enable-nontrapping-float-to-int", "-o", &m, &m])?;
    } else {
        println!("web: wasm-opt not found, module left unoptimised (install Binaryen to shrink it)");
    }
    let page = std::fs::read_to_string(root.join("apps/genha-web/index.html")).map_err(|e| format!("cannot read apps/genha-web/index.html: {e}"))?;
    if !page.contains(MARKER) {
        return Err(format!("apps/genha-web/index.html has no {MARKER} line"));
    }
    std::fs::write(out.join("index.html"), page.replace(MARKER, LOADER)).map_err(|e| format!("cannot write index.html: {e}"))?;
    let size = std::fs::metadata(&module).map(|m| m.len()).unwrap_or(0);
    println!("web: built {} ({:.1} MB of WebAssembly)", out.display(), size as f64 / 1_048_576.0);
    println!("web: serve it, e.g. `python3 -m http.server -d dist/web 8080`, then open http://127.0.0.1:8080");
    Ok(())
}

/// Cargo's target directory (honours `CARGO_TARGET_DIR` and `.cargo/config.toml`).
fn target_dir(root: &Path) -> Result<PathBuf, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    meta.get("target_directory").and_then(serde_json::Value::as_str).map(PathBuf::from).ok_or_else(|| "cargo metadata: no target_directory".into())
}

/// The version of package `name` in Cargo.lock.
fn lock_version(root: &Path, name: &str) -> Result<String, String> {
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).map_err(|e| format!("cannot read Cargo.lock: {e}"))?;
    let mut lines = lock.lines();
    let wanted = format!("name = \"{name}\"");
    while let Some(line) = lines.next() {
        if line.trim() == wanted
            && let Some(v) = lines.next().and_then(|l| l.trim().strip_prefix("version = \"")).and_then(|v| v.strip_suffix('"'))
        {
            return Ok(v.to_string());
        }
    }
    Err(format!("{name} is not in Cargo.lock"))
}

/// Makes sure `wasm-bindgen` (the CLI) is installed at `version`, which must match the crate.
fn ensure_bindgen(version: &str) -> Result<(), String> {
    let installed = Command::new("wasm-bindgen").arg("--version").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if installed.as_deref().is_some_and(|v| v.split_whitespace().nth(1) == Some(version)) {
        return Ok(());
    }
    println!("web: installing wasm-bindgen-cli {version} (found: {})", installed.as_deref().unwrap_or("none"));
    let pinned = format!("={version}");
    run("cargo", &["install", "wasm-bindgen-cli", "--locked", "--version", &pinned]).map_err(|e| {
        format!("{e}\nInstall it by hand with `cargo install wasm-bindgen-cli --locked --version {pinned}`, then run `cargo xtask web` again.")
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn page_has_the_loader_marker() {
        let page = std::fs::read_to_string(crate::root().join("apps/genha-web/index.html")).unwrap();
        assert!(page.contains(super::MARKER));
        assert!(!page.contains("<script"), "the page itself is plain HTML and CSS");
    }

    #[test]
    fn lock_version_is_found() {
        assert!(super::lock_version(&crate::root(), "wasm-bindgen").unwrap().starts_with("0.2."));
    }
}
