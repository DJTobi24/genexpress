use super::*;
use genha_doc::{Color, ElementKind, Fill, ShapeEl, ShapeKind};

fn doc_with_rect() -> (Document, NodeId) {
    let mut doc = Document::new("t", 1000.0, 1000.0);
    let id = doc.alloc_id();
    doc.pages[0].elements.push(Element::new(
        id,
        Frame::new(100.0, 100.0, 200.0, 100.0),
        ElementKind::Shape(ShapeEl { shape: ShapeKind::Rect { corner_radius: 0.0 }, fill: Fill::solid(Color::BLACK), stroke: None, shadow: None }),
    ));
    (doc, id)
}

fn ev(kind: PointerKind, x: f64, y: f64) -> PointerEvent {
    PointerEvent { kind, pos: Point::new(x, y), shift: false, alt: false }
}

fn cx<'a>(doc: &'a Document, sel: &'a [NodeId]) -> ToolContext<'a> {
    ToolContext { doc, page: 0, selection: sel, zoom: 1.0, snap: false, snap_xs: &[], snap_ys: &[] }
}

#[test]
fn click_selects_and_drag_moves() {
    let (doc, id) = doc_with_rect();
    let mut t = ToolState::default();
    let out = t.event(&cx(&doc, &[]), ev(PointerKind::Down, 150.0, 150.0));
    assert_eq!(out.commands[0].command, "select.set");
    assert_eq!(out.commands[0].params["ids"][0], id.0);
    let sel = [id];
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Move, 160.0, 170.0));
    let p = out.preview.unwrap();
    assert_eq!(p.frames[0].1.x, 110.0);
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Up, 160.0, 170.0));
    assert_eq!(out.commands[0].command, "object.setFrames");
    assert_eq!(out.commands[0].params["frames"][0]["frame"]["y"], 120.0);
}

#[test]
fn snapping_while_moving() {
    let (doc, id) = doc_with_rect();
    let sel = [id];
    let xs = [400.0];
    let c = ToolContext { doc: &doc, page: 0, selection: &sel, zoom: 1.0, snap: true, snap_xs: &xs, snap_ys: &[] };
    let mut t = ToolState::default();
    t.event(&c, ev(PointerKind::Down, 150.0, 150.0));
    // Right edge would be at 403: snaps to 400.
    let out = t.event(&c, ev(PointerKind::Up, 253.0, 150.0));
    assert_eq!(out.commands[0].params["frames"][0]["frame"]["x"], 200.0);
    // A click without movement does not snap the object over (double-clicking text to edit it).
    let near = [303.0];
    let c = ToolContext { snap_xs: &near, ..c };
    t.event(&c, ev(PointerKind::Down, 150.0, 150.0));
    let out = t.event(&c, ev(PointerKind::Up, 150.0, 150.0));
    assert!(out.commands.is_empty());
}

#[test]
fn the_text_tool_starts_editing_the_new_box() {
    let (doc, _) = doc_with_rect();
    let mut t = ToolState::default();
    t.set_tool(ToolId::Text);
    let out = t.event(&cx(&doc, &[]), ev(PointerKind::Down, 500.0, 500.0));
    let cmds: Vec<&str> = out.commands.iter().map(|c| c.command.as_str()).collect();
    assert_eq!(cmds, ["text.add", "text.beginEdit"]);
}

#[test]
fn resize_from_corner_handle() {
    let (doc, id) = doc_with_rect();
    let sel = [id];
    let mut t = ToolState::default();
    t.event(&cx(&doc, &sel), ev(PointerKind::Down, 300.0, 200.0)); // bottom-right handle
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Up, 400.0, 250.0));
    let f: Frame = serde_json::from_value(out.commands[0].params["frames"][0]["frame"].clone()).unwrap();
    assert_eq!((f.x, f.y, f.width, f.height), (100.0, 100.0, 300.0, 150.0));
}

#[test]
fn resize_keeps_ratio_with_shift() {
    let f = Frame::new(0.0, 0.0, 200.0, 100.0);
    let r = resize_frame(&f, Handle::BottomRight, Point::new(400.0, 120.0), true, false);
    assert!((r.width / r.height - 2.0).abs() < 1e-9);
}

#[test]
fn rotate_handle_with_shift_snaps_to_15_degrees() {
    let (doc, id) = doc_with_rect();
    let sel = [id];
    let mut t = ToolState::default();
    let knob = Handle::Rotate.position(&doc.pages[0].elements[0].frame, 1.0);
    t.event(&cx(&doc, &sel), ev(PointerKind::Down, knob.x, knob.y));
    let mut up = ev(PointerKind::Up, 400.0, 160.0);
    up.shift = true;
    let out = t.event(&cx(&doc, &sel), up);
    let f: Frame = serde_json::from_value(out.commands[0].params["frames"][0]["frame"].clone()).unwrap();
    assert_eq!(f.rotation % 15.0, 0.0);
    assert!(f.rotation > 0.0);
}

