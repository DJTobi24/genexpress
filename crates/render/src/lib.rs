//! The GenHa renderer (layer L3): pages to pixels with [`vello_cpu`], thumbnails, and PNG /
//! JPEG encoding.
//!
//! A page draws its background, then its elements bottom to top, each in its frame (position,
//! size, rotation, flips) with its opacity. Shapes are filled (solid or gradient) and stroked; a
//! shape's drop shadow is blurred with a vello_cpu filter layer (a plain offset copy when the blur
//! is tiny). Text is laid out by `genha-text` and drawn as glyph outlines, with its shadow,
//! outline or highlight effect; curved text has its glyphs placed along an arc ([`curved_text_path`]).
//! Charts and tables become [`Scene`]s of paths and text ([`chart_scene`], [`table_scene`]), shared
//! with the SVG writer. Pictures are decoded, cropped and adjusted by `genha-effects`,
//! fitted into the frame and clipped to it (or to their mask shape); a picture that is missing or
//! cannot be decoded draws as a grey placeholder instead of failing the page.
//!
//! [`Renderer`] keeps decoded pictures between renders so an editor can redraw cheaply.
#![forbid(unsafe_code)]

mod curve;
mod datavis;
mod paint;
mod pictures;
mod synth;

use genha_color::Color;
use genha_doc::{Document, Element, ElementKind, Fill, ImageEl, Shadow, ShapeEl, ShapeKind, StrokeStyle, TextEffect, TextEl};
use genha_geom::{Frame, Size, fit, rounded_rect};
use genha_text::FontBook;
use kurbo::{Affine, BezPath, Rect, Stroke};
use vello_common::filter_effects::{EdgeMode, Filter, FilterPrimitive};
use vello_cpu::peniko;
use vello_cpu::{Pixmap, RenderContext, Resources};

pub use curve::{curved_text_groups, curved_text_path, curved_text_size};
pub use datavis::{
    CHART_PALETTE, Prim, Scene, TableGeometry, TextLine, TextPrim, chart_colors, chart_scene, format_decimals, format_number, series_color,
    table_fit_columns, table_geometry, table_scene, value_text,
};
pub use paint::{MAX_PATH_DATA, MAX_STROKE_WIDTH, linear_points, radial_radius, shape_path};
pub use synth::{SYNTHETIC_BOLD_EM, SYNTHETIC_SLANT, StyledPath, Synthetic, layout_text, layout_text_el, slant_about, synthetic};

/// Largest raster side, in pixels.
pub const MAX_SIDE: u32 = 16_384;
/// Most pixels in one raster (120 megapixels).
pub const MAX_PIXELS: u64 = 120_000_000;
/// Deepest group nesting drawn.
const MAX_DEPTH: usize = 64;
/// Largest shadow blur, in page pixels.
const MAX_BLUR: f64 = 250.0;
/// Placeholder colour for pictures that cannot be shown.
const PLACEHOLDER: Color = Color::rgb(0xd5, 0xd5, 0xdc);

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no page {0}")]
    NoPage(usize),
    #[error("the picture would be too large ({0}×{1} pixels)")]
    TooLarge(u64, u64),
    #[error("invalid render scale {0}")]
    InvalidScale(f64),
    #[error("could not encode the picture: {0}")]
    Encode(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderOptions {
    /// Device pixels per page pixel.
    pub scale: f64,
    /// Leave the page background out (transparent pixels where nothing is drawn).
    pub transparent_background: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions { scale: 1.0, transparent_background: false }
    }
}

/// Pixels: straight-alpha RGBA8, row-major, `width * height * 4` bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Raster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Raster({}×{})", self.width, self.height)
    }
}

impl Raster {
    /// The pixel at (`x`, `y`), or None outside the raster.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (usize::try_from(y).ok()? * usize::try_from(self.width).ok()? + usize::try_from(x).ok()?) * 4;
        let p = self.rgba.get(i..i + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }
}

