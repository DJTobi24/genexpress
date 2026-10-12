//! Geometry for GenHa (layer L0).
//!
//! Re-exports [`kurbo`] and adds the few design-tool notions the rest of the workspace shares: the
//! [`Frame`] every element sits in (position, size, rotation), fitting one box into another
//! ([`fit`]) and snapping a value to the nearest guide ([`snap`]).
#![forbid(unsafe_code)]

pub use kurbo::{self, Affine, BezPath, Point, Rect, Shape, Size, Vec2};
use serde::{Deserialize, Serialize};

/// Where an element sits on its page: its unrotated box, rotated by `rotation` degrees (clockwise,
/// as on screen where y points down) about the box centre. Units are page pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Default for Frame {
    fn default() -> Self {
        Frame { x: 0.0, y: 0.0, width: 100.0, height: 100.0, rotation: 0.0 }
    }
}

impl Frame {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Frame { x, y, width, height, rotation: 0.0 }
    }

    /// The unrotated box.
    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Maps the element's local space (`0..width` × `0..height`, origin at its top-left) to the page.
    pub fn to_page(&self) -> Affine {
        let c = self.center();
        Affine::translate((c.x, c.y)) * Affine::rotate(self.rotation.to_radians()) * Affine::translate((-self.width / 2.0, -self.height / 2.0))
    }

    /// The inverse of [`Frame::to_page`].
    pub fn from_page(&self) -> Affine {
        self.to_page().inverse()
    }

    /// The axis-aligned bounds of the rotated box on the page.
    pub fn bounds(&self) -> Rect {
        if self.rotation == 0.0 {
            return self.rect().abs();
        }
        let m = self.to_page();
        let corners = [Point::new(0.0, 0.0), Point::new(self.width, 0.0), Point::new(self.width, self.height), Point::new(0.0, self.height)];
        let mut r = Rect::from_points(m * corners[0], m * corners[1]);
        for c in &corners[2..] {
            r = r.union_pt(m * *c);
        }
        r
    }

    /// Whether the page point `p` lies inside the rotated box.
    pub fn contains(&self, p: Point) -> bool {
        let local = self.from_page() * p;
        local.x >= 0.0 && local.y >= 0.0 && local.x <= self.width && local.y <= self.height
    }

    /// True when every number is finite and the size is not negative.
    pub fn is_valid(&self) -> bool {
        [self.x, self.y, self.width, self.height, self.rotation].iter().all(|v| v.is_finite()) && self.width >= 0.0 && self.height >= 0.0
    }

    /// The frame with non-finite numbers replaced and negative sizes made zero (untrusted input).
    pub fn sanitized(&self) -> Frame {
        let f = |v: f64| if v.is_finite() { v } else { 0.0 };
        Frame { x: f(self.x), y: f(self.y), width: f(self.width).max(0.0), height: f(self.height).max(0.0), rotation: f(self.rotation) % 360.0 }
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Frame {
        Frame { x: self.x + dx, y: self.y + dy, ..*self }
    }

    /// Scales position and size about the page origin.
    pub fn scaled(&self, sx: f64, sy: f64) -> Frame {
        Frame { x: self.x * sx, y: self.y * sy, width: self.width * sx, height: self.height * sy, rotation: self.rotation }
    }
}

/// How a box is fitted into another.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// Fill the target, cropping what overflows (keeps the aspect ratio).
    #[default]
    Cover,
    /// Show everything, leaving empty bands (keeps the aspect ratio).
    Contain,
    /// Fill the target exactly, distorting the aspect ratio.
    Stretch,
}

/// The rectangle `content` occupies when fitted into `target` with `mode`, centred.
pub fn fit(content: Size, target: Rect, mode: Fit) -> Rect {
    if content.width <= 0.0 || content.height <= 0.0 || !content.width.is_finite() || !content.height.is_finite() {
        return target;
    }
    let (tw, th) = (target.width(), target.height());
    let (w, h) = match mode {
        Fit::Stretch => (tw, th),
        Fit::Cover | Fit::Contain => {
            let sx = tw / content.width;
            let sy = th / content.height;
            let s = if mode == Fit::Cover { sx.max(sy) } else { sx.min(sy) };
            (content.width * s, content.height * s)
        }
    };
    let c = target.center();
    Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0)
}

