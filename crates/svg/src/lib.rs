//! SVG export for GenHa (layer L3).
//!
//! Writes one page as a self-contained SVG document by hand: the page background, then every
//! element in a `<g transform="matrix(…)">` (its frame, rotation and flips) with its opacity.
//! Shapes become `<path>`s with solid or gradient fills (`<linearGradient>` / `<radialGradient>` in
//! `<defs>`), strokes and dashes; shadows are blurred copies (`feGaussianBlur`). Text becomes
//! `<text>` with a `<tspan>` per laid-out line (so line breaks match the editor), or outlines when
//! [`SvgOptions::text_as_paths`] is set; curved text is always written as outlines. Charts and
//! tables are the renderer's [`Scene`]s written as paths and `<text>`. Pictures are embedded as `data:` URLs — the original
//! file when it is shown uncropped and unadjusted, otherwise the edited pixels as PNG — clipped to
//! the frame's rounded rectangle or to their mask shape.
#![forbid(unsafe_code)]

pub mod import;

use std::fmt::Write as _;

use base64::Engine as _;
use genha_color::Color;
use genha_doc::{Document, Element, ElementKind, Fill, ImageEl, Shadow, ShapeEl, ShapeKind, StrokeStyle, TextAlign, TextEffect, TextEl, TextStyle};
use genha_geom::{Size, fit, rounded_rect};
use genha_render::{Prim, Raster, Scene, chart_scene, curved_text_groups, encode_png, linear_points, radial_radius, shape_path, table_scene};
use genha_text::FontBook;
use kurbo::{Affine, BezPath, PathEl, Rect, Shape};

/// Deepest group nesting written.
const MAX_DEPTH: usize = 64;
/// Largest blur written, in page pixels.
const MAX_BLUR: f64 = 250.0;
/// Stand-ins for paint attributes in drawings written once and painted several ways (control
/// characters, which [`esc`] removes from user text, so they never clash with it).
const FILL: &str = "\u{1}fill\u{1}";
const STROKE: &str = "\u{1}stroke\u{1}";

#[derive(Debug, thiserror::Error)]
pub enum SvgError {
    #[error("no page {0}")]
    NoPage(usize),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SvgOptions {
    /// Write text as glyph outlines instead of `<text>` (looks the same without the fonts).
    pub text_as_paths: bool,
}

/// Exports page `page` of `doc` as an SVG document.
pub fn export_page(doc: &Document, page: usize, fonts: &FontBook, opts: &SvgOptions) -> Result<String, SvgError> {
    let pg = doc.page(page).map_err(|_| SvgError::NoPage(page))?;
    let (w, h) = (num(pg.width), num(pg.height));
    let mut sw = Writer { doc, fonts, opts: *opts, defs: String::new(), body: String::new(), next_id: 0 };
    let page_rect = Rect::new(0.0, 0.0, pg.width, pg.height);
    if pg.background != Fill::None
        && let Some(paint) = sw.paint(&pg.background, page_rect)
    {
        let _ = writeln!(sw.body, r#"<rect width="{w}" height="{h}"{}/>"#, paint.attrs("fill"));
    }
    for el in &pg.elements {
        sw.element(el, Affine::IDENTITY, 0);
    }
    let mut out = String::with_capacity(sw.body.len() + sw.defs.len() + 512);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    );
    let _ = writeln!(out, "<title>{}</title>", esc(&doc.title));
    if !sw.defs.is_empty() {
        out.push_str("<defs>\n");
        out.push_str(&sw.defs);
        out.push_str("</defs>\n");
    }
    out.push_str(&sw.body);
    out.push_str("</svg>\n");
    Ok(out)
}

/// A number for SVG: up to 3 decimals, no trailing zeros, 0 for non-finite values.
fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".into() } else { s.to_string() }
}

/// Escapes text for XML content and attribute values, dropping characters XML cannot hold.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}

