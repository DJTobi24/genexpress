//! "Resize to another size": a page re-laid out for a new width and height.
//!
//! Three modes ([`ResizeMode`]):
//!
//! - **smart** (default) re-lays out the design ([`smart`]): it works out what the page is made of
//!   (backgrounds, bands, cards, buttons, text, rows, grids, decorations; [`model`]), lays the
//!   content out again for the new size with real text measurements ([`flow`]), then makes sure no
//!   text overflows its box, leaves the page or covers other text ([`settle`]);
//! - **scale** scales everything uniformly to fit and centres it; full-bleed backgrounds still
//!   cover the new page;
//! - **none** only changes the page size.

mod flow;
mod measure;
mod model;
mod settle;
mod smart;

use genha_doc::{Element, ElementKind, MAX_PAGE_SIDE, Page, ShapeKind, TextEffect};
use genha_geom::{Frame, Rect};
use genha_text::FontBook;

/// How [`resize_page`] treats the content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResizeMode {
    /// Re-lay out the design for the new size (text reflowed, rows and grids re-arranged).
    #[default]
    Smart,
    /// Scale everything uniformly to fit and centre it.
    Scale,
    /// Change the page size only.
    None,
}

impl ResizeMode {
    /// `smart`, `scale` or `none`.
    pub fn parse(s: &str) -> Option<ResizeMode> {
        match s {
            "smart" => Some(ResizeMode::Smart),
            "scale" => Some(ResizeMode::Scale),
            "none" => Some(ResizeMode::None),
            _ => None,
        }
    }
}

/// A resized page and what the user should know about it (text set below the readable minimum,
/// text that could not be kept apart…).
#[derive(Clone, Debug, PartialEq)]
pub struct Resized {
    pub page: Page,
    pub notes: Vec<String>,
}

/// A copy of `page` resized to `width` × `height` (each clamped to `1..=MAX_PAGE_SIDE`). Text is
/// measured with `fonts` (an empty book measures with approximate advances, as the renderer then
/// draws). Ids, names, z-order and the page background are kept.
pub fn resize_page(page: &Page, width: f64, height: f64, mode: ResizeMode, fonts: &FontBook) -> Resized {
    let ow = side(page.width, 1080.0);
    let oh = side(page.height, 1080.0);
    let nw = side(width, ow);
    let nh = side(height, oh);
    let mut out = page.clone();
    out.width = nw;
    out.height = nh;
    for el in &mut out.elements {
        sanitize(el);
    }
    let mut notes = vec![];
    match mode {
        ResizeMode::None => {}
        ResizeMode::Scale => {
            let s = (nw / ow).min(nh / oh);
            let (tx, ty) = ((nw - ow * s) / 2.0, (nh - oh * s) / 2.0);
            let page_rect = Rect::new(0.0, 0.0, ow, oh);
            for el in &mut out.elements {
                let b = el.frame.bounds();
                let covered = b.intersect(page_rect);
                let full = !matches!(el.kind, ElementKind::Text(_))
                    && covered.width() > 0.0
                    && covered.height() > 0.0
                    && covered.area() >= model::FULL_BLEED * ow * oh;
                let map = if full {
                    let (xa, ya) = (Axis::new(0.0, ow, 0.0, nw, s), Axis::new(0.0, oh, 0.0, nh, s));
                    let ((sx, tx), (sy, ty)) = (xa.stretch(b.x0, b.x1), ya.stretch(b.y0, b.y1));
                    Map { sx, sy, tx, ty }
                } else {
                    Map { sx: s, sy: s, tx, ty }
                };
                apply(el, &map, s);
            }
        }
        ResizeMode::Smart => {
            let min_size = settle::min_text_size(nw, nh);
            let k = nw.min(nh) / ow.min(oh);
            let floors: Vec<f64> = out
                .elements
                .iter()
                .map(|e| match &e.kind {
                    ElementKind::Text(t) => flow::type_floor(t.style.font_size, k, min_size),
                    _ => 0.0,
                })
                .collect();
            smart::relayout(&mut out, ow, oh, fonts);
            settle::settle(&mut out, fonts, &floors, &mut notes);
        }
    }
    Resized { page: out, notes }
}

