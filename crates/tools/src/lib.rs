//! Tools (layer L4): state machines that turn pointer gestures into commands.
//!
//! A tool never edits the document. While a gesture runs it returns a [`ToolOutput::preview`]
//! (frames the UI shows instead of the stored ones, a marquee, snap guides); when the gesture ends
//! it returns [`Invocation`]s, which the engine runs like any other command, so every gesture is
//! undoable, journaled and reachable by agents.
#![forbid(unsafe_code)]

use genha_doc::{Document, Element, NodeId, Page};
use genha_geom::{Frame, Point, Rect, Vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub mod smart;
pub use smart::{Measure, MeasureKind, Scene, Sides, format_px, hover_measures, measure_between};

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
    /// Smart guides on: moves and resizes snap to the page, the other objects, equal gaps and
    /// matching sizes (off: the preference is off, or the user holds Ctrl/Cmd while dragging).
    pub snap: bool,
    /// Extra snap lines from the host (x and y in page coordinates), on top of the page's and the
    /// other objects' edges and centres, which the tool finds itself.
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
    /// Distance hints, equal-spacing and same-size markers (page coordinates).
    pub measures: Vec<Measure>,
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
/// Screen distance the pointer must travel before a press on an object or a handle becomes a drag
/// (a click never nudges anything).
pub const DRAG_START: f64 = 3.0;
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
        /// Where the pointer went down (the handle follows the pointer's travel, not its position).
        start: Point,
    },
    Rotate {
        id: NodeId,
        start_frame: Frame,
        /// Angle of the pointer about the centre when it went down (radians).
        start_angle: f64,
        start: Point,
    },
    /// Several objects scaled with their joint box.
    ResizeMany {
        handle: Handle,
        start_box: Frame,
        frames: Vec<(NodeId, Frame)>,
        start: Point,
    },
    /// Several objects turned about their joint box's centre.
    RotateMany {
        start_box: Frame,
        start_angle: f64,
        frames: Vec<(NodeId, Frame)>,
        start: Point,
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
    /// What the dragged objects snap to and are measured against (collected on press).
    scene: Scene,
    /// The pointer has travelled [`DRAG_START`] since the press: the gesture is a drag.
    dragged: bool,
}

impl ToolState {
    pub fn set_tool(&mut self, tool: ToolId) {
        self.tool = tool;
        self.gesture = Gesture::Idle;
        self.scene = Scene::default();
    }

    pub fn is_dragging(&self) -> bool {
        !matches!(self.gesture, Gesture::Idle)
    }

