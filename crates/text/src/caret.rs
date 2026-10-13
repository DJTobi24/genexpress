//! Caret positions, hit testing and selection boxes for laid-out text.
//!
//! A [`CaretMap`] is built from the same shaped glyph layout the renderer draws
//! ([`crate::layout_glyphs`], with wrapping, alignment, letter spacing, kerning, uppercase and bidi
//! reordering), so a caret placed with it sits exactly between the drawn letters. Glyphs map back
//! to the text through their clusters: a ligature or any glyph covering several characters is split
//! evenly by grapheme clusters, and right-to-left clusters run from their right edge. Indices are
//! characters of the element's text (before uppercasing), from 0 to its length, as in
//! [`crate::edit`]. Coordinates are the text box's own (origin at its top-left, y down), like
//! [`crate::GlyphLayout`].
//!
//! Where a long line wraps, the index between the two lines is both the end of the first and the
//! start of the second; `upstream` picks the first (the caret after the last letter of a line,
//! as End puts it), otherwise the second.

use std::collections::BTreeMap;

use genha_doc::TextStyle;
use kurbo::Rect;
use unicode_segmentation::UnicodeSegmentation;

use crate::{FontBook, GlyphLayout, layout_glyphs};

/// A place the caret can stand on a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretStop {
    /// Character index in the text.
    pub index: usize,
    pub x: f64,
}

/// Where one grapheme cluster is drawn: characters `start..end`, from `x0` to `x1` (left to right).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretSpan {
    pub start: usize,
    pub end: usize,
    pub x0: f64,
    pub x1: f64,
}

/// One laid-out line's caret stops, by increasing index (left to right in left-to-right text),
/// and the spans of its grapheme clusters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaretLine {
    pub stops: Vec<CaretStop>,
    pub spans: Vec<CaretSpan>,
    pub top: f64,
    pub baseline: f64,
    pub height: f64,
}

impl CaretLine {
    /// Index of the line's first stop.
    pub fn start(&self) -> usize {
        self.stops.first().map_or(0, |s| s.index)
    }

    /// Index of the line's last stop (after its last character).
    pub fn end(&self) -> usize {
        self.stops.last().map_or(0, |s| s.index)
    }

    /// The x of index `i` on this line (the nearest stop before it when it has none).
    pub fn x_of(&self, i: usize) -> f64 {
        let mut x = self.stops.first().map_or(0.0, |s| s.x);
        for s in &self.stops {
            if s.index > i {
                break;
            }
            x = s.x;
        }
        x
    }

    /// The stop nearest to `x`.
    fn nearest(&self, x: f64) -> Option<(usize, CaretStop)> {
        let mut best: Option<(usize, CaretStop)> = None;
        for (k, s) in self.stops.iter().enumerate() {
            if best.is_none_or(|(_, b)| (s.x - x).abs() < (b.x - x).abs()) {
                best = Some((k, *s));
            }
        }
        best
    }
}

/// Where the caret is drawn: a vertical bar at `x` from `top`, `height` tall, on line `line`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretPos {
    pub line: usize,
    pub x: f64,
    pub top: f64,
    pub height: f64,
    pub baseline: f64,
}

/// Every caret stop of a laid-out text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaretMap {
    pub lines: Vec<CaretLine>,
    /// Characters in the text.
    pub len: usize,
}

impl CaretMap {
    /// Lays `text` out as the renderer does (`style`, a box `box_width` wide) and maps it.
    pub fn for_text(book: &FontBook, text: &str, style: &TextStyle, box_width: f64) -> CaretMap {
        let gl = layout_glyphs(book, text, style, box_width);
        CaretMap::new(text, &gl)
    }

