//! PDF export for GenHa (layer L3).
//!
//! A hand-written PDF 1.4: each page is rendered by `genha-render` at the requested resolution and
//! embedded as a JPEG image (`DCTDecode`) filling a page of the same size in points (96 page pixels
//! = 72 points = 1 inch). Pages may differ in size. On top of the picture lies an invisible text
//! layer (text render mode 3, standard Helvetica) so the text can be searched, selected and copied.
//! For print, pages can get a bleed (the background and the shapes and pictures touching the edge
//! extend past it), crop marks, and the matching `TrimBox` / `BleedBox`.
#![forbid(unsafe_code)]

use std::fmt::Write as _;

use genha_doc::{Document, Element, ElementKind, Page, ShapeKind};
use genha_geom::Point;
use genha_render::{MAX_PIXELS, MAX_SIDE, RenderError, RenderOptions, Renderer, element_transform, encode_jpeg};
use genha_text::FontBook;

/// Lowest resolution accepted, in dots per inch.
pub const MIN_DPI: f64 = 36.0;
/// Highest resolution accepted, in dots per inch.
pub const MAX_DPI: f64 = 600.0;
/// Largest bleed accepted, in millimetres.
pub const MAX_BLEED_MM: f64 = 20.0;
/// Points per page pixel (page pixels are 1/96 inch, points 1/72 inch).
const PT_PER_PX: f64 = 0.75;
/// Page pixels per millimetre.
const PX_PER_MM: f64 = 96.0 / 25.4;
/// The margin around the bleed that holds the crop marks, in page pixels (10 mm).
const SLUG_PX: f64 = 10.0 * PX_PER_MM;

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
    /// Invisible text over the page pictures, for search, selection and copying.
    pub text_layer: bool,
    /// Bleed around each page in millimetres (0 = none; clamped to `0..=20`).
    pub bleed_mm: f64,
    /// Crop marks in a margin around the bleed.
    pub crop_marks: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions { dpi: 150.0, pages: None, jpeg_quality: 88, text_layer: true, bleed_mm: 0.0, crop_marks: false }
    }
}

impl PdfOptions {
    /// Settings for printing: 300 dpi, 3 mm bleed, crop marks.
    pub fn print() -> Self {
        PdfOptions { dpi: 300.0, jpeg_quality: 92, bleed_mm: 3.0, crop_marks: true, ..PdfOptions::default() }
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
    let bleed = if opts.bleed_mm.is_finite() { opts.bleed_mm.clamp(0.0, MAX_BLEED_MM) * PX_PER_MM } else { 0.0 };
    let slug = if opts.crop_marks { SLUG_PX } else { 0.0 };
    let mut renderer = Renderer::new();
    let mut pdf = Pdf::new();
    // Objects 1–4: catalog, page tree, info, the text layer's font; then three per page.
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 5 + i * 3)).collect();
    pdf.object(1, b"<< /Type /Catalog /Pages 2 0 R >>");
    pdf.object(2, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());
    pdf.object(3, format!("<< /Title {} /Producer (GenHa) /Creator (GenHa) >>", pdf_string(&doc.title)).as_bytes());
    pdf.object(4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    for (k, &index) in pages.iter().enumerate() {
        let page = doc.page(index).map_err(|_| PdfError::NoPage(index))?;
        // With a bleed, render a copy of the page grown by the bleed on every side.
        let (raster, (pw, ph)) = if bleed > 0.0 {
            let mut big = doc.clone();
            big.pages = vec![with_bleed(page, bleed)];
            big.versions.clear();
            let p = big.page(0).map_err(|_| PdfError::NoPage(index))?;
            let scale = fitting_scale(p.width, p.height, dpi / 96.0);
            (renderer.render_page(&big, 0, fonts, &RenderOptions { scale, transparent_background: false })?, (p.width, p.height))
        } else {
            let scale = fitting_scale(page.width, page.height, dpi / 96.0);
            (renderer.render_page(doc, index, fonts, &RenderOptions { scale, transparent_background: false })?, (page.width, page.height))
        };
        let jpeg = encode_jpeg(&raster, opts.jpeg_quality.clamp(1, 100))?;
        let (page_obj, content_obj, image_obj) = (5 + k * 3, 6 + k * 3, 7 + k * 3);
        // Sizes in points: media = bleed area + slug; trim = the page itself.
        let media_w = (pw + 2.0 * slug) * PT_PER_PX;
        let media_h = (ph + 2.0 * slug) * PT_PER_PX;
        let off = (slug + bleed) * PT_PER_PX;
        let trim = [off, off, off + page.width * PT_PER_PX, off + page.height * PT_PER_PX];
        let bleed_box = [slug * PT_PER_PX, slug * PT_PER_PX, media_w - slug * PT_PER_PX, media_h - slug * PT_PER_PX];
        let boxes = if bleed > 0.0 || slug > 0.0 { format!(" /TrimBox [{}] /BleedBox [{}]", nums(&trim), nums(&bleed_box)) } else { String::new() };
        pdf.object(
            page_obj,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}]{boxes} /Resources << /XObject << /Im0 {image_obj} 0 R >> /Font << /F1 4 0 R >> /ProcSet [/PDF /Text /ImageC] >> /Contents {content_obj} 0 R >>",
                num(media_w),
                num(media_h)
            )
            .as_bytes(),
        );
        let mut content =
            format!("q\n{} 0 0 {} {} {} cm\n/Im0 Do\nQ\n", num(pw * PT_PER_PX), num(ph * PT_PER_PX), num(slug * PT_PER_PX), num(slug * PT_PER_PX));
        if opts.crop_marks {
            content.push_str(&crop_marks(trim, bleed * PT_PER_PX, slug * PT_PER_PX));
        }
        if opts.text_layer {
            content.push_str(&text_layer(page, fonts, off, off + page.height * PT_PER_PX));
        }
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

