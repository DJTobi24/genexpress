//! Smart guides: what a moving or resizing selection snaps to, the gaps shown around it, equal
//! spacing and matching sizes. Pure geometry on axis-aligned boxes (page pixels), so the tools
//! stay simple and every rule is unit-tested; the UI only draws what comes out.
//!
//! - Lines: the page's edges and centre, every other object's edges and centre, and any extra
//!   guides the host adds. A moving box snaps its start, centre or end; a resized box its moving
//!   edges.
//! - Sizes: a resized box snaps its width or height to another object's.
//! - Equal spacing: objects in the same row (overlapping the box vertically) or column form gaps; a
//!   box next to a neighbour snaps so its gap equals one of them, or so it sits centred between its
//!   two neighbours.
//! - Distances: the gap to the nearest object on each side, or to the page edge when there is none.

use genha_doc::{Document, Element, ElementKind, NodeId};
use genha_geom::{Point, Rect, Vec2};

/// Most other objects collected (keeps dragging fast on crowded pages).
const MAX_OTHERS: usize = 2000;
/// Most objects in one row or column compared pairwise for equal spacing.
const MAX_ROW: usize = 200;
/// Gaps closer than this (page pixels) count as equal.
const EQUAL_EPS: f64 = 0.5;
/// Lines closer than this count as touching.
const TOUCH_EPS: f64 = 1e-6;
/// Smallest side a resize snaps to.
const MIN_SIDE: f64 = 1.0;
/// Most matching-size markers shown per axis.
const MAX_SAME_SIZE: usize = 2;

/// What a measurement shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasureKind {
    /// The gap to the nearest neighbour or the page edge.
    Distance,
    /// A gap equal to another gap in the same row or column.
    EqualSpacing,
    /// A side as long as the resized box's (drawn along that side, outside the object).
    SameSize,
}

/// A measured segment in page coordinates, horizontal or vertical, with its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measure {
    pub from: Point,
    pub to: Point,
    pub value: f64,
    pub kind: MeasureKind,
}

impl Measure {
    /// The label drawn on the measure ("24", "12.5").
    pub fn label(&self) -> String {
        format_px(self.value)
    }

    /// True for a horizontal segment.
    pub fn horizontal(&self) -> bool {
        (self.to.y - self.from.y).abs() <= (self.to.x - self.from.x).abs()
    }
}

/// A length for a label: whole pixels without decimals, otherwise one decimal.
pub fn format_px(v: f64) -> String {
    if !v.is_finite() {
        return String::new();
    }
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 { format!("{r:.0}") } else { format!("{r:.1}") }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Axis {
    /// The box's extent along the axis.
    fn along(self, r: &Rect) -> (f64, f64) {
        match self {
            Axis::X => (r.x0, r.x1),
            Axis::Y => (r.y0, r.y1),
        }
    }

    /// The box's extent across the axis.
    fn across(self, r: &Rect) -> (f64, f64) {
        match self {
            Axis::X => (r.y0, r.y1),
            Axis::Y => (r.x0, r.x1),
        }
    }

    fn point(self, along: f64, across: f64) -> Point {
        match self {
            Axis::X => Point::new(along, across),
            Axis::Y => Point::new(across, along),
        }
    }

    /// `r` with its extent along the axis replaced.
    fn with_along(self, r: &Rect, lo: f64, hi: f64) -> Rect {
        match self {
            Axis::X => Rect::new(lo, r.y0, hi, r.y1),
            Axis::Y => Rect::new(r.x0, lo, r.x1, hi),
        }
    }
}

/// Whether two spans overlap (touching does not count).
fn overlaps(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// The point across the axis where a measure between `a` and `b` is drawn: the middle of their
/// shared extent, or halfway between their middles when they share none.
fn across_mid(axis: Axis, a: &Rect, b: &Rect) -> f64 {
    let (a0, a1) = axis.across(a);
    let (b0, b1) = axis.across(b);
    let (lo, hi) = (a0.max(b0), a1.min(b1));
    if lo <= hi { (lo + hi) / 2.0 } else { (a0 + a1 + b0 + b1) / 4.0 }
}

fn measure(axis: Axis, from: f64, to: f64, across: f64, kind: MeasureKind) -> Measure {
    Measure { from: axis.point(from, across), to: axis.point(to, across), value: to - from, kind }
}

/// Which sides of a box a resize moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sides {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Sides {
    fn along(self, axis: Axis) -> (bool, bool) {
        match axis {
            Axis::X => (self.left, self.right),
            Axis::Y => (self.top, self.bottom),
        }
    }

    /// A corner moves both axes.
    pub fn corner(self) -> bool {
        (self.left || self.right) && (self.top || self.bottom)
    }
}

/// The neighbours of a box in its row (x) or column (y), and the gaps between the other objects
/// there.
struct Row {
    /// The nearest object before the box (ending at or before its start).
    before: Option<Rect>,
    /// The nearest object after the box (starting at or after its end).
    after: Option<Rect>,
    /// (earlier, later, gap) for neighbouring objects of the row.
    gaps: Vec<(Rect, Rect, f64)>,
}

/// What a selection snaps to and is measured against, collected when a gesture starts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    /// The page's box (0, 0, width, height).
    pub page: Rect,
    /// Axis-aligned bounds of the other visible objects.
    pub others: Vec<Rect>,
    /// Vertical snap lines (x), sorted.
    pub xs: Vec<f64>,
    /// Horizontal snap lines (y), sorted.
    pub ys: Vec<f64>,
}

