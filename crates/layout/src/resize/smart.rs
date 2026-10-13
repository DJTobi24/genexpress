//! The smart re-layout of a whole page: find the structure, pick the largest scale (and, on a
//! page much wider than the design, a split into columns) at which the content fits, place it
//! with the page's whitespace shared out as in the original, then place backgrounds, bands and
//! decorations around it.

use genha_doc::Page;
use genha_geom::{Frame, Rect};
use genha_text::FontBook;

use super::flow::{Laid, Lay, Out};
use super::measure::Measurer;
use super::model::{self, El, Node, Region};
use super::{Axis, Map, apply, scale_sizes, settle, within};

/// Margins are at least this share of the new page's shorter side (when the design had one).
const MIN_MARGIN: f64 = 0.03;
/// Margins are at most this share of the page side.
const MAX_MARGIN: f64 = 0.25;
/// Gap between columns when a design is set in columns on a wide page, share of the width.
const COLUMN_GAP: f64 = 0.05;
/// Steps of the search for the largest scale that fits.
const STEPS: usize = 14;
/// Columns are only chosen when they allow type this much larger than one column.
const COLUMNS_GAIN: f64 = 1.12;
/// The readable minimum, as shares of it tried in turn until the content fits.
const FLOOR_STEPS: [f64; 4] = [1.0, 0.8, 0.6, 0.0];
/// The shortest column holds at least this share of the tallest one's height.
const COLUMN_BALANCE: f64 = 0.3;
/// A band becomes a panel of its own column only when at least this share of the old page high.
const PANEL_MIN: f64 = 0.2;
/// An element within this share of the page side from an edge touches it.
const EDGE: f64 = 0.01;

/// Re-lays out `page` (already given its new size; elements still where they were on the old
/// `ow` × `oh` page).
pub(crate) fn relayout(page: &mut Page, ow: f64, oh: f64, fonts: &FontBook) {
    let src = page.clone();
    let (nw, nh) = (page.width, page.height);
    let m = Measurer::new(fonts);
    let els = model::elements(&src, &m);
    let ids: Vec<usize> = (0..els.len()).collect();
    let old_page = Rect::new(0.0, 0.0, ow, oh);
    let new_page = Rect::new(0.0, 0.0, nw, nh);
    let region = model::region(&els, &ids, old_page, 0);
    let k = nw.min(nh) / ow.min(oh);
    let fit = (nw / ow).min(nh / oh);
    let wide = (nw / nh) / (ow / oh) >= 1.3;
    let min_size = settle::min_text_size(nw, nh);
    let lean = lean(&src, &region);
    let top = Top { els: &els, ow, oh, nw, nh, k, min_margin: MIN_MARGIN * nw.min(nh) };
    // The readable minimum is relaxed step by step when the content cannot fit with it (a dense
    // document on a smaller page); the settle pass reports text set below it.
    let mut chosen: Option<(f64, f64, Laid)> = None;
    for relax in FLOOR_STEPS {
        let floor = min_size * relax;
        let lay = |s: f64| Lay { page: &src, els: &els, m: &m, s, k, min_size: floor, wide, lean, page_w: ow };
        let (s, laid, over) = match &region.flow {
            None => (fit, Laid { w: nw, h: nh, items: vec![] }, 0.0),
            Some(flow) => {
                let n = match flow {
                    Node::Stack(v) => v.len(),
                    _ => 1,
                };
                let mut options = vec![1];
                if n >= 2 && nw / nh >= 1.2 && wide {
                    options.push(2);
                    if n >= 3 && nw / nh >= 2.2 {
                        options.push(3);
                    }
                }
                let mut best: Option<(f64, Laid, f64)> = None;
                for cols in options {
                    let (s, l, over) = top.search(&lay, flow, cols, fit);
                    let better = match &best {
                        None => true,
                        Some((bs, _, bo)) => {
                            if over > 0.5 || *bo > 0.5 {
                                over < *bo
                            } else {
                                s > *bs * COLUMNS_GAIN
                            }
                        }
                    };
                    if better {
                        best = Some((s, l, over));
                    }
                }
                best.unwrap_or((fit, Laid::default(), 0.0))
            }
        };
        let done = over <= 0.5;
        chosen = Some((s, floor, laid));
        if done {
            break;
        }
    }
    let Some((s, floor, mut laid)) = chosen else { return };
    let pass = Lay { page: &src, els: &els, m: &m, s, k, min_size: floor, wide, lean, page_w: ow };
    if let Some(flow) = &region.flow {
        top.bleed(flow, &region, &mut laid);
    }
    pass.extras(&region, old_page, new_page, &mut laid);
    avoid_text(&els, &region, &mut laid, nw, nh);
    // Apply.
    let mut outs: Vec<Option<Out>> = vec![None; page.elements.len()];
    for (i, o) in laid.items {
        if let Some(slot) = outs.get_mut(i)
            && slot.is_none()
        {
            *slot = Some(o);
        }
    }
    let (xa, ya) = (Axis::new(0.0, ow, 0.0, nw, s), Axis::new(0.0, oh, 0.0, nh, s));
    for (i, (el, out)) in page.elements.iter_mut().zip(outs).enumerate() {
        match out {
            Some(Out::Map(map, k)) => apply(el, &map, k),
            Some(Out::Text(f, k)) => {
                scale_sizes(el, k);
                el.frame = Frame { rotation: el.frame.rotation, ..f };
            }
            None => {
                // Not placed (should not happen): anchored like a decoration.
                let b = els.get(i).map_or(el.frame.bounds(), |e| e.b);
                let (sx, tx) = xa.anchor(b.x0, b.x1);
                let (sy, ty) = ya.anchor(b.y0, b.y1);
                apply(el, &Map { sx, sy, tx, ty }, s);
            }
        }
    }
}

