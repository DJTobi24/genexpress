//! Shape outlines, fills and strokes.

use genha_color::Color;
use genha_doc::{Fill, GradientStop, ShapeKind, StrokeStyle};
use genha_geom::{polygon, rounded_rect, star};
use kurbo::{Affine, BezPath, Point, Rect, Shape, Stroke, Vec2};
use vello_cpu::RenderContext;
use vello_cpu::peniko::{self, ColorStop};

/// Longest SVG path data accepted for a [`ShapeKind::Path`], in bytes.
pub const MAX_PATH_DATA: usize = 1 << 20;
/// Widest stroke drawn, in page pixels.
pub const MAX_STROKE_WIDTH: f64 = 10_000.0;

/// The outline of `shape` fitted to a `width` × `height` frame, in element-local coordinates
/// (`0..width` × `0..height`, y down). A [`ShapeKind::Line`] is the horizontal centre line (to be
/// stroked); invalid path data gives an empty path.
pub fn shape_path(shape: &ShapeKind, width: f64, height: f64) -> BezPath {
    if !width.is_finite() || !height.is_finite() {
        return BezPath::new();
    }
    let (w, h) = (width.max(0.0), height.max(0.0));
    let rect = Rect::new(0.0, 0.0, w, h);
    match shape {
        ShapeKind::Rect { corner_radius } => rounded_rect(rect, if corner_radius.is_finite() { *corner_radius } else { 0.0 }),
        ShapeKind::Ellipse => kurbo::Ellipse::from_rect(rect).to_path(0.05),
        ShapeKind::Triangle => {
            let mut p = BezPath::new();
            p.move_to((w / 2.0, 0.0));
            p.line_to((w, h));
            p.line_to((0.0, h));
            p.close_path();
            p
        }
        ShapeKind::Polygon { sides } => polygon(rect, *sides),
        ShapeKind::Star { points, inner_ratio } => star(rect, *points, *inner_ratio),
        ShapeKind::Line => {
            let mut p = BezPath::new();
            p.move_to((0.0, h / 2.0));
            p.line_to((w, h / 2.0));
            p
        }
        ShapeKind::Path { d, view_width, view_height } => svg_path(d, *view_width, *view_height, w, h),
    }
}

fn svg_path(d: &str, vw: f64, vh: f64, w: f64, h: f64) -> BezPath {
    if d.len() > MAX_PATH_DATA {
        return BezPath::new();
    }
    let Ok(mut p) = BezPath::from_svg(d) else { return BezPath::new() };
    if p.elements().iter().any(|el| el.end_point().is_some_and(|pt| !pt.x.is_finite() || !pt.y.is_finite())) {
        return BezPath::new();
    }
    let xf = if vw.is_finite() && vh.is_finite() && vw > 0.0 && vh > 0.0 {
        Affine::scale_non_uniform(w / vw, h / vh)
    } else {
        // No usable view box: fit the path's own bounds to the frame.
        let bb = p.bounding_box();
        let sx = if bb.width() > 0.0 { w / bb.width() } else { 1.0 };
        let sy = if bb.height() > 0.0 { h / bb.height() } else { 1.0 };
        Affine::scale_non_uniform(sx, sy) * Affine::translate((-bb.x0, -bb.y0))
    };
    p.apply_affine(xf);
    p
}

pub(crate) fn pcolor(c: Color) -> peniko::Color {
    peniko::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

/// Gradient stops with offsets made finite, clamped and sorted.
pub(crate) fn clean_stops(stops: &[GradientStop]) -> Vec<(f32, Color)> {
    let mut v: Vec<(f32, Color)> =
        stops.iter().take(256).map(|s| (if s.offset.is_finite() { s.offset.clamp(0.0, 1.0) } else { 0.0 }, s.color)).collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}

/// Start and end points of a linear gradient at `angle` degrees (0 = left to right, 90 = top to
/// bottom) spanning `rect`.
pub fn linear_points(angle: f64, rect: Rect) -> (Point, Point) {
    let a = if angle.is_finite() { angle.to_radians() } else { 0.0 };
    let d = Vec2::new(a.cos(), a.sin());
    let half = (rect.width() * d.x.abs() + rect.height() * d.y.abs()) / 2.0;
    let c = rect.center();
    (c - d * half, c + d * half)
}

/// Radius of a radial gradient over `rect`: from its centre to its farthest edge.
pub fn radial_radius(rect: Rect) -> f64 {
    (rect.width().max(rect.height()) / 2.0).max(1e-6)
}

/// Sets `fill` as the paint for `rect` (in the current transform's space). Returns false when
/// there is nothing to paint.
pub(crate) fn set_fill(ctx: &mut RenderContext, fill: &Fill, rect: Rect) -> bool {
    ctx.reset_paint_transform();
    match fill {
        Fill::None => false,
        Fill::Solid { color } => {
            ctx.set_paint(pcolor(*color));
            color.a > 0
        }
        Fill::Linear { angle, stops } => set_gradient(ctx, stops, |s| {
            let (p0, p1) = linear_points(*angle, rect);
            peniko::Gradient::new_linear(p0, p1).with_stops(s)
        }),
        Fill::Radial { stops } => set_gradient(ctx, stops, |s| peniko::Gradient::new_radial(rect.center(), radial_radius(rect) as f32).with_stops(s)),
    }
}

fn set_gradient(ctx: &mut RenderContext, stops: &[GradientStop], make: impl FnOnce(&[ColorStop]) -> peniko::Gradient) -> bool {
    let stops = clean_stops(stops);
    match stops.as_slice() {
        [] => false,
        [(_, c)] => {
            ctx.set_paint(pcolor(*c));
            true
        }
        _ => {
            let cs: Vec<ColorStop> = stops.iter().map(|(o, c)| ColorStop::from((*o, pcolor(*c)))).collect();
            ctx.set_paint(make(&cs));
            true
        }
    }
}

/// A kurbo stroke for `s`, or None when it draws nothing.
pub(crate) fn stroke_of(s: &StrokeStyle) -> Option<Stroke> {
    if !s.width.is_finite() || s.width <= 0.0 || s.color.a == 0 {
        return None;
    }
    let mut st = Stroke::new(s.width.min(MAX_STROKE_WIDTH));
    let dash: Vec<f64> = s.dash.iter().take(64).copied().filter(|d| d.is_finite() && *d >= 0.0).collect();
    // Very short patterns would cut a long outline into millions of dashes: left solid.
    if !dash.is_empty() && dash.iter().sum::<f64>() >= 0.5 {
        st = st.with_dashes(0.0, dash);
    }
    Some(st)
}
