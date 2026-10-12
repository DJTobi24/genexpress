//! Repository hygiene: asset attribution and the clean-room rule.

use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || ["target", "plan", "dist", "node_modules"].contains(&name.as_str()) {
            continue;
        }
        if p.is_dir() { walk(&p, out) } else { out.push(p) }
    }
}

/// Every file under `assets/` must be named in ASSETS.md.
pub fn assets(root: &Path) -> Result<(), String> {
    let table = std::fs::read_to_string(root.join("ASSETS.md")).map_err(|e| format!("cannot read ASSETS.md: {e}"))?;
    let mut files = vec![];
    walk(&root.join("assets"), &mut files);
    let missing: Vec<String> = files
        .iter()
        .filter_map(|f| f.strip_prefix(root).ok())
        .map(|f| f.to_string_lossy().replace('\\', "/"))
        .filter(|f| !f.ends_with("README.md") && !table.contains(f.as_str()))
        .collect();
    if missing.is_empty() {
        println!("assets: {} files attributed", files.len());
        Ok(())
    } else {
        Err(format!("assets without a row in ASSETS.md:\n  {}", missing.join("\n  ")))
    }
}

/// Markers of Adobe-authored content: copyright notices and Adobe applications' metadata in files.
const MARKERS: &[&str] = &[
    "Copyright Adobe",
    "Adobe Systems Incorporated. All Rights Reserved",
    "Adobe Inc. All rights reserved",
    "<xmp:CreatorTool>Adobe",
    "Adobe Express",
];

/// Files that may name Adobe (they document the clean-room rule or the parity target).
const ALLOWED: &[&str] = &["AGENTS.md", "CLAUDE.md", "README.md", "ROADMAP.md", "NOTICE", "docs/", "xtask/src/checks.rs", ".github/"];

pub fn cleanroom(root: &Path) -> Result<(), String> {
    let mut files = vec![];
    walk(root, &mut files);
    let mut hits = vec![];
    for f in &files {
        let rel = f.strip_prefix(root).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        if ALLOWED.iter().any(|a| rel == *a || (a.ends_with('/') && rel.starts_with(a))) {
            continue;
        }
        let Ok(bytes) = std::fs::read(f) else { continue };
        // Big binaries: look at the start and end only (where metadata lives).
        let hay: Vec<u8> = if bytes.len() > 4 << 20 {
            bytes.iter().take(1 << 20).chain(bytes.iter().skip(bytes.len() - (1 << 20))).copied().collect()
        } else {
            bytes
        };
        let text = String::from_utf8_lossy(&hay);
        for m in MARKERS {
            if text.contains(m) {
                hits.push(format!("{rel}: contains \"{m}\""));
            }
        }
    }
    if hits.is_empty() {
        println!("cleanroom: {} files OK", files.len());
        Ok(())
    } else {
        Err(format!("clean-room check failed (see AGENTS.md › Clean-room):\n  {}", hits.join("\n  ")))
    }
}
