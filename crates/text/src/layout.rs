//! Line breaking and glyph placement.

use std::collections::HashMap;
use std::sync::Arc;

use genexpress_doc::{TextAlign, TextStyle};
use kurbo::{BezPath, Rect, Shape};
use skrifa::charmap::Charmap;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider, OutlineGlyphCollection};

use crate::{FontBook, LineLayout, TextLayout};

/// Most characters laid out (the rest are dropped).
pub const MAX_TEXT_CHARS: usize = 100_000;
/// Largest font size, in pixels.
pub const MAX_FONT_SIZE: f64 = 4096.0;
/// Advance used for every character when no font is available, in ems.
const APPROX_ADVANCE: f64 = 0.55;

/// A character with the face that draws it and its advance (pixels).
#[derive(Clone, Copy)]
struct Glyph {
    ch: char,
    face: Option<usize>,
    gid: Option<GlyphId>,
    advance: f64,
}

/// A face opened for drawing.
struct Open<'a> {
    charmap: Charmap<'a>,
    metrics: GlyphMetrics<'a>,
    outlines: OutlineGlyphCollection<'a>,
}

/// Sanitized style numbers.
struct Params {
    size: f64,
    line_height: f64,
    spacing: f64,
}

fn params(style: &TextStyle) -> Params {
    let size = if style.font_size.is_finite() { style.font_size.clamp(0.0, MAX_FONT_SIZE) } else { 48.0 };
    let lh = if style.line_height.is_finite() { style.line_height.clamp(0.1, 10.0) } else { 1.2 };
    let ls = if style.letter_spacing.is_finite() { style.letter_spacing.clamp(-1000.0, 10_000.0) } else { 0.0 };
    Params { size, line_height: lh * size, spacing: ls / 1000.0 * size }
}

/// Lays out `text` in `style` in a box `box_width` wide (no wrapping when it is not a positive
/// finite number). Origin at the box's top-left, y down.
pub fn layout(book: &FontBook, text: &str, style: &TextStyle, box_width: f64) -> TextLayout {
    let p = params(style);
    let wrap = if box_width.is_finite() && box_width > 0.0 { Some(box_width) } else { None };
    let mut text: String = text.chars().take(MAX_TEXT_CHARS).collect();
    if style.uppercase {
        text = text.to_uppercase();
    }
    let primary = book.select(&style.font_family, style.bold, style.italic);
    let (ascent, descent, ul_off, ul_th) = match primary.and_then(|i| book.face(i)) {
        Some(f) => (f.ascent, f.descent, f.underline_offset, f.underline_thickness),
        None => (0.8, 0.2, 0.1, 0.05),
    };

    // Which face draws each character.
    let mut fallback: Option<Vec<usize>> = None;
    let mut chosen: HashMap<char, Option<usize>> = HashMap::new();
    let paragraphs: Vec<Vec<char>> =
        text.split('\n').map(|para| para.chars().filter(|c| *c != '\r').map(|c| if c == '\t' { ' ' } else { c }).collect()).collect();
    let mut faces_used: Vec<usize> = primary.into_iter().collect();
    for para in &paragraphs {
        for &c in para {
            if chosen.contains_key(&c) {
                continue;
            }
            let face = match primary {
                Some(pi) if book.face(pi).is_some_and(|f| f.covers(c)) => Some(pi),
                Some(_) if !c.is_control() => {
                    let order = fallback.get_or_insert_with(|| book.fallback_order(style.bold, style.italic));
                    order.iter().copied().find(|&i| book.face(i).is_some_and(|f| f.covers(c))).or(primary)
                }
                _ => primary,
            };
            if let Some(f) = face
                && !faces_used.contains(&f)
            {
                faces_used.push(f);
            }
            chosen.insert(c, face);
        }
    }

    // Open the faces used.
    let datas: Vec<(usize, Arc<[u8]>, u32)> =
        faces_used.iter().filter_map(|&i| book.face(i).and_then(|f| f.data().map(|d| (i, d, f.index)))).collect();
    let size = Size::new(p.size as f32);
    let open: HashMap<usize, Open<'_>> = datas
        .iter()
        .filter_map(|(i, d, idx)| {
            let font = FontRef::from_index(d, *idx).ok()?;
            Some((*i, Open { metrics: font.glyph_metrics(size, LocationRef::default()), outlines: font.outline_glyphs(), charmap: font.charmap() }))
        })
        .collect();

    let shape = |c: char| -> Glyph {
        if c.is_control() {
            return Glyph { ch: c, face: None, gid: None, advance: 0.0 };
        }
        let face = chosen.get(&c).copied().flatten();
        match face.and_then(|f| open.get(&f).map(|o| (f, o))) {
            Some((f, o)) => {
                let gid = o.charmap.map(c);
                let adv = |g: GlyphId| o.metrics.advance_width(g).map(f64::from).filter(|a| a.is_finite());
                match gid {
                    Some(g) => Glyph { ch: c, face: Some(f), gid: Some(g), advance: adv(g).unwrap_or(p.size * APPROX_ADVANCE) },
                    None if c.is_whitespace() => {
                        let space = o.charmap.map(' ').and_then(adv).unwrap_or(p.size * 0.25);
                        Glyph { ch: c, face: Some(f), gid: None, advance: space }
                    }
                    // Nobody has it: the face's "missing glyph" box.
                    None => {
                        Glyph { ch: c, face: Some(f), gid: Some(GlyphId::NOTDEF), advance: adv(GlyphId::NOTDEF).unwrap_or(p.size * APPROX_ADVANCE) }
                    }
                }
            }
            None => Glyph { ch: c, face: None, gid: None, advance: p.size * APPROX_ADVANCE },
        }
    };

    let mut lines: Vec<LineLayout> = vec![];
    let mut widest: f64 = 0.0;
    let mut placed: Vec<(Vec<Glyph>, f64)> = vec![];
    for para in &paragraphs {
        let glyphs: Vec<Glyph> = para.iter().map(|&c| shape(c)).collect();
        for range in break_lines(&glyphs, p.spacing, wrap) {
            let g = glyphs.get(range).map(<[Glyph]>::to_vec).unwrap_or_default();
            let w = line_width(&g, p.spacing);
            widest = widest.max(w);
            placed.push((g, w));
        }
    }
    let align_width = wrap.unwrap_or(widest);
    let content = (ascent + descent) * p.size;
    for (i, (glyphs, w)) in placed.into_iter().enumerate() {
        let top = i as f64 * p.line_height;
        let baseline = top + (p.line_height - content) / 2.0 + ascent * p.size;
        let x = match style.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => (align_width - w) / 2.0,
            TextAlign::Right => align_width - w,
        };
        let mut path = BezPath::new();
        let mut pen_x = x;
        let settings = || DrawSettings::unhinted(size, LocationRef::default());
        for g in &glyphs {
            if let (Some(f), Some(gid)) = (g.face, g.gid)
                && let Some(o) = open.get(&f)
                && let Some(outline) = o.outlines.get(gid)
            {
                let mut pen = PathPen { path: &mut path, x: pen_x, y: baseline };
                // A glyph that fails to draw is left out.
                let _ = outline.draw(settings(), &mut pen);
            }
            pen_x += g.advance + p.spacing;
        }
        if style.underline && w > 0.0 {
            let y = baseline + ul_off * p.size;
            let th = (ul_th * p.size).max(0.5);
            let mut bar = Rect::new(x, y - th / 2.0, x + w, y + th / 2.0).to_path(0.1);
            // Same winding as the letters so where they overlap stays filled.
            if path.area().signum() != bar.area().signum() && path.area() != 0.0 {
                bar = bar.reverse_subpaths();
            }
            path.extend(bar);
        }
        let text: String = glyphs.iter().map(|g| g.ch).collect();
        lines.push(LineLayout { path, x, width: w, top, baseline, height: p.line_height, text });
    }
    let height = lines.len() as f64 * p.line_height;
    TextLayout { lines, width: widest, height }
}