    /// Maps `layout`, which [`layout_glyphs`] made from `text`.
    pub fn new(text: &str, layout: &GlyphLayout) -> CaretMap {
        // Byte offset of every character, and the grapheme boundaries as character indices.
        let starts: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
        let len = starts.len();
        let char_at = |b: usize| starts.partition_point(|&s| s < b);
        let graphemes: Vec<usize> = text.grapheme_indices(true).map(|(b, _)| char_at(b)).collect();
        let mut lines = vec![];
        for gl in &layout.lines {
            let (cs, ce) = (char_at(gl.range.start), char_at(gl.range.end).max(char_at(gl.range.start)));
            // Each cluster's extent, in reading order (glyphs come left to right).
            let mut clusters: BTreeMap<usize, (f64, f64, bool)> = BTreeMap::new();
            for g in &gl.glyphs {
                let (a, b) = (g.x.min(g.x + g.advance), g.x.max(g.x + g.advance));
                if !(a.is_finite() && b.is_finite()) {
                    continue;
                }
                let c = char_at(g.cluster).clamp(cs, ce);
                clusters.entry(c).and_modify(|e| *e = (e.0.min(a), e.1.max(b), e.2)).or_insert((a, b, g.rtl));
            }
            let list: Vec<(usize, (f64, f64, bool))> = clusters.into_iter().collect();
            let mut stops: Vec<CaretStop> = vec![];
            let mut spans: Vec<CaretSpan> = vec![];
            let mut end_x = gl.x;
            for (k, &(c, (x0, x1, rtl))) in list.iter().enumerate() {
                let next = list.get(k + 1).map_or(ce, |n| n.0).max(c);
                // Grapheme boundaries inside the cluster split its advance evenly.
                let mut cuts = vec![c];
                cuts.extend(graphemes.iter().copied().filter(|&g| g > c && g < next));
                cuts.push(next);
                let n = (cuts.len() - 1).max(1) as f64;
                let w = x1 - x0;
                let at = |k: usize| if rtl { x1 - w * k as f64 / n } else { x0 + w * k as f64 / n };
                for (s, pair) in cuts.windows(2).enumerate() {
                    let (Some(&i0), Some(&i1)) = (pair.first(), pair.get(1)) else { continue };
                    let (a, b) = (at(s), at(s + 1));
                    stops.push(CaretStop { index: i0, x: a });
                    spans.push(CaretSpan { start: i0, end: i1, x0: a.min(b), x1: a.max(b) });
                    end_x = b;
                }
            }
            // Characters before the first cluster (none normally) start where it starts.
            let first_x = stops.first().map_or(gl.x, |s| s.x);
            if stops.first().is_none_or(|s| s.index > cs) {
                stops.insert(0, CaretStop { index: cs, x: first_x });
            }
            stops.push(CaretStop { index: ce, x: end_x });
            stops.sort_by_key(|s| s.index);
            stops.dedup_by_key(|s| s.index);
            lines.push(CaretLine { stops, spans, top: gl.top, baseline: gl.baseline, height: gl.height });
        }
        CaretMap { lines, len }
    }

    /// The line showing index `i` (see the module docs for `upstream`).
    pub fn line_of(&self, i: usize, upstream: bool) -> usize {
        let mut found = None;
        for (li, l) in self.lines.iter().enumerate() {
            if l.start() <= i && i <= l.end() {
                if upstream {
                    return li;
                }
                found = Some(li);
            }
        }
        found.or_else(|| self.lines.iter().rposition(|l| l.start() <= i)).unwrap_or(0)
    }

    /// Where the caret at index `i` is drawn (None when nothing is laid out).
    pub fn caret(&self, i: usize, upstream: bool) -> Option<CaretPos> {
        let li = self.line_of(i, upstream);
        let l = self.lines.get(li)?;
        Some(CaretPos { line: li, x: l.x_of(i), top: l.top, height: l.height, baseline: l.baseline })
    }

    /// The caret index nearest to the point (`x`, `y`), and whether it is shown at the end of
    /// its line (upstream). Points above or below the text pick the first or last line.
    pub fn hit(&self, x: f64, y: f64) -> (usize, bool) {
        let Some(last) = self.lines.len().checked_sub(1) else { return (0, false) };
        let li = self.lines.iter().position(|l| y < l.top + l.height).unwrap_or(last);
        self.hit_line(li, x)
    }

