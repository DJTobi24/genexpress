use super::*;
use genha_doc::{Color, Fill, GroupEl, NodeId, ShapeEl, ShapeKind, TextEl, TextStyle};

fn rect(id: u64, x: f64, y: f64, anim: Option<Animation>) -> Element {
    let mut e = Element::new(
        NodeId(id),
        Frame::new(x, y, 100.0, 50.0),
        ElementKind::Shape(ShapeEl {
            shape: ShapeKind::Rect { corner_radius: 8.0 },
            fill: Fill::solid(Color::rgb(255, 0, 0)),
            stroke: None,
            shadow: None,
        }),
    );
    e.animation = anim;
    e
}

fn text(id: u64, s: &str, anim: Option<Animation>) -> Element {
    let mut e = Element::new(
        NodeId(id),
        Frame::new(100.0, 100.0, 400.0, 80.0),
        ElementKind::Text(TextEl { text: s.into(), style: TextStyle::default(), effect: Default::default(), curve: 0.0, spans: vec![] }),
    );
    e.animation = anim;
    e
}

fn page(els: Vec<Element>) -> Page {
    let mut p = Page::new(NodeId(1), "p", 1000.0, 800.0);
    p.elements = els;
    p
}

fn anim(kind: AnimationKind, duration: f64, delay: f64) -> Option<Animation> {
    Some(Animation::new(kind, duration, delay))
}

#[test]
fn durations() {
    // Nothing animated: the minimum.
    assert_eq!(page_duration(&page(vec![rect(2, 0.0, 0.0, None)])), MIN_PAGE_DURATION);
    // Last entrance end + hold.
    let p = page(vec![rect(2, 0.0, 0.0, anim(AnimationKind::Fade, 1.0, 0.5)), rect(3, 0.0, 0.0, anim(AnimationKind::Pop, 2.0, 1.0))]);
    assert!((page_duration(&p) - (3.0 + HOLD)).abs() < 1e-9);
    // An explicit duration wins, capped.
    let mut q = p.clone();
    q.duration = Some(5.0);
    assert_eq!(page_duration(&q), 5.0);
    q.duration = Some(1e9);
    assert_eq!(page_duration(&q), MAX_PAGE_DURATION);
    for bad in [f64::NAN, f64::INFINITY, -3.0, 0.0] {
        q.duration = Some(bad);
        assert!((page_duration(&q) - (3.0 + HOLD)).abs() < 1e-9, "{bad}");
    }
    // Hostile timings stay finite and capped.
    let r =
        page(vec![rect(2, 0.0, 0.0, anim(AnimationKind::Fade, f64::INFINITY, f64::NAN)), rect(3, 0.0, 0.0, anim(AnimationKind::Fade, 1e12, 1e12))]);
    let d = page_duration(&r);
    assert!(d.is_finite() && d <= MAX_PAGE_DURATION);
    assert_eq!(frame_times(&r, 30.0, 600).len(), 600);
    assert_eq!(frame_times(&r, f64::NAN, 0).len(), 1);
}

#[test]
fn start_and_end_states() {
    let kinds: Vec<AnimationKind> = ENTRANCES.iter().map(|e| e.0).collect();
    let els: Vec<Element> = kinds.iter().enumerate().map(|(i, k)| rect(i as u64 + 2, 300.0, 300.0, anim(*k, 0.8, 0.1 * i as f64))).collect();
    let mut els = els;
    els.push(text(100, "Hello there", anim(AnimationKind::Typewriter, 1.0, 0.0)));
    let p = page(els);
    let end = page_duration(&p);
    // At (and after) the end the page is exactly the static page.
    assert_eq!(page_at(&p, end), p);
    assert_eq!(page_at(&p, end + 100.0), p);
    assert_eq!(page_at(&p, f64::INFINITY), p);
    // At t = 0 every entrance is at its start: hidden or out of place.
    let start = page_at(&p, 0.0);
    for (a, b) in start.elements.iter().zip(&p.elements) {
        let moved = a.frame != b.frame;
        let faded = a.opacity < 0.05;
        let typed = matches!((&a.kind, &b.kind), (ElementKind::Text(x), ElementKind::Text(y)) if x.text.len() < y.text.len());
        assert!(moved || faded || typed, "{:?} looks finished at t=0", b.animation);
    }
    // Before its delay an element stays at its start.
    let late = page(vec![rect(2, 0.0, 0.0, anim(AnimationKind::Fade, 0.5, 1.0))]);
    assert_eq!(page_at(&late, 0.9).elements[0].opacity, 0.0);
    assert!(page_at(&late, 1.25).elements[0].opacity > 0.5);
    // NaN time reads as the start.
    assert_eq!(page_at(&late, f64::NAN), page_at(&late, 0.0));
}