/// The page-level placement.
struct Top<'a> {
    els: &'a [El],
    ow: f64,
    oh: f64,
    nw: f64,
    nh: f64,
    k: f64,
    min_margin: f64,
}

impl Top<'_> {
    /// The new margin for an old one of `m` on a side of `side` pixels.
    fn margin(&self, m: f64, s: f64, side: f64) -> f64 {
        if m <= 0.5 {
            return 0.0;
        }
        (m * s).max((m * self.k).min(self.min_margin)).min(MAX_MARGIN * side)
    }

    /// How far the content outside bands and sidebars keeps from the page edges and from those
    /// bands and sidebars: left, top, right, bottom (None when everything is in a band).
    fn inset(&self, flow: &Node) -> [Option<f64>; 4] {
        let mut leaves = vec![];
        let mut walls = vec![];
        inset_parts(flow, self.els, &mut leaves, &mut walls);
        let mut m: [Option<f64>; 4] = [None; 4];
        let mut keep = |k: usize, v: f64| {
            if let Some(slot) = m.get_mut(k) {
                *slot = Some(slot.map_or(v.max(0.0), |o: f64| o.min(v.max(0.0))));
            }
        };
        for b in leaves {
            let left = walls.iter().filter(|(r, _, down)| *down && r.x1 <= b.x0 + 1.0).map(|(r, _, _)| r.x1).fold(0.0, f64::max);
            let right = walls.iter().filter(|(r, _, down)| *down && r.x0 >= b.x1 - 1.0).map(|(r, _, _)| r.x0).fold(self.ow, f64::min);
            let top = walls.iter().filter(|(r, across, _)| *across && r.y1 <= b.y0 + 1.0).map(|(r, _, _)| r.y1).fold(0.0, f64::max);
            let bottom = walls.iter().filter(|(r, across, _)| *across && r.y0 >= b.y1 - 1.0).map(|(r, _, _)| r.y0).fold(self.oh, f64::min);
            keep(0, b.x0 - left);
            keep(1, b.y0 - top);
            keep(2, right - b.x1);
            keep(3, bottom - b.y1);
        }
        m
    }

    /// The largest scale (up to `k`) at which the content fits in `cols` columns: (scale, layout,
    /// overflow in pixels; ≤ 0.5 when it fits).
    fn search<'a>(&self, lay: &dyn Fn(f64) -> Lay<'a>, flow: &Node, cols: usize, fit: f64) -> (f64, Laid, f64) {
        let hi = self.k;
        let (l, over) = self.place(&lay(hi), flow, cols);
        if over <= 0.5 {
            return (hi, l, over);
        }
        let lo = (0.2 * self.k).min(0.5 * fit).max(1e-3);
        let (l_lo, over_lo) = self.place(&lay(lo), flow, cols);
        if over_lo > 0.5 {
            return (lo, l_lo, over_lo);
        }
        let (mut a, mut b) = (lo, hi);
        let mut found = (lo, l_lo, over_lo);
        for _ in 0..STEPS {
            let mid = (a * b).sqrt();
            let (l, over) = self.place(&lay(mid), flow, cols);
            if over <= 0.5 {
                a = mid;
                found = (mid, l, over);
            } else {
                b = mid;
            }
        }
        found
    }

    /// The content laid out on the new page: (layout in page coordinates, overflow in pixels).
    fn place(&self, lay: &Lay<'_>, flow: &Node, cols: usize) -> (Laid, f64) {
        let s = lay.s;
        let c = flow.bounds(self.els);
        // The margins text and pictures keep from the page edges and from bands and sidebars
        // (bands and sidebars themselves reach the edges again afterwards).
        let [il, it, ir, ib] = self.inset(flow);
        let ml = self.margin(il.unwrap_or(c.x0), s, self.nw);
        let mt = self.margin(it.unwrap_or(c.y0), s, self.nh);
        let mr = self.margin(ir.unwrap_or(self.ow - c.x1), s, self.nw);
        let mb = self.margin(ib.unwrap_or(self.oh - c.y1), s, self.nh);
        let aw = (self.nw - ml - mr).max(1.0);
        let ah = (self.nh - mt - mb).max(1.0);
        match flow {
            Node::Stack(v) if cols <= 1 => self.stack(lay, v, c, [ml, mt, mr, mb]),
            Node::Stack(v) => self.columns(lay, v, cols, c, [ml, mt, mr, mb]),
            _ => {
                let l = lay.lay(flow, aw, self.ow);
                // Content filling most of the page height is centred on the new one.
                let y =
                    if c.height() >= 0.7 * self.oh { (self.nh - l.h) / 2.0 } else { Axis::new(0.0, self.oh, 0.0, self.nh, s).place(c.y0, c.y1, l.h) };
                let y = if l.h <= ah { within(y, mt, self.nh - mb - l.h) } else { mt };
                let x = self.x_on_page(c, l.w, c, [ml, mr], s, lay.lean);
                let over = (l.h - ah).max(0.0).max(l.w - aw);
                let mut out = Laid { w: self.nw, h: self.nh, items: vec![] };
                out.add(l, x, y);
                (out, over)
            }
        }
    }

    /// One column: the whitespace of the new page is shared between the margins and gaps in
    /// proportion to the old ones, so the design keeps its rhythm (a title near the top, a footer
    /// at the bottom).
    fn stack(&self, lay: &Lay<'_>, v: &[Node], c: Rect, [ml, mt, mr, mb]: [f64; 4]) -> (Laid, f64) {
        let s = lay.s;
        let aw = (self.nw - ml - mr).max(1.0);
        let laid: Vec<Laid> = v.iter().map(|ch| lay.lay(ch, aw, self.ow)).collect();
        let bounds: Vec<Rect> = v.iter().map(|ch| ch.bounds(self.els)).collect();
        let mut orig = vec![c.y0.max(0.0)];
        let mut least = vec![mt];
        for w in bounds.windows(2) {
            if let [a, b] = w {
                let g = (b.y0 - a.y1).max(0.0);
                orig.push(g);
                least.push(g * s);
            }
        }
        orig.push((self.oh - c.y1).max(0.0));
        least.push(mb);
        let content: f64 = laid.iter().map(|l| l.h).sum();
        let free = self.nh - content;
        let need: f64 = least.iter().sum();
        let weight: f64 = orig.iter().sum();
        let last = orig.len() - 1;
        let slots: Vec<f64> = if free >= need {
            let extra = free - need;
            least
                .iter()
                .zip(&orig)
                .enumerate()
                .map(|(i, (l, o))| {
                    let share = if weight > 1e-9 {
                        o / weight
                    } else if i == 0 || i == last {
                        0.5
                    } else {
                        0.0
                    };
                    l + extra * share
                })
                .collect()
        } else {
            let f = if need > 1e-9 { (free / need).max(0.0) } else { 0.0 };
            least.iter().map(|l| l * f).collect()
        };
        let mut out = Laid { w: self.nw, h: self.nh, items: vec![] };
        let mut y = slots.first().copied().unwrap_or(0.0);
        let mut over = need - free;
        for (i, (l, b)) in laid.into_iter().zip(&bounds).enumerate() {
            let x = self.x_on_page(*b, l.w, c, [ml, mr], s, lay.lean);
            over = over.max(l.w - self.nw);
            let h = l.h;
            out.add(l, x, y);
            y += h + slots.get(i + 1).copied().unwrap_or(0.0);
        }
        (out, over)
    }

    /// Where an item that was at `b` (in content that spanned `c`) and is now `w` wide goes across
    /// the page: at its scaled distance from the nearest page edge or the centre; items that span
    /// the whole content width (as close to one side as to the other) go by the design's lean.
    fn x_on_page(&self, b: Rect, w: f64, c: Rect, [ml, mr]: [f64; 2], s: f64, lean: f64) -> f64 {
        let tie = 0.01 * self.ow;
        let spans = (b.x0 - c.x0).abs() <= tie && (c.x1 - b.x1).abs() <= tie;
        let x = if spans && lean < 0.0 {
            ml + (b.x0 - c.x0) * s
        } else if spans && lean > 0.0 {
            self.nw - mr - (c.x1 - b.x1) * s - w
        } else {
            Axis::new(0.0, self.ow, 0.0, self.nw, s).place(b.x0, b.x1, w)
        };
        if w <= self.nw - ml - mr { within(x, ml, self.nw - mr - w) } else { within(x, 0.0, self.nw - w) }
    }

    /// The stack split into `cols` columns side by side (a tall design on a wide page), each
    /// centred vertically.
    fn columns(&self, lay: &Lay<'_>, v: &[Node], cols: usize, c: Rect, [ml, mt, mr, mb]: [f64; 4]) -> (Laid, f64) {
        let s = lay.s;
        let n = v.len();
        let gap = COLUMN_GAP * self.nw;
        let aw = (self.nw - ml - mr).max(1.0);
        let cw = ((aw - (cols as f64 - 1.0) * gap) / cols as f64).max(1.0);
        let ah = (self.nh - mt - mb).max(1.0);
        let laid: Vec<Laid> = v.iter().map(|ch| lay.lay(ch, cw, self.ow)).collect();
        let bounds: Vec<Rect> = v.iter().map(|ch| ch.bounds(self.els)).collect();
        let gaps: Vec<f64> = bounds
            .windows(2)
            .map(|w| match w {
                [a, b] => (b.y0 - a.y1).max(0.0) * s,
                _ => 0.0,
            })
            .collect();
        let hs: Vec<f64> = laid.iter().map(|l| l.h).collect();
        let height = |a: usize, b: usize| -> f64 {
            let hs: f64 = hs.get(a..b).unwrap_or(&[]).iter().sum();
            let gs: f64 = gaps.get(a..b.saturating_sub(1)).unwrap_or(&[]).iter().sum();
            hs + gs
        };
        // The split into contiguous columns: among those that fit, the one cutting at the widest
        // gaps (a kicker stays with its headline); else the one with the shortest tallest column.
        let room = ah;
        let mut best: Option<(bool, f64, f64, Vec<usize>)> = None;
        let lone_band = |i: usize| match v.get(i) {
            Some(Node::Section(sec)) if sec.span_x => self.els.get(sec.root).map(|e| e.b.height()),
            _ => None,
        };
        let mut consider = |cuts: Vec<usize>| {
            let mut edges = vec![0];
            edges.extend(&cuts);
            edges.push(n);
            let heights: Vec<f64> = edges.windows(2).map(|w| if let [a, b] = w { height(*a, *b) } else { 0.0 }).collect();
            let tallest = heights.iter().copied().fold(0.0, f64::max);
            let shortest = heights.iter().copied().fold(f64::INFINITY, f64::min);
            // Columns share the content (no column holding just a handle) and a band alone in a
            // column (a panel) is a substantial one.
            let thin_panel = edges.windows(2).any(|w| matches!(w, [a, b] if b - a == 1 && lone_band(*a).is_some_and(|h| h < PANEL_MIN * self.oh)));
            if shortest < COLUMN_BALANCE * tallest || thin_panel {
                return;
            }
            let cut_gap = cuts.iter().map(|i| gaps.get(i.saturating_sub(1)).copied().unwrap_or(0.0)).fold(f64::INFINITY, f64::min);
            let fits = tallest <= room + 0.5;
            let better = match &best {
                None => true,
                Some((bf, bt, bg, _)) => match (fits, *bf) {
                    (true, false) => true,
                    (false, true) => false,
                    (true, true) => cut_gap > *bg + 1e-9 || ((cut_gap - *bg).abs() <= 1e-9 && tallest < *bt),
                    (false, false) => tallest < *bt - 1e-9,
                },
            };
            if better {
                best = Some((fits, tallest, cut_gap, cuts));
            }
        };
        if cols == 2 {
            for i in 1..n {
                consider(vec![i]);
            }
        } else {
            for i in 1..n {
                for j in (i + 1)..n {
                    consider(vec![i, j]);
                }
            }
        }
        let Some((_, tallest, _, cuts)) = best else { return self.stack(lay, v, c, [ml, mt, mr, mb]) };
        let mut edges = vec![0];
        edges.extend(&cuts);
        edges.push(n);
        let mut out = Laid { w: self.nw, h: self.nh, items: vec![] };
        let mut laid: Vec<Option<Laid>> = laid.into_iter().map(Some).collect();
        // A column without the band that made a margin 0 gets a margin like the sides'.
        let side = ml.max(mr).max(self.min_margin);
        let section_at = |i: usize| match v.get(i) {
            Some(Node::Section(sec)) if sec.span_x => self.els.get(sec.root).map(|e| (sec.root, e.b)),
            _ => None,
        };
        let mut over: f64 = tallest - ah;
        // Columns are top-aligned with each other, the block centred on the page.
        let block_top = mt + ((ah - tallest) / 2.0).max(0.0);
        for (col, w) in edges.windows(2).enumerate() {
            let [a, b] = w else { continue };
            let touches = |top: bool| {
                (*a..*b).any(|i| section_at(i).is_some_and(|(_, r)| if top { r.y0 <= EDGE * self.oh } else { r.y1 >= self.oh * (1.0 - EDGE) }))
            };
            let top = if touches(true) {
                0.0
            } else if mt > 0.5 {
                mt
            } else {
                side
            };
            let bottom = if touches(false) {
                0.0
            } else if mb > 0.5 {
                mb
            } else {
                side
            };
            let room = self.nh - top - bottom;
            let x0 = ml + col as f64 * (cw + gap);
            let xa = Axis::new(c.x0, c.x1, x0, x0 + cw, s);
            let ch = height(*a, *b);
            over = over.max(ch - room);
            let mut y =
                if touches(true) || touches(false) { top + ((room - ch) / 2.0).max(0.0) } else { within(block_top, top, self.nh - bottom - ch) };
            for i in *a..*b {
                let (Some(l), Some(bd)) = (laid.get_mut(i).and_then(Option::take), bounds.get(i)) else { continue };
                let x = within(xa.place_lean(bd.x0, bd.x1, l.w, lay.lean), x0, x0 + cw - l.w);
                let h = l.h;
                out.add(l, x, y);
                y += h + if i + 1 < *b { gaps.get(i).copied().unwrap_or(0.0) } else { 0.0 };
            }
            // A band alone in its column becomes a full-height panel.
            if b - a == 1
                && let Some((root, r0)) = section_at(*a)
                && let Some(slot) = out.items.iter_mut().find(|(i, _)| *i == root)
                && let Out::Map(m, k) = slot.1
            {
                let r = m.rect(r0);
                slot.1 = Out::Map(Map::rect_to_rect(r0, Rect::new(r.x0, 0.0, r.x1, self.nh)), k);
            }
        }
        (out, over)
    }

    /// Bands and cards that reached the old page's edges reach the new page's edges, unless other
    /// content now lies between them and the edge.
    fn bleed(&self, flow: &Node, region: &Region, laid: &mut Laid) {
        let mut sections = vec![];
        top_sections(flow, &mut sections);
        let mut placed: Vec<(usize, Rect)> = vec![];
        for (i, o) in &laid.items {
            if let Some(e) = self.els.get(*i)
                && !region.decor.iter().any(|g| g.contains(i))
                && !region.fills.contains(i)
            {
                placed.push((*i, o.rect(e.outer())));
            }
        }
        for sec in sections {
            let Some(a) = self.els.get(sec.root).map(|e| e.b) else { continue };
            let Some(pos) = laid.items.iter().position(|(i, _)| *i == sec.root) else { continue };
            let Some((_, Out::Map(map, k))) = laid.items.get(pos).copied() else { continue };
            let mut members = vec![];
            sec.inner.ids(&mut members);
            members.push(sec.root);
            let r = map.rect(a);
            let others: Vec<Rect> = placed.iter().filter(|(i, _)| !members.contains(i)).map(|(_, r)| *r).collect();
            let clear = |test: &dyn Fn(&Rect) -> bool| !others.iter().any(test);
            let overlaps_y = |o: &Rect| o.y0 < r.y1 && o.y1 > r.y0;
            let overlaps_x = |o: &Rect| o.x0 < r.x1 && o.x1 > r.x0;
            let mut g = r;
            // A sidebar laid across the page (or a band down it) reaches both sides it now spans.
            let (left, right) = (a.x0 <= EDGE * self.ow, a.x1 >= self.ow * (1.0 - EDGE));
            let (top, bottom) = (a.y0 <= EDGE * self.oh, a.y1 >= self.oh * (1.0 - EDGE));
            let across = (left || right) && r.width() >= 0.6 * self.nw;
            let down = (top || bottom) && r.height() >= 0.6 * self.nh;
            if (left || across) && clear(&|o: &Rect| overlaps_y(o) && o.x1 <= r.x0 + 0.5) {
                g.x0 = 0.0;
            }
            if (right || across) && clear(&|o: &Rect| overlaps_y(o) && o.x0 >= r.x1 - 0.5) {
                g.x1 = self.nw;
            }
            if (top || down) && clear(&|o: &Rect| overlaps_x(o) && o.y1 <= r.y0 + 0.5) {
                g.y0 = 0.0;
            }
            if (bottom || down) && clear(&|o: &Rect| overlaps_x(o) && o.y0 >= r.y1 - 0.5) {
                g.y1 = self.nh;
            }
            if g != r
                && let Some(slot) = laid.items.get_mut(pos)
            {
                slot.1 = Out::Map(Map::rect_to_rect(a, g), k);
            }
        }
    }
}