/// [`resize_page`] in [`ResizeMode::Smart`].
pub fn smart_resize(page: &Page, width: f64, height: f64, fonts: &FontBook) -> Resized {
    resize_page(page, width, height, ResizeMode::Smart, fonts)
}

/// `v` limited to `lo..=hi`; `lo` when the range is empty or not a number (`f64::clamp` would
/// panic).
pub(crate) fn within(v: f64, lo: f64, hi: f64) -> f64 {
    if hi >= lo { v.max(lo).min(hi) } else { lo }
}

fn side(v: f64, fallback: f64) -> f64 {
    if v.is_finite() { v.clamp(1.0, MAX_PAGE_SIDE) } else { fallback }
}

/// Frames made finite (children too).
fn sanitize(el: &mut Element) {
    el.frame = el.frame.sanitized();
    if let ElementKind::Group(g) = &mut el.kind {
        g.children.iter_mut().for_each(sanitize);
    }
}

/// A per-axis scale and offset applied to frames: `p' = p * scale + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Map {
    pub sx: f64,
    pub sy: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Map {
    /// Uniform scale `k` taking `from` (a point of the old page) to `to`.
    pub fn uniform(k: f64, from: (f64, f64), to: (f64, f64)) -> Map {
        Map { sx: k, sy: k, tx: to.0 - from.0 * k, ty: to.1 - from.1 * k }
    }

    /// Takes rectangle `a` onto rectangle `b` (stretching when their shapes differ).
    pub fn rect_to_rect(a: Rect, b: Rect) -> Map {
        let sx = if a.width() > 1e-9 { b.width() / a.width() } else { 1.0 };
        let sy = if a.height() > 1e-9 { b.height() / a.height() } else { 1.0 };
        Map { sx, sy, tx: b.x0 - a.x0 * sx, ty: b.y0 - a.y0 * sy }
    }

    pub fn frame(&self, f: &Frame) -> Frame {
        Frame { x: f.x * self.sx + self.tx, y: f.y * self.sy + self.ty, width: f.width * self.sx, height: f.height * self.sy, rotation: f.rotation }
    }

    pub fn rect(&self, r: Rect) -> Rect {
        Rect::new(r.x0 * self.sx + self.tx, r.y0 * self.sy + self.ty, r.x1 * self.sx + self.tx, r.y1 * self.sy + self.ty)
    }

    pub fn shifted(self, dx: f64, dy: f64) -> Map {
        Map { tx: self.tx + dx, ty: self.ty + dy, ..self }
    }
}

/// An element's extent `a..b` on one axis of an area that goes from `o0..o1` (old) to `n0..n1`
/// (new), with `s` the scale for content.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis {
    pub o0: f64,
    pub o1: f64,
    pub n0: f64,
    pub n1: f64,
    pub s: f64,
}

/// Share of an area's side within which an element counts as reaching that side.
pub(crate) const SPAN_MARGIN: f64 = 0.1;
/// An element counts as centred when its centre is off the area centre by at most this share of
/// its smaller edge gap.
const CENTRE_SLACK: f64 = 0.1;

impl Axis {
    pub fn new(o0: f64, o1: f64, n0: f64, n1: f64, s: f64) -> Axis {
        Axis { o0, o1, n0, n1, s }
    }

    fn old(&self) -> f64 {
        self.o1 - self.o0
    }

    /// Whether `a..b` reaches within [`SPAN_MARGIN`] of both ends.
    pub fn spans(&self, a: f64, b: f64) -> bool {
        a - self.o0 <= SPAN_MARGIN * self.old() && self.o1 - b <= SPAN_MARGIN * self.old()
    }

    /// (scale, offset) that keeps the gaps to both ends (scaled by `s`) and stretches between.
    pub fn stretch(&self, a: f64, b: f64) -> (f64, f64) {
        let n0 = self.n0 + (a - self.o0) * self.s;
        let n1 = self.n1 - (self.o1 - b) * self.s;
        let k = if b - a > 1e-9 { ((n1 - n0) / (b - a)).max(0.0) } else { self.s };
        (k, n0 - a * k)
    }