    /// Abandons the gesture (Escape).
    pub fn cancel(&mut self) {
        self.gesture = Gesture::Idle;
        self.scene = Scene::default();
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
                    // The new box goes straight into text editing, its placeholder selected.
                    let commands = vec![Invocation::new("text.add", p), Invocation::new("text.beginEdit", json!({}))];
                    ToolOutput { commands, ..Default::default() }
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
                self.scene = Scene::default();
                out
            }
        }
    }

    fn select_down(&mut self, cx: &ToolContext, ev: PointerEvent) -> ToolOutput {
        self.base_selection = cx.selection.to_vec();
        self.dragged = false;
        self.scene = Scene::default();
        // Handles of a single selected, unlocked element.
        if let [id] = cx.selection
            && let Some((_, el)) = cx.doc.find(*id)
            && !el.locked
            && let Some(handle) = hit_handle(&el.frame, ev.pos, cx.zoom)
        {
            self.gesture = if handle == Handle::Rotate {
                let v = ev.pos - el.frame.center();
                Gesture::Rotate { id: *id, start_frame: el.frame, start_angle: v.y.atan2(v.x), start: ev.pos }
            } else {
                self.scene = Scene::collect(cx.doc, cx.page, &[*id], cx.snap_xs, cx.snap_ys);
                Gesture::Resize { id: *id, handle, start_frame: el.frame, start: ev.pos }
            };
            return ToolOutput::default();
        }
        // Handles of a multiple selection: its joint box.
        if cx.selection.len() > 1
            && let Some(start_box) = selection_frame(cx.doc, cx.selection)
            && let Some(handle) = hit_handle(&start_box, ev.pos, cx.zoom)
        {
            let frames: Vec<(NodeId, Frame)> =
                cx.selection.iter().filter_map(|id| cx.doc.find(*id).map(|(_, e)| e)).filter(|e| !e.locked).map(|e| (e.id, e.frame)).collect();
            self.gesture = if handle == Handle::Rotate {
                let v = ev.pos - start_box.center();
                Gesture::RotateMany { start_box, start_angle: v.y.atan2(v.x), frames, start: ev.pos }
            } else {
                let ids: Vec<NodeId> = frames.iter().map(|(id, _)| *id).collect();
                self.scene = Scene::collect(cx.doc, cx.page, &ids, cx.snap_xs, cx.snap_ys);
                Gesture::ResizeMany { handle, start_box, frames, start: ev.pos }
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
                let ids: Vec<NodeId> = frames.iter().map(|(id, _)| *id).collect();
                self.scene = Scene::collect(cx.doc, cx.page, &ids, cx.snap_xs, cx.snap_ys);
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
        // A press becomes a drag only once the pointer has really travelled: a click (or a click
        // with a little jitter) never moves, resizes or turns anything, even next to a guide.
        let pressed_at = match &self.gesture {
            Gesture::Move { start, .. }
            | Gesture::Resize { start, .. }
            | Gesture::Rotate { start, .. }
            | Gesture::ResizeMany { start, .. }
            | Gesture::RotateMany { start, .. } => Some(*start),
            _ => None,
        };
        if let Some(s) = pressed_at
            && !self.dragged
        {
            if (ev.pos - s).hypot() * cx.zoom.max(0.01) < DRAG_START {
                return ToolOutput::default();
            }
            self.dragged = true;
        }
        let threshold = SNAP_DISTANCE / cx.zoom.max(0.01);
        match self.gesture.clone() {
            Gesture::Move { start, frames } => {
                let mut d = ev.pos - start;
                // Shift constrains the move to the dominant axis (which alone snaps).
                let (free_x, free_y) = if !ev.shift {
                    (true, true)
                } else if d.x.abs() > d.y.abs() {
                    d.y = 0.0;
                    (true, false)
                } else {
                    d.x = 0.0;
                    (false, true)
                };
                if commit && d.x == 0.0 && d.y == 0.0 {
                    return ToolOutput::default();
                }
                let Some(b0) = joint_bounds(&frames) else { return ToolOutput::default() };
                if cx.snap {
                    let s = self.scene.snap_move(b0 + d, threshold);
                    if free_x {
                        d.x += s.x;
                    }
                    if free_y {
                        d.y += s.y;
                    }
                }
                let moved: Vec<(NodeId, Frame)> = frames.iter().map(|(id, f)| (*id, f.translated(d.x, d.y))).collect();
                if commit {
                    if (d.x, d.y) == (0.0, 0.0) || moved.is_empty() {
                        return ToolOutput::default();
                    }
                    return ToolOutput { commands: vec![frames_cmd(&moved, "Move")], ..Default::default() };
                }
                let b = b0 + d;
                let (guides_x, guides_y) = if cx.snap { self.scene.touching(b, None) } else { (vec![], vec![]) };
                let measures = self.scene.hints(b, cx.snap);
                ToolOutput { preview: Some(Preview { frames: moved, guides_x, guides_y, measures, ..Default::default() }), ..Default::default() }
            }
            Gesture::Resize { id, handle, start_frame, start } => {
                // The handle follows the pointer's travel, so grabbing it off-centre does not jump.
                let p = handle.position(&start_frame, cx.zoom) + (ev.pos - start);
                let raw = resize_frame(&start_frame, handle, p, ev.shift, ev.alt);
                let (f, mut preview) = self.snap_resized(cx, raw, &start_frame, handle, ev);
                if commit {
                    if f == start_frame {
                        return ToolOutput::default();
                    }
                    return ToolOutput { commands: vec![frames_cmd(&[(id, f)], "Resize")], ..Default::default() };
                }
                preview.frames = vec![(id, f)];
                ToolOutput { preview: Some(preview), ..Default::default() }
            }
            Gesture::ResizeMany { handle, start_box, frames, start } => {
                let p = handle.position(&start_box, cx.zoom) + (ev.pos - start);
                let raw = resize_frame(&start_box, handle, p, ev.shift, ev.alt);
                let (b, mut preview) = self.snap_resized(cx, raw, &start_box, handle, ev);
                let sx = if start_box.width > 0.0 { b.width / start_box.width } else { 1.0 };
                let sy = if start_box.height > 0.0 { b.height / start_box.height } else { 1.0 };
                let out: Vec<(NodeId, Frame)> = frames
                    .iter()
                    .map(|(id, f)| {
                        let (w, h) = ((f.width * sx).max(MIN_SIDE), (f.height * sy).max(MIN_SIDE));
                        let c = f.center();
                        let nc = Point::new(b.x + (c.x - start_box.x) * sx, b.y + (c.y - start_box.y) * sy);
                        (*id, Frame { x: nc.x - w / 2.0, y: nc.y - h / 2.0, width: w, height: h, rotation: f.rotation })
                    })
                    .collect();
                if commit {
                    if b == start_box {
                        return ToolOutput::default();
                    }
                    return ToolOutput { commands: vec![frames_cmd(&out, "Resize")], ..Default::default() };
                }
                preview.frames = out;
                ToolOutput { preview: Some(preview), ..Default::default() }
            }
            Gesture::RotateMany { start_box, start_angle, frames, .. } => {
                let c = start_box.center();
                let v = ev.pos - c;
                let mut delta = (v.y.atan2(v.x) - start_angle).to_degrees();
                if ev.shift {
                    delta = (delta / 15.0).round() * 15.0;
                }
                let turn = genha_geom::Affine::rotate_about(delta.to_radians(), c);
                let out: Vec<(NodeId, Frame)> = frames
                    .iter()
                    .map(|(id, f)| {
                        let nc = turn * f.center();
                        (*id, Frame { x: nc.x - f.width / 2.0, y: nc.y - f.height / 2.0, rotation: normalize_degrees(f.rotation + delta), ..*f })
                    })
                    .collect();
                if commit {
                    return ToolOutput { commands: vec![frames_cmd(&out, "Rotate")], ..Default::default() };
                }
                ToolOutput { preview: Some(Preview { frames: out, ..Default::default() }), ..Default::default() }
            }
            Gesture::Rotate { id, start_frame, start_angle, .. } => {
                // Turns by the pointer's travel about the centre (grabbing the knob off-centre does
                // not jump); Shift lands on whole 15° steps.
                let v = ev.pos - start_frame.center();
                let delta = (v.y.atan2(v.x) - start_angle).to_degrees();
                let mut deg = normalize_degrees(start_frame.rotation + delta);
                if ev.shift {
                    deg = normalize_degrees((deg / 15.0).round() * 15.0);
                }
                let f = Frame { rotation: deg, ..start_frame };
                if commit {
                    if f == start_frame {
                        return ToolOutput::default();
                    }
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

impl ToolState {
    /// Snaps a box being resized (`raw` follows the pointer) and gathers the hints to show: guides
    /// on the moving sides, matching sizes, equal spacing and distances. Rotated boxes do not snap
    /// (their distances are measured on their bounds).
    fn snap_resized(&self, cx: &ToolContext, raw: Frame, start: &Frame, handle: Handle, ev: PointerEvent) -> (Frame, Preview) {
        let sides = handle_sides(handle);
        if !cx.snap || normalize_degrees(start.rotation).abs() > 1e-9 {
            let measures = self.scene.hints(raw.bounds(), false);
            return (raw, Preview { measures, ..Default::default() });
        }
        let threshold = SNAP_DISTANCE / cx.zoom.max(0.01);
        let ratio = (ev.shift && sides.corner() && start.width > 0.0 && start.height > 0.0).then(|| start.width / start.height);
        let r = self.scene.snap_resize(raw.rect(), sides, ev.alt, ratio, threshold);
        let f = Frame { x: r.x0, y: r.y0, width: r.width().max(MIN_SIDE), height: r.height().max(MIN_SIDE), rotation: raw.rotation };
        // From the centre both sides of an axis move.
        let moving = if ev.alt {
            let (h, v) = (sides.left || sides.right, sides.top || sides.bottom);
            Sides { left: h, right: h, top: v, bottom: v }
        } else {
            sides
        };
        let (guides_x, guides_y) = self.scene.touching(f.rect(), Some(moving));
        let mut measures = self.scene.same_size(f.rect(), sides);
        measures.extend(self.scene.hints(f.rect(), true));
        (f, Preview { guides_x, guides_y, measures, ..Default::default() })
    }
}

/// The sides of a box a handle moves.
fn handle_sides(handle: Handle) -> Sides {
    let (left, right, top, bottom) = match handle {
        Handle::TopLeft => (true, false, true, false),
        Handle::Top => (false, false, true, false),
        Handle::TopRight => (false, true, true, false),
        Handle::Right => (false, true, false, false),
        Handle::BottomRight => (false, true, false, true),
        Handle::Bottom => (false, false, false, true),
        Handle::BottomLeft => (true, false, false, true),
        Handle::Left => (true, false, false, false),
        Handle::Rotate => (false, false, false, false),
    };
    Sides { left, right, top, bottom }
}

/// The joint axis-aligned bounds of `frames`.
fn joint_bounds(frames: &[(NodeId, Frame)]) -> Option<Rect> {
    frames.iter().map(|(_, f)| f.bounds()).reduce(|a, b| a.union(b))
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

/// The box selection handles go on: a single object's own (rotated) frame, or the joint
/// axis-aligned bounds of several unlocked objects.
pub fn selection_frame(doc: &Document, selection: &[NodeId]) -> Option<Frame> {
    let els: Vec<&Element> = selection.iter().filter_map(|id| doc.find(*id).map(|(_, e)| e)).filter(|e| !e.locked).collect();
    match els.as_slice() {
        [] => None,
        [one] => Some(one.frame),
        many => {
            let b = many.iter().map(|e| e.frame.bounds()).reduce(|a, r| a.union(r))?;
            Some(Frame::new(b.x0, b.y0, b.width(), b.height()))
        }
    }
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
    if handle == Handle::Rotate {
        return *f;
    }
    let Sides { left, right, top, bottom } = handle_sides(handle);
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