#[test]
fn marquee_selects_overlapping() {
    let (doc, id) = doc_with_rect();
    let mut t = ToolState::default();
    t.event(&cx(&doc, &[]), ev(PointerKind::Down, 0.0, 0.0));
    let out = t.event(&cx(&doc, &[]), ev(PointerKind::Move, 120.0, 120.0));
    assert!(out.preview.unwrap().marquee.is_some());
    let out = t.event(&cx(&doc, &[]), ev(PointerKind::Up, 120.0, 120.0));
    assert_eq!(out.commands[0].params["ids"][0], id.0);
}

#[test]
fn shape_tool_drag_and_click() {
    let (doc, _) = doc_with_rect();
    let mut t = ToolState::default();
    t.set_tool(ToolId::Ellipse);
    t.event(&cx(&doc, &[]), ev(PointerKind::Down, 600.0, 600.0));
    let out = t.event(&cx(&doc, &[]), ev(PointerKind::Up, 500.0, 700.0));
    let p = &out.commands[0].params;
    assert_eq!((p["shape"].as_str(), p["x"].as_f64(), p["width"].as_f64()), (Some("ellipse"), Some(500.0), Some(100.0)));
    t.event(&cx(&doc, &[]), ev(PointerKind::Down, 600.0, 600.0));
    let out = t.event(&cx(&doc, &[]), ev(PointerKind::Up, 600.0, 600.0));
    assert_eq!(out.commands[0].params["width"], 250.0);
}

#[test]
fn non_finite_events_are_ignored() {
    let (doc, _) = doc_with_rect();
    let mut t = ToolState::default();
    assert_eq!(t.event(&cx(&doc, &[]), ev(PointerKind::Down, f64::NAN, 0.0)), ToolOutput::default());
}

#[test]
fn multi_selection_resizes_and_rotates_together() {
    let (mut doc, a) = doc_with_rect(); // 100,100 200×100
    let b = doc.alloc_id();
    let mut el = doc.pages[0].elements[0].clone();
    el.id = b;
    el.frame = Frame::new(400.0, 100.0, 100.0, 100.0);
    doc.pages[0].elements.push(el);
    let sel = [a, b];
    let joint = selection_frame(&doc, &sel).unwrap();
    assert_eq!((joint.x, joint.width), (100.0, 400.0));
    let mut t = ToolState::default();
    // Bottom-right handle of the joint box (500, 200) dragged to (900, 300): twice as big.
    t.event(&cx(&doc, &sel), ev(PointerKind::Down, 500.0, 200.0));
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Up, 900.0, 300.0));
    let frames = &out.commands[0].params["frames"];
    let fb: Frame = serde_json::from_value(frames[1]["frame"].clone()).unwrap();
    assert_eq!((fb.x, fb.y, fb.width, fb.height), (700.0, 100.0, 200.0, 200.0));
    // Rotating the pair turns both about the joint centre.
    let knob = Handle::Rotate.position(&joint, 1.0);
    t.event(&cx(&doc, &sel), ev(PointerKind::Down, knob.x, knob.y));
    let c = joint.center();
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Up, c.x + 100.0, c.y));
    let fa: Frame = serde_json::from_value(out.commands[0].params["frames"][0]["frame"].clone()).unwrap();
    assert!((fa.rotation - 90.0).abs() < 1e-6, "{fa:?}");
}

// ---------- smart guides ----------

fn page_scene(others: Vec<Rect>) -> Scene {
    Scene::new(Rect::new(0.0, 0.0, 1000.0, 1000.0), others, &[], &[])
}

fn add_rect(doc: &mut Document, f: Frame) -> NodeId {
    let id = doc.alloc_id();
    let mut el = doc.pages[0].elements[0].clone();
    el.id = id;
    el.frame = f;
    doc.pages[0].elements.push(el);
    id
}

fn snap_cx<'a>(doc: &'a Document, sel: &'a [NodeId]) -> ToolContext<'a> {
    ToolContext { snap: true, ..cx(doc, sel) }
}

fn frame_of(out: &ToolOutput) -> Frame {
    serde_json::from_value(out.commands[0].params["frames"][0]["frame"].clone()).unwrap()
}

