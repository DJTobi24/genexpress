//! Lays a content tree ([`Node`]) out again at a scale `s`, measuring text for real.
//!
//! Every node is laid out into a box of its own (origin at its top-left) no wider than the width
//! it is given; parents place their children's boxes:
//!
//! - **text** takes a width that fits its container (its old width, scaled, grown or shrunk as the
//!   container changed, at most [`MAX_MEASURE_EM`] ems for body text, never narrower than its
//!   longest word), its type scales by `s` but not below the readable minimum, and its height comes
//!   from the real layout;
//! - **rigid** units scale by `s` (less if they would not fit);
//! - **sections** (bands, cards) lay their content out inside their padding and wrap it;
//! - **stacks** keep their (scaled) gaps and each child's alignment (left, centre, right);
//! - **rows** shrink their text when short of room and turn into stacks when they cannot fit;
//!   rows with large gaps (a footer with a handle left and a hint right) spread to the full width;
//! - **grids** choose how many columns fit (wide pages may get more, narrow ones fewer).

use genha_doc::{ElementKind, Page};
use genha_geom::{Frame, Rect};

use super::measure::Measurer;
use super::model::{El, Kind, Node, Region, Section, union};
use super::{Axis, Map, within};

/// Body text lines are kept to at most this many ems when a page gets wider.
const MAX_MEASURE_EM: f64 = 34.0;
/// Text and cards grow at most this much wider than plain scaling when their container widens.
const MAX_GROW: f64 = 1.6;
/// A row whose items would have to shrink below this share of their width turns into a stack.
const MIN_ROW_SQUEEZE: f64 = 0.75;
/// Rows whose gaps add up to this share of their width spread to fill their container.
const SPREAD_GAPS: f64 = 0.2;
/// Rows at least this share of their container's width (a header with a title left and a logo
/// right) spread to its full width.
const SPAN_ROW: f64 = 0.8;
/// The gaps of a spreading row shrink down to this share of its width before anything else does.
const ELASTIC_GAP: f64 = 0.04;
/// A decoration overlapping content by this share of its own area moves with that content.
const FOLLOW: f64 = 0.3;

/// Where an element goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Out {
    /// Moved by the map, sizes scaled by the factor.
    Map(Map, f64),
    /// A re-flowed text: its new (unrotated) frame and the factor its type scales by.
    Text(Frame, f64),
}

impl Out {
    pub fn shifted(self, dx: f64, dy: f64) -> Out {
        match self {
            Out::Map(m, k) => Out::Map(m.shifted(dx, dy), k),
            Out::Text(f, k) => Out::Text(f.translated(dx, dy), k),
        }
    }

    /// Where an element with old bounds `old` ends up.
    pub fn rect(&self, old: Rect) -> Rect {
        match self {
            Out::Map(m, _) => m.rect(old),
            Out::Text(f, _) => f.rect(),
        }
    }
}

/// A laid-out node: its box size and where its elements go (relative to the box).
#[derive(Clone, Debug, Default)]
pub(crate) struct Laid {
    pub w: f64,
    pub h: f64,
    pub items: Vec<(usize, Out)>,
}

impl Laid {
    pub fn add(&mut self, other: Laid, dx: f64, dy: f64) {
        self.items.extend(other.items.into_iter().map(|(i, o)| (i, o.shifted(dx, dy))));
    }
}

/// The layout pass at one scale.
pub(crate) struct Lay<'a> {
    pub page: &'a Page,
    pub els: &'a [El],
    pub m: &'a Measurer<'a>,
    /// Scale for everything.
    pub s: f64,
    /// Ratio of the new page's shorter side to the old one's (see [`type_floor`]).
    pub k: f64,
    /// Readable minimum type size on the new page.
    pub min_size: f64,
    /// The new page is relatively wider than the old one (grids may get more columns).
    pub wide: bool,
    /// The design's dominant alignment (-1 left, 0 centred, 1 right), for items that fill their
    /// container.
    pub lean: f64,
    /// The old page's width (rows spanning most of it spread across the new one).
    pub page_w: f64,
}

