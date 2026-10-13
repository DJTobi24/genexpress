//! File dialogs: native ones on the desktop; in the browser the file picker and downloads
//! (`web.rs`).

use std::path::PathBuf;

pub const PICTURES: (&str, &[&str]) = ("Pictures", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "svg"]);
pub const DESIGNS: (&str, &[&str]) = ("GenHa designs", &["genha", "genexpress"]);

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_open(filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let mut d = rfd::FileDialog::new();
    for (name, exts) in filters {
        d = d.add_filter(*name, exts);
    }
    d.pick_file()
}

/// Several files at once (none when cancelled).
#[cfg(not(target_arch = "wasm32"))]
pub fn pick_many(filters: &[(&str, &[&str])]) -> Option<Vec<PathBuf>> {
    let mut d = rfd::FileDialog::new();
    for (name, exts) in filters {
        d = d.add_filter(*name, exts);
    }
    d.pick_files()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_save(default_name: &str, filter: (&str, &[&str])) -> Option<PathBuf> {
    rfd::FileDialog::new().set_file_name(default_name).add_filter(filter.0, filter.1).save_file()
}

/// In the browser: opens the browser's file picker and returns `None`; the chosen file arrives a
/// frame later (`web::poll`). With designs among the filters it opens as a design, else it is a
/// picture placed into the open design.
#[cfg(target_arch = "wasm32")]
pub fn pick_open(filters: &[(&str, &[&str])]) -> Option<PathBuf> {
    let purpose = if filters.iter().any(|f| f.0 == DESIGNS.0) { crate::web::Purpose::Open } else { crate::web::Purpose::Place };
    crate::web::pick(purpose, filters);
    None
}

/// In the browser files are downloaded instead (`web::save`, `web::export`).
#[cfg(target_arch = "wasm32")]
pub fn pick_save(_: &str, _: (&str, &[&str])) -> Option<PathBuf> {
    None
}

/// A file name made from a design title (no path separators or odd characters).
pub fn file_stem(title: &str) -> String {
    let s: String = title.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '-' }).collect();
    let s = s.trim();
    if s.is_empty() { "design".into() } else { s.to_string() }
}
