//! PDF export for GenHa (layer L3).
//!
//! **Vector PDF (the default)**, written with [`krilla`]: every element is drawn as PDF graphics
//! in its frame (position, rotation, flips) with its opacity, inside a page of the same size in
//! points (96 page pixels = 72 points = 1 inch). Pages may differ in size.
//! - Shapes, icons, lines and borders are PDF paths with solid, linear or radial gradient fills
//!   (gradient transparency included), strokes and dashes. Groups nest; opacity becomes a
//!   transparency group.
//! - Text is real PDF text in the font the layout used, embedded as a subset, so it is selectable,
//!   searchable and sharp. Outline and highlight effects and unblurred shadows are vectors; curved
//!   text is a run of real glyphs, each placed and turned along its arc. A font that cannot be
//!   embedded makes the export retry with letters as outlines ([`PdfOptions::text_as_outlines`]).
//! - Pictures are embedded at their native resolution: the original JPEG bytes when the picture
//!   is an unadjusted JPEG (crops become a clip), otherwise the cropped and adjusted pixels,
//!   compressed losslessly. They are clipped to the frame's rounded rectangle or their mask shape.
//! - Charts and tables are the renderer's [`genha_render::Scene`]s: paths plus real text.
//! - Only what has no PDF equivalent becomes a picture: blurred drop shadows (of shapes and text)
//!   are rendered on their own at [`PdfOptions::dpi`] and placed under their element — never the
//!   whole page.
//!
//! **Raster PDF** ([`PdfOptions::raster`]) keeps the earlier format for compatibility: each page
//! one JPEG picture with an invisible text layer.
//!
//! For print, pages can get a bleed (the background and the shapes and pictures touching the edge
//! extend past it), crop marks and the matching `TrimBox` / `BleedBox`, and colours can be written
//! as DeviceCMYK ([`PdfOptions::cmyk`]; a plain conversion without a colour profile).
#![forbid(unsafe_code)]

mod raster;
mod vector;

use std::fmt::Write as _;

use genha_doc::{Document, Element, ElementKind, Page, ShapeKind};
use genha_render::{MAX_PIXELS, MAX_SIDE, RenderError};
use genha_text::FontBook;
use kurbo::Point;

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
    #[error("could not write the PDF: {0}")]
    Write(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdfOptions {
    /// Resolution in dots per inch (clamped to `36..=600`): of the whole page for a raster PDF; of
    /// the few parts drawn as pictures (blurred shadows) for a vector PDF.
    pub dpi: f64,
    /// Indices of the pages to export, in order; `None` for all.
    pub pages: Option<Vec<usize>>,
    /// Raster PDF only: JPEG quality of the page pictures, 1–100.
    pub jpeg_quality: u8,
    /// Raster PDF only: invisible text over the page pictures, for search, selection and copying.
    pub text_layer: bool,
    /// Bleed around each page in millimetres (0 = none; clamped to `0..=20`).
    pub bleed_mm: f64,
    /// Crop marks in a margin around the bleed.
    pub crop_marks: bool,
    /// Each page as one picture (the earlier format) instead of vector graphics.
    pub raster: bool,
    /// Vector PDF only: colours as DeviceCMYK (a plain conversion, no colour profile).
    pub cmyk: bool,
    /// Vector PDF only: letters as outlines instead of text with embedded fonts (looks the same,
    /// but the text cannot be selected or searched).
    pub text_as_outlines: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            dpi: 300.0,
            pages: None,
            jpeg_quality: 88,
            text_layer: true,
            bleed_mm: 0.0,
            crop_marks: false,
            raster: false,
            cmyk: false,
            text_as_outlines: false,
        }
    }
}

impl PdfOptions {
    /// Settings for printing: 300 dpi, 3 mm bleed, crop marks.
    pub fn print() -> Self {
        PdfOptions { dpi: 300.0, jpeg_quality: 92, bleed_mm: 3.0, crop_marks: true, ..PdfOptions::default() }
    }

    /// The resolution, sanitized.
    fn resolution(&self) -> f64 {
        if self.dpi.is_finite() { self.dpi.clamp(MIN_DPI, MAX_DPI) } else { 300.0 }
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
    if opts.raster { raster::export(doc, fonts, opts, &pages) } else { vector::export(doc, fonts, opts, &pages) }
}

/// Where a page goes on its PDF page, in page pixels (y down): the page grown by the bleed,
/// inside a slug that holds the crop marks.
#[derive(Clone, Copy, Debug)]
struct Geometry {
    bleed: f64,
    slug: f64,
    /// The page with its bleed.
    page_w: f64,
    page_h: f64,
    /// The whole sheet (bleed and slug).
    media_w: f64,
    media_h: f64,
    /// The finished page: `[x0, y0, x1, y1]`.
    trim: [f64; 4],
    /// The page with its bleed: `[x0, y0, x1, y1]`.
    bleed_box: [f64; 4],
}

impl Geometry {
    fn new(page: &Page, opts: &PdfOptions) -> Self {
        let bleed = if opts.bleed_mm.is_finite() { opts.bleed_mm.clamp(0.0, MAX_BLEED_MM) * PX_PER_MM } else { 0.0 };
        let slug = if opts.crop_marks { SLUG_PX } else { 0.0 };
        let (w, h) = (page.width, page.height);
        let (page_w, page_h) = (w + 2.0 * bleed, h + 2.0 * bleed);
        let (media_w, media_h) = (page_w + 2.0 * slug, page_h + 2.0 * slug);
        let off = slug + bleed;
        Geometry {
            bleed,
            slug,
            page_w,
            page_h,
            media_w,
            media_h,
            trim: [off, off, off + w, off + h],
            bleed_box: [slug, slug, media_w - slug, media_h - slug],
        }
    }

    fn has_boxes(&self) -> bool {
        self.bleed > 0.0 || self.slug > 0.0
    }
}

/// The eight crop mark lines at the trim corners, in the slug outside the bleed (page pixels).
fn crop_mark_lines(g: &Geometry) -> Vec<(Point, Point)> {
    let [x0, y0, x1, y1] = g.trim;
    // Marks start 2 pt outside the bleed and run to 2 pt from the sheet's edge (at least 4 pt).
    let gap = g.bleed + 2.0 / PT_PER_PX;
    let len = ((g.slug * PT_PER_PX - 2.0).max(4.0)) / PT_PER_PX;
    let mut out = vec![];
    for (x, y, sx, sy) in [(x0, y0, -1.0, -1.0), (x1, y0, 1.0, -1.0), (x0, y1, -1.0, 1.0), (x1, y1, 1.0, 1.0)] {
        out.push((Point::new(x + sx * gap, y), Point::new(x + sx * (gap + len), y)));
        out.push((Point::new(x, y + sy * gap), Point::new(x, y + sy * (gap + len))));
    }
    out
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

/// Moves `e` (and a group's children) by `dx`, `dy`.
fn shift(e: &mut Element, dx: f64, dy: f64) {
    e.frame = e.frame.translated(dx, dy);
    if let ElementKind::Group(g) = &mut e.kind {
        for c in &mut g.children {
            shift(c, dx, dy);
        }
    }
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

#[cfg(test)]
mod tests;
