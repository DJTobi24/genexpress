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
    ToolContext { doc, page: 0, selection: sel, zoom: 1.0, snap_xs: &[], snap_ys: &[] }
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
    let c = ToolContext { doc: &doc, page: 0, selection: &sel, zoom: 1.0, snap_xs: &xs, snap_ys: &[] };
    let mut t = ToolState::default();
    t.event(&c, ev(PointerKind::Down, 150.0, 150.0));
    // Right edge would be at 403: snaps to 400.
    let out = t.event(&c, ev(PointerKind::Up, 253.0, 150.0));
    assert_eq!(out.commands[0].params["frames"][0]["frame"]["x"], 200.0);
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
