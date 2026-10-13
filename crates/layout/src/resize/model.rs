//! What a page is made of, for the re-layout: facts about each element ([`El`]) and the structure
//! found in an area ([`region`]): backgrounds and bands that follow the page, decorations that stay
//! anchored, and the content as a tree ([`Node`]) of stacks, rows and grids of text, cards and
//! pictures.
//!
//! Detection, in order:
//!
//! 1. **Backgrounds**: non-text elements covering ≥ 90 % of the area, or reaching within 10 % of
//!    all four of its sides (inset frames).
//! 2. **Holders**: shapes and pictures that contain other elements drawn above them. A holder
//!    spanning the area on one axis (a header band, a footer, a sidebar) becomes a **section**
//!    whose content is laid out again inside it; a wide rectangle or picture holding text (a card)
//!    also becomes a section; other holders (buttons, badges, discs, bursts) are **rigid units**
//!    that scale as one. Bands holding nothing stretch along the page.
//! 3. **Free elements**: text is content; groups, charts, tables and pictures are content that
//!    keeps its aspect; a free shape is content when it sits next to text (an icon beside its
//!    label, a divider) or is a large illustration clear of text, and a **decoration** otherwise.
//! 4. **Structure**: the content is cut recursively along the widest clear gaps (an X-Y cut): a
//!    horizontal cut makes a **stack**, a vertical one a **row**; items that overlap and cannot be
//!    cut stay together as a rigid cluster. Rows of the same shape stacked above each other become
//!    columns, and repeated items (similar cards, stat blocks, photo tiles) a **grid**.

use genha_doc::{ElementKind, Page, ShapeKind};
use genha_geom::Rect;

use super::Axis;
use super::measure::Measurer;
use genha_doc::TextAlign;

/// Share of an area a non-text element must cover to count as its background.
pub(crate) const FULL_BLEED: f64 = 0.9;
/// Holders larger than this share of their area are not cards (a glow behind everything).
const HOLDER_MAX_AREA: f64 = 0.5;
/// A rectangle or picture holding text is a card (laid out again inside) when at least this wide,
/// as a share of its area.
const CARD_MIN_WIDTH: f64 = 0.45;
/// A band is at least this many times longer than it is thick.
const BAND_RATIO: f64 = 1.5;
/// …and at least this many times as tall as its tallest text.
const CARD_MIN_HEIGHT: f64 = 2.5;
/// A free shape this close to text (as a share of the area's shorter side) belongs with it.
const NEAR: f64 = 0.06;
/// A free shape at least this large (share of the area) and clear of text is an illustration.
const ILLUSTRATION: f64 = 0.12;
/// Items may overlap this much (share of the area's side) and still be cut apart.
const CUT_SLACK: f64 = 0.012;
/// Sizes within this ratio are "the same" for repeated items.
const SIMILAR: f64 = 0.15;

/// What kind of thing an element is, for the re-layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Straight, unrotated text: re-flowed.
    Text,
    /// Curved or rotated text: moved and scaled as it is.
    FixedText,
    /// A rectangle, line or picture: may stretch.
    Stretchy,
    /// Path data (icons, waves, brush strokes): stretches only as a band.
    Path,
    /// Ellipses, stars, polygons, triangles: never stretched.
    Solid,
    /// Groups, charts, tables, QR codes: keep their aspect and their inner layout.
    Unit,
}

impl Kind {
    pub fn is_text(self) -> bool {
        matches!(self, Kind::Text | Kind::FixedText)
    }

    fn holds(self) -> bool {
        matches!(self, Kind::Stretchy | Kind::Path | Kind::Solid)
    }
}

/// Facts about one element of the page.
#[derive(Clone, Debug)]
pub(crate) struct El {
    /// The frame's bounds.
    pub b: Rect,
    /// Where it visibly is: for straight text, its lines (from the real layout); else `b`.
    pub ink: Rect,
    pub kind: Kind,
    pub hidden: bool,
    /// Text: its font size and alignment (0 and left otherwise).
    pub size: f64,
    pub align: TextAlign,
}