#[test]
fn resizing_snaps_the_moving_edge_to_other_objects() {
    let (mut doc, id) = doc_with_rect(); // 100,100 200×100
    add_rect(&mut doc, Frame::new(400.0, 300.0, 100.0, 100.0));
    let sel = [id];
    let mut t = ToolState::default();
    // Right handle (300, 150) dragged to 397: the right edge lands on the other's left edge.
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, 300.0, 150.0));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Move, 397.0, 150.0));
    let p = out.preview.unwrap();
    assert_eq!(p.frames[0].1.width, 300.0);
    assert_eq!(p.guides_x, vec![400.0]);
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Up, 397.0, 150.0));
    assert_eq!(frame_of(&out).width, 300.0);
    // Holding Ctrl/Cmd (snap off in the context): the edge follows the pointer.
    t.event(&cx(&doc, &sel), ev(PointerKind::Down, 300.0, 150.0));
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Up, 397.0, 150.0));
    assert_eq!(frame_of(&out).width, 297.0);
}

#[test]
fn resizing_snaps_to_another_objects_size() {
    let (mut doc, id) = doc_with_rect(); // 100,100 200×100
    add_rect(&mut doc, Frame::new(600.0, 700.0, 150.0, 40.0));
    let sel = [id];
    let mut t = ToolState::default();
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, 300.0, 150.0));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Move, 253.0, 150.0));
    let p = out.preview.unwrap();
    assert_eq!(p.frames[0].1.width, 150.0);
    let same: Vec<&Measure> = p.measures.iter().filter(|m| m.kind == MeasureKind::SameSize).collect();
    assert_eq!(same.len(), 2, "{same:?}");
    assert!(same.iter().all(|m| m.value == 150.0 && m.horizontal()));
}

#[test]
fn resize_snapping_keeps_proportions_and_works_from_the_centre() {
    let s = page_scene(vec![Rect::new(500.0, 0.0, 600.0, 50.0)]);
    let corner = Sides { right: true, bottom: true, ..Default::default() };
    // x snaps 497 → 500 (the closest), the height follows the 2:1 ratio.
    let r = s.snap_resize(Rect::new(0.0, 0.0, 497.0, 248.5), corner, false, Some(2.0), 6.0);
    assert_eq!((r.x0, r.y0, r.x1, r.y1), (0.0, 0.0, 500.0, 250.0));
    // From the centre both edges move: the right edge reaches 500, the left mirrors it.
    let r = s.snap_resize(Rect::new(103.0, 300.0, 497.0, 340.0), Sides { right: true, ..Default::default() }, true, None, 6.0);
    assert_eq!((r.x0, r.x1), (100.0, 500.0));
    // Nothing near: unchanged; junk threshold: unchanged.
    let b = Rect::new(120.0, 620.0, 260.0, 690.0);
    assert_eq!(s.snap_resize(b, corner, false, None, 6.0), b);
    assert_eq!(s.snap_resize(b, corner, false, None, f64::NAN), b);
}

#[test]
fn rotated_objects_do_not_snap_while_resizing() {
    let (mut doc, id) = doc_with_rect();
    doc.pages[0].elements[0].frame.rotation = 30.0;
    add_rect(&mut doc, Frame::new(400.0, 300.0, 100.0, 100.0));
    let sel = [id];
    let f = doc.pages[0].elements[0].frame;
    let h = Handle::Right.position(&f, 1.0);
    let mut t = ToolState::default();
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, h.x, h.y));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Move, h.x + 40.0, h.y));
    let p = out.preview.unwrap();
    assert!(p.guides_x.is_empty() && p.guides_y.is_empty());
    assert!(!p.measures.is_empty(), "distances still show");
}

#[test]
fn moving_snaps_to_equal_spacing() {
    let s = page_scene(vec![Rect::new(0.0, 0.0, 100.0, 100.0), Rect::new(150.0, 0.0, 250.0, 100.0)]);
    // 52 px after the second object; the first two are 50 apart.
    let b = Rect::new(302.0, 0.0, 402.0, 100.0);
    let d = s.snap_move(b, 6.0);
    assert_eq!((d.x, d.y), (-2.0, 0.0));
    let marks = s.equal_spacing(b + d);
    assert_eq!(marks.len(), 2, "{marks:?}");
    assert!(marks.iter().all(|m| m.kind == MeasureKind::EqualSpacing && m.value == 50.0));
    // Centred between two neighbours.
    let s = page_scene(vec![Rect::new(0.0, 600.0, 100.0, 700.0), Rect::new(400.0, 600.0, 500.0, 700.0)]);
    let b = Rect::new(203.0, 610.0, 303.0, 690.0);
    let d = s.snap_move(b, 6.0);
    assert_eq!(d.x, -3.0);
    assert_eq!(s.equal_spacing(b + d).len(), 2);
    // Vertical: a column works the same way.
    let s = page_scene(vec![Rect::new(0.0, 0.0, 100.0, 100.0), Rect::new(0.0, 130.0, 100.0, 230.0)]);
    let d = s.snap_move(Rect::new(0.0, 262.0, 100.0, 362.0), 6.0);
    assert_eq!(d.y, -2.0);
}