    fn hit_line(&self, li: usize, x: f64) -> (usize, bool) {
        let Some(l) = self.lines.get(li) else { return (0, false) };
        let Some((k, s)) = l.nearest(x) else { return (l.start(), false) };
        let at_end = k + 1 == l.stops.len();
        let upstream = at_end && self.lines.get(li + 1).is_some_and(|n| n.start() == s.index);
        (s.index, upstream)
    }

    /// Up (`delta` < 0) or down from index `i` by `delta` lines, keeping to `x`. Past the first
    /// line goes to the start of the text, past the last to its end.
    pub fn vertical(&self, i: usize, upstream: bool, delta: i64, x: f64) -> (usize, bool) {
        let li = self.line_of(i, upstream) as i64 + delta;
        if li < 0 {
            return (self.lines.first().map_or(0, CaretLine::start), false);
        }
        match usize::try_from(li).ok().filter(|li| *li < self.lines.len()) {
            Some(li) => self.hit_line(li, x),
            None => (self.lines.last().map_or(self.len, CaretLine::end), false),
        }
    }

    /// Home: the start of the line showing `i`.
    pub fn line_start(&self, i: usize, upstream: bool) -> usize {
        self.lines.get(self.line_of(i, upstream)).map_or(0, CaretLine::start)
    }

    /// End: the end of the line showing `i`, and whether that is shown upstream.
    pub fn line_end(&self, i: usize, upstream: bool) -> (usize, bool) {
        let li = self.line_of(i, upstream);
        let Some(l) = self.lines.get(li) else { return (self.len, false) };
        let e = l.end();
        (e, self.lines.get(li + 1).is_some_and(|n| n.start() == e))
    }

    /// The boxes that highlight the selection `from..to` (either order): the selected clusters'
    /// spans, merged where they touch (right-to-left runs can make several per line). A selected
    /// line break shows as a narrow box after its line.
    pub fn selection_rects(&self, from: usize, to: usize) -> Vec<Rect> {
        let (a, b) = (from.min(to), from.max(to));
        let mut out = vec![];
        if a == b {
            return out;
        }
        for (li, l) in self.lines.iter().enumerate() {
            let e = l.end();
            let mut xs: Vec<(f64, f64)> = l.spans.iter().filter(|s| s.start >= a && s.end <= b && s.end > s.start).map(|s| (s.x0, s.x1)).collect();
            let hard_break = self.lines.get(li + 1).is_some_and(|n| n.start() > e);
            if b > e && a <= e && hard_break {
                let x = l.x_of(e);
                xs.push((x, x + (l.height * 0.25).max(1.0)));
            }
            xs.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut cur: Option<(f64, f64)> = None;
            for (x0, x1) in xs {
                match cur {
                    Some((c0, c1)) if x0 <= c1 + 1e-6 => cur = Some((c0, c1.max(x1))),
                    _ => {
                        if let Some((c0, c1)) = cur.filter(|c| c.1 > c.0) {
                            out.push(Rect::new(c0, l.top, c1, l.top + l.height));
                        }
                        cur = Some((x0, x1));
                    }
                }
            }
            if let Some((c0, c1)) = cur.filter(|c| c.1 > c.0) {
                out.push(Rect::new(c0, l.top, c1, l.top + l.height));
            }
        }
        out
    }

    /// The box all lines take (their stops and clusters, top to bottom).
    pub fn bounds(&self) -> Option<Rect> {
        let mut r: Option<Rect> = None;
        for l in &self.lines {
            let xs = l.stops.iter().map(|s| s.x).chain(l.spans.iter().flat_map(|s| [s.x0, s.x1])).filter(|x| x.is_finite());
            let (lo, hi) = xs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)));
            if lo > hi {
                continue;
            }
            let lr = Rect::new(lo, l.top, hi, l.top + l.height);
            r = Some(r.map_or(lr, |r| r.union(lr)));
        }
        r
    }
}