fn contains_any(el: &Element, exclude: &[NodeId]) -> bool {
    exclude.contains(&el.id)
        || match &el.kind {
            ElementKind::Group(g) => g.children.iter().any(|c| contains_any(c, exclude)),
            _ => false,
        }
}

fn finite(r: &Rect) -> bool {
    [r.x0, r.y0, r.x1, r.y1].iter().all(|v| v.is_finite())
}

impl Scene {
    /// The page `page` of `doc` without the objects in `exclude` (a group holding one of them is
    /// left out too), plus extra guide lines from the host.
    pub fn collect(doc: &Document, page: usize, exclude: &[NodeId], extra_xs: &[f64], extra_ys: &[f64]) -> Scene {
        let Some(pg) = doc.pages.get(page) else { return Scene::default() };
        let others: Vec<Rect> = pg
            .elements
            .iter()
            .filter(|e| !e.hidden && !contains_any(e, exclude))
            .map(|e| e.frame.sanitized().bounds())
            .filter(finite)
            .take(MAX_OTHERS)
            .collect();
        Scene::new(Rect::new(0.0, 0.0, pg.width, pg.height), others, extra_xs, extra_ys)
    }

    /// A scene from a page box, the other objects' bounds and extra guide lines.
    pub fn new(page: Rect, others: Vec<Rect>, extra_xs: &[f64], extra_ys: &[f64]) -> Scene {
        let page = if finite(&page) { page.abs() } else { Rect::ZERO };
        let mut xs = vec![page.x0, page.center().x, page.x1];
        let mut ys = vec![page.y0, page.center().y, page.y1];
        for r in &others {
            xs.extend([r.x0, r.center().x, r.x1]);
            ys.extend([r.y0, r.center().y, r.y1]);
        }
        xs.extend(extra_xs.iter().take(MAX_OTHERS));
        ys.extend(extra_ys.iter().take(MAX_OTHERS));
        let tidy = |v: &mut Vec<f64>| {
            v.retain(|x| x.is_finite());
            v.sort_by(f64::total_cmp);
            v.dedup_by(|a, b| (*a - *b).abs() < TOUCH_EPS);
        };
        tidy(&mut xs);
        tidy(&mut ys);
        Scene { page, others, xs, ys }
    }

    fn lines(&self, axis: Axis) -> &[f64] {
        match axis {
            Axis::X => &self.xs,
            Axis::Y => &self.ys,
        }
    }