impl El {
    /// `b` and `ink` together.
    pub fn outer(&self) -> Rect {
        self.b.union(self.ink)
    }
}

/// Facts about every top-level element of `page` (its frames sanitized).
pub(crate) fn elements(page: &Page, m: &Measurer<'_>) -> Vec<El> {
    page.elements
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let f = e.frame;
            let b = f.bounds();
            let kind = match &e.kind {
                ElementKind::Text(t) if f.rotation == 0.0 && t.curve_degrees() == 0.0 => Kind::Text,
                ElementKind::Text(_) => Kind::FixedText,
                _ if e.name.starts_with("QR code") => Kind::Unit,
                ElementKind::Shape(sh) => match sh.shape {
                    ShapeKind::Rect { .. } | ShapeKind::Line => Kind::Stretchy,
                    ShapeKind::Path { .. } => Kind::Path,
                    _ => Kind::Solid,
                },
                ElementKind::Image(_) => Kind::Stretchy,
                ElementKind::Group(_) | ElementKind::Chart(_) | ElementKind::Table(_) => Kind::Unit,
            };
            let ink = match &e.kind {
                ElementKind::Text(t) if kind == Kind::Text => {
                    let (w, h) = m.wrapped(i, t, 1.0, f.width);
                    let (w, h) = (w.clamp(0.0, f.width.max(0.0)), h.max(0.0));
                    let x0 = match t.style.align {
                        TextAlign::Left => f.x,
                        TextAlign::Center => f.x + (f.width - w) / 2.0,
                        TextAlign::Right => f.x + f.width - w,
                    };
                    if w.is_finite() && h.is_finite() { Rect::new(x0, f.y, x0 + w, f.y + h) } else { b }
                }
                _ => b,
            };
            let (size, align) = match &e.kind {
                ElementKind::Text(t) => (t.style.font_size, t.style.align),
                _ => (0.0, TextAlign::Left),
            };
            El { b, ink, kind, hidden: e.hidden, size, align }
        })
        .collect()
}

/// The content of an area, as a tree.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    /// A re-flowed text element.
    Text(usize),
    /// Elements that move and scale together (a button and its label, a group, a cluster); the
    /// first is the holder, if any.
    Rigid(Vec<usize>),
    /// A band or card whose content is laid out again inside it.
    Section(Box<Section>),
    /// Top to bottom.
    Stack(Vec<Node>),
    /// Left to right.
    Row(Vec<Node>),
    /// Repeated items in reading order, `cols` per row in the original.
    Grid(Vec<Node>, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Section {
    /// The band or card.
    pub root: usize,
    /// Its content.
    pub inner: Region,
    /// It spans its area horizontally / vertically (a band), rather than being a card.
    pub span_x: bool,
    pub span_y: bool,
}

/// An area's elements by role.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Region {
    pub flow: Option<Node>,
    /// Anchored to the area (or to the content they overlap), each group as one (a balloon, its
    /// knot and its string).
    pub decor: Vec<Vec<usize>>,
    /// Backgrounds and empty bands, stretched with the area.
    pub fills: Vec<usize>,
}

impl Node {
    /// Every element in the node, holders and section content included.
    pub fn ids(&self, out: &mut Vec<usize>) {
        match self {
            Node::Text(i) => out.push(*i),
            Node::Rigid(v) => out.extend(v),
            Node::Section(s) => {
                out.push(s.root);
                s.inner.ids(out);
            }
            Node::Stack(v) | Node::Row(v) | Node::Grid(v, _) => v.iter().for_each(|n| n.ids(out)),
        }
    }

    /// Where the node is on the old page: text by its lines, sections by their band or card.
    pub fn bounds(&self, els: &[El]) -> Rect {
        match self {
            Node::Text(i) => els.get(*i).map_or(Rect::ZERO, |e| e.ink),
            Node::Rigid(v) => union(v.iter().filter_map(|i| els.get(*i)).map(El::outer)),
            Node::Section(s) => els.get(s.root).map_or(Rect::ZERO, |e| e.b),
            Node::Stack(v) | Node::Row(v) | Node::Grid(v, _) => union(v.iter().map(|n| n.bounds(els))),
        }
    }