/// Renders pages, keeping decoded pictures between renders.
#[derive(Default)]
pub struct Renderer {
    pictures: pictures::PicCache,
    resources: Resources,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Renderer")
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer::default()
    }

    /// Forgets the decoded pictures.
    pub fn clear_cache(&mut self) {
        self.pictures.clear();
    }

    /// Renders page `page` of `doc` at `opts.scale` device pixels per page pixel.
    pub fn render_page(&mut self, doc: &Document, page: usize, fonts: &FontBook, opts: &RenderOptions) -> Result<Raster, RenderError> {
        let pg = doc.page(page).map_err(|_| RenderError::NoPage(page))?;
        let scale = opts.scale;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(RenderError::InvalidScale(scale));
        }
        let (w, h) = raster_size(pg.width * scale, pg.height * scale)?;
        let mut ctx = RenderContext::new(w, h);
        let base = Affine::scale(scale);
        let page_rect = Rect::new(0.0, 0.0, pg.width, pg.height);
        if !opts.transparent_background {
            ctx.set_transform(base);
            let bg = if pg.background == Fill::None { Fill::Solid { color: Color::WHITE } } else { pg.background.clone() };
            if paint::set_fill(&mut ctx, &bg, page_rect) {
                ctx.fill_rect(&page_rect);
            }
        }
        let mut draw = Draw { doc, fonts, scale, pictures: &mut self.pictures };
        for el in &pg.elements {
            draw.element(&mut ctx, el, 0, Affine::IDENTITY);
        }
        ctx.flush();
        let mut pm = Pixmap::new(w, h);
        ctx.render(&mut pm, &mut self.resources);
        let rgba: Vec<u8> = pm.take_unpremultiplied().into_iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
        Ok(Raster { width: u32::from(w), height: u32::from(h), rgba })
    }
}

/// The raster size for a page of `w` × `h` device pixels, checked against the limits.
fn raster_size(w: f64, h: f64) -> Result<(u16, u16), RenderError> {
    let big = |v: f64| if v.is_finite() { v.round().max(0.0) as u64 } else { u64::MAX };
    let (bw, bh) = (big(w), big(h));
    if bw > u64::from(MAX_SIDE) || bh > u64::from(MAX_SIDE) || bw.saturating_mul(bh) > MAX_PIXELS {
        return Err(RenderError::TooLarge(bw, bh));
    }
    let side = |v: u64| u16::try_from(v.max(1)).map_err(|_| RenderError::TooLarge(bw, bh));
    Ok((side(bw)?, side(bh)?))
}

/// Renders page `page` once (no picture cache kept).
pub fn render_page(doc: &Document, page: usize, fonts: &FontBook, opts: &RenderOptions) -> Result<Raster, RenderError> {
    Renderer::new().render_page(doc, page, fonts, opts)
}

/// Renders page `page` to fit in `max_side` × `max_side` pixels (aspect ratio kept).
pub fn render_thumbnail(doc: &Document, page: usize, fonts: &FontBook, max_side: u32) -> Result<Raster, RenderError> {
    let pg = doc.page(page).map_err(|_| RenderError::NoPage(page))?;
    let side = f64::from(max_side.clamp(1, MAX_SIDE));
    let longest = pg.width.max(pg.height);
    if !longest.is_finite() || longest <= 0.0 {
        return Err(RenderError::InvalidScale(longest));
    }
    // A hair under, so rounding never makes the long side exceed `max_side`.
    let scale = side / longest * (1.0 - 1e-9);
    render_page(doc, page, fonts, &RenderOptions { scale, transparent_background: false })
}

/// Encodes `r` as PNG.
pub fn encode_png(r: &Raster) -> Result<Vec<u8>, RenderError> {
    let img = image::RgbaImage::from_raw(r.width, r.height, r.rgba.clone())
        .ok_or_else(|| RenderError::Encode("pixel data does not match the size".into()))?;
    let mut out = vec![];
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| RenderError::Encode(e.to_string()))?;
    Ok(out)
}

