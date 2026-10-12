//! Export: PNG, JPEG, SVG, PDF.

use serde_json::{Value, json};

use super::{bad, bool_or, cmd, f64_or, has_doc, ok, str_req, usize_param};
use crate::{CommandSpec, EngineError, Result, Session};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "export.png",
            "Download PNG…",
            ["File", "Download"],
            None,
            "{path, page?: index (default current), scale?: 0.1–8 = 1, transparent?: bool} — `{page}` in the path is replaced by the page number when all_pages is set; all_pages?: bool",
            has_doc,
            export_png
        ),
        cmd!(
            "export.jpeg",
            "Download JPEG…",
            ["File", "Download"],
            None,
            "{path, page?, scale?, quality?: 1–100 = 90, all_pages?}",
            has_doc,
            export_jpeg
        ),
        cmd!("export.svg", "Download SVG…", ["File", "Download"], None, "{path, page?, text_as_paths?: bool, all_pages?}", has_doc, export_svg),
        cmd!(
            "export.pdf",
            "Download PDF…",
            ["File", "Download"],
            None,
            "{path, dpi?: 36–600 = 150, pages?: [indexes], quality?: 1–100 = 88, text_layer?: bool = true (searchable, selectable text), print?: bool (300 dpi, 3 mm bleed, crop marks), bleed?: mm, crop_marks?: bool}",
            has_doc,
            export_pdf
        ),
    ]
}

/// What to export: one format's bytes for one page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    Svg,
}

/// One page as file bytes.
pub fn page_bytes(s: &Session, page: usize, format: Format, p: &Value) -> Result<Vec<u8>> {
    let d = s.doc()?;
    if page >= d.doc.pages.len() {
        return Err(bad("export", format!("no page {page}")));
    }
    let scale = f64_or(p, "scale", 1.0).clamp(0.1, 8.0);
    match format {
        Format::Png | Format::Jpeg => {
            let transparent = format == Format::Png && bool_or(p, "transparent", false);
            let opts = genha_render::RenderOptions { scale, transparent_background: transparent };
            let r = genha_render::render_page(&d.doc, page, &s.fonts, &opts).map_err(|e| EngineError::Other(e.to_string()))?;
            let bytes = if format == Format::Png {
                genha_render::encode_png(&r)
            } else {
                genha_render::encode_jpeg(&r, f64_or(p, "quality", 90.0).clamp(1.0, 100.0) as u8)
            };
            bytes.map_err(|e| EngineError::Other(e.to_string()))
        }
        Format::Svg => {
            let opts = genha_svg::SvgOptions { text_as_paths: bool_or(p, "text_as_paths", false) };
            genha_svg::export_page(&d.doc, page, &s.fonts, &opts).map(String::into_bytes).map_err(|e| EngineError::Other(e.to_string()))
        }
    }
}

/// The whole design (or some pages) as PDF bytes.
pub fn pdf_bytes(s: &Session, p: &Value) -> Result<Vec<u8>> {
    let d = s.doc()?;
    let pages = match p.get("pages") {
        Some(Value::Array(a)) => Some(a.iter().filter_map(Value::as_u64).filter_map(|v| usize::try_from(v).ok()).collect()),
        _ => None,
    };
    let base = if bool_or(p, "print", false) { genha_pdf::PdfOptions::print() } else { genha_pdf::PdfOptions::default() };
    let opts = genha_pdf::PdfOptions {
        dpi: f64_or(p, "dpi", base.dpi),
        pages,
        jpeg_quality: f64_or(p, "quality", f64::from(base.jpeg_quality)).clamp(1.0, 100.0) as u8,
        text_layer: bool_or(p, "text_layer", base.text_layer),
        bleed_mm: f64_or(p, "bleed", base.bleed_mm),
        crop_marks: bool_or(p, "crop_marks", base.crop_marks),
    };
    genha_pdf::export_pdf(&d.doc, &s.fonts, &opts).map_err(|e| EngineError::Other(e.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write(path: &str, bytes: &[u8]) -> Result<()> {
    genha_format::write_atomic(std::path::Path::new(path), bytes).map_err(|e| EngineError::Other(format!("cannot write {path}: {e}")))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn write(_: &str, _: &[u8]) -> Result<()> {
    Err(EngineError::Other("writing files is not available in the browser".into()))
}

fn export_pages(s: &mut Session, p: &Value, cmd: &str, format: Format) -> Result<Value> {
    let path = str_req(p, "path", cmd)?.to_string();
    let d = s.doc()?;
    let pages: Vec<usize> =
        if bool_or(p, "all_pages", false) { (0..d.doc.pages.len()).collect() } else { vec![usize_param(p, "page").unwrap_or(d.page)] };
    let mut written = vec![];
    for &page in &pages {
        let bytes = page_bytes(s, page, format, p)?;
        let target = if pages.len() > 1 { numbered(&path, page + 1) } else { path.clone() };
        write(&target, &bytes)?;
        written.push(json!({"path": target, "bytes": bytes.len()}));
    }
    Ok(ok(json!({"files": written})))
}

/// `out.png` → `out-2.png` (or the `{page}` placeholder replaced).
fn numbered(path: &str, n: usize) -> String {
    if path.contains("{page}") {
        return path.replace("{page}", &n.to_string());
    }
    let p = std::path::Path::new(path);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    p.with_file_name(format!("{stem}-{n}{ext}")).display().to_string()
}

fn export_png(s: &mut Session, p: &Value) -> Result<Value> {
    export_pages(s, p, "export.png", Format::Png)
}

fn export_jpeg(s: &mut Session, p: &Value) -> Result<Value> {
    export_pages(s, p, "export.jpeg", Format::Jpeg)
}

fn export_svg(s: &mut Session, p: &Value) -> Result<Value> {
    export_pages(s, p, "export.svg", Format::Svg)
}

fn export_pdf(s: &mut Session, p: &Value) -> Result<Value> {
    let path = str_req(p, "path", "export.pdf")?.to_string();
    let bytes = pdf_bytes(s, p)?;
    write(&path, &bytes)?;
    Ok(ok(json!({"files": [{"path": path, "bytes": bytes.len()}]})))
}

#[cfg(test)]
mod tests {
    #[test]
    fn numbering() {
        assert_eq!(super::numbered("/tmp/a.png", 2), "/tmp/a-2.png");
        assert_eq!(super::numbered("/tmp/{page}.png", 3), "/tmp/3.png");
    }
}