    /// The row (x) or column (y) of `b`: objects overlapping it across the axis. `slack` lets a
    /// neighbour overlap the box a little (it is about to snap clear).
    fn row(&self, axis: Axis, b: &Rect, slack: f64) -> Row {
        let (lo, hi) = axis.along(b);
        let members: Vec<&Rect> = self.others.iter().filter(|o| overlaps(axis.across(o), axis.across(b))).take(MAX_ROW).collect();
        let mut before: Option<&Rect> = None;
        let mut after: Option<&Rect> = None;
        for o in &members {
            let (olo, ohi) = axis.along(o);
            if ohi <= lo + slack && olo < lo && before.is_none_or(|p| ohi > axis.along(p).1) {
                before = Some(o);
            }
            if olo >= hi - slack && ohi > hi && after.is_none_or(|p| olo < axis.along(p).0) {
                after = Some(o);
            }
        }
        let mut gaps = vec![];
        for a in &members {
            let a_hi = axis.along(a).1;
            // The nearest object starting after `a` ends, sharing extent with it across the axis.
            let next = members
                .iter()
                .filter(|c| axis.along(c).0 >= a_hi && overlaps(axis.across(a), axis.across(c)))
                .min_by(|p, q| axis.along(p).0.total_cmp(&axis.along(q).0));
            if let Some(c) = next {
                let g = axis.along(c).0 - a_hi;
                if g > EQUAL_EPS {
                    gaps.push((**a, **c, g));
                }
            }
        }
        Row { before: before.copied(), after: after.copied(), gaps }
    }

    /// The shift along `axis` that snaps a box being moved, 0 when nothing is within `threshold`.
    fn move_shift(&self, axis: Axis, b: &Rect, threshold: f64) -> f64 {
        let (lo, hi) = axis.along(b);
        let mut best: Option<f64> = None;
        let mut consider = |shift: f64| {
            if shift.is_finite() && shift.abs() <= threshold && best.is_none_or(|s| shift.abs() < s.abs()) {
                best = Some(shift);
            }
        };
        for e in [lo, (lo + hi) / 2.0, hi] {
            if let (_, Some(t)) = genha_geom::snap(e, self.lines(axis), threshold) {
                consider(t - e);
            }
        }
        let row = self.row(axis, b, threshold);
        for (_, _, g) in &row.gaps {
            if let Some(p) = &row.before {
                consider(axis.along(p).1 + g - lo);
            }
            if let Some(n) = &row.after {
                consider(axis.along(n).0 - g - hi);
            }
        }
        if let (Some(p), Some(n)) = (&row.before, &row.after) {
            let free = axis.along(n).0 - axis.along(p).1 - (hi - lo);
            if free > 0.0 {
                consider(axis.along(p).1 + free / 2.0 - lo);
            }
        }
        best.unwrap_or(0.0)
    }

    /// The shift that snaps a box being moved to the lines and equal gaps within `threshold`.
    pub fn snap_move(&self, b: Rect, threshold: f64) -> Vec2 {
        if !finite(&b) || !threshold.is_finite() || threshold < 0.0 {
            return Vec2::ZERO;
        }
        Vec2::new(self.move_shift(Axis::X, &b, threshold), self.move_shift(Axis::Y, &b, threshold))
    }

