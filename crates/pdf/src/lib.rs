//! PDF export for GenHa (layer L3).
//!
//! A minimal, hand-written PDF 1.4: each page is rendered by `genha-render` at the requested
//! resolution and embedded as a JPEG image (`DCTDecode`) filling a page of the same size in points
//! (96 page pixels = 72 points = 1 inch). Pages may differ in size. The document title goes into
//! the info dictionary. Text is not selectable in this first version (pictures of the pages).
#![forbid(unsafe_code)]

use std::fmt::Write as _;

use genha_doc::Document;
use genha_render::{MAX_PIXELS, MAX_SIDE, RenderError, RenderOptions, Renderer, encode_jpeg};
use genha_text::FontBook;

/// Lowest resolution accepted, in dots per inch.
pub const MIN_DPI: f64 = 36.0;
/// Highest resolution accepted, in dots per inch.
pub const MAX_DPI: f64 = 600.0;
/// Points per page pixel (page pixels are 1/96 inch, points 1/72 inch).
const PT_PER_PX: f64 = 0.75;

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("no page {0}")]
    NoPage(usize),
    #[error("no pages to export")]
    NoPages,
    #[error(transparent)]
    Render(#[from] RenderError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdfOptions {
    /// Resolution pages are rasterized at (clamped to `36..=600`).
    pub dpi: f64,
    /// Indices of the pages to export, in order; `None` for all.
    pub pages: Option<Vec<usize>>,
    /// JPEG quality, 1–100.
    pub jpeg_quality: u8,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions { dpi: 150.0, pages: None, jpeg_quality: 88 }
    }
}

/// Exports `doc` as a PDF.
pub fn export_pdf(doc: &Document, fonts: &FontBook, opts: &PdfOptions) -> Result<Vec<u8>, PdfError> {
    let pages: Vec<usize> = match &opts.pages {
        Some(p) => p.clone(),
        None => (0..doc.pages.len()).collect(),
    };
    if pages.is_empty() {
        return Err(PdfError::NoPages);
    }
    if let Some(&bad) = pages.iter().find(|&&i| i >= doc.pages.len()) {
        return Err(PdfError::NoPage(bad));
    }
    let dpi = if opts.dpi.is_finite() { opts.dpi.clamp(MIN_DPI, MAX_DPI) } else { 150.0 };
    let mut renderer = Renderer::new();
    let mut pdf = Pdf::new();
    // Objects 1–3: catalog, page tree, info; then three per page.
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + i * 3)).collect();
    pdf.object(1, b"<< /Type /Catalog /Pages 2 0 R >>");
    pdf.object(2, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());
    pdf.object(3, format!("<< /Title {} /Producer (GenHa) /Creator (GenHa) >>", pdf_string(&doc.title)).as_bytes());
    for (k, &index) in pages.iter().enumerate() {
        let page = doc.page(index).map_err(|_| PdfError::NoPage(index))?;
        let scale = fitting_scale(page.width, page.height, dpi / 96.0);
        let raster = renderer.render_page(doc, index, fonts, &RenderOptions { scale, transparent_background: false })?;
        let jpeg = encode_jpeg(&raster, opts.jpeg_quality.clamp(1, 100))?;
        let (page_obj, content_obj, image_obj) = (4 + k * 3, 5 + k * 3, 6 + k * 3);
        let (w, h) = (num(page.width * PT_PER_PX), num(page.height * PT_PER_PX));
        pdf.object(
            page_obj,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Resources << /XObject << /Im0 {image_obj} 0 R >> /ProcSet [/PDF /ImageC] >> /Contents {content_obj} 0 R >>"
            )
            .as_bytes(),
        );
        let content = format!("q\n{w} 0 0 {h} 0 0 cm\n/Im0 Do\nQ\n");
        pdf.stream(content_obj, &format!("<< /Length {} >>", content.len()), content.as_bytes());
        let dict = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>",
            raster.width,
            raster.height,
            jpeg.len()
        );
        pdf.stream(image_obj, &dict, &jpeg);
    }
    Ok(pdf.finish(1, 3))
}

/// `wanted`, reduced if needed so a `w` × `h` page fits the renderer's limits.
fn fitting_scale(w: f64, h: f64, wanted: f64) -> f64 {
    let mut s = wanted;
    let longest = w.max(h);
    if longest.is_finite() && longest > 0.0 {
        s = s.min(f64::from(MAX_SIDE) / longest * 0.999);
    }
    let area = w * h;
    if area.is_finite() && area > 0.0 {
        s = s.min((MAX_PIXELS as f64 / area).sqrt() * 0.999);
    }
    s
}

/// A number for PDF: up to 3 decimals, no exponent.
fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_string() }
}

/// A PDF text string: `(…)` with escapes for ASCII, else UTF-16BE hex with a byte order mark.
pub fn pdf_string(s: &str) -> String {
    if s.is_ascii() {
        let mut out = String::from("(");
        for c in s.chars() {
            match c {
                '(' | ')' | '\\' => {
                    out.push('\\');
                    out.push(c);
                }
                c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                    let _ = write!(out, "\\{:03o}", c as u32);
                }
                c => out.push(c),
            }
        }
        out.push(')');
        out
    } else {
        let mut out = String::from("<FEFF");
        for unit in s.encode_utf16() {
            let _ = write!(out, "{unit:04X}");
        }
        out.push('>');
        out
    }
}

/// A PDF being written: the bytes so far and where each object starts.
struct Pdf {
    out: Vec<u8>,
    /// Byte offset of object `i` at index `i - 1`.
    offsets: Vec<usize>,
}

impl Pdf {
    fn new() -> Self {
        let mut out = b"%PDF-1.4\n".to_vec();
        // A comment with bytes above 127 marks the file as binary for transfer tools.
        out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
        Pdf { out, offsets: vec![] }
    }

    fn mark(&mut self, id: usize) {
        if self.offsets.len() < id {
            self.offsets.resize(id, 0);
        }
        if let Some(slot) = id.checked_sub(1).and_then(|i| self.offsets.get_mut(i)) {
            *slot = self.out.len();
        }
        self.out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    }

    fn object(&mut self, id: usize, body: &[u8]) {
        self.mark(id);
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendobj\n");
    }

    fn stream(&mut self, id: usize, dict: &str, data: &[u8]) {
        self.mark(id);
        self.out.extend_from_slice(dict.as_bytes());
        self.out.extend_from_slice(b"\nstream\n");
        self.out.extend_from_slice(data);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn finish(mut self, root: usize, info: usize) -> Vec<u8> {
        let xref = self.out.len();
        let size = self.offsets.len() + 1;
        let mut t = format!("xref\n0 {size}\n0000000000 65535 f \n");
        for off in &self.offsets {
            let _ = writeln!(t, "{off:010} 00000 n ");
        }
        let _ = write!(t, "trailer\n<< /Size {size} /Root {root} 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n");
        self.out.extend_from_slice(t.as_bytes());
        self.out
    }
}

#[cfg(test)]
mod tests;
