//! Align, distribute and tidy a selection.

use genexpress_geom::{Frame, Rect};
use serde::{Deserialize, Serialize};

/// How to align a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    Left,
    CenterH,
    Right,
    Top,
    CenterV,
    Bottom,
}

impl Align {
    pub const ALL: [Align; 6] = [Align::Left, Align::CenterH, Align::Right, Align::Top, Align::CenterV, Align::Bottom];

    /// `left`, `center-h`, `right`, `top`, `center-v` or `bottom` (as serialized).
    pub fn id(self) -> &'static str {
        match self {
            Align::Left => "left",
            Align::CenterH => "center-h",
            Align::Right => "right",
            Align::Top => "top",
            Align::CenterV => "center-v",
            Align::Bottom => "bottom",
        }
    }

    pub fn from_id(id: &str) -> Option<Align> {
        Align::ALL.into_iter().find(|a| a.id() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Axis {
    Horizontal,
    Vertical,
}

fn finite_rect(r: Rect) -> bool {
    [r.x0, r.y0, r.x1, r.y1].iter().all(|v| v.is_finite())
}

/// The union of the frames' (rotated) bounds, or `None` for no frames.
pub fn selection_bounds(frames: &[Frame]) -> Option<Rect> {
    frames.iter().map(|f| f.sanitized().bounds()).reduce(|a, b| a.union(b))
}

/// Aligns each frame's (rotated) bounds to `to` — the page rectangle when aligning a single
/// element or "to page" — or, when `to` is `None` (or not finite), to the selection's bounds.
/// Frames are only moved.
pub fn align(frames: &[Frame], how: Align, to: Option<Rect>) -> Vec<Frame> {
    let target = match to.filter(|r| finite_rect(*r)).map(|r| r.abs()).or_else(|| selection_bounds(frames)) {
        Some(t) => t,
        None => return vec![],
    };
    frames
        .iter()
        .map(|f| {
            let f = f.sanitized();
            let b = f.bounds();
            let (dx, dy) = match how {
                Align::Left => (target.x0 - b.x0, 0.0),
                Align::CenterH => (target.center().x - b.center().x, 0.0),
                Align::Right => (target.x1 - b.x1, 0.0),
                Align::Top => (0.0, target.y0 - b.y0),
                Align::CenterV => (0.0, target.center().y - b.center().y),
                Align::Bottom => (0.0, target.y1 - b.y1),
            };
            f.translated(dx, dy)
        })
        .collect()
}

/// Spreads the frames along `axis` so the gaps between them are equal, keeping the outermost
/// positions of the selection. Fewer than three frames are returned unchanged.
pub fn distribute(frames: &[Frame], axis: Axis) -> Vec<Frame> {
    let frames: Vec<Frame> = frames.iter().map(Frame::sanitized).collect();
    if frames.len() < 3 {
        return frames;
    }
    let span = |b: Rect| match axis {
        Axis::Horizontal => (b.x0, b.x1),
        Axis::Vertical => (b.y0, b.y1),
    };
    let mut order: Vec<(usize, f64, f64)> = frames.iter().enumerate().map(|(i, f)| (i, span(f.bounds()).0, span(f.bounds()).1)).collect();
    order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let start = order.iter().map(|o| o.1).fold(f64::INFINITY, f64::min);
    let end = order.iter().map(|o| o.2).fold(f64::NEG_INFINITY, f64::max);
    let total: f64 = order.iter().map(|o| o.2 - o.1).sum();
    let gap = (end - start - total) / (order.len() - 1) as f64;
    let mut out = frames.clone();
    let mut pos = start;
    for (i, a, b) in order {
        if let Some(f) = out.get_mut(i) {
            *f = match axis {
                Axis::Horizontal => f.translated(pos - a, 0.0),
                Axis::Vertical => f.translated(0.0, pos - a),
            };
        }
        pos += (b - a) + gap;
    }
    out
}

/// Arranges the frames in a tidy grid of `columns` columns (clamped to `1..=frames.len()`) with
/// `gap` between cells, in reading order (top to bottom, then left to right), starting at the
/// selection's top-left corner. Every cell is as big as the largest frame; frames are centred in
/// their cells and keep their size.
pub fn tidy_grid(frames: &[Frame], columns: usize, gap: f64) -> Vec<Frame> {
    let frames: Vec<Frame> = frames.iter().map(Frame::sanitized).collect();
    let Some(sel) = selection_bounds(&frames) else { return vec![] };
    let columns = columns.clamp(1, frames.len());
    let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
    let bounds: Vec<Rect> = frames.iter().map(Frame::bounds).collect();
    let cw = bounds.iter().map(Rect::width).fold(0.0, f64::max);
    let ch = bounds.iter().map(Rect::height).fold(0.0, f64::max);
    let mut order: Vec<usize> = (0..frames.len()).collect();
    order.sort_by(|&a, &b| {
        let (ra, rb) = (bounds.get(a).copied().unwrap_or_default(), bounds.get(b).copied().unwrap_or_default());
        ra.y0.total_cmp(&rb.y0).then(ra.x0.total_cmp(&rb.x0)).then(a.cmp(&b))
    });
    let mut out = frames.clone();
    for (k, i) in order.into_iter().enumerate() {
        let (col, row) = ((k % columns) as f64, (k / columns) as f64);
        let cell_cx = sel.x0 + col * (cw + gap) + cw / 2.0;
        let cell_cy = sel.y0 + row * (ch + gap) + ch / 2.0;
        if let (Some(f), Some(b)) = (out.get_mut(i), bounds.get(i)) {
            *f = f.translated(cell_cx - b.center().x, cell_cy - b.center().y);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> Vec<Frame> {
        vec![Frame::new(10.0, 10.0, 50.0, 20.0), Frame::new(100.0, 40.0, 30.0, 30.0), Frame::new(300.0, 0.0, 100.0, 60.0)]
    }

    #[test]
    fn align_to_selection_and_page() {
        let l = align(&frames(), Align::Left, None);
        assert!(l.iter().all(|f| f.x == 10.0));
        let r = align(&frames(), Align::Right, None);
        assert!(r.iter().all(|f| f.x + f.width == 400.0));
        let b = align(&frames(), Align::Bottom, None);
        assert!(b.iter().all(|f| f.y + f.height == 70.0));
        let page = Rect::new(0.0, 0.0, 1000.0, 500.0);
        let c = align(&frames()[..1], Align::CenterH, Some(page));
        assert_eq!(c[0].center().x, 500.0);
        let v = align(&frames()[..1], Align::CenterV, Some(page));
        assert_eq!(v[0].center().y, 250.0);
        assert_eq!(align(&frames(), Align::Top, Some(page)).iter().map(|f| f.y).collect::<Vec<_>>(), vec![0.0; 3]);
        assert!(align(&[], Align::Left, None).is_empty());
        // Rotated frames align by their visible bounds.
        let rot = Frame { rotation: 90.0, ..Frame::new(100.0, 100.0, 100.0, 20.0) };
        let a = align(&[rot], Align::Top, Some(page));
        assert!(a[0].bounds().y0.abs() < 1e-9);
    }

    #[test]
    fn align_ids_round_trip() {
        for a in Align::ALL {
            assert_eq!(Align::from_id(a.id()), Some(a));
            assert_eq!(serde_json::to_string(&a).unwrap(), format!("\"{}\"", a.id()));
        }
        assert_eq!(Align::from_id("middle"), None);
        assert_eq!(serde_json::to_string(&Axis::Vertical).unwrap(), "\"vertical\"");
    }

    #[test]
    fn distribute_equal_gaps() {
        let d = distribute(&frames(), Axis::Horizontal);
        assert_eq!(d[0].x, 10.0);
        assert_eq!(d[2].x + d[2].width, 400.0);
        let g1 = d[1].x - (d[0].x + d[0].width);
        let g2 = d[2].x - (d[1].x + d[1].width);
        assert!((g1 - g2).abs() < 1e-9 && g1 > 0.0);
        assert!(d.iter().zip(frames()).all(|(a, b)| a.y == b.y));
        let v = distribute(&frames(), Axis::Vertical);
        assert!(v.iter().zip(frames()).all(|(a, b)| a.x == b.x));
        assert_eq!(distribute(&frames()[..2], Axis::Horizontal), frames()[..2].to_vec());
    }

    #[test]
    fn tidy_grid_rows_and_columns() {
        let fs: Vec<Frame> = (0..5).map(|i| Frame::new(f64::from(i) * 37.0, f64::from(5 - i) * 13.0, 40.0, 30.0)).collect();
        let g = tidy_grid(&fs, 2, 10.0);
        assert_eq!(g.len(), 5);
        let mut xs: Vec<f64> = g.iter().map(|f| f.x).collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        assert_eq!(xs.len(), 2);
        let mut ys: Vec<f64> = g.iter().map(|f| f.y).collect();
        ys.sort_by(f64::total_cmp);
        ys.dedup();
        assert_eq!(ys, vec![13.0, 53.0, 93.0]);
        assert!(tidy_grid(&[], 3, 1.0).is_empty());
        assert_eq!(tidy_grid(&fs, 0, f64::NAN).iter().map(|f| f.x).collect::<Vec<_>>(), vec![0.0; 5]);
    }
}