/// Snaps `value` to the nearest of `targets` within `threshold`; returns the snapped value and the
/// target it snapped to.
pub fn snap(value: f64, targets: &[f64], threshold: f64) -> (f64, Option<f64>) {
    let mut best: Option<(f64, f64)> = None;
    for &t in targets {
        let d = (t - value).abs();
        if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, t));
        }
    }
    match best {
        Some((_, t)) => (t, Some(t)),
        None => (value, None),
    }
}

/// A rounded rectangle (or plain one when `radius` is 0) as a path, the radius clamped to fit.
pub fn rounded_rect(rect: Rect, radius: f64) -> BezPath {
    let r = radius.max(0.0).min(rect.width().abs() / 2.0).min(rect.height().abs() / 2.0);
    if r <= 0.0 || !r.is_finite() { rect.to_path(0.1) } else { kurbo::RoundedRect::from_rect(rect, r).to_path(0.1) }
}

/// A regular polygon with `sides` corners inscribed in `rect`, first corner at the top.
pub fn polygon(rect: Rect, sides: u32) -> BezPath {
    let n = sides.clamp(3, 64);
    star_like(rect, n, None)
}

/// A star with `points` tips inscribed in `rect`; `inner` is the inner radius as a fraction of the outer.
pub fn star(rect: Rect, points: u32, inner: f64) -> BezPath {
    let inner = if inner.is_finite() { inner.clamp(0.05, 1.0) } else { 0.5 };
    star_like(rect, points.clamp(3, 64), Some(inner))
}

fn star_like(rect: Rect, n: u32, inner: Option<f64>) -> BezPath {
    let c = rect.center();
    let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
    let steps = if inner.is_some() { n * 2 } else { n };
    let mut path = BezPath::new();
    for i in 0..steps {
        let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * f64::from(i) / f64::from(steps);
        let k = match inner {
            Some(f) if i % 2 == 1 => f,
            _ => 1.0,
        };
        let p = Point::new(c.x + rx * k * a.cos(), c.y + ry * k * a.sin());
        if i == 0 { path.move_to(p) } else { path.line_to(p) }
    }
    path.close_path();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_bounds_and_contains() {
        let f = Frame { x: 0.0, y: 0.0, width: 100.0, height: 100.0, rotation: 45.0 };
        let b = f.bounds();
        let d = 100.0 * std::f64::consts::SQRT_2;
        assert!((b.width() - d).abs() < 1e-9);
        assert!(f.contains(Point::new(50.0, 50.0)));
        // A corner of the unrotated box is outside the rotated one.
        assert!(!f.contains(Point::new(2.0, 2.0)));
        let back = f.from_page() * (f.to_page() * Point::new(10.0, 20.0));
        assert!((back.x - 10.0).abs() < 1e-9 && (back.y - 20.0).abs() < 1e-9);
    }

    #[test]
    fn fit_modes() {
        let target = Rect::new(0.0, 0.0, 200.0, 100.0);
        let cover = fit(Size::new(100.0, 100.0), target, Fit::Cover);
        assert_eq!((cover.width(), cover.height()), (200.0, 200.0));
        let contain = fit(Size::new(100.0, 100.0), target, Fit::Contain);
        assert_eq!((contain.width(), contain.height()), (100.0, 100.0));
        assert_eq!(contain.center(), target.center());
        assert_eq!(fit(Size::new(0.0, 5.0), target, Fit::Cover), target);
    }

    #[test]
    fn snapping() {
        assert_eq!(snap(98.0, &[0.0, 100.0], 5.0), (100.0, Some(100.0)));
        assert_eq!(snap(50.0, &[0.0, 100.0], 5.0), (50.0, None));
    }

    #[test]
    fn sanitize_untrusted_frames() {
        let f = Frame { x: f64::NAN, y: 1.0, width: -5.0, height: f64::INFINITY, rotation: 720.0 }.sanitized();
        assert!(f.is_valid());
        assert_eq!(f.width, 0.0);
    }

    #[test]
    fn shapes_have_expected_corners() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(polygon(r, 6).elements().len(), 7);
        assert_eq!(star(r, 5, 0.5).elements().len(), 11);
        assert_eq!(polygon(r, 0).elements().len(), 4);
    }

    #[test]
    fn frame_serde_omits_zero_rotation() {
        let s = serde_json::to_string(&Frame::new(1.0, 2.0, 3.0, 4.0)).unwrap();
        assert_eq!(s, r#"{"x":1.0,"y":2.0,"width":3.0,"height":4.0}"#);
    }
}