#[test]
fn easing_endpoints() {
    for f in [ease_out_cubic, ease_out_back, ease_bounce] {
        assert!(f(0.0).abs() < 1e-12);
        assert_eq!(f(1.0), 1.0);
        assert_eq!(f(-5.0), f(0.0));
        assert_eq!(f(9.0), 1.0);
    }
    // Back and bounce overshoot; cubic does not.
    assert!((0..100).map(|i| ease_out_back(i as f64 / 100.0)).fold(0.0, f64::max) > 1.05);
    assert!((0..100).map(|i| ease_bounce(i as f64 / 100.0)).fold(0.0, f64::max) > 1.05);
    assert!((0..=100).all(|i| ease_out_cubic(i as f64 / 100.0) <= 1.0));
    // Monotonic cubic.
    assert!((1..=100).all(|i| ease_out_cubic(i as f64 / 100.0) >= ease_out_cubic((i - 1) as f64 / 100.0)));
}

#[test]
fn slides_start_outside_their_place() {
    let p = page(vec![
        rect(2, 300.0, 300.0, anim(AnimationKind::SlideLeft, 1.0, 0.0)),
        rect(3, 300.0, 300.0, anim(AnimationKind::SlideUp, 1.0, 0.0)),
        rect(4, 300.0, 300.0, anim(AnimationKind::SlideRight, 1.0, 0.0)),
        rect(5, 300.0, 300.0, anim(AnimationKind::SlideDown, 1.0, 0.0)),
    ]);
    let s = page_at(&p, 0.0);
    assert_eq!(s.elements[0].frame.x, 300.0 + 0.2 * 1000.0, "slide left comes from the right");
    assert_eq!(s.elements[1].frame.y, 300.0 + 0.2 * 800.0, "slide up comes from below");
    assert_eq!(s.elements[2].frame.x, 300.0 - 0.2 * 1000.0);
    assert_eq!(s.elements[3].frame.y, 300.0 - 0.2 * 800.0);
    // Halfway it is most of the way there (ease-out).
    let m = page_at(&p, 0.5);
    let left = m.elements[0].frame.x - 300.0;
    assert!(left > 0.0 && left < 0.5 * 0.2 * 1000.0, "{left}");
}

#[test]
fn pop_zoom_and_spin_scale_about_the_centre() {
    let p = page(vec![
        rect(2, 300.0, 300.0, anim(AnimationKind::Pop, 1.0, 0.0)),
        rect(3, 300.0, 300.0, anim(AnimationKind::Zoom, 1.0, 0.0)),
        rect(4, 300.0, 300.0, anim(AnimationKind::Spin, 1.0, 0.0)),
    ]);
    let s = page_at(&p, 0.0);
    let c = p.elements[0].frame.center();
    for e in &s.elements {
        assert!((e.frame.center().x - c.x).abs() < 1e-9 && (e.frame.center().y - c.y).abs() < 1e-9);
    }
    assert!((s.elements[0].frame.width - 60.0).abs() < 1e-9, "pop starts at 60 %");
    assert!((s.elements[1].frame.width - 130.0).abs() < 1e-9, "zoom starts at 130 %");
    assert!(s.elements[2].frame.rotation < -100.0);
    // Pop overshoots on its way in.
    let widest = (1..100).map(|i| page_at(&p, i as f64 / 100.0).elements[0].frame.width).fold(0.0, f64::max);
    assert!(widest > 100.5, "{widest}");
    // The rect's corner radius scales with it.
    let ElementKind::Shape(sh) = &s.elements[0].kind else { panic!() };
    let ShapeKind::Rect { corner_radius } = sh.shape else { panic!() };
    assert!((corner_radius - 8.0 * 0.6).abs() < 1e-9);
}

#[test]
fn bounce_overshoots_and_settles() {
    let p = page(vec![rect(2, 300.0, 300.0, anim(AnimationKind::Bounce, 1.0, 0.0))]);
    assert!(page_at(&p, 0.0).elements[0].frame.y < 300.0 - 100.0, "starts above");
    let lowest = (1..100).map(|i| page_at(&p, i as f64 / 100.0).elements[0].frame.y).fold(0.0, f64::max);
    assert!(lowest > 300.0 + 5.0, "overshoots below its place: {lowest}");
    assert_eq!(page_at(&p, 1.0).elements[0].frame.y, 300.0);
}

#[test]
fn typewriter_reveals_text() {
    let p = page(vec![text(2, "abcdefghij", anim(AnimationKind::Typewriter, 1.0, 0.0))]);
    let at = |t: f64| match &page_at(&p, t).elements[0].kind {
        ElementKind::Text(x) => x.text.clone(),
        _ => panic!(),
    };
    assert_eq!(at(0.0), "");
    assert_eq!(at(0.5), "abcde");
    assert_eq!(at(1.0), "abcdefghij");
    // Multi-byte characters are never split.
    let q = page(vec![text(2, "héllo wörld ✓", anim(AnimationKind::Typewriter, 1.0, 0.0))]);
    for i in 0..=20 {
        let _ = page_at(&q, i as f64 / 20.0);
    }
}

