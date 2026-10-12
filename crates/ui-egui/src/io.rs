//! Native file dialogs (desktop only; the web build uses drag and drop and downloads).

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

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_save(default_name: &str, filter: (&str, &[&str])) -> Option<PathBuf> {
    rfd::FileDialog::new().set_file_name(default_name).add_filter(filter.0, filter.1).save_file()
}

#[cfg(target_arch = "wasm32")]
pub fn pick_open(_: &[(&str, &[&str])]) -> Option<PathBuf> {
    None
}

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