/// The flow's items outside sections (`leaves`) and its bands and sidebars (`walls`: rect, spans
/// across, spans down).
fn inset_parts(n: &Node, els: &[El], leaves: &mut Vec<Rect>, walls: &mut Vec<(Rect, bool, bool)>) {
    match n {
        Node::Text(_) | Node::Rigid(_) => leaves.push(n.bounds(els)),
        Node::Section(sec) => {
            if sec.span_x || sec.span_y {
                walls.push((n.bounds(els), sec.span_x, sec.span_y));
            } else {
                leaves.push(n.bounds(els));
            }
        }
        Node::Stack(v) | Node::Row(v) | Node::Grid(v, _) => v.iter().for_each(|c| inset_parts(c, els, leaves, walls)),
    }
}

/// The design's dominant text alignment: -1 left, 0 centred, 1 right (by amount of text).
fn lean(page: &Page, region: &Region) -> f64 {
    let mut ids = vec![];
    if let Some(f) = &region.flow {
        f.ids(&mut ids);
    }
    let mut votes = [0usize; 3];
    for i in ids {
        if let Some(genha_doc::Element { kind: genha_doc::ElementKind::Text(t), .. }) = page.elements.get(i) {
            let n = t.text.chars().count().min(200);
            let slot = match t.style.align {
                genha_doc::TextAlign::Left => 0,
                genha_doc::TextAlign::Center => 1,
                genha_doc::TextAlign::Right => 2,
            };
            if let Some(v) = votes.get_mut(slot) {
                *v += n;
            }
        }
    }
    let [l, c, r] = votes;
    if l > c && l >= r {
        -1.0
    } else if r > c && r > l {
        1.0
    } else {
        0.0
    }
}

