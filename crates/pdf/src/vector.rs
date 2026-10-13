//! Vector PDF with krilla: the page walked element by element as PDF paths, text with embedded
//! font subsets, and pictures (see the crate docs).
//!
//! krilla's surface works like a canvas with y pointing down, in points. Each page pushes one
//! transform from page pixels to points, then every element pushes its own frame transform and
//! draws in element-local coordinates, as the renderer and the SVG writer do. Every `push_*` is
//! matched by a `pop` in the same function, with no early return in between (krilla panics on an
//! unbalanced stack).

use std::collections::HashMap;
use std::sync::Arc;

use genha_color::Color;
use genha_doc::{
    AssetId, Document, Element, ElementKind, Fill, GradientStop, ImageEl, Page, Shadow, ShapeEl, SpanStyle, StrokeStyle, TextEffect, TextEl,
};
use genha_geom::{Frame, Size, fit, rounded_rect};
use genha_render::{
    MAX_STROKE_WIDTH, RenderOptions, Scene, chart_scene, curved_text_groups, element_transform, linear_points, radial_radius, shape_path, table_scene,
};
use genha_text::{FontBook, GlyphLayout, GlyphLine, PlacedGlyph};
use krilla::color::{cmyk, rgb};
use krilla::error::KrillaError;
use krilla::geom::{Path, PathBuilder, Transform};
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::metadata::Metadata;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{FillRule, LineCap, LineJoin, LinearGradient, RadialGradient, SpreadMethod, Stop, Stroke, StrokeDash};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape, Vec2};

use crate::{Geometry, PT_PER_PX, PdfError, PdfOptions, crop_mark_lines, fitting_scale, with_bleed};

/// Deepest group nesting drawn.
const MAX_DEPTH: usize = 64;
/// Largest shadow blur, in page pixels (as in the renderer).
const MAX_BLUR: f64 = 250.0;
/// Shadows blurred less than this (page pixels) are drawn as sharp vector copies.
const MIN_BLUR: f64 = 0.5;
/// Curves smaller than this (degrees) are drawn straight (as in the renderer).
const MIN_CURVE: f64 = 0.5;
/// Most pixels in one rasterized shadow (they are blurry: more would only make the file larger).
const MAX_SHADOW_PIXELS: f64 = 8_000_000.0;
/// Most device pixels per shadow blur radius (σ) in a rasterized shadow.
const SHADOW_PX_PER_SIGMA: f64 = 12.0;
/// Placeholder colour for pictures that cannot be shown (as in the renderer).
const PLACEHOLDER: Color = Color::rgb(0xd5, 0xd5, 0xdc);
/// Crop mark line width, in points.
const MARK_WIDTH_PT: f64 = 0.3;

/// How the document is written; relaxed when krilla refuses a font or a picture.
#[derive(Clone, Copy, Debug)]
struct Mode {
    /// Letters as outlines instead of text with embedded fonts.
    outline_text: bool,
    /// JPEG pictures embedded as they are (else decoded and recompressed losslessly).
    passthrough: bool,
}

/// Writes the vector PDF of `pages` (indices checked by the caller). When krilla cannot embed a
/// font, the document is written again with letters as outlines; when it cannot embed a JPEG as
/// it is, again with pictures decoded.
pub(crate) fn export(doc: &Document, fonts: &FontBook, opts: &PdfOptions, pages: &[usize]) -> Result<Vec<u8>, PdfError> {
    let mut mode = Mode { outline_text: opts.text_as_outlines, passthrough: true };
    let mut last = String::new();
    for _ in 0..3 {
        match write(doc, fonts, opts, pages, mode)? {
            Ok(bytes) => return Ok(bytes),
            Err(KrillaError::Font(_, msg)) if !mode.outline_text => {
                last = msg;
                mode.outline_text = true;
            }
            Err(KrillaError::Image(_, _, msg)) if mode.passthrough => {
                last = msg;
                mode.passthrough = false;
            }
            Err(e) => return Err(PdfError::Write(e.to_string())),
        }
    }
    Err(PdfError::Write(last))
}

/// One attempt: our own errors outside, krilla's inside (so the caller can retry).
fn write(doc: &Document, fonts: &FontBook, opts: &PdfOptions, pages: &[usize], mode: Mode) -> Result<Result<Vec<u8>, KrillaError>, PdfError> {
    let mut document = krilla::Document::new();
    let title = doc.title.trim();
    let mut meta = Metadata::new().creator("GenHa".into()).producer("GenHa".into());
    if !title.is_empty() {
        meta = meta.title(title.to_string());
    }
    document.set_metadata(meta);
    let mut w = Writer { doc, fonts, mode, cmyk: opts.cmyk, dpi: opts.resolution(), font_cache: HashMap::new(), pictures: HashMap::new() };
    for &index in pages {
        let page = doc.page(index).map_err(|_| PdfError::NoPage(index))?;
        let g = Geometry::new(page, opts);
        let grown;
        let pg: &Page = if g.bleed > 0.0 {
            grown = with_bleed(page, g.bleed);
            &grown
        } else {
            page
        };
        let pt = |v: f64| f32_of(v * PT_PER_PX);
        let (Some(mw), Some(mh)) = (pt(g.media_w), pt(g.media_h)) else {
            return Err(PdfError::Write(format!("page {} has no usable size", index + 1)));
        };
        let Some(mut settings) = PageSettings::from_wh(mw, mh) else { return Err(PdfError::Write(format!("page {} has no usable size", index + 1))) };
        if g.has_boxes() {
            let r = |b: [f64; 4]| krilla::geom::Rect::from_ltrb(pt(b[0])?, pt(b[1])?, pt(b[2])?, pt(b[3])?);
            settings = settings.with_trim_box(r(g.trim)).with_bleed_box(r(g.bleed_box));
        }
        let mut kpage = document.start_page_with(settings);
        let mut s = kpage.surface();
        w.page(&mut s, pg, &g);
        if opts.crop_marks {
            w.crop_marks(&mut s, &g);
        }
        s.finish();
        kpage.finish();
    }
    Ok(document.finish())
}

/// An `f32` for krilla, or None when it does not fit.
fn f32_of(v: f64) -> Option<f32> {
    let f = v as f32;
    f.is_finite().then_some(f)
}

