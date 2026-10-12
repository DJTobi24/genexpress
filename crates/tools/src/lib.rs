//! Tools (layer L4): state machines that turn pointer gestures into commands.
//!
//! A tool never edits the document. While a gesture runs it returns a [`ToolOutput::preview`]
//! (frames the UI shows instead of the stored ones, a marquee, snap guides); when the gesture ends
//! it returns [`Invocation`]s, which the engine runs like any other command, so every gesture is
//! undoable, journaled and reachable by agents.
#![forbid(unsafe_code)]

use genexpress_doc::{Document, Element, NodeId, Page};
use genexpress_geom::{Frame, Point, Rect, Vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolId {
    #[default]
    Select,
    Rect,
    Ellipse,
    Line,
    Text,
    Hand,
}

impl ToolId {
    pub const ALL: [ToolId; 6] = [ToolId::Select, ToolId::Rect, ToolId::Ellipse, ToolId::Line, ToolId::Text, ToolId::Hand];

    pub fn id(self) -> &'static str {
        match self {
            ToolId::Select => "select",
            ToolId::Rect => "rect",
            ToolId::Ellipse => "ellipse",
            ToolId::Line => "line",
            ToolId::Text => "text",
            ToolId::Hand => "hand",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ToolId::Select => "Select",
            ToolId::Rect => "Rectangle",
            ToolId::Ellipse => "Ellipse",
            ToolId::Line => "Line",
            ToolId::Text => "Text",
            ToolId::Hand => "Hand",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            ToolId::Select => "V",
            ToolId::Rect => "R",
            ToolId::Ellipse => "O",
            ToolId::Line => "L",
            ToolId::Text => "T",
            ToolId::Hand => "H",
        }
    }

    pub fn from_id(id: &str) -> Option<ToolId> {
        ToolId::ALL.into_iter().find(|t| t.id() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Down,
    Move,
    Up,
}

/// A pointer event in page coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub pos: Point,
    /// Shift: add to the selection, keep proportions, constrain angles.
    pub shift: bool,
    /// Alt: resize from the centre.
    pub alt: bool,
}

/// What a tool sees of the document.
pub struct ToolContext<'a> {
    pub doc: &'a Document,
    pub page: usize,
    pub selection: &'a [NodeId],
    /// Screen pixels per page pixel (handles keep a constant screen size).
    pub zoom: f64,
    /// Snap targets (x and y lines in page coordinates); empty disables snapping.
    pub snap_xs: &'a [f64],
    pub snap_ys: &'a [f64],
}

impl ToolContext<'_> {
    fn page(&self) -> Option<&Page> {
        self.doc.pages.get(self.page)
    }
}

/// A command for the engine to run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Invocation {
    pub command: String,
    pub params: Value,
}

impl Invocation {
    pub fn new(command: &str, params: Value) -> Self {
        Invocation { command: command.into(), params }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    /// Elements drawn at these frames instead of their stored ones.
    pub frames: Vec<(NodeId, Frame)>,
    /// A frame for an element being drawn.
    pub creating: Option<Frame>,
    pub marquee: Option<Rect>,
    /// Snap lines hit, page coordinates.
    pub guides_x: Vec<f64>,
    pub guides_y: Vec<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolOutput {
    pub commands: Vec<Invocation>,
    pub preview: Option<Preview>,
    /// Pan the view by this many page pixels (Hand tool).
    pub pan: Option<Vec2>,
}

/// Selection handles: corners, edge midpoints and the rotation knob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Rotate,
}

impl Handle {
    pub const ALL: [Handle; 9] = [
        Handle::TopLeft,
        Handle::Top,
        Handle::TopRight,
        Handle::Right,
        Handle::BottomRight,
        Handle::Bottom,
        Handle::BottomLeft,
        Handle::Left,
        Handle::Rotate,
    ];

    /// Position in the element's local box (fractions), the rotation knob above the top edge.
    fn local(self, f: &Frame, zoom: f64) -> Point {
        let (w, h) = (f.width, f.height);
        match self {
            Handle::TopLeft => Point::new(0.0, 0.0),
            Handle::Top => Point::new(w / 2.0, 0.0),
            Handle::TopRight => Point::new(w, 0.0),
            Handle::Right => Point::new(w, h / 2.0),
            Handle::BottomRight => Point::new(w, h),
            Handle::Bottom => Point::new(w / 2.0, h),
            Handle::BottomLeft => Point::new(0.0, h),
            Handle::Left => Point::new(0.0, h / 2.0),
            Handle::Rotate => Point::new(w / 2.0, -ROTATE_OFFSET / zoom.max(0.01)),
        }
    }