    /// Whether the node holds any text.
    pub fn has_text(&self, els: &[El]) -> bool {
        let mut ids = vec![];
        self.ids(&mut ids);
        ids.iter().any(|i| els.get(*i).is_some_and(|e| e.kind.is_text()))
    }
}

impl Region {
    pub fn ids(&self, out: &mut Vec<usize>) {
        if let Some(n) = &self.flow {
            n.ids(out);
        }
        self.decor.iter().for_each(|g| out.extend(g));
        out.extend(&self.fills);
    }
}

pub(crate) fn union(rects: impl Iterator<Item = Rect>) -> Rect {
    rects.reduce(|a, b| a.union(b)).unwrap_or(Rect::ZERO)
}

fn area_of(r: Rect) -> f64 {
    r.width().max(0.0) * r.height().max(0.0)
}

fn overlap(a: Rect, b: Rect) -> f64 {
    let i = a.intersect(b);
    if i.width() > 0.0 && i.height() > 0.0 { i.area() } else { 0.0 }
}

/// Whether `inner` lies inside `outer`, with a little slack.
fn inside(outer: Rect, inner: Rect) -> bool {
    let tol = 2.0_f64.max(0.02 * outer.width().min(outer.height()));
    inner.x0 >= outer.x0 - tol && inner.y0 >= outer.y0 - tol && inner.x1 <= outer.x1 + tol && inner.y1 <= outer.y1 + tol
}

/// The gap between two rectangles (0 when they touch or overlap).
fn gap(a: Rect, b: Rect) -> f64 {
    let dx = (a.x0 - b.x1).max(b.x0 - a.x1).max(0.0);
    let dy = (a.y0 - b.y1).max(b.y0 - a.y1).max(0.0);
    dx.hypot(dy)
}