/// `page` grown by `bleed` on every side: content moves in by the bleed, and rectangles and
/// pictures that touch an edge stretch out to the new edge, so nothing white shows when trimmed.
fn with_bleed(page: &Page, bleed: f64) -> Page {
    let mut p = page.clone();
    let (w, h) = (page.width, page.height);
    p.width = w + 2.0 * bleed;
    p.height = h + 2.0 * bleed;
    for e in &mut p.elements {
        shift(e, bleed, bleed);
        let bleeds = e.frame.rotation == 0.0
            && match &e.kind {
                ElementKind::Image(_) => true,
                ElementKind::Shape(s) => matches!(s.shape, ShapeKind::Rect { .. }),
                _ => false,
            };
        if !bleeds {
            continue;
        }
        let f = &mut e.frame;
        let eps = 0.5;
        if f.x - bleed <= eps {
            f.width += f.x;
            f.x = 0.0;
        }
        if f.y - bleed <= eps {
            f.height += f.y;
            f.y = 0.0;
        }
        if f.x + f.width >= w + bleed - eps {
            f.width = p.width - f.x;
        }
        if f.y + f.height >= h + bleed - eps {
            f.height = p.height - f.y;
        }
    }
    p
}

fn shift(e: &mut Element, dx: f64, dy: f64) {
    e.frame = e.frame.translated(dx, dy);
    if let ElementKind::Group(g) = &mut e.kind {
        for c in &mut g.children {
            shift(c, dx, dy);
        }
    }
}

/// Crop marks at the trim corners, in the slug outside the bleed (all in points).
fn crop_marks(trim: [f64; 4], bleed: f64, slug: f64) -> String {
    let [x0, y0, x1, y1] = trim;
    let gap = bleed + 2.0; // marks start just outside the bleed
    let len = (slug - 2.0).max(4.0);
    let mut s = String::from("q\n0 G 0.3 w\n");
    for (x, y, sx, sy) in [(x0, y0, -1.0, -1.0), (x1, y0, 1.0, -1.0), (x0, y1, -1.0, 1.0), (x1, y1, 1.0, 1.0)] {
        // Horizontal mark on the trim line, vertical mark on the trim line.
        let _ = writeln!(s, "{} {} m {} {} l S", num(x + sx * gap), num(y), num(x + sx * (gap + len)), num(y));
        let _ = writeln!(s, "{} {} m {} {} l S", num(x), num(y + sy * gap), num(x), num(y + sy * (gap + len)));
    }
    s.push_str("Q\n");
    s
}

/// Invisible text (render mode 3) where the page's text is, for search, selection and copying.
/// `x0` is the trim box's left edge and `top` its top edge, in points.
fn text_layer(page: &Page, fonts: &FontBook, x0: f64, top: f64) -> String {
    let mut out = String::new();
    page.for_each(&mut |e| {
        if e.hidden {
            return;
        }
        let ElementKind::Text(t) = &e.kind else { return };
        let size = t.style.font_size;
        if !size.is_finite() || size <= 0.0 {
            return;
        }
        let m = element_transform(e);
        let layout = genha_text::layout(fonts, &t.text, &t.style, e.frame.width);
        // Page pixels (y down) to points (y up).
        let to_pt = |p: Point| (x0 + p.x * PT_PER_PX, top - p.y * PT_PER_PX);
        let c = m.as_coeffs();
        for line in &layout.lines {
            let text = line.text.trim_end();
            if text.is_empty() {
                continue;
            }
            let (ox, oy) = to_pt(m * Point::new(line.x, line.baseline));
            // Unit vectors of the line's x axis and up axis in points, scaled to 1 font unit.
            let (ax, ay) = (c[0] * PT_PER_PX, -c[1] * PT_PER_PX);
            let (bx, by) = (-c[2] * PT_PER_PX, c[3] * PT_PER_PX);
            let (enc, count) = win_ansi(text);
            // Stretch Helvetica's (approximate) width to the line's real width.
            let natural = count as f64 * 0.52 * size;
            let tz = if natural > 0.0 && line.width > 0.0 { (line.width / natural * 100.0).clamp(10.0, 1000.0) } else { 100.0 };
            let _ = writeln!(
                out,
                "BT 3 Tr /F1 {} Tf {} Tz {} {} {} {} {} {} Tm {enc} Tj ET",
                num(size),
                num(tz),
                num(ax),
                num(ay),
                num(bx),
                num(by),
                num(ox),
                num(oy)
            );
        }
    });
    out
}

/// `s` as a PDF string in WinAnsi encoding (characters outside it become `?`), and its length.
fn win_ansi(s: &str) -> (String, usize) {
    let mut out = String::from("(");
    let mut n = 0;
    for ch in s.chars() {
        n += 1;
        let b: u8 = match ch {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(ch);
                continue;
            }
            c if (' '..='~').contains(&c) => {
                out.push(c);
                continue;
            }
            c if ('\u{a0}'..='\u{ff}').contains(&c) => c as u32 as u8,
            '€' => 0x80,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '…' => 0x85,
            _ => b'?',
        };
        let _ = write!(out, "\\{b:03o}");
    }
    out.push(')');
    (out, n)
}

fn nums(v: &[f64]) -> String {
    v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(" ")
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