    /// Page position of the handle.
    pub fn position(self, f: &Frame, zoom: f64) -> Point {
        f.to_page() * self.local(f, zoom)
    }
}

/// Screen distance of the rotation knob above the box.
pub const ROTATE_OFFSET: f64 = 28.0;
/// Screen radius within which a handle is hit.
pub const HANDLE_RADIUS: f64 = 7.0;
/// Screen distance within which edges snap.
pub const SNAP_DISTANCE: f64 = 6.0;
/// Smallest element side a drag creates or resizes to.
const MIN_SIDE: f64 = 1.0;

#[derive(Clone, Debug, Default)]
enum Gesture {
    #[default]
    Idle,
    Move {
        start: Point,
        frames: Vec<(NodeId, Frame)>,
    },
    Resize {
        id: NodeId,
        handle: Handle,
        start_frame: Frame,
    },
    Rotate {
        id: NodeId,
        start_frame: Frame,
    },
    Marquee {
        start: Point,
        add: bool,
    },
    Create {
        start: Point,
    },
    Pan {
        last: Point,
    },
}

/// The active tool and the gesture in progress.
#[derive(Default)]
pub struct ToolState {
    pub tool: ToolId,
    gesture: Gesture,
    /// Selection when the gesture began (shift-marquee adds to it).
    base_selection: Vec<NodeId>,
}

impl ToolState {
    pub fn set_tool(&mut self, tool: ToolId) {
        self.tool = tool;
        self.gesture = Gesture::Idle;
    }

    pub fn is_dragging(&self) -> bool {
        !matches!(self.gesture, Gesture::Idle)
    }

    /// Abandons the gesture (Escape).
    pub fn cancel(&mut self) {
        self.gesture = Gesture::Idle;
    }

    pub fn event(&mut self, cx: &ToolContext, ev: PointerEvent) -> ToolOutput {
        if !ev.pos.x.is_finite() || !ev.pos.y.is_finite() {
            return ToolOutput::default();
        }
        match self.tool {
            ToolId::Select => self.select_event(cx, ev),
            ToolId::Rect | ToolId::Ellipse | ToolId::Line => self.create_event(cx, ev),
            ToolId::Text => {
                if ev.kind == PointerKind::Down {
                    let (w, h) = (400.0, 80.0);
                    let p = json!({"x": ev.pos.x - w / 2.0, "y": ev.pos.y - h / 2.0, "width": w, "height": h, "text": "Add your text"});
                    ToolOutput { commands: vec![Invocation::new("text.add", p)], ..Default::default() }
                } else {
                    ToolOutput::default()
                }
            }
            ToolId::Hand => self.hand_event(ev),
        }
    }

    fn hand_event(&mut self, ev: PointerEvent) -> ToolOutput {
        match (ev.kind, &self.gesture) {
            (PointerKind::Down, _) => {
                self.gesture = Gesture::Pan { last: ev.pos };
                ToolOutput::default()
            }
            (PointerKind::Move, Gesture::Pan { last }) => {
                let d = ev.pos - *last;
                // The view moves with the pointer, so the page point under it stays the same.
                ToolOutput { pan: Some(d), ..Default::default() }
            }
            (PointerKind::Up, _) => {
                self.gesture = Gesture::Idle;
                ToolOutput::default()
            }
            _ => ToolOutput::default(),
        }
    }