/// Width of a line: advances plus spacing between letters, trailing spaces left out.
fn line_width(glyphs: &[Glyph], spacing: f64) -> f64 {
    let end = glyphs.iter().rposition(|g| !g.ch.is_whitespace()).map_or(0, |i| i + 1);
    let visible = glyphs.get(..end).unwrap_or(&[]);
    let adv: f64 = visible.iter().map(|g| g.advance).sum();
    adv + spacing * visible.len().saturating_sub(1) as f64
}

/// Greedy word wrap: ranges of `glyphs` per line. Breaks after spaces, or between letters when a
/// word alone is wider than the box. Spaces starting a wrapped line are skipped.
fn break_lines(glyphs: &[Glyph], spacing: f64, wrap: Option<f64>) -> Vec<std::ops::Range<usize>> {
    let n = glyphs.len();
    let Some(max) = wrap else { return std::iter::once(0..n).collect() };
    let mut out = vec![];
    let mut start = 0;
    if n == 0 {
        return std::iter::once(0..0).collect();
    }
    while start < n {
        let mut width = 0.0;
        let mut last_break: Option<usize> = None;
        let mut i = start;
        while let Some(g) = glyphs.get(i) {
            let w = g.advance + if i > start { spacing } else { 0.0 };
            // Spaces may hang past the edge.
            if !g.ch.is_whitespace() && width + w > max + 1e-9 && i > start {
                break;
            }
            width += w;
            if g.ch.is_whitespace() {
                last_break = Some(i + 1);
            }
            i += 1;
        }
        let end = if i >= n {
            n
        } else {
            match last_break {
                Some(b) if b > start => b,
                _ => i,
            }
        };
        out.push(start..end);
        start = end;
        while glyphs.get(start).is_some_and(|g| g.ch.is_whitespace()) {
            start += 1;
        }
    }
    out
}

/// Draws font outlines (y up, at the glyph origin) into a path (y down, at `x`, `y`).
struct PathPen<'p> {
    path: &'p mut BezPath,
    x: f64,
    y: f64,
}

impl PathPen<'_> {
    fn pt(&self, x: f32, y: f32) -> (f64, f64) {
        (self.x + f64::from(x), self.y - f64::from(y))
    }
}

impl OutlinePen for PathPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.move_to(p);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.line_to(p);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (c, p) = (self.pt(cx0, cy0), self.pt(x, y));
        self.path.quad_to(c, p);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (c0, c1, p) = (self.pt(cx0, cy0), self.pt(cx1, cy1), self.pt(x, y));
        self.path.curve_to(c0, c1, p);
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

/// The natural (unwrapped) size of `text` in `style`.
pub fn measure(book: &FontBook, text: &str, style: &TextStyle) -> (f64, f64) {
    let l = layout(book, text, style, f64::INFINITY);
    (l.width, l.height)
}
