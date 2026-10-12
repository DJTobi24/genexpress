//! "Resize to another size" that keeps a design readable.

use genha_doc::{Element, ElementKind, MAX_PAGE_SIDE, Page, ShapeKind, TextAlign, TextEffect};
use genha_geom::{Frame, Rect};

/// Share of the old page an element must cover to count as a full-bleed background.
const FULL_BLEED: f64 = 0.9;
/// An element whose gaps to both page edges on an axis are at most this share of the page spans
/// that axis (inset frames, header bands, sidebars, footers).
const SPAN_MARGIN: f64 = 0.1;
/// An element counts as centred when its centre is off the page centre by at most this share of
/// its smaller edge gap.
const CENTRE_SLACK: f64 = 0.1;
/// Rough advance of a character in ems, to estimate where text actually is inside its box.
const EM_PER_CHAR: f64 = 0.55;

/// A per-axis scale and offset applied to frames: `p' = p * scale + offset`.
#[derive(Clone, Copy, Debug)]
struct Map {
    sx: f64,
    sy: f64,
    tx: f64,
    ty: f64,
}

impl Map {
    fn frame(&self, f: &Frame) -> Frame {
        Frame { x: f.x * self.sx + self.tx, y: f.y * self.sy + self.ty, width: f.width * self.sx, height: f.height * self.sy, rotation: f.rotation }
    }
}

fn side(v: f64, fallback: f64) -> f64 {
    if v.is_finite() { v.clamp(1.0, MAX_PAGE_SIDE) } else { fallback }
}

/// One axis of the resize: old and new page length and the uniform scale.
#[derive(Clone, Copy)]
struct AxisMap {
    old: f64,
    new: f64,
    s: f64,
}

impl AxisMap {
    /// Whether `a..b` reaches within [`SPAN_MARGIN`] of both edges.
    fn spans(&self, a: f64, b: f64) -> bool {
        a <= SPAN_MARGIN * self.old && self.old - b <= SPAN_MARGIN * self.old
    }

    /// (scale, offset) that keeps the gaps to both edges (scaled by `s`) and stretches between.
    fn stretch(&self, a: f64, b: f64) -> (f64, f64) {
        let (n0, n1) = (a * self.s, self.new - (self.old - b) * self.s);
        let k = if b - a > 1e-9 { ((n1 - n0) / (b - a)).max(0.0) } else { self.s };
        (k, n0 - a * k)
    }

    /// (scale, offset) that scales by `s` and keeps the extent `a..b` (where the content visibly
    /// is) at the same distance from the nearest of start edge, centre or end edge.
    fn anchor(&self, a: f64, b: f64) -> (f64, f64) {
        let s = self.s;
        let size = (b - a) * s;
        let lead = a;
        let trail = self.old - b;
        let centre_off = (a + b) / 2.0 - self.old / 2.0;
        let start = if centre_off.abs() <= CENTRE_SLACK * lead.abs().min(trail.abs()) + 1e-6 * self.old {
            self.new / 2.0 + centre_off * s - size / 2.0
        } else if lead.abs() <= trail.abs() {
            lead * s
        } else {
            self.new - trail * s - size
        };
        (s, start - a * s)
    }
}