    /// Where `a..b`, scaled to `size`, starts: at the same (scaled) distance from the nearest of
    /// start, centre or end.
    pub fn place(&self, a: f64, b: f64, size: f64) -> f64 {
        self.place_lean(a, b, size, 0.0)
    }

    /// [`Axis::place`], with `lean` (-1 start, 0 centre, 1 end) deciding for an extent that
    /// fills the whole area (and so is as close to the start as to the end).
    pub fn place_lean(&self, a: f64, b: f64, size: f64, lean: f64) -> f64 {
        let s = self.s;
        let lead = a - self.o0;
        let trail = self.o1 - b;
        let tie = 0.01 * self.old().abs();
        if lead.abs() <= tie && trail.abs() <= tie && lean != 0.0 {
            return if lean < 0.0 { self.n0 + lead * s } else { self.n1 - trail * s - size };
        }
        let centre_off = (a + b) / 2.0 - (self.o0 + self.o1) / 2.0;
        if centre_off.abs() <= CENTRE_SLACK * lead.abs().min(trail.abs()) + tie {
            (self.n0 + self.n1) / 2.0 + centre_off * s - size / 2.0
        } else if lead.abs() <= trail.abs() {
            self.n0 + lead * s
        } else {
            self.n1 - trail * s - size
        }
    }

    /// (scale, offset) that scales by `s` and anchors `a..b` with [`Axis::place`].
    pub fn anchor(&self, a: f64, b: f64) -> (f64, f64) {
        let start = self.place(a, b, (b - a) * self.s);
        (self.s, start - a * self.s)
    }
}

/// Moves and scales an element (frame by `map`, sizes, strokes, effects… by `s`).
pub(crate) fn apply(el: &mut Element, map: &Map, s: f64) {
    el.frame = map.frame(&el.frame);
    if let ElementKind::Group(g) = &mut el.kind {
        for c in &mut g.children {
            apply(c, map, s);
        }
        return;
    }
    scale_sizes(el, s);
}

/// Scales what has a size besides the frame: font sizes, strokes, corner radii, shadows, effects.
pub(crate) fn scale_sizes(el: &mut Element, s: f64) {
    if !s.is_finite() || s <= 0.0 {
        return;
    }
    let k = |v: &mut f64| *v *= s;
    match &mut el.kind {
        ElementKind::Shape(sh) => {
            if let ShapeKind::Rect { corner_radius } = &mut sh.shape {
                k(corner_radius);
            }
            if let Some(st) = &mut sh.stroke {
                k(&mut st.width);
                st.dash.iter_mut().for_each(k);
            }
            if let Some(sd) = &mut sh.shadow {
                k(&mut sd.offset_x);
                k(&mut sd.offset_y);
                k(&mut sd.blur);
            }
        }
        ElementKind::Text(t) => {
            t.scale_sizes(s);
            match &mut t.effect {
                TextEffect::None => {}
                TextEffect::Shadow { offset_x, offset_y, blur, .. } => {
                    k(offset_x);
                    k(offset_y);
                    k(blur);
                }
                TextEffect::Outline { width, .. } => k(width),
                TextEffect::Highlight { padding, corner_radius, .. } => {
                    k(padding);
                    k(corner_radius);
                }
            }
        }
        ElementKind::Image(img) => {
            k(&mut img.corner_radius);
            if let Some(st) = &mut img.border {
                k(&mut st.width);
                st.dash.iter_mut().for_each(k);
            }
        }
        ElementKind::Group(g) => {
            for c in &mut g.children {
                scale_sizes(c, s);
            }
        }
        ElementKind::Chart(c) => c.font_size = (c.font_size * s).max(1.0),
        ElementKind::Table(t) => {
            t.font_size = (t.font_size * s).max(1.0);
            k(&mut t.padding);
            if let Some(st) = &mut t.border {
                k(&mut st.width);
            }
            if let Some(st) = &mut t.outer_border {
                k(&mut st.width);
            }
            t.row_heights.iter_mut().for_each(k);
            for f in &mut t.cell_styles {
                if let Some(size) = &mut f.style.font_size {
                    *size = (*size * s).max(1.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
