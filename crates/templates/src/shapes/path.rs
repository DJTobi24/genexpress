//! A tiny SVG path builder for icons drawn in code (100 × 100 view box, y down).
//!
//! Every part is written clockwise on screen, every hole anticlockwise, so icons look the same
//! with the non-zero and the even-odd fill rule (as long as parts never overlap).

use std::fmt::Write as _;

pub(crate) type Pt = (f64, f64);

/// A number with at most two decimals and no trailing zeros.
fn n(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// The point at `r` from (`cx`, `cy`), `deg` degrees clockwise from 12 o'clock.
pub(crate) fn at(cx: f64, cy: f64, r: f64, deg: f64) -> Pt {
    let a = deg.to_radians();
    (cx + r * a.sin(), cy - r * a.cos())
}

/// `points` turned `deg` degrees clockwise about `c`.
pub(crate) fn rotate(points: &[Pt], c: Pt, deg: f64) -> Vec<Pt> {
    let (s, co) = deg.to_radians().sin_cos();
    points.iter().map(|(x, y)| (c.0 + (x - c.0) * co - (y - c.1) * s, c.1 + (x - c.0) * s + (y - c.1) * co)).collect()
}

/// Twice the signed area of a polygon (positive = clockwise on screen).
fn signed_area(points: &[Pt]) -> f64 {
    let len = points.len();
    (0..len)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % len]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum()
}

/// Path data under construction.
#[derive(Default)]
pub(crate) struct P(String);

impl P {
    pub fn new() -> P {
        P(String::new())
    }

    pub fn done(self) -> String {
        self.0.trim().to_string()
    }