#[test]
fn text_scales_with_its_font() {
    let p = page(vec![text(2, "Big", anim(AnimationKind::Zoom, 1.0, 0.0))]);
    let s = page_at(&p, 0.0);
    let ElementKind::Text(t) = &s.elements[0].kind else { panic!() };
    assert!((t.style.font_size - 48.0 * 1.3).abs() < 1e-9);
    assert!((s.elements[0].frame.width - 400.0 * 1.3).abs() < 1e-9);
}

#[test]
fn groups_carry_their_children() {
    let a = rect(3, 100.0, 100.0, None);
    let b = rect(4, 300.0, 100.0, anim(AnimationKind::Fade, 1.0, 0.0));
    let mut g = Element::new(NodeId(2), Frame::new(100.0, 100.0, 300.0, 50.0), ElementKind::Group(GroupEl { children: vec![a, b] }));
    g.animation = anim(AnimationKind::SlideUp, 1.0, 0.0);
    let p = page(vec![g]);
    let s = page_at(&p, 0.0);
    let ElementKind::Group(sg) = &s.elements[0].kind else { panic!() };
    let dy = 0.2 * 800.0;
    assert_eq!(s.elements[0].frame.y, 100.0 + dy);
    assert_eq!(sg.children[0].frame.y, 100.0 + dy, "children move with the group");
    assert_eq!(sg.children[1].frame.y, 100.0 + dy);
    assert_eq!(sg.children[1].opacity, 0.0, "and play their own animation");
    assert_eq!(page_at(&p, 10.0), p);
    // A group zooming scales its children about the group's centre.
    let mut z = p.clone();
    z.elements[0].animation = anim(AnimationKind::Zoom, 1.0, 0.0);
    let s = page_at(&z, 0.0);
    let ElementKind::Group(sg) = &s.elements[0].kind else { panic!() };
    let gc = z.elements[0].frame.center();
    assert!((sg.children[0].frame.center().x - (gc.x + (150.0 - gc.x) * 1.3)).abs() < 1e-9);
}

#[test]
fn loops_and_exits() {
    let mut a = Animation::new(AnimationKind::Fade, 0.5, 0.0);
    a.looping = Some(LoopKind::Pulse);
    let p = page(vec![rect(2, 300.0, 300.0, Some(a.clone()))]);
    let d = page_duration(&p);
    assert!(d >= 0.5 + 2.0 * LOOP_PERIOD);
    // Mid-cycle the element is bigger; at the end it is at rest.
    let mid = page_at(&p, 0.5 + LOOP_PERIOD / 2.0);
    assert!(mid.elements[0].frame.width > 103.0);
    assert_eq!(page_at(&p, d), p);
    for kind in [LoopKind::Float, LoopKind::Sway, LoopKind::Blink] {
        let mut a = a.clone();
        a.looping = Some(kind);
        let p = page(vec![rect(2, 300.0, 300.0, Some(a))]);
        assert_ne!(page_at(&p, 0.5 + LOOP_PERIOD / 4.0), p, "{kind:?}");
        assert_eq!(page_at(&p, page_duration(&p)), p);
    }
    // An exit leaves before the page ends, continuing the movement.
    let mut e = Animation::new(AnimationKind::SlideLeft, 0.5, 0.0);
    e.exit = Some(AnimationKind::SlideLeft);
    let p = page(vec![rect(2, 300.0, 300.0, Some(e))]);
    let d = page_duration(&p);
    assert_eq!(page_at(&p, 0.5), p, "in place between entrance and exit");
    let gone = page_at(&p, d);
    assert!(gone.elements[0].opacity < 0.01);
    assert!(page_at(&p, d - 0.1).elements[0].frame.x < 300.0, "exits to the left");
}

#[test]
fn ids_match_the_file_format() {
    for (k, id, label) in ENTRANCES {
        assert_eq!(serde_json::to_value(k).unwrap(), serde_json::json!(id));
        assert_eq!(parse_kind(id), Some(*k));
        assert_eq!(kind_id(*k), *id);
        assert_eq!(kind_label(*k), *label);
        assert!(label.is_ascii());
    }
    for (k, id, _) in LOOPS {
        assert_eq!(serde_json::to_value(k).unwrap(), serde_json::json!(id));
        assert_eq!(parse_loop(id), Some(*k));
        assert_eq!(loop_id(*k), *id);
    }
    assert_eq!(parse_kind("Slide_Left"), Some(AnimationKind::SlideLeft));
    assert_eq!(parse_kind("wobble"), None);
}