/// Sorts out the elements `ids` (indices into `els`, bottom first) of `area`.
pub(crate) fn region(els: &[El], ids: &[usize], area: Rect, depth: usize) -> Region {
    let (aw, ah) = (area.width().max(1.0), area.height().max(1.0));
    let xa = Axis::new(area.x0, area.x1, 0.0, 0.0, 1.0);
    let ya = Axis::new(area.y0, area.y1, 0.0, 0.0, 1.0);
    let get = |i: usize| els.get(i);
    let mut out = Region::default();
    let mut taken: Vec<usize> = vec![];
    // 1. Hidden elements and backgrounds.
    for &i in ids {
        let Some(e) = get(i) else { continue };
        if e.hidden {
            out.decor.push(vec![i]);
            taken.push(i);
        } else if matches!(e.kind, Kind::Stretchy | Kind::Path) {
            // Circles, stars and badges are never stretched: only rectangles, pictures and paths
            // make backgrounds.
            let full = overlap(e.b, area) >= FULL_BLEED * aw * ah || (xa.spans(e.b.x0, e.b.x1) && ya.spans(e.b.y0, e.b.y1));
            if full {
                out.fills.push(i);
                taken.push(i);
            }
        }
    }
    // 2. Holders, largest first: each takes the elements above it that it contains.
    let mut holders: Vec<usize> = ids.iter().copied().filter(|i| !taken.contains(i) && get(*i).is_some_and(|e| e.kind.holds())).collect();
    holders.sort_by(|a, b| {
        let ar = |i: usize| get(i).map_or(0.0, |e| area_of(e.b));
        ar(*b).total_cmp(&ar(*a)).then(a.cmp(b))
    });
    let mut items: Vec<(Rect, Node)> = vec![];
    let mut free_shapes: Vec<usize> = vec![];
    for h in holders {
        if taken.contains(&h) {
            continue;
        }
        let Some(he) = get(h) else { continue };
        // A band is long along the axis it spans (a seal as tall as a thin banner is not one).
        // (or reaches both ends exactly: a full-height panel).
        let ends_x = he.b.x0 <= area.x0 + 0.01 * aw && he.b.x1 >= area.x1 - 0.01 * aw;
        let ends_y = he.b.y0 <= area.y0 + 0.01 * ah && he.b.y1 >= area.y1 - 0.01 * ah;
        let long_x = he.b.width() >= BAND_RATIO * he.b.height() || (ends_x && he.b.width() >= he.b.height());
        let long_y = he.b.height() >= BAND_RATIO * he.b.width() || (ends_y && he.b.height() >= he.b.width());
        let spans =
            matches!(he.kind, Kind::Stretchy | Kind::Path) && ((long_x && xa.spans(he.b.x0, he.b.x1)) || (long_y && ya.spans(he.b.y0, he.b.y1)));
        let small = area_of(he.b) <= HOLDER_MAX_AREA * aw * ah;
        if !spans && !small {
            continue;
        }
        let members: Vec<usize> =
            ids.iter().copied().filter(|&j| j > h && !taken.contains(&j) && get(j).is_some_and(|e| !e.hidden && inside(he.b, e.outer()))).collect();
        if members.is_empty() {
            if spans {
                out.fills.push(h);
                taken.push(h);
            }
            continue;
        }
        taken.push(h);
        taken.extend(&members);
        let has_text = members.iter().any(|j| get(*j).is_some_and(|e| e.kind.is_text()));
        // A card holds several things with room around them; a button (a pill and its label) is
        // a unit.
        let line = members.iter().filter_map(|j| get(*j)).filter(|e| e.kind.is_text()).map(|e| e.ink.height()).fold(0.0, f64::max);
        let roomy = members.len() >= 2 && he.b.height() >= CARD_MIN_HEIGHT * line;
        let card = has_text && roomy && he.kind == Kind::Stretchy && he.b.width() >= CARD_MIN_WIDTH * aw && depth < 3;
        if (spans || card) && depth < 3 {
            let inner = region(els, &members, he.b, depth + 1);
            let sec = Section {
                root: h,
                inner,
                span_x: spans && long_x && xa.spans(he.b.x0, he.b.x1),
                span_y: spans && long_y && ya.spans(he.b.y0, he.b.y1),
            };
            items.push((he.b, Node::Section(Box::new(sec))));
        } else {
            let mut v = vec![h];
            v.extend(members);
            let node = Node::Rigid(v);
            items.push((node.bounds(els), node));
        }
    }
    // 3. Free elements.
    for &i in ids {
        if taken.contains(&i) {
            continue;
        }
        let Some(e) = get(i) else { continue };
        match e.kind {
            Kind::Text => items.push((e.ink, Node::Text(i))),
            Kind::FixedText | Kind::Unit => items.push((e.outer(), Node::Rigid(vec![i]))),
            Kind::Stretchy | Kind::Path | Kind::Solid => free_shapes.push(i),
        }
    }
    // Free shapes that touch form one piece (a balloon, its knot and its string). Pieces next to
    // text belong with it; large ones clear of text are illustrations; the rest are decorations.
    let texty: Vec<Rect> = items.iter().filter(|(_, n)| n.has_text(els)).map(|(r, _)| *r).collect();
    let near = NEAR * aw.min(ah);
    for piece in touching(els, &free_shapes) {
        let b = union(piece.iter().filter_map(|i| get(*i)).map(|e| e.b));
        let over_text = texty.iter().any(|t| overlap(*t, b) > 0.02 * area_of(b).min(area_of(*t)));
        let next_to_text = texty.iter().any(|t| gap(*t, b) <= near);
        let big = area_of(b) >= ILLUSTRATION * aw * ah;
        if !over_text && (next_to_text || big) {
            items.push((b, Node::Rigid(piece)));
        } else {
            out.decor.push(piece);
        }
    }
    if !items.is_empty() {
        let slack = (CUT_SLACK * aw, CUT_SLACK * ah);
        out.flow = Some(tidy(els, cut(items, slack)));
    }
    out.decor.sort_unstable();
    out.fills.sort_unstable();
    out
}

