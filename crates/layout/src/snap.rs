//! Smart guides: snapping a moving element to the page and to other elements.

use genexpress_doc::{ElementKind, NodeId, Page};
use genexpress_geom::{Frame, Rect};

/// Positions an element may snap to: vertical guide lines (`xs`) and horizontal ones (`ys`),
/// sorted and without duplicates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapGuides {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
}

/// Most elements considered for guides (keeps dragging fast on crowded pages).
const MAX_GUIDE_ELEMENTS: usize = 2000;

fn contains_any(el: &genexpress_doc::Element, exclude: &[NodeId]) -> bool {
    if exclude.contains(&el.id) {
        return true;
    }
    match &el.kind {
        ElementKind::Group(g) => g.children.iter().any(|c| contains_any(c, exclude)),
        _ => false,
    }
}

/// The page's edges and centre lines plus the left/centre/right and top/middle/bottom lines of
/// every visible top-level element, except those in `exclude` (typically the selection being
/// dragged; a group containing an excluded element is skipped too).
pub fn snap_guides(page: &Page, exclude: &[NodeId]) -> SnapGuides {
    let (w, h) = (page.width, page.height);
    let mut xs = vec![0.0, w / 2.0, w];
    let mut ys = vec![0.0, h / 2.0, h];
    for el in page.elements.iter().filter(|e| !e.hidden && !contains_any(e, exclude)).take(MAX_GUIDE_ELEMENTS) {
        let b = el.frame.sanitized().bounds();
        xs.extend([b.x0, b.center().x, b.x1]);
        ys.extend([b.y0, b.center().y, b.y1]);
    }
    let tidy = |v: &mut Vec<f64>| {
        v.retain(|x| x.is_finite());
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    };
    tidy(&mut xs);
    tidy(&mut ys);
    SnapGuides { xs, ys }
}

/// The closest guide to any of `edges` within `threshold`, as (shift, guide).
fn best(edges: [f64; 3], guides: &[f64], threshold: f64) -> Option<(f64, f64)> {
    let mut found: Option<(f64, f64)> = None;
    for e in edges {
        let (_, hit) = genexpress_geom::snap(e, guides, threshold);
        if let Some(g) = hit
            && found.is_none_or(|(d, _)| (g - e).abs() < d.abs())
        {
            found = Some((g - e, g));
        }
    }
    found
}

/// Moves `frame` so the closest of its left/centre/right lines lands on an x guide and the closest
/// of its top/middle/bottom lines on a y guide, when within `threshold` page pixels. Returns the
/// snapped frame and the x and y guides it now touches (to draw them).
pub fn snap_frame(frame: Frame, guides: &SnapGuides, threshold: f64) -> (Frame, Vec<f64>, Vec<f64>) {
    let frame = frame.sanitized();
    if !threshold.is_finite() || threshold < 0.0 {
        return (frame, vec![], vec![]);
    }
    let lines = |b: Rect| ([b.x0, b.center().x, b.x1], [b.y0, b.center().y, b.y1]);
    let (bx, by) = lines(frame.bounds());
    let dx = best(bx, &guides.xs, threshold).map_or(0.0, |(d, _)| d);
    let dy = best(by, &guides.ys, threshold).map_or(0.0, |(d, _)| d);
    let snapped = frame.translated(dx, dy);
    let (bx, by) = lines(snapped.bounds());
    let touching =
        |edges: [f64; 3], guides: &[f64]| -> Vec<f64> { guides.iter().copied().filter(|g| edges.iter().any(|e| (g - e).abs() < 1e-6)).collect() };
    let hx = touching(bx, &guides.xs);
    let hy = touching(by, &guides.ys);
    (snapped, hx, hy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use genexpress_color::Color;
    use genexpress_doc::{Element, Fill, ShapeEl, ShapeKind};

    fn page() -> Page {
        let mut p = Page::new(NodeId(1), "P", 1000.0, 800.0);
        for (id, f) in [(2, Frame::new(100.0, 100.0, 200.0, 100.0)), (3, Frame::new(600.0, 500.0, 100.0, 100.0))] {
            p.elements.push(Element::new(
                NodeId(id),
                f,
                ElementKind::Shape(ShapeEl { shape: ShapeKind::Ellipse, fill: Fill::solid(Color::BLACK), stroke: None, shadow: None }),
            ));
        }
        p
    }

    #[test]
    fn guides_include_page_and_elements() {
        let g = snap_guides(&page(), &[NodeId(3)]);
        assert_eq!(g.xs, vec![0.0, 100.0, 200.0, 300.0, 500.0, 1000.0]);
        assert_eq!(g.ys, vec![0.0, 100.0, 150.0, 200.0, 400.0, 800.0]);
        let all = snap_guides(&page(), &[]);
        assert!(all.xs.contains(&650.0));
    }

    #[test]
    fn snapping_moves_to_nearest_guide() {
        let g = snap_guides(&page(), &[NodeId(3)]);
        // Right edge at 297 → snaps to 300; middle at 402 → snaps to the page centre 400.
        let (f, hx, hy) = snap_frame(Frame::new(197.0, 352.0, 100.0, 100.0), &g, 5.0);
        assert_eq!((f.x, f.y), (200.0, 350.0));
        assert_eq!(hx, vec![200.0, 300.0]);
        assert_eq!(hy, vec![400.0]);
        // Too far: nothing moves.
        let (f, hx, hy) = snap_frame(Frame::new(420.0, 620.0, 10.0, 10.0), &g, 5.0);
        assert_eq!((f.x, f.y), (420.0, 620.0));
        assert!(hx.is_empty() && hy.is_empty());
        let (f, ..) = snap_frame(Frame::new(1.0, 1.0, 10.0, 10.0), &g, f64::NAN);
        assert_eq!(f.x, 1.0);
    }
}
