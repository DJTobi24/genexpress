use super::*;

fn rect(doc: &mut Document) -> Element {
    let id = doc.alloc_id();
    Element::new(
        id,
        Frame::new(10.0, 10.0, 100.0, 50.0),
        ElementKind::Shape(ShapeEl {
            shape: ShapeKind::Rect { corner_radius: 4.0 },
            fill: Fill::solid(Color::rgb(255, 0, 0)),
            stroke: None,
            shadow: None,
        }),
    )
}

#[test]
fn find_and_remove_through_groups() {
    let mut doc = Document::new("t", 500.0, 500.0);
    let a = rect(&mut doc);
    let b = rect(&mut doc);
    let (aid, bid) = (a.id, b.id);
    let gid = doc.alloc_id();
    let group = Element::new(gid, Frame::new(0.0, 0.0, 10.0, 10.0), ElementKind::Group(GroupEl { children: vec![a, b] }));
    doc.pages[0].elements.push(group);
    assert_eq!(doc.find(bid).map(|(p, e)| (p, e.id)), Some((0, bid)));
    assert!(doc.remove(aid).is_some());
    assert!(doc.find(aid).is_none());
    assert_eq!(doc.pages[0].ids(), vec![gid, bid]);
}

#[test]
fn json_round_trip_is_readable() {
    let mut doc = Document::new("Round trip", 1080.0, 1080.0);
    let el = rect(&mut doc);
    doc.pages[0].elements.push(el);
    let asset = doc.add_asset(Asset { name: "a.png".into(), mime: "image/png".into(), width: 1, height: 1, data: Bytes::new(vec![1, 2, 3]) });
    let iid = doc.alloc_id();
    doc.pages[0].elements.push(Element::new(iid, Frame::new(0.0, 0.0, 10.0, 10.0), ElementKind::Image(ImageEl::new(asset))));
    let tid = doc.alloc_id();
    doc.pages[0].elements.push(Element::new(
        tid,
        Frame::new(0.0, 0.0, 10.0, 10.0),
        ElementKind::Text(TextEl { text: "Hello".into(), style: TextStyle::default(), effect: TextEffect::None }),
    ));
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("\"type\": \"shape\""));
    assert!(json.contains("\"kind\": \"rect\""));
    assert!(json.contains("\"#ff0000\""));
    assert!(json.contains("\"AQID\""), "asset bytes are base64");
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(back, doc);
}

#[test]
fn repair_fixes_untrusted_documents() {
    let mut doc = Document::new("t", f64::NAN, -3.0);
    let mut a = rect(&mut doc);
    a.frame.width = f64::INFINITY;
    a.opacity = f32::NAN;
    let mut b = a.clone();
    b.id = a.id; // duplicate id
    doc.pages[0].elements.push(a);
    doc.pages[0].elements.push(b);
    doc.next_id = 0;
    let notes = doc.repair();
    assert!(!notes.is_empty());
    let ids = doc.pages[0].ids();
    assert_ne!(ids[0], ids[1]);
    assert!(doc.next_id > ids.iter().map(|i| i.0).max().unwrap());
    assert!(doc.pages[0].elements.iter().all(|e| e.frame.is_valid() && e.opacity == 1.0));
    assert_eq!(doc.pages[0].height, 1.0);
    assert_eq!(doc.pages[0].width, 1080.0);

    let mut empty = Document { pages: vec![], ..Document::default() };
    empty.repair();
    assert_eq!(empty.pages.len(), 1);
}

#[test]
fn presets_are_sane() {
    let mut seen = std::collections::HashSet::new();
    for p in PRESETS {
        assert!(seen.insert(p.id), "duplicate preset {}", p.id);
        assert!(p.width >= 1.0 && p.height >= 1.0 && p.width <= MAX_PAGE_SIDE && p.height <= MAX_PAGE_SIDE);
    }
    assert!(preset::find("instagram-story").is_some());
    assert_eq!(preset::categories().first(), Some(&"Social"));
}

#[test]
fn unused_assets() {
    let mut doc = Document::default();
    let a = doc.add_asset(Asset { name: "x".into(), mime: "image/png".into(), width: 0, height: 0, data: Bytes::default() });
    assert_eq!(doc.unused_assets(), vec![a]);
}

#[test]
fn animation_serde_is_backwards_compatible() {
    // Files written before loops and exits existed still read.
    let a: Animation = serde_json::from_str(r#"{"kind":"slide-left","duration":0.5}"#).unwrap();
    assert_eq!(a, Animation::new(AnimationKind::SlideLeft, 0.5, 0.0));
    let full = Animation { looping: Some(LoopKind::Pulse), exit: Some(AnimationKind::Fade), ..Animation::new(AnimationKind::Pop, 0.6, 0.2) };
    let json = serde_json::to_string(&full).unwrap();
    assert!(json.contains(r#""loop":"pulse""#) && json.contains(r#""exit":"fade""#), "{json}");
    assert_eq!(serde_json::from_str::<Animation>(&json).unwrap(), full);
    // A plain entrance writes no extra fields.
    assert!(!serde_json::to_string(&Animation::new(AnimationKind::Fade, 1.0, 0.0)).unwrap().contains("loop"));
}
