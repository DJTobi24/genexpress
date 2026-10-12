//! Photo-grid / collage layouts.

use genha_geom::{Frame, Rect};

/// Most photos a collage layout holds.
pub const MAX_COLLAGE: usize = 9;

/// A collage layout: one cell per photo, in `0..=1` coordinates of the collage area.
#[derive(Clone, Debug, PartialEq)]
pub struct CollageLayout {
    pub id: &'static str,
    pub label: &'static str,
    pub cells: Vec<Rect>,
}

/// `n` equal slices of `0..1`.
fn slices(n: usize) -> Vec<(f64, f64)> {
    let n = n.max(1);
    (0..n).map(|i| (i as f64 / n as f64, (i + 1) as f64 / n as f64)).collect()
}

fn columns(n: usize, x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<Rect> {
    slices(n).into_iter().map(|(a, b)| Rect::new(x0 + a * (x1 - x0), y0, x0 + b * (x1 - x0), y1)).collect()
}

fn rows(n: usize, x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<Rect> {
    slices(n).into_iter().map(|(a, b)| Rect::new(x0, y0 + a * (y1 - y0), x1, y0 + b * (y1 - y0))).collect()
}

/// A near-square grid; the last row's cells widen to fill it.
fn grid(n: usize) -> Vec<Rect> {
    let cols = (1..=n).find(|c| c * c >= n).unwrap_or(1);
    let row_count = n.div_ceil(cols);
    let mut cells = vec![];
    for (r, (y0, y1)) in slices(row_count).into_iter().enumerate() {
        let in_row = (n - r * cols).min(cols);
        cells.extend(columns(in_row, 0.0, 1.0, y0, y1));
    }
    cells
}

/// The layouts available for `count` photos (clamped to `1..=MAX_COLLAGE`), simplest first.
pub fn collage_layouts(count: usize) -> Vec<CollageLayout> {
    let n = count.clamp(1, MAX_COLLAGE);
    let l = |id, label, cells| CollageLayout { id, label, cells };
    if n == 1 {
        return vec![l("single", "Single", vec![Rect::new(0.0, 0.0, 1.0, 1.0)])];
    }
    let g = grid(n);
    let cols = columns(n, 0.0, 1.0, 0.0, 1.0);
    let distinct = g != cols;
    let mut v = vec![l("grid", "Grid", g)];
    if n <= 5 {
        if distinct {
            v.push(l("columns", "Columns", cols));
        }
        v.push(l("rows", "Rows", rows(n, 0.0, 1.0, 0.0, 1.0)));
    }
    // One large photo with the others in a strip.
    let rest = n - 1;
    let mut hero_left = vec![Rect::new(0.0, 0.0, 0.62, 1.0)];
    hero_left.extend(rows(rest, 0.62, 1.0, 0.0, 1.0));
    v.push(l("hero-left", "Feature left", hero_left));
    let mut hero_top = vec![Rect::new(0.0, 0.0, 1.0, 0.62)];
    hero_top.extend(columns(rest, 0.0, 1.0, 0.62, 1.0));
    v.push(l("hero-top", "Feature top", hero_top));
    if n >= 3 {
        // Two unequal columns with different row counts, so the cells are staggered.
        let left = n / 2;
        let mut mosaic = rows(left, 0.0, 0.56, 0.0, 1.0);
        mosaic.extend(rows(n - left, 0.56, 1.0, 0.0, 1.0));
        v.push(l("mosaic", "Mosaic", mosaic));
    }
    if n >= 4 {
        // A large photo in the middle row, the others split above and below.
        let above = (n - 1) / 2;
        let below = n - 1 - above;
        let mut cells = columns(above, 0.0, 1.0, 0.0, 0.25);
        cells.push(Rect::new(0.0, 0.25, 1.0, 0.75));
        cells.extend(columns(below, 0.0, 1.0, 0.75, 1.0));
        v.push(l("spotlight", "Spotlight", cells));
    }
    v
}

/// The cells of `layout` placed in `area` with `gap` pixels between neighbouring cells (cells
/// touching the area's edge stay flush with it).
pub fn cells_in(area: Rect, layout: &CollageLayout, gap: f64) -> Vec<Frame> {
    let area = area.abs();
    let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
    let half = gap / 2.0;
    let edge = |v: f64| v <= 1e-9 || v >= 1.0 - 1e-9;
    layout
        .cells
        .iter()
        .map(|c| {
            let inset = |v: f64, dir: f64| if edge(v) { 0.0 } else { half * dir };
            let x0 = area.x0 + c.x0.clamp(0.0, 1.0) * area.width() + inset(c.x0, 1.0);
            let x1 = area.x0 + c.x1.clamp(0.0, 1.0) * area.width() + inset(c.x1, -1.0);
            let y0 = area.y0 + c.y0.clamp(0.0, 1.0) * area.height() + inset(c.y0, 1.0);
            let y1 = area.y0 + c.y1.clamp(0.0, 1.0) * area.height() + inset(c.y1, -1.0);
            Frame::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)).sanitized()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_tile_the_area() {
        for n in 1..=MAX_COLLAGE {
            let layouts = collage_layouts(n);
            assert!(!layouts.is_empty());
            let mut ids = std::collections::HashSet::new();
            for l in &layouts {
                assert!(ids.insert(l.id), "duplicate {} for {n}", l.id);
                assert_eq!(l.cells.len(), n, "{} for {n}", l.id);
                let area: f64 = l.cells.iter().map(|c| c.area()).sum();
                assert!((area - 1.0).abs() < 1e-9, "{} for {n} covers {area}", l.id);
                for c in &l.cells {
                    assert!(c.x0 >= 0.0 && c.y0 >= 0.0 && c.x1 <= 1.0 && c.y1 <= 1.0 && c.width() > 0.0 && c.height() > 0.0);
                }
            }
            if n >= 3 {
                assert!(layouts.len() >= 4, "{n} photos get several layouts");
            }
        }
        assert_eq!(collage_layouts(0)[0].cells.len(), 1);
        assert_eq!(collage_layouts(50)[0].cells.len(), MAX_COLLAGE);
    }

    #[test]
    fn cells_with_gaps() {
        let grid = collage_layouts(4).into_iter().find(|l| l.id == "grid").unwrap();
        let cells = cells_in(Rect::new(0.0, 0.0, 210.0, 110.0), &grid, 10.0);
        assert_eq!(cells[0], Frame::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(cells[1], Frame::new(110.0, 0.0, 100.0, 50.0));
        assert_eq!(cells[3], Frame::new(110.0, 60.0, 100.0, 50.0));
        let tight = cells_in(Rect::new(0.0, 0.0, 4.0, 4.0), &grid, 1e6);
        assert!(tight.iter().all(|f| f.is_valid()));
    }
}