    /// The best new extent along `axis` for a box being resized, as (displacement, lo, hi).
    fn resize_axis(&self, axis: Axis, r: &Rect, sides: Sides, from_center: bool, threshold: f64) -> Option<(f64, f64, f64)> {
        let (lo_moves, hi_moves) = sides.along(axis);
        if !lo_moves && !hi_moves {
            return None;
        }
        let (lo, hi) = axis.along(r);
        let mut best: Option<(f64, f64, f64)> = None;
        let mut consider = |nlo: f64, nhi: f64| {
            if !nlo.is_finite() || !nhi.is_finite() || nhi - nlo < MIN_SIDE {
                return;
            }
            let disp = (nlo - lo).abs().max((nhi - hi).abs());
            if disp <= threshold && best.is_none_or(|b| disp < b.0) {
                best = Some((disp, nlo, nhi));
            }
        };
        let lines = self.lines(axis);
        let sizes = self.others.iter().map(|o| {
            let (a, b) = axis.along(o);
            b - a
        });
        if from_center {
            let c = (lo + hi) / 2.0;
            if let (_, Some(t)) = genha_geom::snap(hi, lines, threshold) {
                consider(2.0 * c - t, t);
            }
            if let (_, Some(t)) = genha_geom::snap(lo, lines, threshold) {
                consider(t, 2.0 * c - t);
            }
            for len in sizes {
                consider(c - len / 2.0, c + len / 2.0);
            }
            return best;
        }
        let row = self.row(axis, r, threshold);
        if hi_moves {
            if let (_, Some(t)) = genha_geom::snap(hi, lines, threshold) {
                consider(lo, t);
            }
            if let Some(n) = &row.after {
                for (_, _, g) in &row.gaps {
                    consider(lo, axis.along(n).0 - g);
                }
            }
            for len in sizes {
                consider(lo, lo + len);
            }
        } else {
            if let (_, Some(t)) = genha_geom::snap(lo, lines, threshold) {
                consider(t, hi);
            }
            if let Some(p) = &row.before {
                for (_, _, g) in &row.gaps {
                    consider(axis.along(p).1 + g, hi);
                }
            }
            for len in sizes {
                consider(hi - len, hi);
            }
        }
        best
    }

    /// Snaps the moving sides of a box being resized (`r`, already following the pointer) to lines,
    /// equal gaps and other objects' sizes within `threshold`. `from_center`: both sides of an axis
    /// move together about the centre. `ratio` (width / height): keep proportions at a corner; the
    /// axis that snaps closest wins and the other follows.
    pub fn snap_resize(&self, r: Rect, sides: Sides, from_center: bool, ratio: Option<f64>, threshold: f64) -> Rect {
        if !finite(&r) || !threshold.is_finite() || threshold < 0.0 {
            return r;
        }
        let sx = self.resize_axis(Axis::X, &r, sides, from_center, threshold);
        let sy = self.resize_axis(Axis::Y, &r, sides, from_center, threshold);
        match ratio.filter(|q| q.is_finite() && *q > 0.0 && sides.corner()) {
            Some(q) => {
                let (axis, span, len_other) = match (sx, sy) {
                    (Some(x), Some(y)) if y.0 < x.0 => (Axis::Y, y, (y.2 - y.1) * q),
                    (Some(x), _) => (Axis::X, x, (x.2 - x.1) / q),
                    (None, Some(y)) => (Axis::Y, y, (y.2 - y.1) * q),
                    (None, None) => return r,
                };
                let other = if axis == Axis::X { Axis::Y } else { Axis::X };
                let snapped = axis.with_along(&r, span.1, span.2);
                let (olo, ohi) = other.along(&r);
                let (lo_moves, _) = sides.along(other);
                let (nlo, nhi) = if from_center {
                    let c = (olo + ohi) / 2.0;
                    (c - len_other / 2.0, c + len_other / 2.0)
                } else if lo_moves {
                    (ohi - len_other, ohi)
                } else {
                    (olo, olo + len_other)
                };
                other.with_along(&snapped, nlo, nhi)
            }
            None => {
                let mut out = r;
                if let Some((_, lo, hi)) = sx {
                    out = Axis::X.with_along(&out, lo, hi);
                }
                if let Some((_, lo, hi)) = sy {
                    out = Axis::Y.with_along(&out, lo, hi);
                }
                out
            }
        }
    }