#[test]
fn distances_to_neighbours_and_page_edges() {
    let s = page_scene(vec![Rect::new(0.0, 320.0, 100.0, 380.0)]);
    let m = s.distances(Rect::new(300.0, 300.0, 400.0, 400.0));
    let vals: Vec<(f64, bool)> = m.iter().map(|m| (m.value, m.horizontal())).collect();
    assert_eq!(vals, vec![(200.0, true), (600.0, true), (300.0, false), (600.0, false)]);
    // The gap to the neighbour is drawn across their shared height.
    assert_eq!((m[0].from, m[0].to), (Point::new(100.0, 350.0), Point::new(300.0, 350.0)));
    // A box sticking out of the page measures nothing towards that edge.
    assert_eq!(s.distances(Rect::new(-10.0, 500.0, 50.0, 560.0)).iter().filter(|m| m.horizontal()).count(), 0);
    assert_eq!(format_px(12.0), "12");
    assert_eq!(format_px(12.46), "12.5");
    assert_eq!(format_px(f64::NAN), "");
}

#[test]
fn alt_hover_measures_to_the_hovered_object_or_the_page() {
    let (mut doc, id) = doc_with_rect(); // 100,100 200×100
    add_rect(&mut doc, Frame::new(500.0, 120.0, 100.0, 60.0));
    let m = hover_measures(&doc, 0, &[id], Point::new(550.0, 150.0));
    // The gap (200) and the top and bottom edge offsets (20, 20).
    let mut vals: Vec<f64> = m.iter().map(|m| m.value).collect();
    vals.sort_by(f64::total_cmp);
    assert_eq!(vals, vec![20.0, 20.0, 200.0], "{m:?}");
    // Over empty page: distances to the page edges.
    let m = hover_measures(&doc, 0, &[id], Point::new(800.0, 800.0));
    let mut vals: Vec<f64> = m.iter().map(|m| m.value).collect();
    vals.sort_by(f64::total_cmp);
    assert_eq!(vals, vec![100.0, 100.0, 700.0, 800.0]);
    assert!(hover_measures(&doc, 0, &[], Point::new(1.0, 1.0)).is_empty());
    assert!(hover_measures(&doc, 7, &[id], Point::new(1.0, 1.0)).is_empty());
}

#[test]
fn clicks_on_objects_and_handles_never_nudge() {
    let (mut doc, id) = doc_with_rect();
    add_rect(&mut doc, Frame::new(303.0, 400.0, 50.0, 50.0)); // a guide 3 px off the right edge
    let sel = [id];
    let mut t = ToolState::default();
    // A click with a pixel of jitter on the object.
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, 150.0, 150.0));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Move, 151.0, 150.0));
    assert!(out.preview.is_none());
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Up, 151.0, 151.0));
    assert!(out.commands.is_empty());
    // A click on a handle, off its centre.
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, 302.0, 203.0));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Up, 302.0, 203.0));
    assert!(out.commands.is_empty());
    // A click on the rotation knob.
    let knob = Handle::Rotate.position(&doc.pages[0].elements[0].frame, 1.0);
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, knob.x + 2.0, knob.y));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Up, knob.x + 2.0, knob.y));
    assert!(out.commands.is_empty());
    // Grabbing a handle off its centre: the edge follows the pointer's travel.
    t.event(&cx(&doc, &sel), ev(PointerKind::Down, 303.0, 203.0));
    let out = t.event(&cx(&doc, &sel), ev(PointerKind::Up, 403.0, 253.0));
    let f = frame_of(&out);
    assert_eq!((f.width, f.height), (300.0, 150.0));
}

#[test]
fn moving_reports_guides_and_distances() {
    let (mut doc, id) = doc_with_rect(); // 100,100 200×100
    add_rect(&mut doc, Frame::new(600.0, 100.0, 100.0, 100.0));
    let sel = [id];
    let mut t = ToolState::default();
    t.event(&snap_cx(&doc, &sel), ev(PointerKind::Down, 150.0, 150.0));
    let out = t.event(&snap_cx(&doc, &sel), ev(PointerKind::Move, 200.0, 152.0));
    let p = out.preview.unwrap();
    // y snapped back onto the neighbour's top/middle/bottom lines.
    assert_eq!(p.frames[0].1.y, 100.0);
    assert_eq!(p.guides_y, vec![100.0, 150.0, 200.0]);
    // 150 px to the page's left edge, 250 to the neighbour on the right.
    let horiz: Vec<f64> = p.measures.iter().filter(|m| m.horizontal()).map(|m| m.value).collect();
    assert_eq!(horiz, vec![150.0, 250.0]);
}
