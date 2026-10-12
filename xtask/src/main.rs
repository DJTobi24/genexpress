//! Build tooling: `cargo xtask <command>`.

mod checks;
mod layers;

use std::path::PathBuf;
use std::process::{Command, ExitCode};

const USAGE: &str = "usage: cargo xtask <command>

commands:
  ci          everything a commit must pass: fmt, clippy (-D warnings, no-panic lints), tests,
              layers, assets, cleanroom, wasm
  layers      check that crates only depend on lower layers and UI crates stay at L6+
  assets      check that every file under assets/ has a row in ASSETS.md
  cleanroom   check for Adobe copyright notices and files made with Adobe applications
  wasm        check that the engine and below build for wasm32-unknown-unknown";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("ci") => ci(),
        Some("layers") => cmd_layers(),
        Some("assets") => checks::assets(&root()),
        Some("cleanroom") => checks::cleanroom(&root()),
        Some("wasm") => wasm(),
        Some("-h" | "--help" | "help") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(PathBuf::from).unwrap_or(manifest)
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    println!("$ {program} {}", args.join(" "));
    let status = Command::new(program).args(args).current_dir(root()).status().map_err(|e| format!("cannot run {program}: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("`{program} {}` failed", args.join(" "))) }
}

fn ci() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--check"])?;
    run("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
    run("cargo", &["test", "--workspace", "-q"])?;
    cmd_layers()?;
    checks::assets(&root())?;
    checks::cleanroom(&root())?;
    wasm()?;
    println!("ci: all checks passed");
    Ok(())
}

fn cmd_layers() -> Result<(), String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root())
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err("cargo metadata failed".into());
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let crates = layers::from_metadata(&meta)?;
    let errors = layers::check(&crates);
    if errors.is_empty() {
        println!("layers: {} crates OK", crates.len());
        Ok(())
    } else {
        Err(format!("layering violations:\n  {}", errors.join("\n  ")))
    }
}

/// Crates that must build for the browser (everything up to the engine, plus the MCP-free UI).
const WASM_CRATES: &[&str] = &[
    "genha-geom",
    "genha-color",
    "genha-doc",
    "genha-text",
    "genha-effects",
    "genha-layout",
    "genha-templates",
    "genha-motion",
    "genha-render",
    "genha-svg",
    "genha-pdf",
    "genha-format",
    "genha-tools",
    "genha-engine",
];

fn wasm() -> Result<(), String> {
    let installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-unknown-unknown"));
    if !matches!(installed, Ok(true)) {
        println!("wasm: skipped (run `rustup target add wasm32-unknown-unknown`)");
        return Ok(());
    }
    let mut args = vec!["check", "--target", "wasm32-unknown-unknown", "-q"];
    for c in WASM_CRATES {
        if root().join("crates").join(c.trim_start_matches("genha-")).exists() {
            args.push("-p");
            args.push(c);
        }
    }
    run("cargo", &args)
}