/// A copy of `page` resized to `width` × `height` (clamped to `1..=MAX_PAGE_SIDE`), keeping the
/// design readable:
///
/// - everything is scaled uniformly by the smaller of the two ratios, so text, logos and shapes are
///   never distorted (font sizes, strokes, corner radii and shadows scale too);
/// - each element is re-anchored to whichever is nearest of its page edges or the page centre, on
///   each axis: an element hugging the bottom-right corner stays there, a centred title stays
///   centred. Text is anchored by where its letters roughly are (from its alignment), so a
///   left-aligned headline in a wide box stays aligned with the other left-aligned text;
/// - full-bleed backgrounds (non-text elements covering ≥ 90 % of the old page, or reaching within
///   10 % of all four edges, like an inset border) are stretched to the new page, keeping their
///   scaled margins (pictures keep their aspect ratio through their cover fit);
/// - rectangles, lines, paths and pictures that span one axis (a header band, a sidebar, a
///   footer) stretch along that axis and are anchored on the other;
/// - groups' children move and scale with their group;
/// - ids, names, order and the page background are kept.
pub fn smart_resize(page: &Page, width: f64, height: f64) -> Page {
    let ow = side(page.width, 1080.0);
    let oh = side(page.height, 1080.0);
    let nw = side(width, ow);
    let nh = side(height, oh);
    let s = (nw / ow).min(nh / oh);
    let (xa, ya) = (AxisMap { old: ow, new: nw, s }, AxisMap { old: oh, new: nh, s });
    let page_rect = Rect::new(0.0, 0.0, ow, oh);
    let mut out = page.clone();
    out.width = nw;
    out.height = nh;
    // First pass: how each top-level element moves on its own.
    let mut plans: Vec<(Map, bool, Rect, bool)> = Vec::with_capacity(out.elements.len());
    for el in &out.elements {
        let f = el.frame.sanitized();
        let b = f.bounds();
        let covered = b.intersect(page_rect);
        let covered_area = if covered.width() > 0.0 && covered.height() > 0.0 { covered.area() } else { 0.0 };
        let is_text = matches!(el.kind, ElementKind::Text(_));
        let full_bleed = !is_text && (covered_area >= FULL_BLEED * ow * oh || (xa.spans(b.x0, b.x1) && ya.spans(b.y0, b.y1)));
        let stretchy = match &el.kind {
            ElementKind::Shape(sh) => matches!(sh.shape, ShapeKind::Rect { .. } | ShapeKind::Line | ShapeKind::Path { .. }),
            ElementKind::Image(_) => true,
            ElementKind::Text(_) | ElementKind::Group(_) | ElementKind::Chart(_) | ElementKind::Table(_) => false,
        };
        let (ix0, ix1) = ink_x(el, &f, b);
        let (sx, tx) = if full_bleed || (stretchy && xa.spans(b.x0, b.x1)) { xa.stretch(b.x0, b.x1) } else { xa.anchor(ix0, ix1) };
        let (sy, ty) = if full_bleed || (stretchy && ya.spans(b.y0, b.y1)) { ya.stretch(b.y0, b.y1) } else { ya.anchor(b.y0, b.y1) };
        let uniform = (sx - s).abs() < 1e-9 && (sy - s).abs() < 1e-9;
        plans.push((Map { sx, sy, tx, ty }, uniform, b, matches!(el.kind, ElementKind::Shape(_) | ElementKind::Image(_))));
    }
    // Second pass: an element sitting on a card, pill or disc (a shape or picture that contains it
    // and is only scaled, not stretched) moves with it, so labels stay on their buttons. Larger
    // elements are resolved first, so nested cards follow their outer card.
    let mut order: Vec<usize> = (0..plans.len()).collect();
    order.sort_by(|&a, &b| {
        let area = |i: usize| plans.get(i).map_or(0.0, |p| p.2.area());
        area(b).total_cmp(&area(a))
    });
    let mut resolved: Vec<Option<Map>> = vec![None; plans.len()];
    for (k, &i) in order.iter().enumerate() {
        let Some(&(own, _, bi, _)) = plans.get(i) else { continue };
        let container = order
            .get(..k)
            .unwrap_or(&[])
            .iter()
            .rev()
            .filter_map(|&j| plans.get(j).map(|p| (j, p)))
            .find(|(_, (_, uniform, bj, is_box))| *uniform && *is_box && bj.area() > bi.area() && contains(*bj, bi))
            .and_then(|(j, _)| resolved.get(j).copied().flatten());
        if let Some(slot) = resolved.get_mut(i) {
            *slot = Some(container.unwrap_or(own));
        }
    }
    for (el, map) in out.elements.iter_mut().zip(resolved) {
        el.frame = el.frame.sanitized();
        if let Some(map) = map {
            apply(el, &map, s);
        }
    }
    out
}

/// Whether `outer` contains `inner`, with a pixel of slack.
fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x0 >= outer.x0 - 1.0 && inner.y0 >= outer.y0 - 1.0 && inner.x1 <= outer.x1 + 1.0 && inner.y1 <= outer.y1 + 1.0
}