/// Encodes `r` as JPEG at `quality` (1–100), composited on white.
pub fn encode_jpeg(r: &Raster, quality: u8) -> Result<Vec<u8>, RenderError> {
    let expected = u64::from(r.width) * u64::from(r.height) * 4;
    if r.rgba.len() as u64 != expected || r.width == 0 || r.height == 0 {
        return Err(RenderError::Encode("pixel data does not match the size".into()));
    }
    let rgb: Vec<u8> = r
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| {
            let a = u16::from(a);
            let over = |c: u8| ((u16::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
            [over(r), over(g), over(b)]
        })
        .collect();
    let img = image::RgbImage::from_raw(r.width, r.height, rgb).ok_or_else(|| RenderError::Encode("pixel data does not match the size".into()))?;
    let mut out = vec![];
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
    img.write_with_encoder(enc).map_err(|e| RenderError::Encode(e.to_string()))?;
    Ok(out)
}

/// One page being drawn.
struct Draw<'a> {
    doc: &'a Document,
    fonts: &'a FontBook,
    scale: f64,
    pictures: &'a mut pictures::PicCache,
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

/// An element's transform from its local box to the page (rotation and flips included).
pub fn element_transform(el: &Element) -> Affine {
    let f = el.frame.sanitized();
    f.to_page() * flips(el, f.width, f.height)
}

fn sane(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { 0.0 }
}

impl Draw<'_> {
    /// Draws `el`, its page coordinates mapped by `outer` (mirrored groups).
    fn element(&mut self, ctx: &mut RenderContext, el: &Element, depth: usize, outer: Affine) {
        if el.hidden || depth > MAX_DEPTH {
            return;
        }
        let opacity = if el.opacity.is_finite() { el.opacity.clamp(0.0, 1.0) } else { 1.0 };
        if opacity <= 0.0 {
            return;
        }
        let layer = opacity < 1.0;
        if layer {
            ctx.set_transform(Affine::IDENTITY);
            ctx.push_opacity_layer(opacity);
        }
        let frame = el.frame.sanitized();
        let local = outer * frame.to_page() * flips(el, frame.width, frame.height);
        match &el.kind {
            ElementKind::Shape(s) => self.shape(ctx, s, &frame, local),
            ElementKind::Text(t) => self.text(ctx, t, &frame, local),
            ElementKind::Image(im) => self.image(ctx, im, &frame, local),
            ElementKind::Chart(c) => {
                let scene = chart_scene(self.fonts, &self.doc.brand, c, frame.width, frame.height);
                self.scene(ctx, &scene, &frame, local);
            }
            ElementKind::Table(t) => {
                let scene = table_scene(self.fonts, t, frame.width, frame.height);
                self.scene(ctx, &scene, &frame, local);
            }
            ElementKind::Group(g) => {
                // Children are in page coordinates; a flipped group mirrors them about its centre.
                let c = frame.center();
                let rot = Affine::rotate(frame.rotation.to_radians());
                let sx = if el.flip_h { -1.0 } else { 1.0 };
                let sy = if el.flip_v { -1.0 } else { 1.0 };
                let mirror = if el.flip_h || el.flip_v {
                    Affine::translate((c.x, c.y)) * rot * Affine::scale_non_uniform(sx, sy) * rot.inverse() * Affine::translate((-c.x, -c.y))
                } else {
                    Affine::IDENTITY
                };
                for child in &g.children {
                    self.element(ctx, child, depth + 1, outer * mirror);
                }
            }
        }
        if layer {
            ctx.pop_layer();
        }
    }

    /// The device transform for element-local coordinates.
    fn device(&self, local: Affine) -> Affine {
        Affine::scale(self.scale) * local
    }

    /// Draws a drop shadow of `draw_shape` (called with the transform to draw in): offset by
    /// the shadow's offset in page pixels, blurred, in its colour.
    fn shadow(&self, ctx: &mut RenderContext, local: Affine, sh: &Shadow, draw_shape: &mut dyn FnMut(&mut RenderContext, Affine)) {
        let Shadow { color, offset_x: dx, offset_y: dy, blur } = *sh;
        if color.a == 0 {
            return;
        }
        let (dx, dy) = (sane(dx, -100_000.0, 100_000.0), sane(dy, -100_000.0, 100_000.0));
        let blur = sane(blur, 0.0, MAX_BLUR);
        let shifted = Affine::scale(self.scale) * Affine::translate((dx, dy)) * local;
        if blur * self.scale >= 0.5 {
            // The blurred alpha of the shape, coloured; the content moves rather than the filter
            // (vello_cpu skips layer content entirely outside the view before filtering).
            let filter = Filter::from_primitive(FilterPrimitive::DropShadowOnly {
                dx: 0.0,
                dy: 0.0,
                std_deviation: (blur / 2.0) as f32,
                color: paint::pcolor(color),
                edge_mode: EdgeMode::None,
            });
            ctx.set_transform(Affine::scale(self.scale));
            ctx.push_filter_layer(filter);
            ctx.set_paint(peniko::Color::BLACK);
            draw_shape(ctx, shifted);
            ctx.pop_layer();
        } else {
            ctx.set_transform(Affine::IDENTITY);
            ctx.push_opacity_layer(f32::from(color.a) / 255.0);
            ctx.set_paint(paint::pcolor(color.with_alpha(255)));
            draw_shape(ctx, shifted);
            ctx.pop_layer();
        }
    }

    fn shape(&mut self, ctx: &mut RenderContext, s: &ShapeEl, frame: &Frame, local: Affine) {
        let (w, h) = (frame.width, frame.height);
        let path = shape_path(&s.shape, w, h);
        let is_line = matches!(s.shape, ShapeKind::Line);
        // A line without a stroke is drawn with its fill colour, 2 px wide.
        let line_fallback =
            if is_line && s.stroke.is_none() { s.fill.primary_color().map(|color| StrokeStyle { color, width: 2.0, dash: vec![] }) } else { None };
        let stroke_style = s.stroke.as_ref().or(line_fallback.as_ref());
        let stroke = stroke_style.and_then(paint::stroke_of);
        let fills = !is_line && s.fill != Fill::None;
        if let Some(sh) = &s.shadow {
            let st = stroke.clone();
            self.shadow(ctx, local, sh, &mut |c, xf| {
                c.set_transform(xf);
                if fills {
                    c.fill_path(&path);
                }
                if let Some(st) = &st {
                    c.set_stroke(st.clone());
                    c.stroke_path(&path);
                }
            });
        }
        ctx.set_transform(self.device(local));
        if fills && paint::set_fill(ctx, &s.fill, Rect::new(0.0, 0.0, w, h)) {
            ctx.fill_path(&path);
        }
        if let (Some(st), Some(style)) = (stroke, stroke_style) {
            ctx.reset_paint_transform();
            ctx.set_paint(paint::pcolor(style.color));
            ctx.set_stroke(st);
            ctx.stroke_path(&path);
        }
    }

    /// Draws a chart's or table's primitives in the element's box.
    fn scene(&mut self, ctx: &mut RenderContext, scene: &Scene, frame: &Frame, local: Affine) {
        let device = self.device(local);
        ctx.set_transform(device);
        if let Some(bg) = &scene.background {
            let r = Rect::new(0.0, 0.0, frame.width, frame.height);
            if paint::set_fill(ctx, bg, r) {
                ctx.fill_rect(&r);
            }
        }
        ctx.reset_paint_transform();
        for p in &scene.prims {
            match p {
                Prim::Fill { path, color } => {
                    ctx.set_paint(paint::pcolor(*color));
                    ctx.fill_path(path);
                }
                Prim::Stroke { path, color, width, round } => {
                    let mut st = Stroke::new(width.clamp(0.0, MAX_STROKE_WIDTH));
                    if *round {
                        st = st.with_join(kurbo::Join::Round).with_caps(kurbo::Cap::Round);
                    }
                    ctx.set_paint(paint::pcolor(*color));
                    ctx.set_stroke(st);
                    ctx.stroke_path(path);
                }
                Prim::Text(tp) => {
                    ctx.set_paint(paint::pcolor(tp.color));
                    ctx.fill_path(&tp.path);
                    embolden(ctx, &tp.path, tp.embolden);
                }
            }
        }
    }

    fn text(&mut self, ctx: &mut RenderContext, t: &TextEl, frame: &Frame, local: Affine) {
        let (layout, straight) = layout_text_el(self.fonts, t, frame.width);
        let curved = curved_text_groups(self.fonts, t, frame.width, frame.height);
        let groups = curved.as_ref().unwrap_or(&straight);
        let mut glyphs = BezPath::new();
        for g in groups {
            glyphs.extend(g.path.iter());
        }
        let bold = groups.iter().map(|g| g.embolden).fold(0.0, f64::max);
        let device = self.device(local);
        match &t.effect {
            TextEffect::None => {}
            TextEffect::Shadow { color, offset_x, offset_y, blur } => {
                let sh = Shadow { color: *color, offset_x: *offset_x, offset_y: *offset_y, blur: *blur };
                self.shadow(ctx, local, &sh, &mut |c, xf| {
                    c.set_transform(xf);
                    for g in groups {
                        c.fill_path(&g.path);
                        embolden(c, &g.path, g.embolden);
                    }
                });
            }
            TextEffect::Outline { color, width } => {
                let width = sane(*width, 0.0, MAX_STROKE_WIDTH);
                if width > 0.0 && color.a > 0 {
                    ctx.set_transform(device);
                    ctx.reset_paint_transform();
                    ctx.set_paint(paint::pcolor(*color));
                    // Twice as wide, half of it covered by the letters drawn on top.
                    ctx.set_stroke(Stroke::new(width * 2.0 + bold).with_join(kurbo::Join::Round));
                    ctx.stroke_path(&glyphs);
                }
            }
            // Line boxes do not follow a curve: curved text is drawn without them.
            TextEffect::Highlight { .. } if curved.is_some() => {}
            TextEffect::Highlight { color, padding, corner_radius } => {
                let pad = sane(*padding, 0.0, 10_000.0);
                ctx.set_transform(device);
                ctx.reset_paint_transform();
                ctx.set_paint(paint::pcolor(*color));
                for line in layout.lines.iter().filter(|l| l.width > 0.0) {
                    let r = highlight_rect(line, pad);
                    ctx.fill_path(&rounded_rect(r, sane(*corner_radius, 0.0, 10_000.0)));
                }
            }
        }
        ctx.set_transform(device);
        ctx.reset_paint_transform();
        for g in groups {
            if g.style.color.a == 0 {
                continue;
            }
            ctx.set_paint(paint::pcolor(g.style.color));
            ctx.fill_path(&g.path);
            embolden(ctx, &g.path, g.embolden);
        }
    }

    fn image(&mut self, ctx: &mut RenderContext, im: &ImageEl, frame: &Frame, local: Affine) {
        let (w, h) = (frame.width, frame.height);
        let frame_rect = Rect::new(0.0, 0.0, w, h);
        let clip = match &im.mask {
            Some(m) => shape_path(m, w, h),
            None => rounded_rect(frame_rect, sane(im.corner_radius, 0.0, 1e9)),
        };
        let device = self.device(local);
        // How many device pixels the frame's width covers, to pick a picture reduction.
        let [a, b, c, d, _, _] = device.as_coeffs();
        let px_per_unit = (a * d - b * c).abs().sqrt();
        let asset = self.doc.assets.get(&im.asset);
        let probe_w = asset.and_then(|a| genha_effects::probe(&a.data).ok()).map(|i| (f64::from(i.width), f64::from(i.height)));
        let shown_w = probe_w.map_or(w, |(pw, ph)| {
            let c = im.crop.sanitized();
            fit(Size::new(pw * c.width, ph * c.height), frame_rect, im.fit).width()
        });
        let pic = self.pictures.get(im.asset, asset, im, shown_w * px_per_unit);
        ctx.set_transform(device);
        match pic {
            Some(pic) => {
                let target = fit(Size::new(pic.width, pic.height), frame_rect, im.fit);
                let (pw, ph) = (f64::from(pic.pixmap.width()), f64::from(pic.pixmap.height()));
                let sampler = peniko::ImageSampler { quality: peniko::ImageQuality::Medium, ..Default::default() };
                ctx.set_paint(vello_cpu::Image { image: vello_cpu::ImageSource::Pixmap(pic.pixmap.clone()), sampler });
                ctx.set_paint_transform(
                    Affine::translate((target.x0, target.y0)) * Affine::scale_non_uniform(target.width() / pw, target.height() / ph),
                );
                let covers = target.x0 <= 1e-9 && target.y0 <= 1e-9 && target.x1 >= w - 1e-9 && target.y1 >= h - 1e-9;
                if covers {
                    ctx.fill_path(&clip);
                } else {
                    ctx.push_clip_layer(&clip);
                    ctx.fill_rect(&target);
                    ctx.pop_layer();
                }
                ctx.reset_paint_transform();
            }
            None => {
                ctx.reset_paint_transform();
                ctx.set_paint(paint::pcolor(PLACEHOLDER));
                ctx.fill_path(&clip);
            }
        }
        if let Some(border) = &im.border
            && let Some(st) = paint::stroke_of(border)
        {
            ctx.set_transform(device);
            ctx.set_paint(paint::pcolor(border.color));
            ctx.set_stroke(st);
            ctx.stroke_path(&clip);
        }
    }
}

/// Synthetic bold: strokes `glyphs` `width` wide in the current paint (already filled with it).
fn embolden(ctx: &mut RenderContext, glyphs: &BezPath, width: f64) {
    if width > 0.0 && width.is_finite() {
        ctx.set_stroke(Stroke::new(width.min(MAX_STROKE_WIDTH)).with_join(kurbo::Join::Round));
        ctx.stroke_path(glyphs);
    }
}

/// The box behind a highlighted line.
fn highlight_rect(line: &genha_text::LineLayout, pad: f64) -> Rect {
    Rect::new(line.x - pad, line.top - pad * 0.25, line.x + line.width + pad, line.top + line.height + pad * 0.25)
}

#[cfg(test)]
mod tests;