    fn sep(&mut self) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
    }

    /// Hand-written path data, appended as is.
    pub fn raw(mut self, d: &str) -> P {
        self.sep();
        self.0.push_str(d);
        self
    }

    /// A closed polygon, turned the right way for a part (`hole` false) or a hole.
    pub fn poly(mut self, points: &[Pt], hole: bool) -> P {
        if points.len() < 3 {
            return self;
        }
        let mut pts = points.to_vec();
        if (signed_area(&pts) < 0.0) != hole {
            pts.reverse();
        }
        self.sep();
        for (i, (x, y)) in pts.iter().enumerate() {
            let _ = write!(self.0, "{}{} {} ", if i == 0 { 'M' } else { 'L' }, n(*x), n(*y));
        }
        self.0.push('Z');
        self
    }

    pub fn rect(self, x: f64, y: f64, w: f64, h: f64, hole: bool) -> P {
        self.poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], hole)
    }

    /// A rounded rectangle.
    pub fn rrect(mut self, x: f64, y: f64, w: f64, h: f64, r: f64, hole: bool) -> P {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        if r <= 0.0 {
            return self.rect(x, y, w, h, hole);
        }
        let (x1, y1) = (x + w, y + h);
        self.sep();
        let a = |s: &mut String, ex: f64, ey: f64, sweep: u8| {
            let _ = write!(s, "A{} {} 0 0 {sweep} {} {} ", n(r), n(r), n(ex), n(ey));
        };
        if hole {
            let _ = write!(self.0, "M{} {} ", n(x + r), n(y));
            a(&mut self.0, x, y + r, 0);
            let _ = write!(self.0, "L{} {} ", n(x), n(y1 - r));
            a(&mut self.0, x + r, y1, 0);
            let _ = write!(self.0, "L{} {} ", n(x1 - r), n(y1));
            a(&mut self.0, x1, y1 - r, 0);
            let _ = write!(self.0, "L{} {} ", n(x1), n(y + r));
            a(&mut self.0, x1 - r, y, 0);
        } else {
            let _ = write!(self.0, "M{} {} L{} {} ", n(x + r), n(y), n(x1 - r), n(y));
            a(&mut self.0, x1, y + r, 1);
            let _ = write!(self.0, "L{} {} ", n(x1), n(y1 - r));
            a(&mut self.0, x1 - r, y1, 1);
            let _ = write!(self.0, "L{} {} ", n(x + r), n(y1));
            a(&mut self.0, x, y1 - r, 1);
            let _ = write!(self.0, "L{} {} ", n(x), n(y + r));
            a(&mut self.0, x + r, y, 1);
        }
        self.0.push('Z');
        self
    }

    pub fn ellipse(mut self, cx: f64, cy: f64, rx: f64, ry: f64, hole: bool) -> P {
        let s = u8::from(!hole);
        self.sep();
        let _ = write!(
            self.0,
            "M{} {} A{} {} 0 1 {s} {} {} A{} {} 0 1 {s} {} {} Z",
            n(cx + rx),
            n(cy),
            n(rx),
            n(ry),
            n(cx - rx),
            n(cy),
            n(rx),
            n(ry),
            n(cx + rx),
            n(cy)
        );
        self
    }

    pub fn circle(self, cx: f64, cy: f64, r: f64, hole: bool) -> P {
        self.ellipse(cx, cy, r, r, hole)
    }

    /// A ring: a disc of radius `r_out` with a hole of radius `r_in`.
    pub fn ring(self, cx: f64, cy: f64, r_out: f64, r_in: f64) -> P {
        self.circle(cx, cy, r_out, false).circle(cx, cy, r_in, true)
    }

    /// A band between radii `r_in` and `r_out` from `a0` to `a1` degrees (clockwise from 12
    /// o'clock, `a1` > `a0`); `r_in` 0 makes a pie slice.
    #[allow(clippy::too_many_arguments)]
    pub fn sector(mut self, cx: f64, cy: f64, r_in: f64, r_out: f64, a0: f64, a1: f64, hole: bool) -> P {
        let large = u8::from(a1 - a0 > 180.0);
        let (o0, o1) = (at(cx, cy, r_out, a0), at(cx, cy, r_out, a1));
        let (i0, i1) = (at(cx, cy, r_in, a0), at(cx, cy, r_in, a1));
        let (ro, ri) = (n(r_out), n(r_in));
        self.sep();
        let w = &mut self.0;
        if r_in <= 0.0 {
            if hole {
                let _ = write!(w, "M{} {} L{} {} A{ro} {ro} 0 {large} 0 {} {} Z", n(cx), n(cy), n(o1.0), n(o1.1), n(o0.0), n(o0.1));
            } else {
                let _ = write!(w, "M{} {} L{} {} A{ro} {ro} 0 {large} 1 {} {} Z", n(cx), n(cy), n(o0.0), n(o0.1), n(o1.0), n(o1.1));
            }
        } else if hole {
            let _ = write!(
                w,
                "M{} {} A{ri} {ri} 0 {large} 1 {} {} L{} {} A{ro} {ro} 0 {large} 0 {} {} Z",
                n(i0.0),
                n(i0.1),
                n(i1.0),
                n(i1.1),
                n(o1.0),
                n(o1.1),
                n(o0.0),
                n(o0.1)
            );
        } else {
            let _ = write!(
                w,
                "M{} {} A{ro} {ro} 0 {large} 1 {} {} L{} {} A{ri} {ri} 0 {large} 0 {} {} Z",
                n(o0.0),
                n(o0.1),
                n(o1.0),
                n(o1.1),
                n(i1.0),
                n(i1.1),
                n(i0.0),
                n(i0.1)
            );
        }
        self
    }

    /// A thick line from `a` to `b` with round ends of radius `r`.
    pub fn capsule(mut self, a: Pt, b: Pt, r: f64, hole: bool) -> P {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len <= 0.0 {
            return self.circle(a.0, a.1, r, hole);
        }
        let (nx, ny) = (-dy / len * r, dx / len * r);
        let (a0, a1, b0, b1) = ((a.0 - nx, a.1 - ny), (a.0 + nx, a.1 + ny), (b.0 - nx, b.1 - ny), (b.0 + nx, b.1 + ny));
        let rr = n(r);
        self.sep();
        if hole {
            let _ = write!(
                self.0,
                "M{} {} A{rr} {rr} 0 0 0 {} {} L{} {} A{rr} {rr} 0 0 0 {} {} Z",
                n(a0.0),
                n(a0.1),
                n(a1.0),
                n(a1.1),
                n(b1.0),
                n(b1.1),
                n(b0.0),
                n(b0.1)
            );
        } else {
            let _ = write!(
                self.0,
                "M{} {} L{} {} A{rr} {rr} 0 0 1 {} {} L{} {} A{rr} {rr} 0 0 1 {} {} Z",
                n(a0.0),
                n(a0.1),
                n(b0.0),
                n(b0.1),
                n(b1.0),
                n(b1.1),
                n(a1.0),
                n(a1.1),
                n(a0.0),
                n(a0.1)
            );
        }
        self
    }

    /// A thick line from `a` to `b` with square ends, `w` wide.
    pub fn bar(self, a: Pt, b: Pt, w: f64, hole: bool) -> P {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len <= 0.0 {
            return self;
        }
        let (nx, ny) = (-dy / len * w / 2.0, dx / len * w / 2.0);
        self.poly(&[(a.0 - nx, a.1 - ny), (b.0 - nx, b.1 - ny), (b.0 + nx, b.1 + ny), (a.0 + nx, a.1 + ny)], hole)
    }

    /// A smooth closed curve through `points` (Catmull-Rom splines as cubic Béziers).
    pub fn smooth(mut self, points: &[Pt], hole: bool) -> P {
        let len = points.len();
        if len < 3 {
            return self;
        }
        let mut pts = points.to_vec();
        if (signed_area(&pts) < 0.0) != hole {
            pts.reverse();
        }
        let p = |i: usize| pts[i % len];
        self.sep();
        let (x0, y0) = p(0);
        let _ = write!(self.0, "M{} {} ", n(x0), n(y0));
        for i in 0..len {
            let (a, b, c, d) = (p(i + len - 1), p(i), p(i + 1), p(i + 2));
            let c1 = (b.0 + (c.0 - a.0) / 6.0, b.1 + (c.1 - a.1) / 6.0);
            let c2 = (c.0 - (d.0 - b.0) / 6.0, c.1 - (d.1 - b.1) / 6.0);
            let _ = write!(self.0, "C{} {} {} {} {} {} ", n(c1.0), n(c1.1), n(c2.0), n(c2.1), n(c.0), n(c.1));
        }
        self.0.push('Z');
        self
    }
}