/// Steps by which a decoration or band that now covers text is made smaller.
const AVOID_STEPS: [f64; 4] = [0.8, 0.64, 0.5, 0.4];

fn overlap_area(a: Rect, b: Rect) -> f64 {
    let i = a.intersect(b);
    if i.width() > 0.0 && i.height() > 0.0 { i.area() } else { 0.0 }
}

/// Decorations and bands that did not cover text on the old page but do on the new one are made
/// smaller towards the corner or edge they belong to, until they clear the text (or are 40 % of
/// their size; then they stay as they are).
fn avoid_text(els: &[El], region: &Region, laid: &mut Laid, nw: f64, nh: f64) {
    let area = |r: Rect| r.width().max(0.0) * r.height().max(0.0);
    let hits = |r: Rect, ts: &[Rect]| ts.iter().any(|t| overlap_area(*t, r) > 0.02 * area(*t).min(area(r)));
    let visible_text = |i: usize| els.get(i).is_some_and(|e| e.kind.is_text() && !e.hidden);
    let texts: Vec<Rect> = laid.items.iter().filter(|(i, _)| visible_text(*i)).filter_map(|(i, o)| els.get(*i).map(|e| o.rect(e.ink))).collect();
    let old_texts: Vec<Rect> = els.iter().filter(|e| e.kind.is_text() && !e.hidden).map(|e| e.ink).collect();
    // (members, whether it is a fill: a band or a background; None for decorations)
    let mut groups: Vec<(Vec<usize>, Option<bool>)> = region.decor.iter().map(|g| (g.clone(), None)).collect();
    let (xa, ya) = (Axis::new(0.0, nw, 0.0, nw, 1.0), Axis::new(0.0, nh, 0.0, nh, 1.0));
    for &i in &region.fills {
        if let Some(e) = els.get(i) {
            groups.push((vec![i], Some(e.b.width() >= e.b.height())));
        }
    }
    for (group, band) in groups {
        let members: Vec<(usize, Rect)> = group.iter().filter_map(|i| els.get(*i).filter(|e| !e.hidden).map(|e| (*i, e.b))).collect();
        if members.is_empty() {
            continue;
        }
        let old = model::union(members.iter().map(|(_, b)| *b));
        if hits(old, &old_texts) {
            continue;
        }
        let new_of = |laid: &Laid, i: usize, b: Rect| laid.items.iter().find(|(j, _)| *j == i).map(|(_, o)| o.rect(b));
        let new = model::union(members.iter().filter_map(|(i, b)| new_of(laid, *i, *b)));
        // A band is told apart from a background by what it spans on the new page.
        let band = band.map(|_| (xa.spans(new.x0, new.x1), ya.spans(new.y0, new.y1)));
        if matches!(band, Some((true, true))) || !hits(new, &texts) {
            continue;
        }
        let ax = if new.center().x >= nw / 2.0 { new.x1 } else { new.x0 };
        let ay = if new.center().y >= nh / 2.0 { new.y1 } else { new.y0 };
        let mut fix: Option<(Map, f64)> = None;
        for f in AVOID_STEPS {
            let (fx, fy) = match band {
                Some((true, _)) => (1.0, f),
                Some((_, true)) => (f, 1.0),
                _ => (f, f),
            };
            let shrink = Map { sx: fx, sy: fy, tx: ax - ax * fx, ty: ay - ay * fy };
            if !hits(shrink.rect(new), &texts) {
                fix = Some((shrink, fx.min(fy)));
                break;
            }
        }
        // A small decoration that still covers text moves to a free corner of the page.
        if fix.is_none() && band.is_none() {
            let pad = 0.03 * nw.min(nh);
            let corners =
                [(pad, pad), (nw - pad - new.width(), pad), (pad, nh - pad - new.height()), (nw - pad - new.width(), nh - pad - new.height())];
            fix = corners
                .iter()
                .map(|&(x, y)| Map { sx: 1.0, sy: 1.0, tx: x - new.x0, ty: y - new.y0 })
                .find(|m| !hits(m.rect(new), &texts) && new.width() < 0.3 * nw && new.height() < 0.3 * nh)
                .map(|m| (m, 1.0));
        }
        let Some((shift, k_shift)) = fix else { continue };
        for (i, o) in laid.items.iter_mut() {
            if group.contains(i)
                && let Out::Map(m, k) = *o
            {
                *o = Out::Map(
                    Map { sx: m.sx * shift.sx, sy: m.sy * shift.sy, tx: m.tx * shift.sx + shift.tx, ty: m.ty * shift.sy + shift.ty },
                    k * k_shift,
                );
            }
        }
    }
}

/// The sections of a flow that are not inside another section.
fn top_sections<'a>(n: &'a Node, out: &mut Vec<&'a model::Section>) {
    match n {
        Node::Section(s) => out.push(s),
        Node::Stack(v) | Node::Row(v) | Node::Grid(v, _) => v.iter().for_each(|c| top_sections(c, out)),
        Node::Text(_) | Node::Rigid(_) => {}
    }
}