fn hex(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn alpha(c: Color) -> Option<String> {
    (c.a < 255).then(|| num(f64::from(c.a) / 255.0))
}

/// SVG path data for `p`.
fn path_data(p: &BezPath) -> String {
    let mut d = String::new();
    for el in p.elements() {
        let _ = match el {
            PathEl::MoveTo(a) => write!(d, "M{} {}", num(a.x), num(a.y)),
            PathEl::LineTo(a) => write!(d, "L{} {}", num(a.x), num(a.y)),
            PathEl::QuadTo(a, b) => write!(d, "Q{} {} {} {}", num(a.x), num(a.y), num(b.x), num(b.y)),
            PathEl::CurveTo(a, b, c) => write!(d, "C{} {} {} {} {} {}", num(a.x), num(a.y), num(b.x), num(b.y), num(c.x), num(c.y)),
            PathEl::ClosePath => write!(d, "Z"),
        };
    }
    d
}

fn matrix(m: Affine) -> String {
    let c = m.as_coeffs();
    format!("matrix({} {} {} {} {} {})", num(c[0]), num(c[1]), num(c[2]), num(c[3]), num(c[4]), num(c[5]))
}

fn sane(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { 0.0 }
}

/// A paint reference: a colour or `url(#id)`, with its opacity.
struct Paint {
    value: String,
    opacity: Option<String>,
}

impl Paint {
    fn color(c: Color) -> Paint {
        Paint { value: hex(c), opacity: alpha(c) }
    }

    /// ` fill="…" fill-opacity="…"` (or stroke).
    fn attrs(&self, what: &str) -> String {
        match &self.opacity {
            Some(o) => format!(r#" {what}="{}" {what}-opacity="{o}""#, self.value),
            None => format!(r#" {what}="{}""#, self.value),
        }
    }
}

/// Mirrors a `w` × `h` box about its centre.
fn flips(el: &Element, w: f64, h: f64) -> Affine {
    if !el.flip_h && !el.flip_v {
        return Affine::IDENTITY;
    }
    let sx = if el.flip_h { -1.0 } else { 1.0 };
    let sy = if el.flip_v { -1.0 } else { 1.0 };
    Affine::translate((w / 2.0, h / 2.0)) * Affine::scale_non_uniform(sx, sy) * Affine::translate((-w / 2.0, -h / 2.0))
}

struct Writer<'a> {
    doc: &'a Document,
    fonts: &'a FontBook,
    opts: SvgOptions,
    defs: String,
    body: String,
    next_id: u32,
}

impl Writer<'_> {
    fn id(&mut self, kind: &str) -> String {
        self.next_id = self.next_id.saturating_add(1);
        format!("ge-{kind}-{}", self.next_id)
    }

    /// The paint for `fill` over `rect` (gradients go to `<defs>`), None when nothing is painted.
    fn paint(&mut self, fill: &Fill, rect: Rect) -> Option<Paint> {
        let stops = match fill {
            Fill::None => return None,
            Fill::Solid { color } => return Some(Paint::color(*color)),
            Fill::Linear { stops, .. } | Fill::Radial { stops } => stops,
        };
        let mut stops: Vec<(f32, Color)> =
            stops.iter().take(256).map(|s| (if s.offset.is_finite() { s.offset.clamp(0.0, 1.0) } else { 0.0 }, s.color)).collect();
        stops.sort_by(|a, b| a.0.total_cmp(&b.0));
        match stops.as_slice() {
            [] => return None,
            [(_, c)] => return Some(Paint::color(*c)),
            _ => {}
        }
        let id = self.id("grad");
        match fill {
            Fill::Linear { angle, .. } => {
                let (p0, p1) = linear_points(*angle, rect);
                let _ = writeln!(
                    self.defs,
                    r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{}" y1="{}" x2="{}" y2="{}">"#,
                    num(p0.x),
                    num(p0.y),
                    num(p1.x),
                    num(p1.y)
                );
            }
            _ => {
                let c = rect.center();
                let _ = writeln!(
                    self.defs,
                    r#"<radialGradient id="{id}" gradientUnits="userSpaceOnUse" cx="{}" cy="{}" r="{}">"#,
                    num(c.x),
                    num(c.y),
                    num(radial_radius(rect))
                );
            }
        }
        for (o, c) in &stops {
            let op = alpha(*c).map(|a| format!(r#" stop-opacity="{a}""#)).unwrap_or_default();
            let _ = writeln!(self.defs, r#"<stop offset="{}" stop-color="{}"{op}/>"#, num(f64::from(*o)), hex(*c));
        }
        self.defs.push_str(if matches!(fill, Fill::Linear { .. }) { "</linearGradient>\n" } else { "</radialGradient>\n" });
        Some(Paint { value: format!("url(#{id})"), opacity: None })
    }

    /// A blur filter of `std_dev` covering `region` (user space), returning its id.
    fn blur_filter(&mut self, std_dev: f64, region: Rect) -> String {
        let id = self.id("blur");
        let r = region.inflate(std_dev * 3.0 + 2.0, std_dev * 3.0 + 2.0);
        let _ = writeln!(
            self.defs,
            r#"<filter id="{id}" filterUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><feGaussianBlur stdDeviation="{}"/></filter>"#,
            num(r.x0),
            num(r.y0),
            num(r.width()),
            num(r.height()),
            num(std_dev)
        );
        id
    }

    fn element(&mut self, el: &Element, outer: Affine, depth: usize) {
        if el.hidden || depth > MAX_DEPTH {
            return;
        }
        let opacity = if el.opacity.is_finite() { el.opacity.clamp(0.0, 1.0) } else { 1.0 };
        if opacity <= 0.0 {
            return;
        }
        let frame = el.frame.sanitized();
        let local = outer * frame.to_page() * flips(el, frame.width, frame.height);
        let op = if opacity < 1.0 { format!(r#" opacity="{}""#, num(f64::from(opacity))) } else { String::new() };
        match &el.kind {
            ElementKind::Group(g) => {
                let c = frame.center();
                let mirror = if el.flip_h || el.flip_v {
                    let rot = Affine::rotate(frame.rotation.to_radians());
                    let s = Affine::scale_non_uniform(if el.flip_h { -1.0 } else { 1.0 }, if el.flip_v { -1.0 } else { 1.0 });
                    Affine::translate((c.x, c.y)) * rot * s * rot.inverse() * Affine::translate((-c.x, -c.y))
                } else {
                    Affine::IDENTITY
                };
                let _ = writeln!(self.body, "<g{op}>");
                for child in &g.children {
                    self.element(child, outer * mirror, depth + 1);
                }
                self.body.push_str("</g>\n");
            }
            ElementKind::Shape(s) => {
                let _ = writeln!(self.body, "<g{op}>");
                self.shape(s, frame.width, frame.height, local);
                self.body.push_str("</g>\n");
            }
            ElementKind::Text(t) => {
                let _ = writeln!(self.body, "<g{op}>");
                self.text(t, frame.width, frame.height, local);
                self.body.push_str("</g>\n");
            }
            ElementKind::Image(im) => {
                let _ = writeln!(self.body, "<g{op}>");
                self.image(im, frame.width, frame.height, local);
                self.body.push_str("</g>\n");
            }
            ElementKind::Chart(c) => {
                let scene = chart_scene(self.fonts, &self.doc.brand, c, frame.width, frame.height);
                self.scene(&scene, frame.width, frame.height, local, &op);
            }
            ElementKind::Table(t) => {
                let scene = table_scene(self.fonts, t, frame.width, frame.height);
                self.scene(&scene, frame.width, frame.height, local, &op);
            }
        }
    }

    /// A shadow: `content` (SVG drawn in local coordinates, [`FILL`] / [`STROKE`] replaced by the
    /// shadow's paint attributes) offset in page space and blurred.
    fn shadow(&mut self, sh: &Shadow, local: Affine, bounds: Rect, content: &str) {
        if sh.color.a == 0 {
            return;
        }
        let (dx, dy) = (sane(sh.offset_x, -1e5, 1e5), sane(sh.offset_y, -1e5, 1e5));
        let blur = sane(sh.blur, 0.0, MAX_BLUR);
        let filter = if blur > 0.0 { format!(r#" filter="url(#{})""#, self.blur_filter(blur / 2.0, bounds)) } else { String::new() };
        let xf = Affine::translate((dx, dy)) * local;
        let paint = Paint::color(sh.color.with_alpha(255));
        let shadow_op = alpha(sh.color).map(|a| format!(r#" opacity="{a}""#)).unwrap_or_default();
        let body = content.replace(FILL, &paint.attrs("fill")).replace(STROKE, &paint.attrs("stroke"));
        let _ = writeln!(self.body, r#"<g transform="{}"{shadow_op}><g{filter}>{body}</g></g>"#, matrix(xf));
    }

    fn shape(&mut self, s: &ShapeEl, w: f64, h: f64, local: Affine) {
        let path = shape_path(&s.shape, w, h);
        let d = path_data(&path);
        let is_line = matches!(s.shape, ShapeKind::Line);
        let line_fallback =
            if is_line && s.stroke.is_none() { s.fill.primary_color().map(|color| StrokeStyle { color, width: 2.0, dash: vec![] }) } else { None };
        let stroke = s.stroke.as_ref().or(line_fallback.as_ref()).filter(|st| st.width.is_finite() && st.width > 0.0 && st.color.a > 0);
        let stroke_geom = stroke.map(stroke_geometry).unwrap_or_default();
        let fills = !is_line && s.fill != Fill::None;
        if let Some(sh) = &s.shadow {
            let fill_part = if fills { FILL } else { r#" fill="none""# };
            let stroke_part = if stroke.is_some() { format!("{STROKE}{stroke_geom}") } else { String::new() };
            let bounds = path.bounding_box().inflate(stroke.map_or(0.0, |s| s.width), stroke.map_or(0.0, |s| s.width));
            self.shadow(sh, local, bounds, &format!(r#"<path d="{d}"{fill_part}{stroke_part}/>"#));
        }
        let fill_attr = if fills { self.paint(&s.fill, Rect::new(0.0, 0.0, w, h)).map(|p| p.attrs("fill")) } else { None };
        let fill_attr = fill_attr.unwrap_or_else(|| r#" fill="none""#.into());
        let stroke_attr = stroke.map(|st| format!("{}{stroke_geom}", Paint::color(st.color).attrs("stroke"))).unwrap_or_default();
        let _ = writeln!(self.body, r#"<path transform="{}" d="{d}"{fill_attr}{stroke_attr}/>"#, matrix(local));
    }

    fn text(&mut self, t: &TextEl, w: f64, h: f64, local: Affine) {
        let (layout, straight) = genha_render::layout_text_el(self.fonts, t, w);
        let curved = curved_text_groups(self.fonts, t, w, h);
        let groups = curved.as_ref().unwrap_or(&straight);
        let mut glyphs = BezPath::new();
        for g in groups {
            glyphs.extend(g.path.iter());
        }
        let as_paths = self.opts.text_as_paths || curved.is_some();
        let bold = groups.iter().map(|g| g.embolden).fold(0.0, f64::max);
        let bounds = glyphs.bounding_box().union(Rect::new(0.0, 0.0, w.max(0.0), layout.height));
        let m = matrix(local);
        match &t.effect {
            TextEffect::None => {}
            TextEffect::Shadow { color, offset_x, offset_y, blur } => {
                let sh = Shadow { color: *color, offset_x: *offset_x, offset_y: *offset_y, blur: *blur };
                let body = self.letters(t, &layout, groups, as_paths, Letters::Shadow);
                self.shadow(&sh, local, bounds, &body);
            }
            TextEffect::Outline { color, width } => {
                let width = sane(*width, 0.0, 10_000.0);
                if width > 0.0 && color.a > 0 {
                    let p = Paint::color(*color);
                    let stroke = format!(r#"{} stroke-width="{}" stroke-linejoin="round""#, p.attrs("stroke"), num(width * 2.0 + bold));
                    let body = self.letters(t, &layout, groups, as_paths, Letters::Outline).replace(FILL, &p.attrs("fill")).replace(STROKE, &stroke);
                    let _ = writeln!(self.body, r#"<g transform="{m}">{body}</g>"#);
                }
            }
            // Line boxes do not follow a curve: curved text is written without them.
            TextEffect::Highlight { .. } if curved.is_some() => {}
            TextEffect::Highlight { color, padding, corner_radius } => {
                let pad = sane(*padding, 0.0, 10_000.0);
                let r = sane(*corner_radius, 0.0, 10_000.0);
                let fill = Paint::color(*color).attrs("fill");
                let _ = write!(self.body, r#"<g transform="{m}">"#);
                for line in layout.lines.iter().filter(|l| l.width > 0.0) {
                    let rect = Rect::new(line.x - pad, line.top - pad * 0.25, line.x + line.width + pad, line.top + line.height + pad * 0.25);
                    let _ = write!(self.body, r#"<path d="{}"{fill}/>"#, path_data(&rounded_rect(rect, r)));
                }
                self.body.push_str("</g>\n");
            }
        }
        let body = self.letters(t, &layout, groups, as_paths, Letters::Own);
        let _ = writeln!(self.body, r#"<g transform="{m}">{body}</g>"#);
    }

    /// The letters of `t`: outlines (`groups`, one `<path>` per style) or `<text>` with a
    /// `<tspan>` per line and, inside it, one per style run. Painted in each run's colour
    /// ([`Letters::Own`]) or with [`FILL`] / [`STROKE`] standing for one paint.
    fn letters(&self, t: &TextEl, layout: &genha_text::TextLayout, groups: &[genha_render::StyledPath], as_paths: bool, how: Letters) -> String {
        let mut s = String::new();
        if as_paths {
            for g in groups {
                // Synthetic bold: the letters are also stroked in their own paint.
                let bold = if g.embolden > 0.0 {
                    format!(r#" stroke-width="{}" stroke-linejoin="round""#, num(g.embolden.min(genha_render::MAX_STROKE_WIDTH)))
                } else {
                    String::new()
                };
                let (fill, stroke) = match how {
                    Letters::Own => {
                        let p = Paint::color(g.style.color);
                        (p.attrs("fill"), if bold.is_empty() { String::new() } else { format!("{}{bold}", p.attrs("stroke")) })
                    }
                    Letters::Shadow => (FILL.to_string(), if bold.is_empty() { String::new() } else { format!("{STROKE}{bold}") }),
                    Letters::Outline => (FILL.to_string(), STROKE.to_string()),
                };
                let _ = write!(s, r#"<path d="{}"{fill}{stroke}/>"#, path_data(&g.path));
            }
            return s;
        }
        let st = &t.style;
        let rich = !t.spans.is_empty();
        s.push_str(r#"<text xml:space="preserve""#);
        s.push_str(&self.font_attrs(st, None, !rich));
        match how {
            Letters::Own => s.push_str(&Paint::color(st.color).attrs("fill")),
            Letters::Shadow => s.push_str(FILL),
            Letters::Outline => {
                s.push_str(FILL);
                s.push_str(STROKE);
            }
        }
        s.push('>');
        let runs = if rich { t.style_runs() } else { vec![] };
        for line in &layout.lines {
            if !rich {
                s.push_str(&tspan(st.align, line.x, line.width, line.baseline, &line.text));
                continue;
            }
            // The line's text cut by style runs, without its trailing spaces.
            let mut pieces: Vec<(String, &TextStyle)> = vec![];
            for (r, rs) in &runs {
                let (a, b) = (r.start.max(line.range.start), r.end.min(line.range.end));
                if a >= b {
                    continue;
                }
                let text: String = t.text.get(a..b).unwrap_or("").chars().filter(|c| *c != '\n').map(|c| if c == '\t' { ' ' } else { c }).collect();
                let text = if st.uppercase { text.to_uppercase() } else { text };
                pieces.push((text, rs));
            }
            while let Some(last) = pieces.last_mut() {
                let trimmed = last.0.trim_end().len();
                last.0.truncate(trimmed);
                if !last.0.is_empty() {
                    break;
                }
                pieces.pop();
            }
            s.push_str(&tspan_open(st.align, line.x, line.width, line.baseline));
            for (text, rs) in pieces {
                let mut attrs = self.font_attrs(rs, Some(st), true);
                if how == Letters::Own && rs.color != st.color {
                    attrs.push_str(&Paint::color(rs.color).attrs("fill"));
                }
                if attrs.is_empty() {
                    s.push_str(&esc(&text));
                } else {
                    let _ = write!(s, "<tspan{attrs}>{}</tspan>", esc(&text));
                }
            }
            s.push_str("</tspan>");
        }
        s.push_str("</text>");
        s
    }

    /// A chart's or table's primitives in the element's box.
    fn scene(&mut self, scene: &Scene, w: f64, h: f64, local: Affine, op: &str) {
        let _ = writeln!(self.body, r#"<g transform="{}"{op}>"#, matrix(local));
        if let Some(bg) = &scene.background {
            let r = Rect::new(0.0, 0.0, w, h);
            if let Some(paint) = self.paint(bg, r) {
                let _ = writeln!(self.body, r#"<rect width="{}" height="{}"{}/>"#, num(w), num(h), paint.attrs("fill"));
            }
        }
        for p in &scene.prims {
            match p {
                Prim::Fill { path, color } => {
                    let _ = writeln!(self.body, r#"<path d="{}"{}/>"#, path_data(path), Paint::color(*color).attrs("fill"));
                }
                Prim::Stroke { path, color, width, round } => {
                    let round = if *round { r#" stroke-linejoin="round" stroke-linecap="round""# } else { "" };
                    let _ = writeln!(
                        self.body,
                        r#"<path d="{}" fill="none"{} stroke-width="{}"{round}/>"#,
                        path_data(path),
                        Paint::color(*color).attrs("stroke"),
                        num(width.min(genha_render::MAX_STROKE_WIDTH))
                    );
                }
                Prim::Text(tp) => {
                    let fill = Paint::color(tp.color).attrs("fill");
                    if self.opts.text_as_paths {
                        let bold = if tp.embolden > 0.0 {
                            let w = num(tp.embolden.min(genha_render::MAX_STROKE_WIDTH));
                            format!(r#"{} stroke-width="{w}" stroke-linejoin="round""#, Paint::color(tp.color).attrs("stroke"))
                        } else {
                            String::new()
                        };
                        let _ = writeln!(self.body, r#"<path d="{}"{fill}{bold}/>"#, path_data(&tp.path));
                    } else {
                        let mut s = self.text_open(&tp.style);
                        s.push_str(&fill);
                        if tp.transform != Affine::IDENTITY {
                            // Turned text (a chart's vertical axis title).
                            let _ = write!(s, r#" transform="{}""#, matrix(tp.transform));
                        }
                        s.push('>');
                        for l in &tp.lines {
                            s.push_str(&tspan(tp.style.align, l.x, l.width, l.baseline, &l.text));
                        }
                        let _ = writeln!(self.body, "{s}</text>");
                    }
                }
            }
        }
        self.body.push_str("</g>\n");
    }

    /// `<text …` with the font attributes of `st` (unclosed: paint attributes and `>` follow).
    fn text_open(&self, st: &TextStyle) -> String {
        format!(r#"<text xml:space="preserve"{}"#, self.font_attrs(st, None, true))
    }

    /// The font attributes of `st` (each with a leading space): all of them, or with `base` only
    /// those that differ from it (a style run inside a `<text>` in `base`); text decorations only
    /// with `decoration` (they cannot be turned off in a nested run).
    fn font_attrs(&self, st: &TextStyle, base: Option<&TextStyle>, decoration: bool) -> String {
        let face_differs = base.is_none_or(|b| b.font_family != st.font_family || b.bold != st.bold || b.italic != st.italic);
        let size = sane(st.font_size, 0.0, genha_text::MAX_FONT_SIZE);
        let mut s = String::new();
        if face_differs {
            s.push_str(&format!(r#" font-family="{}""#, esc(&self.family_list(st))));
        }
        if base.is_none_or(|b| b.font_size != st.font_size) {
            let _ = write!(s, r#" font-size="{}""#, num(size));
        }
        if face_differs {
            let weight = if st.font_family.trim().eq_ignore_ascii_case("display") {
                if st.bold { "900" } else { "800" }
            } else if st.bold {
                "bold"
            } else {
                "normal"
            };
            let _ = write!(s, r#" font-weight="{weight}" font-style="{}""#, if st.italic { "italic" } else { "normal" });
        }
        if decoration {
            match (st.underline, st.strikethrough) {
                (true, true) => s.push_str(r#" text-decoration="underline line-through""#),
                (true, false) => s.push_str(r#" text-decoration="underline""#),
                (false, true) => s.push_str(r#" text-decoration="line-through""#),
                (false, false) => {}
            }
        }
        let spacing = sane(st.letter_spacing, -1000.0, 10_000.0) / 1000.0 * size;
        if spacing != 0.0 && base.is_none_or(|b| b.font_size != st.font_size) {
            let _ = write!(s, r#" letter-spacing="{}""#, num(spacing));
        }
        s
    }

    /// The `font-family` list for `st`: the family asked for, the one the book sets it in, and a
    /// generic family.
    fn family_list(&self, st: &TextStyle) -> String {
        let mut family = vec![];
        if let Some(resolved) = self.fonts.resolve_family(&st.font_family, st.bold, st.italic) {
            family.push(format!("'{}'", resolved.replace('\'', "")));
        }
        let generic = match st.font_family.trim().to_lowercase().as_str() {
            "serif" => "serif",
            "mono" | "monospace" => "monospace",
            _ => "sans-serif",
        };
        if !matches!(st.font_family.trim().to_lowercase().as_str(), "sans" | "serif" | "mono" | "display" | "sans-serif" | "monospace") {
            let f = format!("'{}'", st.font_family.replace('\'', ""));
            if !family.contains(&f) {
                family.insert(0, f);
            }
        }
        family.push(generic.into());
        family.join(", ")
    }

    fn image(&mut self, im: &ImageEl, w: f64, h: f64, local: Affine) {
        let frame_rect = Rect::new(0.0, 0.0, w, h);
        let clip = match &im.mask {
            Some(m) => shape_path(m, w, h),
            None => rounded_rect(frame_rect, sane(im.corner_radius, 0.0, 1e9)),
        };
        let clip_id = self.id("clip");
        let _ = writeln!(self.defs, r#"<clipPath id="{clip_id}"><path d="{}"/></clipPath>"#, path_data(&clip));
        let m = matrix(local);
        match picture(self.doc, im) {
            Some((uri, pw, ph)) => {
                let t = fit(Size::new(pw, ph), frame_rect, im.fit);
                let _ = writeln!(
                    self.body,
                    r#"<g transform="{m}" clip-path="url(#{clip_id})"><image x="{}" y="{}" width="{}" height="{}" preserveAspectRatio="none" href="{uri}"/></g>"#,
                    num(t.x0),
                    num(t.y0),
                    num(t.width()),
                    num(t.height())
                );
            }
            None => {
                let _ = writeln!(self.body, r##"<path transform="{m}" d="{}" fill="#d5d5dc"/>"##, path_data(&clip));
            }
        }
        if let Some(b) = &im.border
            && b.width.is_finite()
            && b.width > 0.0
            && b.color.a > 0
        {
            let _ = writeln!(
                self.body,
                r#"<path transform="{m}" d="{}" fill="none"{}{}/>"#,
                path_data(&clip),
                Paint::color(b.color).attrs("stroke"),
                stroke_geometry(b)
            );
        }
    }
}

/// How [`Writer::letters`] paints text.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Letters {
    /// Each style run in its own colour (and synthetic bold).
    Own,
    /// One paint, [`FILL`], and [`STROKE`] for synthetic bold (a shadow).
    Shadow,
    /// One paint, [`FILL`] and [`STROKE`] on every letter (an outline).
    Outline,
}

/// A `<tspan>` for one laid-out line, anchored at the end its alignment keeps in place.
fn tspan(align: TextAlign, x: f64, width: f64, baseline: f64, text: &str) -> String {
    format!("{}{}</tspan>", tspan_open(align, x, width, baseline), esc(text.trim_end()))
}

/// The opening `<tspan …>` of [`tspan`].
fn tspan_open(align: TextAlign, x: f64, width: f64, baseline: f64) -> String {
    let (x, anchor) = match align {
        TextAlign::Left => (x, "start"),
        TextAlign::Center => (x + width / 2.0, "middle"),
        TextAlign::Right => (x + width, "end"),
    };
    format!(r#"<tspan x="{}" y="{}" text-anchor="{anchor}">"#, num(x), num(baseline))
}

/// ` stroke-width="…"` and dashes for `s`.
fn stroke_geometry(s: &StrokeStyle) -> String {
    let mut a = format!(r#" stroke-width="{}""#, num(s.width.min(genha_render::MAX_STROKE_WIDTH)));
    let dash: Vec<f64> = s.dash.iter().take(64).copied().filter(|d| d.is_finite() && *d >= 0.0).collect();
    if !dash.is_empty() && dash.iter().sum::<f64>() >= 0.5 {
        let _ = write!(a, r#" stroke-dasharray="{}""#, dash.iter().map(|d| num(*d)).collect::<Vec<_>>().join(" "));
    }
    a
}

/// The picture of `im` as a `data:` URL with the size it is fitted by (the cropped size), or None
/// when it is missing or cannot be read.
fn picture(doc: &Document, im: &ImageEl) -> Option<(String, f64, f64)> {
    let asset = doc.assets.get(&im.asset)?;
    let info = genha_effects::probe(&asset.data).ok()?;
    let b64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    if im.crop.is_full() && im.adjustments.is_identity() {
        // The original file, as it is.
        return Some((format!("data:{};base64,{}", info.mime, b64(&asset.data)), f64::from(info.width), f64::from(info.height)));
    }
    let img = genha_effects::decode(&asset.data).ok()?;
    let mut img = genha_effects::crop(&img, &im.crop);
    genha_effects::apply_adjustments(&mut img, &im.adjustments);
    let (width, height) = img.dimensions();
    let png = encode_png(&Raster { width, height, rgba: img.into_raw() }).ok()?;
    Some((format!("data:image/png;base64,{}", b64(&png)), f64::from(width), f64::from(height)))
}

#[cfg(test)]
mod tests;