fn sane(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { 0.0 }
}

fn norm(v: f64) -> NormalizedF32 {
    NormalizedF32::new(sane(v, 0.0, 1.0) as f32).unwrap_or(NormalizedF32::ONE)
}

/// A krilla transform for `m`, or None when a coefficient is not finite.
fn ktransform(m: Affine) -> Option<Transform> {
    let [a, b, c, d, e, f] = m.as_coeffs();
    Some(Transform::from_row(f32_of(a)?, f32_of(b)?, f32_of(c)?, f32_of(d)?, f32_of(e)?, f32_of(f)?))
}

/// `p` as a krilla path, or None when it is empty or not finite.
fn kpath(p: &BezPath) -> Option<Path> {
    let mut b = PathBuilder::new();
    let mut any = false;
    for el in p.elements() {
        match *el {
            PathEl::MoveTo(a) => b.move_to(f32_of(a.x)?, f32_of(a.y)?),
            PathEl::LineTo(a) => {
                any = true;
                b.line_to(f32_of(a.x)?, f32_of(a.y)?);
            }
            PathEl::QuadTo(a, c) => {
                any = true;
                b.quad_to(f32_of(a.x)?, f32_of(a.y)?, f32_of(c.x)?, f32_of(c.y)?);
            }
            PathEl::CurveTo(a, c, d) => {
                any = true;
                b.cubic_to(f32_of(a.x)?, f32_of(a.y)?, f32_of(c.x)?, f32_of(c.y)?, f32_of(d.x)?, f32_of(d.y)?);
            }
            PathEl::ClosePath => b.close(),
        }
    }
    if !any {
        return None;
    }
    b.finish()
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

/// Gradient stops with offsets made finite, clamped and sorted.
fn clean_stops(stops: &[GradientStop]) -> Vec<(f32, Color)> {
    let mut v: Vec<(f32, Color)> =
        stops.iter().take(256).map(|s| (if s.offset.is_finite() { s.offset.clamp(0.0, 1.0) } else { 0.0 }, s.color)).collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}

/// Steps between two stops of different opacity, to follow premultiplied blending.
const ALPHA_STEPS: usize = 16;

/// Stops that make a PDF gradient (which blends colour and opacity separately) look like the
/// renderer's, which blends premultiplied colours: a fully transparent stop takes its neighbour's
/// colour instead of tinting the fade, and between stops of different opacity extra stops follow
/// the premultiplied blend.
fn premultiplied(stops: &[(f32, Color)]) -> Vec<(f32, Color)> {
    let mut out: Vec<(f32, Color)> = Vec::with_capacity(stops.len());
    for (i, &(o, c)) in stops.iter().enumerate() {
        if let Some(&(po, pc)) = i.checked_sub(1).and_then(|p| stops.get(p))
            && pc.a != c.a
        {
            for k in 1..ALPHA_STEPS {
                let t = k as f32 / ALPHA_STEPS as f32;
                let (a0, a1) = (f32::from(pc.a) / 255.0, f32::from(c.a) / 255.0);
                let a = a0 + (a1 - a0) * t;
                if a <= 0.0 {
                    continue;
                }
                let ch = |x: u8, y: u8| ((f32::from(x) * a0 * (1.0 - t) + f32::from(y) * a1 * t) / a).round().clamp(0.0, 255.0) as u8;
                let col = Color::rgba(ch(pc.r, c.r), ch(pc.g, c.g), ch(pc.b, c.b), (a * 255.0).round().clamp(0.0, 255.0) as u8);
                out.push((po + (o - po) * t, col));
            }
        }
        out.push((o, c));
    }
    // A transparent stop shows no colour: give it its visible neighbour's.
    let n = out.len();
    for i in 0..n {
        if out.get(i).is_some_and(|s| s.1.a == 0) {
            let near = (1..n).find_map(|d| {
                let pick = |j: Option<usize>| j.and_then(|j| out.get(j)).filter(|s| s.1.a > 0).map(|s| s.1);
                pick(i.checked_sub(d)).or_else(|| pick(i.checked_add(d)))
            });
            if let (Some(c), Some(slot)) = (near, out.get_mut(i)) {
                slot.1 = c.with_alpha(0);
            }
        }
    }
    out
}

/// Plain sRGB → CMYK (no colour profile): black from the brightest channel, the rest from what
/// is left.
fn to_cmyk(c: Color) -> [u8; 4] {
    let (r, g, b) = (f32::from(c.r) / 255.0, f32::from(c.g) / 255.0, f32::from(c.b) / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 - 1e-6 {
        return [0, 0, 0, 255];
    }
    let q = |v: f32| ((1.0 - v - k) / (1.0 - k) * 255.0).round().clamp(0.0, 255.0) as u8;
    [q(r), q(g), q(b), (k * 255.0).round().clamp(0.0, 255.0) as u8]
}

/// Decoded pixels for krilla: RGB or CMYK, with alpha when not opaque.
#[derive(Clone, Hash)]
struct Pixels {
    color: Arc<Vec<u8>>,
    alpha: Option<Arc<Vec<u8>>>,
    width: u32,
    height: u32,
    cmyk: bool,
}

impl CustomImage for Pixels {
    fn color_channel(&self) -> &[u8] {
        &self.color
    }
    fn alpha_channel(&self) -> Option<&[u8]> {
        self.alpha.as_deref().map(Vec::as_slice)
    }
    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn icc_profile(&self) -> Option<&[u8]> {
        None
    }
    fn color_space(&self) -> ImageColorspace {
        if self.cmyk { ImageColorspace::Cmyk } else { ImageColorspace::Rgb }
    }
}

/// A picture ready to place: the image, the rectangle the whole of it covers and the part of
/// that rectangle that shows (element-local coordinates).
#[derive(Clone)]
struct Picture {
    image: Image,
    full: Rect,
    shown: Rect,
}

/// What a picture was made from.
type PicKey = (AssetId, (usize, usize), [u64; 4], [u32; 8], bool, bool);

/// A font file's bytes for krilla.
struct FontBytes(Arc<[u8]>);

impl AsRef<[u8]> for FontBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

struct Writer<'a> {
    doc: &'a Document,
    fonts: &'a FontBook,
    mode: Mode,
    cmyk: bool,
    /// Resolution of rasterized parts (dots per inch).
    dpi: f64,
    /// krilla fonts by font book face; `None` when krilla cannot read it.
    font_cache: HashMap<usize, Option<Font>>,
    pictures: HashMap<PicKey, Option<(Image, f64, f64)>>,
}

impl Writer<'_> {
    // ---------- paint ----------

    fn color(&self, c: Color) -> krilla::color::Color {
        if self.cmyk {
            let [cc, m, y, k] = to_cmyk(c);
            cmyk::Color::new(cc, m, y, k).into()
        } else {
            rgb::Color::new(c.r, c.g, c.b).into()
        }
    }

    fn solid(&self, c: Color) -> krilla::paint::Fill {
        krilla::paint::Fill { paint: self.color(c).into(), opacity: norm(f64::from(c.a) / 255.0), rule: FillRule::NonZero }
    }

    /// The fill for `fill` over `rect` (local coordinates), None when nothing is painted.
    fn fill(&self, fill: &Fill, rect: Rect) -> Option<krilla::paint::Fill> {
        let stops = match fill {
            Fill::None => return None,
            Fill::Solid { color } => return (color.a > 0).then(|| self.solid(*color)),
            Fill::Linear { stops, .. } | Fill::Radial { stops } => clean_stops(stops),
        };
        match stops.as_slice() {
            [] => return None,
            [(_, c)] => return (c.a > 0).then(|| self.solid(*c)),
            _ => {}
        }
        let kstops: Vec<Stop> = premultiplied(&stops)
            .iter()
            .map(|(o, c)| Stop { offset: norm(f64::from(*o)), color: self.color(*c), opacity: norm(f64::from(c.a) / 255.0) })
            .collect();
        let paint: krilla::paint::Paint = match fill {
            Fill::Linear { angle, .. } => {
                let (p0, p1) = linear_points(*angle, rect);
                LinearGradient {
                    x1: f32_of(p0.x)?,
                    y1: f32_of(p0.y)?,
                    x2: f32_of(p1.x)?,
                    y2: f32_of(p1.y)?,
                    transform: Transform::identity(),
                    spread_method: SpreadMethod::Pad,
                    stops: kstops,
                    anti_alias: false,
                }
                .into()
            }
            _ => {
                let c = rect.center();
                let (cx, cy) = (f32_of(c.x)?, f32_of(c.y)?);
                RadialGradient {
                    fx: cx,
                    fy: cy,
                    fr: 0.0,
                    cx,
                    cy,
                    cr: f32_of(radial_radius(rect))?,
                    transform: Transform::identity(),
                    spread_method: SpreadMethod::Pad,
                    stops: kstops,
                    anti_alias: false,
                }
                .into()
            }
        };
        Some(krilla::paint::Fill { paint, opacity: NormalizedF32::ONE, rule: FillRule::NonZero })
    }

    /// A round-joined, round-capped stroke (as the renderer's), None when it draws nothing.
    fn stroke(&self, color: Color, width: f64, dash: &[f64]) -> Option<Stroke> {
        if !width.is_finite() || width <= 0.0 || color.a == 0 {
            return None;
        }
        let dash: Vec<f64> = dash.iter().take(64).copied().filter(|d| d.is_finite() && *d >= 0.0).collect();
        // Very short patterns are left solid, as in the renderer.
        let dash = (!dash.is_empty() && dash.iter().sum::<f64>() >= 0.5)
            .then(|| StrokeDash { array: dash.iter().filter_map(|d| f32_of(*d)).collect(), offset: 0.0 });
        Some(Stroke {
            paint: self.color(color).into(),
            width: f32_of(width.min(MAX_STROKE_WIDTH))?,
            miter_limit: 4.0,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            opacity: norm(f64::from(color.a) / 255.0),
            dash,
        })
    }

    fn style_stroke(&self, s: &StrokeStyle) -> Option<Stroke> {
        self.stroke(s.color, s.width, &s.dash)
    }

    // ---------- drawing helpers ----------

    /// Runs `f` with `m` concatenated to the current transform.
    fn transformed(&mut self, s: &mut Surface, m: Affine, f: impl FnOnce(&mut Self, &mut Surface)) {
        let Some(t) = ktransform(m) else { return };
        s.push_transform(&t);
        f(self, s);
        s.pop();
    }

    fn fill_path(s: &mut Surface, path: &BezPath, fill: krilla::paint::Fill) {
        if let Some(p) = kpath(path) {
            s.set_stroke(None);
            s.set_fill(Some(fill));
            s.draw_path(&p);
        }
    }

    fn stroke_path(s: &mut Surface, path: &BezPath, stroke: Stroke) {
        if let Some(p) = kpath(path) {
            s.set_fill(None);
            s.set_stroke(Some(stroke));
            s.draw_path(&p);
        }
    }

    // ---------- pages ----------

    fn page(&mut self, s: &mut Surface, pg: &Page, g: &Geometry) {
        let to_pt = Affine::scale(PT_PER_PX) * Affine::translate((g.slug, g.slug));
        self.transformed(s, to_pt, |w, s| {
            let rect = Rect::new(0.0, 0.0, pg.width, pg.height);
            // Nothing outside the page (and its bleed) shows, as on the PNG export.
            let clip = kpath(&rect.to_path(0.1));
            if let Some(c) = &clip {
                s.push_clip_path(c, &FillRule::NonZero);
            }
            if let Some(f) = w.fill(&pg.background, rect) {
                Self::fill_path(s, &rect.to_path(0.1), f);
            }
            for el in &pg.elements {
                w.element(s, el, Affine::IDENTITY, 0);
            }
            if clip.is_some() {
                s.pop();
            }
        });
    }

    fn crop_marks(&mut self, s: &mut Surface, g: &Geometry) {
        let mut path = BezPath::new();
        for (a, b) in crop_mark_lines(g) {
            path.move_to(a);
            path.line_to(b);
        }
        // Registration black in CMYK (shows on every plate), plain black otherwise.
        let stroke = Stroke {
            paint: if self.cmyk { cmyk::Color::new(255, 255, 255, 255).into() } else { rgb::Color::black().into() },
            width: (MARK_WIDTH_PT / PT_PER_PX) as f32,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            ..Stroke::default()
        };
        self.transformed(s, Affine::scale(PT_PER_PX), |_, s| Self::stroke_path(s, &path, stroke));
    }

    // ---------- elements ----------

    /// Draws `el`, its page coordinates mapped by `outer` (mirrored groups).
    fn element(&mut self, s: &mut Surface, el: &Element, outer: Affine, depth: usize) {
        if el.hidden || depth > MAX_DEPTH {
            return;
        }
        let opacity = if el.opacity.is_finite() { el.opacity.clamp(0.0, 1.0) } else { 1.0 };
        if opacity <= 0.0 {
            return;
        }
        let frame = el.frame.sanitized();
        let local = outer * frame.to_page() * flips(el, frame.width, frame.height);
        let layer = opacity < 1.0;
        if layer {
            s.push_opacity(norm(f64::from(opacity)));
        }
        match &el.kind {
            ElementKind::Group(g) => {
                // Children are in page coordinates; a flipped group mirrors them about its centre.
                let mirror = if el.flip_h || el.flip_v {
                    let c = frame.center();
                    let rot = Affine::rotate(frame.rotation.to_radians());
                    let sc = Affine::scale_non_uniform(if el.flip_h { -1.0 } else { 1.0 }, if el.flip_v { -1.0 } else { 1.0 });
                    Affine::translate((c.x, c.y)) * rot * sc * rot.inverse() * Affine::translate((-c.x, -c.y))
                } else {
                    Affine::IDENTITY
                };
                for child in &g.children {
                    self.element(s, child, outer * mirror, depth + 1);
                }
            }
            ElementKind::Shape(sh) => self.shape(s, el, sh, &frame, local, outer),
            ElementKind::Text(t) => self.text(s, el, t, &frame, local, outer),
            ElementKind::Image(im) => self.transformed(s, local, |w, s| w.image(s, im, frame.width, frame.height)),
            ElementKind::Chart(c) => {
                let scene = chart_scene(self.fonts, &self.doc.brand, c, frame.width, frame.height);
                self.transformed(s, local, |w, s| w.scene(s, &scene, frame.width, frame.height));
            }
            ElementKind::Table(t) => {
                let scene = table_scene(self.fonts, t, frame.width, frame.height);
                self.transformed(s, local, |w, s| w.scene(s, &scene, frame.width, frame.height));
            }
        }
        if layer {
            s.pop();
        }
    }

    /// A drop shadow: a sharp vector copy (`vector`, drawn in local coordinates with the given
    /// opaque colour) when it is not blurred, else `silhouette` (the element in the shadow's
    /// colour, page coordinates) rendered on its own, blurred and placed as a picture. `bounds`
    /// is what the element covers, in local coordinates.
    #[allow(clippy::too_many_arguments)]
    fn shadow(
        &mut self,
        s: &mut Surface,
        sh: &Shadow,
        local: Affine,
        outer: Affine,
        bounds: Rect,
        silhouette: Element,
        vector: impl FnOnce(&mut Self, &mut Surface, Color),
    ) {
        if sh.color.a == 0 {
            return;
        }
        let (dx, dy) = (sane(sh.offset_x, -1e5, 1e5), sane(sh.offset_y, -1e5, 1e5));
        let blur = sane(sh.blur, 0.0, MAX_BLUR);
        if blur < MIN_BLUR {
            // An opaque copy in a transparency group, so fill and border overlap as one shape.
            s.push_opacity(norm(f64::from(sh.color.a) / 255.0));
            self.transformed(s, Affine::translate((dx, dy)) * local, |w, s| vector(w, s, sh.color.with_alpha(255)));
            s.pop();
            return;
        }
        let page_bounds = (element_transform(&silhouette) * bounds.to_path(0.1)).bounding_box();
        self.raster_shadow(s, silhouette, sh.color, Vec2::new(dx, dy), blur / 2.0, page_bounds, outer);
    }

    /// Renders `el` (page coordinates, before `offset`) alone, blurs it by `sigma` page pixels and
    /// draws it as a picture in `color`.
    #[allow(clippy::too_many_arguments)]
    fn raster_shadow(&mut self, s: &mut Surface, mut el: Element, color: Color, offset: Vec2, sigma: f64, bounds: Rect, outer: Affine) {
        let margin = sigma * 3.0 + 2.0;
        let b = (bounds + offset).inflate(margin, margin);
        let (bw, bh) = (b.width(), b.height());
        if !(bw.is_finite() && bh.is_finite() && bw > 0.0 && bh > 0.0) {
            return;
        }
        let mut scale = self.dpi / 96.0;
        // A blurred picture needs few pixels per blur radius.
        scale = scale.min(SHADOW_PX_PER_SIGMA / sigma.max(1e-6)).min((MAX_SHADOW_PIXELS / (bw * bh)).sqrt());
        scale = fitting_scale(bw, bh, scale);
        if !(scale.is_finite() && scale > 0.0) {
            return;
        }
        crate::shift(&mut el, offset.x - b.x0, offset.y - b.y0);
        let mut tmp = Document::new("shadow", bw, bh);
        let Some(p) = tmp.pages.first_mut() else { return };
        p.background = Fill::None;
        p.elements = vec![el];
        let Ok(r) = genha_render::render_page(&tmp, 0, self.fonts, &RenderOptions { scale, transparent_background: true }) else { return };
        let alpha: Vec<u8> = r.rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
        let Some(mask) = image::GrayImage::from_raw(r.width, r.height, alpha) else { return };
        let radius = (sigma * scale) as f32;
        let small = (r.width.min(r.height) as f32) / 4.0;
        let blurred = if radius >= 0.3 && radius.is_finite() { image::imageops::fast_blur(&mask, radius.min(small.max(0.3))) } else { mask };
        let a = u16::from(color.a);
        let alpha: Vec<u8> = blurred.into_raw().into_iter().map(|v| ((u16::from(v) * a + 127) / 255) as u8).collect();
        let Some(image) = self.image_of(&color_plane(color, alpha.len()), alpha, r.width, r.height) else { return };
        let (Some(w), Some(h)) = (f32_of(bw), f32_of(bh)) else { return };
        let Some(size) = krilla::geom::Size::from_wh(w, h) else { return };
        self.transformed(s, outer * Affine::translate((b.x0, b.y0)), |_, s| s.draw_image(image, size));
    }

    fn shape(&mut self, s: &mut Surface, el: &Element, sh: &ShapeEl, frame: &Frame, local: Affine, outer: Affine) {
        let (w, h) = (frame.width, frame.height);
        let path = shape_path(&sh.shape, w, h);
        let is_line = matches!(sh.shape, genha_doc::ShapeKind::Line);
        // A line without a stroke is drawn with its fill colour, 2 px wide.
        let line_fallback =
            if is_line && sh.stroke.is_none() { sh.fill.primary_color().map(|color| StrokeStyle { color, width: 2.0, dash: vec![] }) } else { None };
        let stroke_style = sh.stroke.as_ref().or(line_fallback.as_ref()).filter(|st| st.width.is_finite() && st.width > 0.0 && st.color.a > 0);
        let fills = !is_line && sh.fill != Fill::None;
        if let Some(shadow) = &sh.shadow {
            let sw = stroke_style.map_or(0.0, |st| st.width.min(MAX_STROKE_WIDTH));
            let bounds = path.bounding_box().inflate(sw, sw);
            let mut sil = el.clone();
            sil.opacity = 1.0;
            let paint = shadow.color.with_alpha(255);
            sil.kind = ElementKind::Shape(ShapeEl {
                shape: sh.shape.clone(),
                fill: if fills || is_line { Fill::Solid { color: paint } } else { Fill::None },
                stroke: stroke_style.map(|st| StrokeStyle { color: paint, ..st.clone() }),
                shadow: None,
            });
            let dash = stroke_style.map(|st| st.dash.clone()).unwrap_or_default();
            let width = stroke_style.map_or(0.0, |st| st.width);
            self.shadow(s, shadow, local, outer, bounds, sil, |w, s, c| {
                if fills {
                    Self::fill_path(s, &path, w.solid(c));
                }
                if let Some(st) = w.stroke(c, width, &dash) {
                    Self::stroke_path(s, &path, st);
                }
            });
        }
        self.transformed(s, local, |w, s| {
            if fills && let Some(f) = w.fill(&sh.fill, Rect::new(0.0, 0.0, frame.width, frame.height)) {
                Self::fill_path(s, &path, f);
            }
            if let Some(st) = stroke_style.and_then(|st| w.style_stroke(st)) {
                Self::stroke_path(s, &path, st);
            }
        });
    }

    // ---------- text ----------

    fn text(&mut self, s: &mut Surface, el: &Element, t: &TextEl, frame: &Frame, local: Affine, outer: Affine) {
        let (w, h) = (frame.width, frame.height);
        let layout = genha_text::layout_el(self.fonts, t, w);
        let curved = curved_text_groups(self.fonts, t, w, h);
        // The letters' outlines (with underlines), as the renderer fills them: for effects, and
        // for the letters themselves when fonts are not embedded.
        let mut outlines = BezPath::new();
        match &curved {
            Some(groups) => groups.iter().for_each(|g| outlines.extend(g.path.iter())),
            None => layout.lines.iter().for_each(|line| outlines.extend(line.path.iter())),
        }
        match &t.effect {
            TextEffect::None => {}
            TextEffect::Shadow { color, offset_x, offset_y, blur } => {
                let sh = Shadow { color: *color, offset_x: *offset_x, offset_y: *offset_y, blur: *blur };
                let bounds = outlines.bounding_box().union(Rect::new(0.0, 0.0, w.max(0.0), layout.height));
                let mut sil = el.clone();
                sil.opacity = 1.0;
                let mut silhouette = TextEl { effect: TextEffect::None, ..t.clone() };
                silhouette.set_style_all(&SpanStyle { color: Some(color.with_alpha(255)), ..SpanStyle::default() });
                sil.kind = ElementKind::Text(silhouette);
                // The copy is outlines, so the text is found once when searching.
                self.shadow(s, &sh, local, outer, bounds, sil, |w, s, c| Self::fill_path(s, &outlines, w.solid(c)));
            }
            TextEffect::Outline { color, width } => {
                let width = sane(*width, 0.0, MAX_STROKE_WIDTH);
                // Twice as wide, half of it covered by the letters drawn on top.
                if let Some(st) = self.stroke(*color, width * 2.0, &[]) {
                    self.transformed(s, local, |_, s| Self::stroke_path(s, &outlines, st));
                }
            }
            // Line boxes do not follow a curve: curved text is drawn without them.
            TextEffect::Highlight { .. } if curved.is_some() => {}
            TextEffect::Highlight { color, padding, corner_radius } => {
                let pad = sane(*padding, 0.0, 10_000.0);
                let r = sane(*corner_radius, 0.0, 10_000.0);
                if color.a > 0 {
                    let fill = self.solid(*color);
                    let mut boxes = BezPath::new();
                    for line in layout.lines.iter().filter(|l| l.width > 0.0) {
                        let rect = Rect::new(line.x - pad, line.top - pad * 0.25, line.x + line.width + pad, line.top + line.height + pad * 0.25);
                        boxes.extend(rounded_rect(rect, r));
                    }
                    self.transformed(s, local, |_, s| Self::fill_path(s, &boxes, fill));
                }
            }
        }
        if layout.styles.iter().all(|st| st.color.a == 0) {
            return;
        }
        if self.mode.outline_text {
            // Each style's outlines in its colour.
            let mut by_style: Vec<(Color, BezPath)> = vec![];
            match &curved {
                Some(groups) => by_style.extend(groups.iter().map(|g| (g.style.color, g.path.clone()))),
                None => {
                    for line in &layout.lines {
                        for (si, p) in line.styled_paths() {
                            let Some(st) = layout.styles.get(si) else { continue };
                            by_style.push((st.color, p.clone()));
                        }
                    }
                }
            }
            self.transformed(s, local, |wr, s| {
                for (c, p) in by_style.iter().filter(|(c, _)| c.a > 0) {
                    Self::fill_path(s, p, wr.solid(*c));
                }
            });
            return;
        }
        let upper = t.style.uppercase;
        if let Some(groups) = &curved {
            match curved_glyphs(self.fonts, t, w, h) {
                // Each glyph real text, turned to follow the arc.
                Some((gl, xfs)) => {
                    let paints = self.run_paints(&gl.styles);
                    self.transformed(s, local, |wr, s| {
                        for (line, line_xfs) in gl.lines.iter().zip(&xfs) {
                            let texts = cluster_texts(line, &t.text, upper);
                            for (k, (g, xf)) in line.glyphs.iter().zip(line_xfs).enumerate() {
                                let text = texts.get(k..=k).unwrap_or(&[]);
                                wr.transformed(s, *xf, |wr, s| wr.glyph_run(s, std::slice::from_ref(g), text, line.baseline, &paints));
                            }
                        }
                    });
                }
                None => self.transformed(s, local, |wr, s| {
                    for g in groups.iter().filter(|g| g.style.color.a > 0) {
                        Self::fill_path(s, &g.path, wr.solid(g.style.color));
                    }
                }),
            }
            return;
        }
        let gl = genha_text::layout_glyphs_el(self.fonts, t, w);
        let paints = self.run_paints(&gl.styles);
        self.transformed(s, local, |wr, s| {
            for line in &gl.lines {
                let texts = cluster_texts(line, &t.text, upper);
                wr.glyph_run(s, &line.glyphs, &texts, line.baseline, &paints);
                wr.decorations(s, line, &paints);
            }
        });
    }

    /// How each style of a laid-out text is painted: its colour, and the synthetic bold and
    /// italic its face lacks.
    fn run_paints(&self, styles: &[genha_doc::TextStyle]) -> Vec<RunPaint> {
        styles
            .iter()
            .map(|st| {
                let syn = genha_render::synthetic(self.fonts, st);
                RunPaint {
                    fill: (st.color.a > 0).then(|| self.solid(st.color)),
                    bold: if syn.embolden > 0.0 { self.stroke(st.color, syn.embolden, &[]) } else { None },
                    slant: syn.slant,
                }
            })
            .collect()
    }

    /// The underlines and strikethroughs of a line, each in its style's colour.
    fn decorations(&mut self, s: &mut Surface, line: &GlyphLine, paints: &[RunPaint]) {
        for d in &line.decorations {
            if let Some(fill) = paints.get(d.style).and_then(|p| p.fill.clone()) {
                Self::fill_path(s, &d.rect.to_path(0.1), fill);
            }
        }
    }

    /// The krilla font for face `face` of the font book, loaded once.
    fn font(&mut self, face: usize) -> Option<Font> {
        let fonts = self.fonts;
        self.font_cache
            .entry(face)
            .or_insert_with(|| {
                let (data, index) = fonts.face_data(face)?;
                let bytes: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(FontBytes(data));
                Font::new(krilla::Data::from(bytes), index)
            })
            .clone()
    }

    /// Real text for `glyphs` (part of one line, in display order, positions from the layout),
    /// in runs of one face and style each, painted as `paints` says for the glyph's style; `texts`
    /// holds the text each glyph stands for (see [`cluster_texts`]). Glyphs of a font krilla cannot
    /// read are filled as outlines.
    fn glyph_run(&mut self, s: &mut Surface, glyphs: &[PlacedGlyph], texts: &[String], baseline: f64, paints: &[RunPaint]) {
        let mut i = 0;
        while let Some(first) = glyphs.get(i) {
            let end = glyphs.iter().skip(i).position(|g| g.face != first.face || g.style != first.style).map_or(glyphs.len(), |n| i + n);
            let run = glyphs.get(i..end).unwrap_or(&[]);
            let run_texts = texts.get(i..end).unwrap_or(&[]);
            i = end.max(i + 1);
            let Some(paint) = paints.get(first.style) else { continue };
            let Some(fill) = paint.fill.clone() else { continue };
            let size = first.size;
            let Some(fsize) = f32_of(size).filter(|v| *v > 0.0) else { continue };
            let Some(face) = first.face else { continue };
            let Some(font) = self.font(face) else {
                let mut p = BezPath::new();
                for g in run {
                    p.extend(g.path.iter());
                }
                Self::fill_path(s, &genha_render::slant_about(p, baseline, paint.slant), fill);
                continue;
            };
            // The drawn glyphs, each with the text of its cluster (glyphs of one cluster share
            // it, so a ligature copies as its letters); a space without a glyph joins the text
            // of the glyph before it, so copied text keeps it.
            let mut text = String::new();
            let mut placed: Vec<(&PlacedGlyph, std::ops::Range<usize>)> = vec![];
            let mut last: Option<(usize, std::ops::Range<usize>)> = None;
            for (k, g) in run.iter().enumerate() {
                let range = match &last {
                    Some((c, r)) if *c == g.cluster => r.clone(),
                    _ => {
                        let start = text.len();
                        match run_texts.get(k) {
                            Some(t) => text.push_str(t),
                            None => text.push(g.ch),
                        }
                        start..text.len()
                    }
                };
                last = Some((g.cluster, range.clone()));
                match g.glyph_id {
                    Some(_) => placed.push((g, range)),
                    None => {
                        if let Some(prev) = placed.last_mut()
                            && prev.1.end == range.start
                        {
                            prev.1.end = range.end;
                        }
                    }
                }
            }
            let Some(x0) = placed.first().map(|(g, _)| g.x) else { continue };
            let norm = |v: f64| Some((v / size) as f32).filter(|v| v.is_finite()).unwrap_or(0.0);
            let kglyphs: Vec<KrillaGlyph> = placed
                .iter()
                .enumerate()
                .filter_map(|(k, (g, range))| {
                    // Advances from the layout's positions (kerning and letter spacing included).
                    let next = placed.get(k + 1).map_or(g.x + g.advance, |n| n.0.x);
                    // krilla's vertical offsets point up, the layout's down.
                    Some(KrillaGlyph::new(GlyphId::new(g.glyph_id?), norm(next - g.x), norm(g.x_offset), norm(-g.y_offset), 0.0, range.clone(), None))
                })
                .collect();
            let (Some(x), Some(y)) = (f32_of(x0), f32_of(baseline)) else { continue };
            let bold = paint.bold.clone();
            let draw = |s: &mut Surface| {
                // Synthetic bold: the letters are also stroked in their colour.
                s.set_stroke(bold);
                s.set_fill(Some(fill));
                s.draw_glyphs(krilla::geom::Point::from_xy(x, y), &kglyphs, font, &text, fsize, false);
                s.set_stroke(None);
            };
            if paint.slant == 0.0 {
                draw(s);
            } else {
                // Synthetic italic: the run sheared about its baseline (as the renderer does).
                let shear = Affine::new([1.0, 0.0, -paint.slant, 1.0, paint.slant * baseline, 0.0]);
                self.transformed(s, shear, |_, s| draw(s));
            }
        }
    }

    // ---------- charts and tables ----------

    fn scene(&mut self, s: &mut Surface, scene: &Scene, w: f64, h: f64) {
        let r = Rect::new(0.0, 0.0, w, h);
        if let Some(bg) = &scene.background
            && let Some(f) = self.fill(bg, r)
        {
            Self::fill_path(s, &r.to_path(0.1), f);
        }
        for p in &scene.prims {
            match p {
                genha_render::Prim::Fill { path, color } => {
                    if color.a > 0 {
                        Self::fill_path(s, path, self.solid(*color));
                    }
                }
                genha_render::Prim::Stroke { path, color, width, .. } => {
                    // The renderer's strokes are round-joined and round-capped either way.
                    if let Some(st) = self.stroke(*color, *width, &[]) {
                        Self::stroke_path(s, path, st);
                    }
                }
                genha_render::Prim::Text(tp) => {
                    let fill = self.solid(tp.color);
                    if self.mode.outline_text || tp.color.a == 0 {
                        if tp.color.a > 0 {
                            Self::fill_path(s, &tp.path, fill);
                        }
                        continue;
                    }
                    // Each line laid out again on its own, then moved where the scene put it.
                    let style = genha_doc::TextStyle { align: genha_doc::TextAlign::Left, ..tp.style.clone() };
                    for line in &tp.lines {
                        let gl = genha_text::layout_glyphs(self.fonts, &line.text, &style, f64::INFINITY);
                        let Some(gline) = gl.lines.first() else { continue };
                        let shift = Affine::translate((line.x - gline.x, line.baseline - gline.baseline));
                        let paints = [RunPaint { fill: Some(fill.clone()), bold: None, slant: 0.0 }];
                        self.transformed(s, shift, |wr, s| {
                            let texts = cluster_texts(gline, &line.text, style.uppercase);
                            wr.glyph_run(s, &gline.glyphs, &texts, gline.baseline, &paints);
                            wr.decorations(s, gline, &paints);
                        });
                    }
                }
            }
        }
    }

    // ---------- pictures ----------

    /// Pixels as a krilla image (RGB, or CMYK when writing CMYK): `color` is RGB triples.
    fn image_of(&self, rgb_triples: &[u8], alpha: Vec<u8>, width: u32, height: u32) -> Option<Image> {
        let n = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
        if n == 0 || rgb_triples.len() != n * 3 || alpha.len() != n {
            return None;
        }
        let color = if self.cmyk {
            rgb_triples.as_chunks::<3>().0.iter().flat_map(|&[r, g, b]| to_cmyk(Color::rgb(r, g, b))).collect()
        } else {
            rgb_triples.to_vec()
        };
        let alpha = alpha.iter().any(|a| *a != 255).then(|| Arc::new(alpha));
        Image::from_custom(Pixels { color: Arc::new(color), alpha, width, height, cmyk: self.cmyk }, true).ok()
    }

    /// The picture of `im` for a `w` × `h` frame, None when it is missing or unreadable.
    fn picture(&mut self, im: &ImageEl, w: f64, h: f64) -> Option<Picture> {
        let asset = self.doc.assets.get(&im.asset)?;
        let info = genha_effects::probe(&asset.data).ok()?;
        let c = im.crop.sanitized();
        let frame = Rect::new(0.0, 0.0, w, h);
        let jpeg = info.mime == "image/jpeg" && im.adjustments.is_identity() && self.mode.passthrough && !self.cmyk;
        let a = &im.adjustments;
        let key: PicKey = (
            im.asset,
            (Arc::as_ptr(&asset.data.0).cast::<u8>() as usize, asset.data.len()),
            if jpeg { [0; 4] } else { [c.x.to_bits(), c.y.to_bits(), c.width.to_bits(), c.height.to_bits()] },
            [a.brightness, a.contrast, a.saturation, a.warmth, a.sharpen, a.blur, a.grayscale, a.sepia].map(f32::to_bits),
            a.invert,
            jpeg,
        );
        if !self.pictures.contains_key(&key) {
            let passed = if jpeg {
                // The file as it is; the crop becomes a clip.
                let bytes: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(FontBytes(asset.data.0.clone()));
                Image::from_jpeg(krilla::Data::from(bytes), true).ok().map(|i| (i, f64::from(info.width), f64::from(info.height)))
            } else {
                None
            };
            let made = passed.or_else(|| {
                // Decoded: cropped and adjusted, except a JPEG krilla could not take as it is,
                // which stays whole (the crop is still a clip).
                let img = genha_effects::decode(&asset.data).ok()?;
                let mut img = if jpeg { img } else { genha_effects::crop(&img, &im.crop) };
                genha_effects::apply_adjustments(&mut img, &im.adjustments);
                let (pw, ph) = img.dimensions();
                let raw = img.into_raw();
                let rgb: Vec<u8> = raw.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, _]| [r, g, b]).collect();
                let alpha: Vec<u8> = raw.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
                Some((self.image_of(&rgb, alpha, pw, ph)?, f64::from(pw), f64::from(ph)))
            });
            self.pictures.insert(key, made);
        }
        let (image, pw, ph) = self.pictures.get(&key)?.clone()?;
        if jpeg {
            // The cropped part fitted into the frame; the whole picture scaled around it.
            let shown = fit(Size::new(pw * c.width, ph * c.height), frame, im.fit);
            let (fw, fh) = (shown.width() / c.width.max(1e-9), shown.height() / c.height.max(1e-9));
            let full = Rect::new(shown.x0 - c.x * fw, shown.y0 - c.y * fh, shown.x0 - c.x * fw + fw, shown.y0 - c.y * fh + fh);
            Some(Picture { image, full, shown })
        } else {
            let shown = fit(Size::new(pw, ph), frame, im.fit);
            Some(Picture { image, full: shown, shown })
        }
    }

    /// An image element in its local coordinates (`w` × `h`).
    fn image(&mut self, s: &mut Surface, im: &ImageEl, w: f64, h: f64) {
        let frame = Rect::new(0.0, 0.0, w, h);
        let clip = match &im.mask {
            Some(m) => shape_path(m, w, h),
            None => rounded_rect(frame, sane(im.corner_radius, 0.0, 1e9)),
        };
        let Some(kclip) = kpath(&clip) else { return };
        match self.picture(im, w, h) {
            Some(pic) => {
                let size = f32_of(pic.full.width()).zip(f32_of(pic.full.height())).and_then(|(w, h)| krilla::geom::Size::from_wh(w, h));
                if let Some(size) = size {
                    s.push_clip_path(&kclip, &FillRule::NonZero);
                    let inner = kpath(&pic.shown.to_path(0.1));
                    if let Some(p) = &inner {
                        s.push_clip_path(p, &FillRule::NonZero);
                    }
                    self.transformed(s, Affine::translate((pic.full.x0, pic.full.y0)), |_, s| s.draw_image(pic.image, size));
                    if inner.is_some() {
                        s.pop();
                    }
                    s.pop();
                }
            }
            None => Self::fill_path(s, &clip, self.solid(PLACEHOLDER)),
        }
        if let Some(b) = &im.border
            && let Some(st) = self.style_stroke(b)
        {
            Self::stroke_path(s, &clip, st);
        }
    }
}