/// Where the letters of an unrotated text element roughly are horizontally (its bounds for
/// anything else).
fn ink_x(el: &Element, f: &Frame, b: Rect) -> (f64, f64) {
    let ElementKind::Text(t) = &el.kind else { return (b.x0, b.x1) };
    if f.rotation != 0.0 {
        return (b.x0, b.x1);
    }
    let st = &t.style;
    let spacing = 1.0 + (st.letter_spacing / 1000.0).max(0.0);
    let chars = t.text.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f64;
    let est = (chars * EM_PER_CHAR * st.font_size * spacing).clamp(0.0, b.width());
    if !est.is_finite() {
        return (b.x0, b.x1);
    }
    match st.align {
        TextAlign::Left => (b.x0, b.x0 + est),
        TextAlign::Center => (b.center().x - est / 2.0, b.center().x + est / 2.0),
        TextAlign::Right => (b.x1 - est, b.x1),
    }
}

fn apply(el: &mut Element, map: &Map, s: f64) {
    el.frame = map.frame(&el.frame);
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
            t.style.font_size = (t.style.font_size * s).max(1.0);
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
                c.frame = c.frame.sanitized();
                apply(c, map, s);
            }
        }
        ElementKind::Chart(c) => c.font_size = (c.font_size * s).max(1.0),
        ElementKind::Table(t) => {
            t.font_size = (t.font_size * s).max(1.0);
            k(&mut t.padding);
            if let Some(st) = &mut t.border {
                k(&mut st.width);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genha_color::Color;
    use genha_doc::{Fill, GroupEl, NodeId, ShapeEl, TextEl, TextStyle};

    fn shape(id: u64, f: Frame) -> Element {
        Element::new(
            NodeId(id),
            f,
            ElementKind::Shape(ShapeEl {
                shape: ShapeKind::Rect { corner_radius: 10.0 },
                fill: Fill::solid(Color::BLACK),
                stroke: None,
                shadow: None,
            }),
        )
    }

    fn page() -> Page {
        let mut p = Page::new(NodeId(1), "P", 1000.0, 1000.0);
        p.elements.push(shape(2, Frame::new(0.0, 0.0, 1000.0, 1000.0)));
        let style = TextStyle { font_size: 80.0, ..TextStyle::default() };
        p.elements.push(Element::new(
            NodeId(3),
            Frame::new(200.0, 450.0, 600.0, 100.0),
            ElementKind::Text(TextEl { text: "Title".into(), style, effect: TextEffect::Outline { color: Color::WHITE, width: 4.0 }, curve: 0.0 }),
        ));
        p.elements.push(shape(4, Frame::new(860.0, 860.0, 100.0, 100.0))); // bottom-right logo
        p.elements.push(shape(5, Frame::new(40.0, 40.0, 100.0, 50.0))); // top-left badge
        let children = vec![shape(7, Frame::new(40.0, 800.0, 50.0, 50.0)), shape(8, Frame::new(100.0, 800.0, 50.0, 50.0))];
        p.elements.push(Element::new(NodeId(6), Frame::new(40.0, 800.0, 110.0, 50.0), ElementKind::Group(GroupEl { children })));
        p
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn square_to_wide_banner() {
        let p = smart_resize(&page(), 2000.0, 500.0);
        assert_eq!((p.width, p.height), (2000.0, 500.0));
        assert_eq!(p.ids(), page().ids());
        let s = 0.5;
        // Background stretched.
        assert_eq!(p.elements[0].frame, Frame::new(0.0, 0.0, 2000.0, 500.0));
        // Title stays centred and its font scales.
        let t = &p.elements[1];
        assert!(close(t.frame.center().x, 1000.0) && close(t.frame.center().y, 250.0), "{:?}", t.frame);
        assert!(close(t.frame.width, 300.0));
        match &t.kind {
            ElementKind::Text(tx) => {
                assert!(close(tx.style.font_size, 80.0 * s));
                assert_eq!(tx.effect, TextEffect::Outline { color: Color::WHITE, width: 2.0 });
            }
            _ => panic!(),
        }
        // Logo hugs the bottom-right corner with a scaled margin.
        let l = p.elements[2].frame;
        assert!(close(l.x + l.width, 2000.0 - 40.0 * s) && close(l.y + l.height, 500.0 - 40.0 * s), "{l:?}");
        assert!(close(l.width, 50.0) && close(l.height, 50.0));
        // Top-left badge stays top-left.
        let b = p.elements[3].frame;
        assert!(close(b.x, 20.0) && close(b.y, 20.0));
        match &p.elements[3].kind {
            ElementKind::Shape(sh) => assert_eq!(sh.shape, ShapeKind::Rect { corner_radius: 5.0 }),
            _ => panic!(),
        }
    }

    #[test]
    fn groups_move_their_children() {
        let p = smart_resize(&page(), 1000.0, 2000.0);
        let g = &p.elements[4];
        let ElementKind::Group(gr) = &g.kind else { panic!() };
        // Scale 1: the group (nearest the left edge and bottom) keeps its left gap and bottom gap.
        assert!(close(g.frame.x, 40.0) && close(g.frame.y + g.frame.height, 2000.0 - 150.0), "{:?}", g.frame);
        assert!(close(gr.children[0].frame.x, g.frame.x) && close(gr.children[0].frame.y, g.frame.y));
        assert!(close(gr.children[1].frame.x - gr.children[0].frame.x, 60.0));
    }

    #[test]
    fn everything_stays_inside_and_sizes_are_clamped() {
        for (w, h) in [(300.0, 1200.0), (1584.0, 396.0), (f64::NAN, 50.0), (1e9, -5.0)] {
            let p = smart_resize(&page(), w, h);
            assert!(p.width >= 1.0 && p.width <= MAX_PAGE_SIDE && p.height >= 1.0 && p.height <= MAX_PAGE_SIDE);
            let r = Rect::new(-1e-6, -1e-6, p.width + 1e-6, p.height + 1e-6);
            p.for_each(&mut |e| {
                let b = e.frame.bounds();
                assert!(r.contains(b.origin()) && r.contains(Point::new(b.x1, b.y1)), "{w}x{h}: {:?} outside", e.frame);
            });
        }
    }

    #[test]
    fn frames_bands_and_left_aligned_text() {
        let mut p = Page::new(NodeId(1), "P", 1000.0, 1000.0);
        p.elements.push(shape(2, Frame::new(50.0, 50.0, 900.0, 900.0))); // inset border
        p.elements.push(shape(3, Frame::new(0.0, 0.0, 300.0, 1000.0))); // sidebar
        p.elements.push(Element::new(
            NodeId(4),
            Frame::new(40.0, 0.0, 40.0, 40.0),
            ElementKind::Shape(ShapeEl { shape: ShapeKind::Ellipse, fill: Fill::None, stroke: None, shadow: None }),
        ));
        let style = TextStyle { font_size: 40.0, align: genha_doc::TextAlign::Left, ..TextStyle::default() };
        p.elements.push(Element::new(
            NodeId(5),
            Frame::new(100.0, 400.0, 800.0, 50.0),
            ElementKind::Text(TextEl { text: "Short".into(), style, effect: TextEffect::None, curve: 0.0 }),
        ));
        let r = smart_resize(&p, 1000.0, 2000.0);
        // The inset border follows the page with its margins.
        assert_eq!(r.elements[0].frame, Frame::new(50.0, 50.0, 900.0, 1900.0));
        // The sidebar stays full height and keeps its width.
        assert_eq!(r.elements[1].frame, Frame::new(0.0, 0.0, 300.0, 2000.0));
        // A circle never stretches.
        assert_eq!((r.elements[2].frame.width, r.elements[2].frame.height), (40.0, 40.0));
        // Left-aligned text in a box wider than its words keeps its left edge.
        assert_eq!(r.elements[3].frame.x, 100.0);
    }

    #[test]
    fn labels_stay_on_their_buttons() {
        let mut p = Page::new(NodeId(1), "P", 1000.0, 1000.0);
        p.elements.push(shape(2, Frame::new(380.0, 900.0, 300.0, 60.0))); // a centred pill
        let style = TextStyle { font_size: 40.0, align: genha_doc::TextAlign::Left, ..TextStyle::default() };
        p.elements.push(Element::new(
            NodeId(3),
            Frame::new(400.0, 910.0, 260.0, 40.0),
            ElementKind::Text(TextEl { text: "Go".into(), style, effect: TextEffect::None, curve: 0.0 }),
        ));
        let r = smart_resize(&p, 2000.0, 1000.0);
        let (pill, label) = (r.elements[0].frame, r.elements[1].frame);
        assert!(close(pill.center().x, 1030.0), "{pill:?}");
        assert!(close(label.x - pill.x, 20.0) && close(label.y - pill.y, 10.0), "{label:?}");
    }

    use genha_geom::Point;
}