/// The size text of size `size` is not set below: the readable minimum `min`, or for text that
/// was already smaller, its own size (or `size × k` when larger).
pub(crate) fn type_floor(size: f64, k: f64, min: f64) -> f64 {
    min.min(size.max(size * k))
}

fn balanced(n: usize, c: usize) -> usize {
    let c = c.clamp(1, n.max(1));
    let rows = n.div_ceil(c);
    n.div_ceil(rows.max(1)).max(1)
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    v.get(v.len() / 2).copied()
}

impl Lay<'_> {
    fn el(&self, i: usize) -> Option<&El> {
        self.els.get(i)
    }

    /// How much wider than plain scaling a container became (`aw` new, `ow` old).
    fn growth(&self, aw: f64, ow: f64) -> f64 {
        if ow > 1e-9 && self.s > 0.0 { (aw / (ow * self.s)).clamp(0.05, MAX_GROW) } else { 1.0 }
    }

    /// Lays out `n` in at most `aw` (`ow`: the width the container had on the old page).
    pub fn lay(&self, n: &Node, aw: f64, ow: f64) -> Laid {
        let aw = aw.max(1.0);
        match n {
            Node::Text(i) => self.text(*i, aw, ow),
            Node::Rigid(v) => self.rigid(v, aw),
            Node::Section(s) => self.section(s, aw, ow, false),
            Node::Stack(v) => self.stack(v, aw, ow, None),
            Node::Row(v) => self.row(v, aw, ow),
            Node::Grid(v, c) => self.grid(v, *c, aw, ow),
        }
    }

    /// The type scale for a text of size `size`: `s`, but not below the readable minimum (text
    /// that was smaller than that keeps its size, or what plain scaling gives when larger).
    pub fn type_scale(&self, size: f64) -> f64 {
        let size = if size.is_finite() && size > 0.0 { size } else { return self.s };
        let floor = type_floor(size, self.k, self.min_size);
        if size * self.s < floor { floor / size } else { self.s }
    }

    fn text(&self, i: usize, aw: f64, ow: f64) -> Laid {
        let (Some(e), Some(ElementKind::Text(t))) = (self.el(i), self.page.elements.get(i).map(|e| &e.kind)) else {
            return self.rigid(&[i], aw);
        };
        if e.kind != Kind::Text {
            return self.rigid(&[i], aw);
        }
        let size0 = t.style.font_size.max(1.0);
        let mut ft = self.type_scale(size0);
        let g = self.growth(aw, ow);
        let bw0 = e.b.width().max(1.0);
        let mut w = bw0 * ft * g;
        if g > 1.0 {
            w = w.min((bw0 * ft).max(MAX_MEASURE_EM * size0 * ft));
        }
        w = w.min(aw).max(1.0);
        // Never narrower than its old lines (scaled) while the container has room for them: a
        // line that just fitted keeps fitting.
        let old_lines = e.ink.width() * ft + 1.0;
        if old_lines <= aw * 1.02 {
            w = w.max(old_lines);
        }
        let word = self.m.longest_word(i, t, ft);
        if word + 0.5 > w {
            if word + 0.5 <= aw {
                w = word + 0.5;
            } else if word > 0.0 {
                ft *= (aw - 0.5).max(1.0) / word;
                w = aw;
            }
        }
        let (ink, h) = self.m.wrapped(i, t, ft, w);
        // The box hugs the lines (they break the same way at that width), with a hair to spare.
        let bw = (ink + 1.0 + 0.004 * ink).min(w).max(1.0);
        let bh = h.ceil().max(1.0);
        Laid { w: bw, h: bh, items: vec![(i, Out::Text(Frame::new(0.0, 0.0, bw, bh), ft))] }
    }

    /// Whether a node can change width (it re-flows text).
    fn flexible(&self, n: &Node) -> bool {
        match n {
            Node::Text(i) => self.el(*i).is_some_and(|e| e.kind == Kind::Text),
            Node::Rigid(_) => false,
            Node::Section(_) => true,
            Node::Stack(v) | Node::Row(v) | Node::Grid(v, _) => v.iter().any(|c| self.flexible(c)),
        }
    }

    fn rigid(&self, v: &[usize], aw: f64) -> Laid {
        let r = union(v.iter().filter_map(|i| self.el(*i)).map(El::outer));
        let mut sc = self.s;
        if r.width() * sc > aw && r.width() > 1e-9 {
            sc = aw / r.width();
        }
        let map = Map::uniform(sc, (r.x0, r.y0), (0.0, 0.0));
        Laid { w: r.width() * sc, h: r.height() * sc, items: v.iter().map(|i| (*i, Out::Map(map, sc))).collect() }
    }

    /// A band or card; `band`: as a band across the full width (a sidebar stacked on a tall page).
    fn section(&self, sec: &Section, aw: f64, ow: f64, band: bool) -> Laid {
        let Some(a) = self.el(sec.root).map(|e| e.b) else { return Laid::default() };
        let s = self.s;
        let inner = sec.inner.flow.as_ref();
        let c = inner.map_or(a, |n| n.bounds(self.els));
        let (pl, pt) = ((c.x0 - a.x0).max(0.0) * s, (c.y0 - a.y0).max(0.0) * s);
        let (pr, pb) = ((a.x1 - c.x1).max(0.0) * s, (a.y1 - c.y1).max(0.0) * s);
        let w = if sec.span_x || band { aw } else { (a.width() * s * self.growth(aw, ow).clamp(0.75, 1.35)).min(aw) };
        let w = w.max((pl + pr + 1.0).min(aw));
        // The padding gives way before the content shrinks.
        let natural = inner.map_or(0.0, |n| n.bounds(self.els).width() * s);
        let squeeze = if pl + pr > 1e-9 && w - pl - pr < natural { ((w - natural) / (pl + pr)).clamp(0.0, 1.0) } else { 1.0 };
        let (pl, pr) = (pl * squeeze, pr * squeeze);
        let inner_aw = (w - pl - pr).max(1.0);
        let mut out = Laid { w, h: 0.0, items: vec![] };
        let h = match inner {
            Some(n) => {
                let l = self.lay(n, inner_aw, c.width());
                let ix = within(Axis::new(a.x0, a.x1, 0.0, w, s).place_lean(c.x0, c.x1, l.w, self.lean), pl.min(w - l.w).max(0.0), w - pr - l.w);
                let ih = l.h;
                out.add(l, ix, pt);
                pt + ih + pb
            }
            None => a.height() * s,
        };
        out.h = h;
        let new = Rect::new(0.0, 0.0, w, h);
        out.items.push((sec.root, Out::Map(Map::rect_to_rect(a, new), s)));
        self.extras(&sec.inner, a, new, &mut out);
        out
    }

    /// Places an area's backgrounds, bands and decorations once its content is placed: `old` is the
    /// area on the old page, `new` where it went.
    pub fn extras(&self, region: &Region, old: Rect, new: Rect, out: &mut Laid) {
        let s = self.s;
        let xa = Axis::new(old.x0, old.x1, new.x0, new.x1, s);
        let ya = Axis::new(old.y0, old.y1, new.y0, new.y1, s);
        for &i in &region.fills {
            let Some(e) = self.el(i) else { continue };
            let b = e.b;
            // Backgrounds stretch both ways; a band along the axis it spans and is anchored on the
            // other.
            let (span_x, span_y) = (xa.spans(b.x0, b.x1), ya.spans(b.y0, b.y1));
            let background = span_x == span_y;
            let (sx, tx) = if span_x || background { xa.stretch(b.x0, b.x1) } else { xa.anchor(b.x0, b.x1) };
            let (sy, ty) = if span_y || background { ya.stretch(b.y0, b.y1) } else { ya.anchor(b.y0, b.y1) };
            out.items.push((i, Out::Map(Map { sx, sy, tx, ty }, s)));
        }
        let is_decor = |j: &usize| region.decor.iter().any(|g| g.contains(j));
        for group in &region.decor {
            let b = union(group.iter().filter_map(|i| self.el(*i)).map(|e| e.b));
            // The content it overlaps most, if it overlaps enough.
            let mut best: Option<(f64, usize, Out)> = None;
            for (j, o) in &out.items {
                let Some(ej) = self.el(*j) else { continue };
                if ej.hidden || region.fills.contains(j) || is_decor(j) {
                    continue;
                }
                // Text by its lines: a box wider than its words does not catch decorations.
                let r = ej.ink.intersect(b);
                let ov = if r.width() > 0.0 && r.height() > 0.0 { r.area() } else { 0.0 };
                if best.as_ref().is_none_or(|(o0, _, _)| ov > *o0) {
                    best = Some((ov, *j, *o));
                }
            }
            let area = b.width().max(0.0) * b.height().max(0.0);
            let map = match best {
                Some((ov, j, o)) if area > 0.0 && ov >= FOLLOW * area => {
                    let Some(ej) = self.el(j) else { continue };
                    let from = ej.ink;
                    let to = o.rect(from);
                    let k = match o {
                        Out::Map(m, _) => m.sx.min(m.sy),
                        Out::Text(_, k) => k,
                    };
                    let rel = |v: f64, a: f64, len: f64| if len.abs() > 1e-9 { (v - a) / len } else { 0.5 };
                    let (cx, cy) = (b.center().x, b.center().y);
                    let nx = to.x0 + rel(cx, from.x0, from.width()) * to.width();
                    let ny = to.y0 + rel(cy, from.y0, from.height()) * to.height();
                    Map::uniform(k, (cx, cy), (nx, ny))
                }
                _ => {
                    let (sx, tx) = xa.anchor(b.x0, b.x1);
                    let (sy, ty) = ya.anchor(b.y0, b.y1);
                    // One scale for the whole group, so its pieces stay together.
                    let k = sx.min(sy);
                    Map { sx: k, sy: k, tx, ty }
                }
            };
            let k = map.sx.min(map.sy);
            for &i in group {
                out.items.push((i, Out::Map(map, k)));
            }
        }
    }

    /// Children top to bottom. With a `reference` (the container on the old page) the stack fills
    /// `aw` and children keep their alignment in the container; else it is as wide as its widest
    /// child and they keep their alignment in the stack.
    pub fn stack(&self, v: &[Node], aw: f64, ow: f64, reference: Option<Rect>) -> Laid {
        let laid: Vec<Laid> = v.iter().map(|c| self.lay(c, aw, ow)).collect();
        let bounds: Vec<Rect> = v.iter().map(|c| c.bounds(self.els)).collect();
        let rf = reference.unwrap_or_else(|| union(bounds.iter().copied()));
        let width = match reference {
            Some(_) => aw,
            None => laid.iter().map(|l| l.w).fold(0.0, f64::max).min(aw),
        };
        let xa = Axis::new(rf.x0, rf.x1, 0.0, width, self.s);
        let mut out = Laid { w: width, h: 0.0, items: vec![] };
        let mut y = 0.0;
        let n = laid.len();
        for (k, (l, b)) in laid.into_iter().zip(&bounds).enumerate() {
            let x = within(xa.place_lean(b.x0, b.x1, l.w, self.lean), 0.0, width - l.w);
            let h = l.h;
            out.add(l, x, y);
            y += h;
            if k + 1 < n
                && let Some(next) = bounds.get(k + 1)
            {
                y += (next.y0 - b.y1).max(0.0) * self.s;
            }
        }
        out.h = y;
        out
    }

    fn row(&self, v: &[Node], aw: f64, ow: f64) -> Laid {
        let s = self.s;
        let bounds: Vec<Rect> = v.iter().map(|c| c.bounds(self.els)).collect();
        let rf = union(bounds.iter().copied());
        let gaps: Vec<f64> = bounds
            .windows(2)
            .map(|w| match w {
                [a, b] => (b.x0 - a.x1).max(0.0),
                _ => 0.0,
            })
            .collect();
        // Rows with large gaps (a handle left, a hint right; a dish and its price) or spanning
        // the page spread to the full width: their gaps give way first and take what is left.
        let sidebar = v.iter().any(|c| matches!(c, Node::Section(sec) if sec.span_y));
        let spans = rf.width() >= SPAN_ROW * self.page_w && !sidebar;
        let gap_weight: f64 = gaps.iter().sum();
        let spread = rf.width() > 0.0 && (spans || gap_weight >= SPREAD_GAPS * rf.width());
        let elastic = ELASTIC_GAP * aw;
        let least: Vec<f64> = gaps.iter().map(|g| if spread { (g * s).min(elastic) } else { g * s }).collect();
        let gap_sum: f64 = least.iter().sum();
        let base: Vec<f64> = bounds.iter().map(|b| b.width() * s).collect();
        // Text (and what holds it) gives and takes room; pictures, icons and buttons keep their
        // size unless nothing else can give.
        let flexible: Vec<bool> = v.iter().map(|c| self.flexible(c)).collect();
        let fixed: f64 = base.iter().zip(&flexible).filter(|(_, f)| !**f).map(|(b, _)| *b).sum();
        let flex: f64 = base.iter().zip(&flexible).filter(|(_, f)| **f).map(|(b, _)| *b).sum();
        let room = (aw - gap_sum).max(0.0);
        let (f_flex, f_fixed) = if flex > 1e-9 {
            (((room - fixed) / flex).min(self.growth(aw, ow)), 1.0)
        } else if fixed > 1e-9 {
            ((room / fixed).min(1.0), (room / fixed).min(1.0))
        } else {
            (1.0, 1.0)
        };
        if (f_flex < MIN_ROW_SQUEEZE || f_fixed < MIN_ROW_SQUEEZE) && v.len() > 1 {
            return self.wrap(v, aw, &gaps);
        }
        let laid: Vec<Laid> = v
            .iter()
            .zip(&bounds)
            .zip(&base)
            .zip(&flexible)
            .map(|(((c, b), bw), fl)| self.lay(c, (bw * if *fl { f_flex } else { f_fixed }).max(1.0), b.width()))
            .collect();
        let used: f64 = laid.iter().map(|l| l.w).sum::<f64>() + gap_sum;
        // Text boxes may keep their old lines a hair wider than their share.
        if used > aw * 1.03 + 1.0 && v.len() > 1 {
            return self.wrap(v, aw, &gaps);
        }
        let extra = if spread { (aw - used).max(0.0) } else { 0.0 };
        let height = laid.iter().map(|l| l.h).fold(0.0, f64::max);
        let ya = Axis::new(rf.y0, rf.y1, 0.0, height, s);
        let mut out = Laid { w: if spread { aw } else { used.min(aw) }, h: height, items: vec![] };
        let mut x = 0.0;
        let n = laid.len();
        for (k, (l, b)) in laid.into_iter().zip(&bounds).enumerate() {
            let y = within(ya.place(b.y0, b.y1, l.h), 0.0, height - l.h);
            let w = l.w;
            out.add(l, x, y);
            x += w;
            if k + 1 < n {
                let g = gaps.get(k).copied().unwrap_or(0.0);
                x += least.get(k).copied().unwrap_or(0.0) + if gap_weight > 0.0 { extra * g / gap_weight } else { 0.0 };
            }
        }
        // Sidebars (sections spanning the old page's height) take the row's full height.
        for (c, b) in v.iter().zip(&bounds) {
            if let Node::Section(sec) = c
                && sec.span_y
                && let Some(pos) = out.items.iter().position(|(i, _)| *i == sec.root)
                && let Some((_, Out::Map(m, k))) = out.items.get(pos).copied()
            {
                let r = m.rect(*b);
                let grown = Rect::new(r.x0, 0.0, r.x1, height.max(r.y1));
                if let Some(slot) = out.items.get_mut(pos) {
                    slot.1 = Out::Map(Map::rect_to_rect(*b, grown), k);
                }
            }
        }
        out
    }

    /// A row that does not fit: its items stacked, centred on each other.
    fn wrap(&self, v: &[Node], aw: f64, gaps: &[f64]) -> Laid {
        let laid: Vec<Laid> = v
            .iter()
            .map(|c| {
                // Stacked, each item has the whole width: it may grow as much as its own width allows.
                let own = c.bounds(self.els).width();
                match c {
                    Node::Section(sec) if sec.span_y => self.section(sec, aw, own, true),
                    _ => self.lay(c, aw, own.max(1.0)),
                }
            })
            .collect();
        let gap = within(median(gaps.to_vec()).unwrap_or(0.0) * self.s, 0.01 * aw, 0.06 * aw);
        let width = laid.iter().map(|l| l.w).fold(0.0, f64::max);
        let mut out = Laid { w: width, h: 0.0, items: vec![] };
        let mut y = 0.0;
        let n = laid.len();
        for (k, l) in laid.into_iter().enumerate() {
            let h = l.h;
            let x = (width - l.w) / 2.0;
            out.add(l, x, y);
            y += h + if k + 1 < n { gap } else { 0.0 };
        }
        out.h = y;
        out
    }

    fn grid(&self, v: &[Node], cols0: usize, aw: f64, ow: f64) -> Laid {
        let s = self.s;
        let n = v.len();
        if n == 0 {
            return Laid::default();
        }
        let cols0 = cols0.clamp(1, n);
        let bounds: Vec<Rect> = v.iter().map(|c| c.bounds(self.els)).collect();
        // The original gaps between neighbours in a row and between rows.
        let mut gx = vec![];
        let mut gy = vec![];
        for (i, b) in bounds.iter().enumerate() {
            if (i + 1) % cols0 != 0
                && let Some(nb) = bounds.get(i + 1)
            {
                gx.push(nb.x0 - b.x1);
            }
            if let Some(nb) = bounds.get(i + cols0) {
                gy.push(nb.y0 - b.y1);
            }
        }
        let item_w = bounds.iter().map(|b| b.width()).fold(0.0, f64::max);
        let fallback = 0.06 * item_w;
        let gx0 = median(gx.clone()).map(|g| g.max(0.0));
        let gy0 = median(gy.clone()).map(|g| g.max(0.0));
        let gxs = gx0.or(gy0).unwrap_or(fallback) * s;
        let gys = gy0.or(gx0).unwrap_or(fallback) * s;
        // Cards may widen with their container, like text.
        let grow = if v.iter().any(|x| matches!(x, Node::Section(_))) { self.growth(aw, ow).clamp(0.75, 1.35) } else { 1.0 };
        let pref = (item_w * s).max(1.0);
        let fit = (1..=n).rev().find(|c| *c as f64 * pref + (*c as f64 - 1.0) * gxs <= aw + 0.5).unwrap_or(1);
        let c = if fit >= cols0 { if self.wide && cols0 < n { balanced(n, fit) } else { cols0 } } else { balanced(n, fit) };
        let cell = ((aw - (c as f64 - 1.0) * gxs) / c as f64).min(pref * grow).max(1.0);
        let laid: Vec<Laid> = v.iter().zip(&bounds).map(|(item, b)| self.lay(item, cell, b.width())).collect();
        let cw = laid.iter().map(|l| l.w).fold(0.0, f64::max);
        let width = c as f64 * cw + (c as f64 - 1.0) * gxs;
        let mut out = Laid { w: width, h: 0.0, items: vec![] };
        let mut y = 0.0;
        let rows = n.div_ceil(c);
        // Items keep their alignment in their old column (a list stays flush left).
        let columns: Vec<Rect> = (0..cols0).map(|j| union(bounds.iter().skip(j).step_by(cols0).copied())).collect();
        let mut it = laid.into_iter().zip(&bounds).enumerate();
        for r in 0..rows {
            let row: Vec<(usize, (Laid, &Rect))> = it.by_ref().take(c).collect();
            let count = row.len();
            let rh = row.iter().map(|(_, (l, _))| l.h).fold(0.0, f64::max);
            // A short last row is centred.
            let x0 = (c - count) as f64 * (cw + gxs) / 2.0;
            for (j, (k, (l, b))) in row.into_iter().enumerate() {
                let cell_x = x0 + j as f64 * (cw + gxs);
                let col = columns.get(k % cols0).copied().unwrap_or(*b);
                let x = within(Axis::new(col.x0, col.x1, cell_x, cell_x + cw, s).place_lean(b.x0, b.x1, l.w, self.lean), cell_x, cell_x + cw - l.w);
                out.add(l, x, y);
            }
            y += rh + if r + 1 < rows { gys } else { 0.0 };
        }
        out.h = y;
        out
    }
}