/// The text each glyph of `line` stands for: its cluster in `source` (the text that was laid
/// out), upper-cased when the style is. Glyphs of one cluster get the same text.
fn cluster_texts(line: &GlyphLine, source: &str, upper: bool) -> Vec<String> {
    let mut starts: Vec<usize> = line.glyphs.iter().map(|g| g.cluster).collect();
    starts.sort_unstable();
    starts.dedup();
    line.glyphs
        .iter()
        .map(|g| {
            let end = starts.iter().copied().find(|&c| c > g.cluster).unwrap_or(line.range.end).max(g.cluster);
            let t = source.get(g.cluster..end).unwrap_or("").trim_end_matches(['\n', '\r']).replace('\t', " ");
            if t.is_empty() {
                g.ch.to_string()
            } else if upper {
                t.to_uppercase()
            } else {
                t
            }
        })
        .collect()
}

/// RGB triples of one colour for `n` pixels.
fn color_plane(c: Color, n: usize) -> Vec<u8> {
    [c.r, c.g, c.b].repeat(n)
}

/// How one style of a text is painted in real text.
struct RunPaint {
    /// None: transparent, nothing drawn.
    fill: Option<krilla::paint::Fill>,
    /// Synthetic bold: a stroke in the text's colour over the letters.
    bold: Option<Stroke>,
    /// Synthetic italic: shear about the baseline (0: none).
    slant: f64,
}