    /// The snap lines touching the box's start, centre or end (`all`), or only the sides in
    /// `sides` (a resize), as (xs, ys).
    pub fn touching(&self, b: Rect, sides: Option<Sides>) -> (Vec<f64>, Vec<f64>) {
        let pick = |axis: Axis| -> Vec<f64> {
            let (lo, hi) = axis.along(&b);
            let edges: Vec<f64> = match sides {
                None => vec![lo, (lo + hi) / 2.0, hi],
                Some(s) => {
                    let (l, h) = s.along(axis);
                    let mut v = vec![];
                    if l {
                        v.push(lo);
                    }
                    if h {
                        v.push(hi);
                    }
                    v
                }
            };
            self.lines(axis).iter().copied().filter(|g| edges.iter().any(|e| (g - e).abs() < TOUCH_EPS.max(e.abs() * 1e-12))).collect()
        };
        (pick(Axis::X), pick(Axis::Y))
    }

    /// Equal-spacing markers for `b`: its gap to a neighbour equal to a gap between two other
    /// objects of its row or column (each such gap marked), or equal gaps on both sides.
    pub fn equal_spacing(&self, b: Rect) -> Vec<Measure> {
        let mut out = vec![];
        if !finite(&b) {
            return out;
        }
        for axis in [Axis::X, Axis::Y] {
            let (lo, hi) = axis.along(&b);
            let row = self.row(axis, &b, 0.0);
            let gb = row.before.as_ref().map(|p| (p, lo - axis.along(p).1));
            let ga = row.after.as_ref().map(|n| (n, axis.along(n).0 - hi));
            let mut marks: Vec<Measure> = vec![];
            let mut push = |m: Measure| {
                if !marks.iter().any(|o| o.from == m.from && o.to == m.to) {
                    marks.push(m);
                }
            };
            let both_equal = matches!((gb, ga), (Some((_, x)), Some((_, y))) if x > EQUAL_EPS && (x - y).abs() < EQUAL_EPS);
            for (o, gap) in [gb, ga].into_iter().flatten() {
                if gap <= EQUAL_EPS {
                    continue;
                }
                let equal: Vec<&(Rect, Rect, f64)> = row.gaps.iter().filter(|(_, _, g)| (g - gap).abs() < EQUAL_EPS).collect();
                if equal.is_empty() && !both_equal {
                    continue;
                }
                let m = if axis.along(o).1 <= lo {
                    measure(axis, axis.along(o).1, lo, across_mid(axis, o, &b), MeasureKind::EqualSpacing)
                } else {
                    measure(axis, hi, axis.along(o).0, across_mid(axis, &b, o), MeasureKind::EqualSpacing)
                };
                push(m);
                for (p, n, _) in equal {
                    push(measure(axis, axis.along(p).1, axis.along(n).0, across_mid(axis, p, n), MeasureKind::EqualSpacing));
                }
            }
            out.extend(marks);
        }
        out
    }

    /// The gaps from `b` to its nearest neighbour on each side, or to the page edge where there is
    /// none (only when the box is inside the page on that side).
    pub fn distances(&self, b: Rect) -> Vec<Measure> {
        let mut out = vec![];
        if !finite(&b) {
            return out;
        }
        for axis in [Axis::X, Axis::Y] {
            let (lo, hi) = axis.along(&b);
            let (plo, phi) = axis.along(&self.page);
            let (alo, ahi) = axis.across(&b);
            let mid = (alo + ahi) / 2.0;
            let row = self.row(axis, &b, 0.0);
            match &row.before {
                Some(p) => out.push(measure(axis, axis.along(p).1, lo, across_mid(axis, p, &b), MeasureKind::Distance)),
                None if lo > plo && hi <= phi => out.push(measure(axis, plo, lo, mid, MeasureKind::Distance)),
                None => {}
            }
            match &row.after {
                Some(n) => out.push(measure(axis, hi, axis.along(n).0, across_mid(axis, &b, n), MeasureKind::Distance)),
                None if hi < phi && lo >= plo => out.push(measure(axis, hi, phi, mid, MeasureKind::Distance)),
                None => {}
            }
        }
        out.retain(|m| m.value > 1e-6);
        out
    }