/// `ids` grouped into pieces whose bounds touch or overlap (each sorted, in order of first id).
fn touching(els: &[El], ids: &[usize]) -> Vec<Vec<usize>> {
    let n = ids.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &[usize], mut i: usize) -> usize {
        while let Some(&p) = parent.get(i) {
            if p == i {
                break;
            }
            i = p;
        }
        i
    }
    let rect = |k: usize| ids.get(k).and_then(|i| els.get(*i)).map(|e| e.b);
    for a in 0..n {
        for b in (a + 1)..n {
            if let (Some(ra), Some(rb)) = (rect(a), rect(b))
                && gap(ra, rb) <= 1.5
            {
                let (x, y) = (root(&parent, a), root(&parent, b));
                if let Some(slot) = parent.get_mut(x.max(y)) {
                    *slot = x.min(y);
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![];
    let mut roots: Vec<usize> = vec![];
    for (k, &i) in ids.iter().enumerate() {
        let r = root(&parent, k);
        match roots.iter().position(|x| *x == r) {
            Some(p) => {
                if let Some(g) = groups.get_mut(p) {
                    g.push(i);
                }
            }
            None => {
                roots.push(r);
                groups.push(vec![i]);
            }
        }
    }
    groups
}

/// Splits items into groups separated by clear gaps along one axis (`y` when `vertical`).
fn split(items: Vec<(Rect, Node)>, vertical: bool, slack: f64) -> Vec<Vec<(Rect, Node)>> {
    let lo = |r: &Rect| if vertical { r.y0 } else { r.x0 };
    let hi = |r: &Rect| if vertical { r.y1 } else { r.x1 };
    let mut items = items;
    items.sort_by(|a, b| lo(&a.0).total_cmp(&lo(&b.0)).then(hi(&a.0).total_cmp(&hi(&b.0))));
    let mut groups: Vec<Vec<(Rect, Node)>> = vec![];
    let mut end = f64::NEG_INFINITY;
    for it in items {
        let (a, b) = (lo(&it.0), hi(&it.0));
        // Slack lets slightly overlapping items be cut apart, but never more than a third of the
        // smaller one.
        let size = (b - a).max(0.0);
        let tol = slack.min(size / 3.0);
        match groups.last_mut() {
            Some(g) if a < end - tol => {
                g.push(it);
                end = end.max(b);
            }
            _ => {
                groups.push(vec![it]);
                end = b;
            }
        }
    }
    groups
}

/// The X-Y cut: stacks of rows of stacks…
fn cut(items: Vec<(Rect, Node)>, slack: (f64, f64)) -> Node {
    if items.len() == 1 {
        if let Some((_, n)) = items.into_iter().next() {
            return n;
        }
        return Node::Rigid(vec![]);
    }
    let rows = split(items, true, slack.1);
    if rows.len() > 1 {
        return Node::Stack(rows.into_iter().map(|g| cut(g, slack)).collect());
    }
    let items = rows.into_iter().flatten().collect::<Vec<_>>();
    let cols = split(items, false, slack.0);
    if cols.len() > 1 {
        return Node::Row(cols.into_iter().map(|g| cut(g, slack)).collect());
    }
    // Overlapping items that cannot be cut apart stay together.
    let mut ids = vec![];
    for (_, n) in cols.into_iter().flatten() {
        n.ids(&mut ids);
    }
    ids.sort_unstable();
    ids.dedup();
    Node::Rigid(ids)
}

/// Turns stacked rows of the same shape into columns, and repeated items into grids.
fn tidy(els: &[El], n: Node) -> Node {
    match n {
        Node::Stack(v) => {
            let v = flatten(v.into_iter().map(|c| tidy(els, c)).collect(), true);
            let v = transpose_runs(els, v);
            let v = repeats(els, v, false);
            single_or(v, Node::Stack)
        }
        Node::Row(v) => {
            let v = flatten(v.into_iter().map(|c| tidy(els, c)).collect(), false);
            let v = repeats(els, v, true);
            single_or(v, Node::Row)
        }
        Node::Section(mut s) => {
            if let Some(f) = s.inner.flow.take() {
                s.inner.flow = Some(tidy(els, f));
            }
            Node::Section(s)
        }
        other => other,
    }
}

fn single_or(v: Vec<Node>, make: fn(Vec<Node>) -> Node) -> Node {
    if v.len() == 1 {
        return v.into_iter().next().unwrap_or(Node::Rigid(vec![]));
    }
    make(v)
}

/// Runs of two or more stacked rows with as many items each, lined up in columns (boxes over
/// their labels), become a grid (all items alike) or a row of columns.
fn transpose_runs(els: &[El], v: Vec<Node>) -> Vec<Node> {
    let mut out = vec![];
    let mut i = 0;
    while i < v.len() {
        let mut j = i + 1;
        while j < v.len() && v.get(i..=j).and_then(|r| aligned_rows(els, r)).is_some() {
            j += 1;
        }
        match v.get(i..j).filter(|r| r.len() >= 2).and_then(|r| aligned_rows(els, r).map(|c| (r, c))) {
            Some((run, cols)) => {
                out.push(transposed(els, run, cols));
                i = j;
            }
            None => {
                if let Some(n) = v.get(i) {
                    out.push(n.clone());
                }
                i += 1;
            }
        }
    }
    out
}

fn transposed(els: &[El], rows: &[Node], cols: usize) -> Node {
    let all: Vec<Node> = rows.iter().flat_map(row_items).cloned().collect();
    if all.iter().all(|x| grid_item(els, x)) && all.iter().all(|x| all.first().is_some_and(|f| similar(els, x, f))) {
        return Node::Grid(all, cols);
    }
    let mut columns: Vec<Vec<Node>> = vec![vec![]; cols];
    for r in rows {
        for (j, x) in row_items(r).iter().enumerate() {
            if let Some(c) = columns.get_mut(j) {
                c.push(x.clone());
            }
        }
    }
    let columns: Vec<Node> = columns.into_iter().map(Node::Stack).collect();
    single_or(repeats(els, columns, true), Node::Row)
}

/// Longest repeat period looked for (a card; a row and its bar; a title, text and divider).
const MAX_PERIOD: usize = 3;

/// Runs of repeated items become grids: two or more consecutive repeats of a group of up to
/// [`MAX_PERIOD`] siblings (each group a stack or row of its own when longer than one).
fn repeats(els: &[El], v: Vec<Node>, row: bool) -> Vec<Node> {
    let n = v.len();
    let same = |a: usize, b: usize| match (v.get(a), v.get(b)) {
        (Some(x), Some(y)) => similar(els, x, y),
        _ => false,
    };
    let mut out = vec![];
    let mut i = 0;
    while i < n {
        // (period, repeats) covering the most siblings from `i`.
        let mut best: Option<(usize, usize)> = None;
        for p in 1..=MAX_PERIOD {
            if i + 2 * p > n || !v.get(i..i + p).is_some_and(|g| g.iter().any(|x| grid_item(els, x))) {
                continue;
            }
            let mut reps = 1;
            while i + (reps + 1) * p <= n && (0..p).all(|q| same(i + q, i + reps * p + q)) {
                reps += 1;
            }
            if reps >= 2 && best.is_none_or(|(bp, br)| reps * p > bp * br) {
                best = Some((p, reps));
            }
        }
        match best {
            Some((p, reps)) => {
                let groups: Vec<Node> = (0..reps)
                    .filter_map(|r| v.get(i + r * p..i + (r + 1) * p))
                    .map(|g| match g {
                        [one] => one.clone(),
                        many if row => Node::Row(many.to_vec()),
                        many => Node::Stack(many.to_vec()),
                    })
                    .collect();
                let cols = if row { groups.len() } else { 1 };
                out.push(Node::Grid(groups, cols));
                i += p * reps;
            }
            None => {
                if let Some(x) = v.get(i) {
                    out.push(x.clone());
                }
                i += 1;
            }
        }
    }
    out
}

/// Nested stacks in a stack (rows in a row) merged into it.
fn flatten(v: Vec<Node>, stack: bool) -> Vec<Node> {
    let mut out = vec![];
    for n in v {
        match n {
            Node::Stack(inner) if stack => out.extend(inner),
            Node::Row(inner) if !stack => out.extend(inner),
            other => out.push(other),
        }
    }
    out
}

/// The items of a row (a lone item is a row of one).
fn row_items(n: &Node) -> &[Node] {
    match n {
        Node::Row(v) | Node::Grid(v, _) => v,
        other => std::slice::from_ref(other),
    }
}

/// When every node in `v` is a row of the same count (2+) of items alike (three boxes; their three
/// labels) and the items line up in columns, that count. Rows of different items (a dish and its
/// price) stay rows.
fn aligned_rows(els: &[El], v: &[Node]) -> Option<usize> {
    let first = v.first()?;
    let cols = match first {
        Node::Row(r) => r.len(),
        Node::Grid(r, c) if r.len() == *c => *c,
        _ => return None,
    };
    if cols < 2 {
        return None;
    }
    let ok = v.iter().all(|r| matches!(r, Node::Row(x) if x.len() == cols) || matches!(r, Node::Grid(x, c) if x.len() == cols && *c == cols));
    if !ok {
        return None;
    }
    let homogeneous = v.iter().all(|r| {
        let items = row_items(r);
        items.iter().all(|x| items.first().is_some_and(|f| similar(els, x, f)))
    });
    if !homogeneous {
        return None;
    }
    let base: Vec<Rect> = row_items(first).iter().map(|n| n.bounds(els)).collect();
    for r in v.iter().skip(1) {
        for (j, x) in row_items(r).iter().enumerate() {
            let (a, b) = (base.get(j)?, x.bounds(els));
            if a.x0.max(b.x0) >= a.x1.min(b.x1) {
                return None;
            }
            // and only that column
            for (k, other) in base.iter().enumerate() {
                if k != j && other.x0.max(b.x0) < other.x1.min(b.x1) {
                    return None;
                }
            }
        }
    }
    Some(cols)
}

/// Items that can be re-arranged in a grid: anything holding more than text (paragraphs, and rows
/// of text like a dish and its price, keep their order).
fn grid_item(els: &[El], n: &Node) -> bool {
    let mut ids = vec![];
    n.ids(&mut ids);
    ids.iter().any(|i| els.get(*i).is_some_and(|e| !e.kind.is_text()))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= SIMILAR * a.abs().max(b.abs()) + 2.0
}

/// Whether two nodes look like repeats of one another: same structure, boxes of about the same
/// size, text in the same size (what the text says may differ).
pub(crate) fn similar(els: &[El], a: &Node, b: &Node) -> bool {
    let (ra, rb) = (a.bounds(els), b.bounds(els));
    match (a, b) {
        (Node::Text(i), Node::Text(j)) => match (els.get(*i), els.get(*j)) {
            (Some(x), Some(y)) => close(x.b.width(), y.b.width()) && close(x.size, y.size) && x.align == y.align,
            _ => false,
        },
        (Node::Rigid(x), Node::Rigid(y)) => x.len() == y.len() && close(ra.width(), rb.width()) && close(ra.height(), rb.height()),
        (Node::Section(_), Node::Section(_)) => close(ra.width(), rb.width()) && close(ra.height(), rb.height()),
        (Node::Stack(x), Node::Stack(y)) | (Node::Row(x), Node::Row(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| similar(els, p, q)),
        (Node::Grid(x, c), Node::Grid(y, d)) => c == d && x.len() == y.len() && x.iter().zip(y).all(|(p, q)| similar(els, p, q)),
        _ => false,
    }
}