/// Curved text glyph by glyph: the layout and, per line and glyph, the transform from the
/// glyph's place in the layout to its place on the arc, centred in the `w` × `h` box — the same
/// placement as `genha_render::curved_text_path`. None when the text is straight.
fn curved_glyphs(fonts: &FontBook, t: &TextEl, w: f64, h: f64) -> Option<(GlyphLayout, Vec<Vec<Affine>>)> {
    let deg = t.curve_degrees();
    if deg.abs() < MIN_CURVE {
        return None;
    }
    let gl = genha_text::layout_glyphs_el(fonts, t, w);
    let widest = gl.width;
    let first = gl.lines.first()?;
    if !(widest.is_finite() && widest > 0.0) {
        return None;
    }
    let theta = deg.abs().to_radians();
    let r0 = widest / theta;
    let arch = deg > 0.0;
    let box_w = if w.is_finite() && w > 0.0 { w } else { widest };
    let cx = box_w / 2.0;
    let b0 = first.baseline;
    let center = Point::new(cx, if arch { b0 + r0 } else { b0 - r0 });
    let min_r = t.style.font_size.max(1.0) * 0.2;
    let mut xfs: Vec<Vec<Affine>> = vec![];
    let mut bounds: Option<Rect> = None;
    for line in &gl.lines {
        let dr = line.baseline - b0;
        let r = if arch { r0 - dr } else { r0 + dr }.max(min_r);
        let mut row = vec![];
        for g in &line.glyphs {
            let gc = g.x + g.advance / 2.0;
            let phi = (gc - cx) / r;
            let (pos, rot) = if arch {
                (center + Vec2::new(r * phi.sin(), -r * phi.cos()), phi)
            } else {
                (center + Vec2::new(r * phi.sin(), r * phi.cos()), -phi)
            };
            let xf = Affine::translate(pos.to_vec2()) * Affine::rotate(rot) * Affine::translate((-gc, -line.baseline));
            let cell = Rect::new(g.x, line.top, g.x + g.advance, line.top + line.height);
            let cb = (xf * cell.to_path(0.1)).bounding_box();
            bounds = Some(bounds.map_or(cb, |b| b.union(cb)));
            if !g.path.is_empty() {
                let pb = (xf * g.path.clone()).bounding_box();
                bounds = Some(bounds.map_or(pb, |b| b.union(pb)));
            }
            row.push(xf);
        }
        xfs.push(row);
    }
    let c = bounds?.center();
    let target = Point::new(if w.is_finite() { w / 2.0 } else { c.x }, if h.is_finite() { h / 2.0 } else { c.y });
    let centre = Affine::translate(target - c);
    let xfs = xfs.into_iter().map(|row| row.into_iter().map(|xf| centre * xf).collect()).collect();
    Some((gl, xfs))
}

#[cfg(test)]
pub(crate) fn test_to_cmyk(c: Color) -> [u8; 4] {
    to_cmyk(c)
}

#[cfg(test)]
pub(crate) fn test_curved_glyphs(fonts: &FontBook, t: &TextEl, w: f64, h: f64) -> Option<(GlyphLayout, Vec<Vec<Affine>>)> {
    curved_glyphs(fonts, t, w, h)
}