    fn create_event(&mut self, cx: &ToolContext, ev: PointerEvent) -> ToolOutput {
        match (ev.kind, self.gesture.clone()) {
            (PointerKind::Down, _) => {
                self.gesture = Gesture::Create { start: ev.pos };
                ToolOutput {
                    preview: Some(Preview { creating: Some(Frame::new(ev.pos.x, ev.pos.y, 0.0, 0.0)), ..Default::default() }),
                    ..Default::default()
                }
            }
            (PointerKind::Move, Gesture::Create { start }) => {
                let f = drag_frame(start, ev.pos, ev.shift, ev.alt, self.tool == ToolId::Line);
                ToolOutput { preview: Some(Preview { creating: Some(f), ..Default::default() }), ..Default::default() }
            }
            (PointerKind::Up, Gesture::Create { start }) => {
                self.gesture = Gesture::Idle;
                let mut f = drag_frame(start, ev.pos, ev.shift, ev.alt, self.tool == ToolId::Line);
                // A click (no real drag) makes a default-sized shape centred on the point.
                if f.width * cx.zoom < 4.0 && f.height * cx.zoom < 4.0 {
                    let side = cx.page().map(|p| p.width.min(p.height) * 0.25).unwrap_or(200.0).max(10.0);
                    let h = if self.tool == ToolId::Line { 4.0 } else { side };
                    f = Frame::new(start.x - side / 2.0, start.y - h / 2.0, side, h);
                }
                let kind = match self.tool {
                    ToolId::Ellipse => "ellipse",
                    ToolId::Line => "line",
                    _ => "rect",
                };
                let p = json!({"shape": kind, "x": f.x, "y": f.y, "width": f.width.max(MIN_SIDE), "height": f.height.max(MIN_SIDE), "rotation": f.rotation});
                ToolOutput { commands: vec![Invocation::new("shape.add", p)], ..Default::default() }
            }
            _ => ToolOutput::default(),
        }
    }

    fn select_event(&mut self, cx: &ToolContext, ev: PointerEvent) -> ToolOutput {
        match ev.kind {
            PointerKind::Down => self.select_down(cx, ev),
            PointerKind::Move => self.select_drag(cx, ev, false),
            PointerKind::Up => {
                let out = self.select_drag(cx, ev, true);
                self.gesture = Gesture::Idle;
                out
            }
        }
    }

    fn select_down(&mut self, cx: &ToolContext, ev: PointerEvent) -> ToolOutput {
        self.base_selection = cx.selection.to_vec();
        // Handles of a single selected, unlocked element.
        if let [id] = cx.selection
            && let Some((_, el)) = cx.doc.find(*id)
            && !el.locked
            && let Some(handle) = hit_handle(&el.frame, ev.pos, cx.zoom)
        {
            self.gesture = if handle == Handle::Rotate {
                Gesture::Rotate { id: *id, start_frame: el.frame }
            } else {
                Gesture::Resize { id: *id, handle, start_frame: el.frame }
            };
            return ToolOutput::default();
        }
        let hit = cx.page().and_then(|p| hit_test(p, ev.pos));
        match hit {
            Some(el) => {
                let mut sel = cx.selection.to_vec();
                let mut commands = vec![];
                if ev.shift {
                    if let Some(i) = sel.iter().position(|s| *s == el.id) {
                        sel.remove(i);
                    } else {
                        sel.push(el.id);
                    }
                    commands.push(select_cmd(&sel));
                } else if !sel.contains(&el.id) {
                    sel = vec![el.id];
                    commands.push(select_cmd(&sel));
                }
                let frames: Vec<(NodeId, Frame)> =
                    sel.iter().filter_map(|id| cx.doc.find(*id).map(|(_, e)| e)).filter(|e| !e.locked).map(|e| (e.id, e.frame)).collect();
                self.gesture = Gesture::Move { start: ev.pos, frames };
                ToolOutput { commands, ..Default::default() }
            }
            None => {
                self.gesture = Gesture::Marquee { start: ev.pos, add: ev.shift };
                let commands = if ev.shift || cx.selection.is_empty() { vec![] } else { vec![select_cmd(&[])] };
                ToolOutput { commands, ..Default::default() }
            }
        }
    }