    /// Markers on other objects as wide (or as tall) as `b`, for the axes a resize changes; the
    /// box's own side is marked too.
    pub fn same_size(&self, b: Rect, sides: Sides) -> Vec<Measure> {
        let mut out = vec![];
        if !finite(&b) {
            return out;
        }
        for axis in [Axis::X, Axis::Y] {
            let (l, h) = sides.along(axis);
            if !l && !h {
                continue;
            }
            let (lo, hi) = axis.along(&b);
            let len = hi - lo;
            let c = b.center();
            let mut same: Vec<&Rect> = self
                .others
                .iter()
                .filter(|o| {
                    let (a, z) = axis.along(o);
                    ((z - a) - len).abs() < TOUCH_EPS.max(len.abs() * 1e-9)
                })
                .collect();
            if same.is_empty() {
                continue;
            }
            same.sort_by(|p, q| (p.center() - c).hypot().total_cmp(&(q.center() - c).hypot()));
            // Drawn along the top edge (widths) or the left edge (heights).
            let side = |r: &Rect| {
                let (a, z) = axis.along(r);
                measure(axis, a, z, axis.across(r).0, MeasureKind::SameSize)
            };
            out.push(side(&b));
            out.extend(same.into_iter().take(MAX_SAME_SIZE).map(side));
        }
        out
    }

    /// All hints for a box being moved or resized: equal spacing (when snapping) and distances
    /// (always; a gap already marked as equal spacing is not repeated).
    pub fn hints(&self, b: Rect, snapping: bool) -> Vec<Measure> {
        let mut out = if snapping { self.equal_spacing(b) } else { vec![] };
        for d in self.distances(b) {
            if !out.iter().any(|m| m.from == d.from && m.to == d.to) {
                out.push(d);
            }
        }
        out
    }
}

/// Distances from box `a` to box `t`: the gap on each axis where they are apart, or the gaps
/// between their matching edges where one spans the other (Alt-hover).
pub fn measure_between(a: Rect, t: Rect) -> Vec<Measure> {
    let mut out = vec![];
    if !finite(&a) || !finite(&t) {
        return out;
    }
    for axis in [Axis::X, Axis::Y] {
        let (alo, ahi) = axis.along(&a);
        let (tlo, thi) = axis.along(&t);
        let across = across_mid(axis, &a, &t);
        if ahi <= tlo {
            out.push(measure(axis, ahi, tlo, across, MeasureKind::Distance));
        } else if thi <= alo {
            out.push(measure(axis, thi, alo, across, MeasureKind::Distance));
        } else {
            let mid = {
                let (c0, c1) = axis.across(&a);
                (c0 + c1) / 2.0
            };
            out.push(measure(axis, tlo.min(alo), tlo.max(alo), mid, MeasureKind::Distance));
            out.push(measure(axis, thi.min(ahi), thi.max(ahi), mid, MeasureKind::Distance));
        }
    }
    out.retain(|m| m.value > 1e-6);
    out
}

/// Alt-hover: with a selection on page `page`, the distances from its bounds to the object under
/// `p` (or to the page when there is none, or when the pointer is over the selection itself).
pub fn hover_measures(doc: &Document, page: usize, selection: &[NodeId], p: Point) -> Vec<Measure> {
    let Some(pg) = doc.pages.get(page) else { return vec![] };
    if !p.x.is_finite() || !p.y.is_finite() {
        return vec![];
    }
    let Some(sel) = selection.iter().filter_map(|id| pg.find(*id)).map(|e| e.frame.sanitized().bounds()).reduce(|a, b| a.union(b)) else {
        return vec![];
    };
    let target = match crate::hit_test(pg, p) {
        Some(e) if !selection.contains(&e.id) => e.frame.sanitized().bounds(),
        _ => Rect::new(0.0, 0.0, pg.width, pg.height),
    };
    measure_between(sel, target)
}