    fn select_drag(&mut self, cx: &ToolContext, ev: PointerEvent, commit: bool) -> ToolOutput {
        match self.gesture.clone() {
            Gesture::Move { start, frames } => {
                let mut d = ev.pos - start;
                if ev.shift {
                    // Constrain to the dominant axis.
                    if d.x.abs() > d.y.abs() { d.y = 0.0 } else { d.x = 0.0 }
                }
                let (dx, dy, gx, gy) = snap_delta(cx, &frames, d);
                let moved: Vec<(NodeId, Frame)> = frames.iter().map(|(id, f)| (*id, f.translated(dx, dy))).collect();
                if commit {
                    if (dx, dy) == (0.0, 0.0) || moved.is_empty() {
                        return ToolOutput::default();
                    }
                    return ToolOutput { commands: vec![frames_cmd(&moved, "Move")], ..Default::default() };
                }
                ToolOutput { preview: Some(Preview { frames: moved, guides_x: gx, guides_y: gy, ..Default::default() }), ..Default::default() }
            }
            Gesture::Resize { id, handle, start_frame } => {
                let f = resize_frame(&start_frame, handle, ev.pos, ev.shift, ev.alt);
                if commit {
                    if f == start_frame {
                        return ToolOutput::default();
                    }
                    return ToolOutput { commands: vec![frames_cmd(&[(id, f)], "Resize")], ..Default::default() };
                }
                ToolOutput { preview: Some(Preview { frames: vec![(id, f)], ..Default::default() }), ..Default::default() }
            }
            Gesture::Rotate { id, start_frame } => {
                let c = start_frame.center();
                let v = ev.pos - c;
                let mut deg = v.y.atan2(v.x).to_degrees() + 90.0;
                if ev.shift {
                    deg = (deg / 15.0).round() * 15.0;
                }
                let f = Frame { rotation: normalize_degrees(deg), ..start_frame };
                if commit {
                    return ToolOutput { commands: vec![frames_cmd(&[(id, f)], "Rotate")], ..Default::default() };
                }
                ToolOutput { preview: Some(Preview { frames: vec![(id, f)], ..Default::default() }), ..Default::default() }
            }
            Gesture::Marquee { start, add } => {
                let r = Rect::from_points(start, ev.pos);
                if commit {
                    let Some(page) = cx.page() else { return ToolOutput::default() };
                    let mut sel = if add { self.base_selection.clone() } else { vec![] };
                    for e in &page.elements {
                        if !e.hidden && !e.locked && !sel.contains(&e.id) && rects_overlap(&r, &e.frame.bounds()) {
                            sel.push(e.id);
                        }
                    }
                    if r.width() * cx.zoom < 2.0 && r.height() * cx.zoom < 2.0 && !add {
                        sel.clear();
                    }
                    return ToolOutput { commands: vec![select_cmd(&sel)], ..Default::default() };
                }
                ToolOutput { preview: Some(Preview { marquee: Some(r), ..Default::default() }), ..Default::default() }
            }
            _ => ToolOutput::default(),
        }
    }
}

fn select_cmd(ids: &[NodeId]) -> Invocation {
    Invocation::new("select.set", json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}

fn frames_cmd(frames: &[(NodeId, Frame)], label: &str) -> Invocation {
    let list: Vec<Value> = frames.iter().map(|(id, f)| json!({"id": id.0, "frame": f})).collect();
    Invocation::new("object.setFrames", json!({"frames": list, "label": label}))
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1
}

/// The topmost visible element under `p` (groups are hit as a whole).
pub fn hit_test(page: &Page, p: Point) -> Option<&Element> {
    page.elements.iter().rev().find(|e| !e.hidden && e.frame.contains(p))
}

/// The handle of `frame` under `p`, if any.
pub fn hit_handle(frame: &Frame, p: Point, zoom: f64) -> Option<Handle> {
    let r = HANDLE_RADIUS / zoom.max(0.01);
    Handle::ALL.into_iter().find(|h| (h.position(frame, zoom) - p).hypot() <= r)
}

fn normalize_degrees(d: f64) -> f64 {
    let d = d.rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
}

/// Snaps a move of `frames` by `d` to the context's guides (using their joint bounds).
fn snap_delta(cx: &ToolContext, frames: &[(NodeId, Frame)], d: Vec2) -> (f64, f64, Vec<f64>, Vec<f64>) {
    let Some(first) = frames.first() else { return (d.x, d.y, vec![], vec![]) };
    if cx.snap_xs.is_empty() && cx.snap_ys.is_empty() {
        return (d.x, d.y, vec![], vec![]);
    }
    let mut b = first.1.bounds();
    for (_, f) in frames.iter().skip(1) {
        b = b.union(f.bounds());
    }
    let b = b + d;
    let threshold = SNAP_DISTANCE / cx.zoom.max(0.01);
    let best = |cands: [f64; 3], targets: &[f64]| -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for c in cands {
            let (snapped, hit) = genexpress_geom::snap(c, targets, threshold);
            if let Some(t) = hit {
                let adj = snapped - c;
                if best.is_none_or(|(a, _)| adj.abs() < a.abs()) {
                    best = Some((adj, t));
                }
            }
        }
        best
    };
    let sx = best([b.x0, b.center().x, b.x1], cx.snap_xs);
    let sy = best([b.y0, b.center().y, b.y1], cx.snap_ys);
    (
        d.x + sx.map_or(0.0, |s| s.0),
        d.y + sy.map_or(0.0, |s| s.0),
        sx.map(|s| vec![s.1]).unwrap_or_default(),
        sy.map(|s| vec![s.1]).unwrap_or_default(),
    )
}

/// The frame a drag from `a` to `b` draws.
fn drag_frame(a: Point, b: Point, square: bool, from_center: bool, line: bool) -> Frame {
    if line {
        let v = b - a;
        let len = v.hypot();
        let mut deg = v.y.atan2(v.x).to_degrees();
        if square {
            deg = (deg / 45.0).round() * 45.0;
        }
        let c = a.midpoint(a + Vec2::new(len * deg.to_radians().cos(), len * deg.to_radians().sin()));
        return Frame { x: c.x - len / 2.0, y: c.y - 2.0, width: len, height: 4.0, rotation: deg };
    }
    let mut w = b.x - a.x;
    let mut h = b.y - a.y;
    if square {
        let s = w.abs().max(h.abs());
        w = s.copysign(w);
        h = s.copysign(h);
    }
    if from_center {
        return Frame::new(a.x - w.abs(), a.y - h.abs(), w.abs() * 2.0, h.abs() * 2.0);
    }
    Frame::new(a.x.min(a.x + w), a.y.min(a.y + h), w.abs(), h.abs())
}

/// Resizes `f` by dragging `handle` to page point `p`, in the element's rotated space.
fn resize_frame(f: &Frame, handle: Handle, p: Point, keep_ratio: bool, from_center: bool) -> Frame {
    let local = f.from_page() * p;
    let (mut x0, mut y0, mut x1, mut y1) = (0.0, 0.0, f.width, f.height);
    let (left, right, top, bottom) = match handle {
        Handle::TopLeft => (true, false, true, false),
        Handle::Top => (false, false, true, false),
        Handle::TopRight => (false, true, true, false),
        Handle::Right => (false, true, false, false),
        Handle::BottomRight => (false, true, false, true),
        Handle::Bottom => (false, false, false, true),
        Handle::BottomLeft => (true, false, false, true),
        Handle::Left => (true, false, false, false),
        Handle::Rotate => return *f,
    };
    if left {
        x0 = local.x.min(x1 - MIN_SIDE);
    }
    if right {
        x1 = local.x.max(x0 + MIN_SIDE);
    }
    if top {
        y0 = local.y.min(y1 - MIN_SIDE);
    }
    if bottom {
        y1 = local.y.max(y0 + MIN_SIDE);
    }
    let corner = (left || right) && (top || bottom);
    if keep_ratio && corner && f.width > 0.0 && f.height > 0.0 {
        let ratio = f.width / f.height;
        let (w, h) = (x1 - x0, y1 - y0);
        if w / h > ratio {
            let nh = w / ratio;
            if top { y0 = y1 - nh } else { y1 = y0 + nh }
        } else {
            let nw = h * ratio;
            if left { x0 = x1 - nw } else { x1 = x0 + nw }
        }
    }
    if from_center {
        // Mirror the change on the opposite side.
        if left || right {
            let (dl, dr) = (x0, x1 - f.width);
            let d = if left { -dl } else { dr };
            x0 = -d;
            x1 = f.width + d;
        }
        if top || bottom {
            let (dt, db) = (y0, y1 - f.height);
            let d = if top { -dt } else { db };
            y0 = -d;
            y1 = f.height + d;
        }
    }
    let (w, h) = ((x1 - x0).max(MIN_SIDE), (y1 - y0).max(MIN_SIDE));
    // The new box's centre in page space; rotation is kept.
    let c = f.to_page() * Point::new(x0 + w / 2.0, y0 + h / 2.0);
    Frame { x: c.x - w / 2.0, y: c.y - h / 2.0, width: w, height: h, rotation: f.rotation }
}

#[cfg(test)]
mod tests;
