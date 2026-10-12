use serde_json::json;

use crate::{EngineError, Session};
use genha_doc::{AnimationKind, LoopKind, NodeId};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 400, "height": 300})).unwrap();
    s
}

fn add_rect(s: &mut Session, x: f64) -> u64 {
    s.execute("shape.add", &json!({"shape": "rect", "x": x, "y": 100, "width": 80, "height": 60, "fill": "#ff3366"})).unwrap()["id"].as_u64().unwrap()
}

fn anim(s: &Session, id: u64) -> Option<genha_doc::Animation> {
    s.doc().unwrap().doc.element(NodeId(id)).unwrap().animation.clone()
}

fn tmp(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("genha-motion-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).display().to_string()
}

#[test]
fn set_patch_and_remove() {
    let mut s = session();
    let a = add_rect(&mut s, 10.0);
    // Not animated yet: a kind is needed.
    assert!(matches!(s.execute("animation.set", &json!({"duration": 1})), Err(EngineError::BadParams { .. })));
    s.execute("animation.set", &json!({"kind": "pop"})).unwrap();
    let got = anim(&s, a).unwrap();
    assert_eq!((got.kind, got.duration, got.delay), (AnimationKind::Pop, 0.6, 0.0));
    // Later calls change only what they name.
    s.execute("animation.set", &json!({"delay": 0.5, "loop": "pulse", "exit": "fade"})).unwrap();
    let got = anim(&s, a).unwrap();
    assert_eq!((got.kind, got.delay, got.looping, got.exit), (AnimationKind::Pop, 0.5, Some(LoopKind::Pulse), Some(AnimationKind::Fade)));
    s.execute("animation.set", &json!({"loop": "none", "exit": null})).unwrap();
    assert_eq!(anim(&s, a).unwrap().looping, None);
    assert_eq!(anim(&s, a).unwrap().exit, None);
    // Hostile numbers are refused or capped, never stored.
    assert!(s.execute("animation.set", &json!({"duration": -1})).is_err());
    assert!(s.execute("animation.set", &json!({"kind": "wobble"})).is_err());
    assert!(s.execute("animation.set", &json!({"loop": 7})).is_err());
    s.execute("animation.set", &json!({"duration": 1e9})).unwrap();
    assert_eq!(anim(&s, a).unwrap().duration, genha_motion::MAX_ANIMATION_SECONDS);
    // Undo restores, remove and kind none clear.
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(anim(&s, a).unwrap().duration, 0.6);
    s.execute("animation.remove", &json!({"ids": [a]})).unwrap();
    assert!(anim(&s, a).is_none());
    s.execute("animation.set", &json!({"ids": [a], "kind": "slide_left"})).unwrap();
    assert_eq!(anim(&s, a).unwrap().kind, AnimationKind::SlideLeft);
    s.execute("animation.set", &json!({"ids": [a], "kind": "none"})).unwrap();
    assert!(anim(&s, a).is_none());
}

#[test]
fn apply_to_page_staggers_and_skips_backgrounds() {
    let mut s = session();
    let bg = s.execute("shape.add", &json!({"shape": "rect", "x": 0, "y": 0, "width": 400, "height": 300})).unwrap()["id"].as_u64().unwrap();
    let a = add_rect(&mut s, 10.0);
    let b = add_rect(&mut s, 200.0);
    let r = s.execute("animation.applyToPage", &json!({"kind": "rise", "stagger": 0.4})).unwrap();
    assert_eq!(r["animated"], 2);
    assert!(anim(&s, bg).is_none(), "the background stays still");
    assert_eq!(anim(&s, a).unwrap().delay, 0.0);
    assert!((anim(&s, b).unwrap().delay - 0.4).abs() < 1e-9);
    assert!((r["page_duration"].as_f64().unwrap() - (0.4 + 0.6 + genha_motion::HOLD)).abs() < 1e-9);
    // One undo step.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(anim(&s, a).is_none() && anim(&s, b).is_none());
    s.execute("animation.applyToPage", &json!({"kind": "fade", "include_background": true})).unwrap();
    assert!(anim(&s, bg).is_some());
    s.execute("animation.applyToPage", &json!({"kind": "none", "include_background": true})).unwrap();
    assert!(anim(&s, bg).is_none() && anim(&s, a).is_none());
    assert!(s.execute("animation.applyToPage", &json!({})).is_err());
    assert!(s.execute("animation.applyToPage", &json!({"kind": "fade", "page": 9})).is_err());
}

#[test]
fn list_and_timeline() {
    let mut s = session();
    let l = s.execute("animation.list", &json!({})).unwrap();
    assert_eq!(l["entrances"].as_array().unwrap().len(), genha_motion::ENTRANCES.len());
    assert_eq!(l["loops"][0]["id"], "pulse");
    let a = add_rect(&mut s, 10.0);
    s.execute("animation.set", &json!({"ids": [a], "kind": "zoom", "duration": 1.0, "delay": 0.5, "exit": "fade"})).unwrap();
    let t = s.execute("animation.timeline", &json!({})).unwrap();
    assert_eq!(t["animations"][0]["kind"], "zoom");
    assert_eq!(t["animations"][0]["end"], 1.5);
    assert_eq!(t["duration"].as_f64().unwrap(), 1.5 + 1.0 + genha_motion::HOLD);
}

#[test]
fn gif_and_apng_exports_decode() {
    use image::AnimationDecoder;
    let mut s = session();
    let a = add_rect(&mut s, 10.0);
    s.execute("text.add", &json!({"text": "Hi"})).unwrap();
    s.execute("animation.applyToPage", &json!({"kind": "slide-up"})).unwrap();
    s.execute("animation.set", &json!({"ids": [a], "kind": "pop"})).unwrap();
    let total = genha_motion::page_duration(s.doc().unwrap().doc.page(0).unwrap());

    let gif = tmp("a.gif");
    let r = s.execute("export.gif", &json!({"path": gif, "fps": 10, "scale": 0.5})).unwrap();
    assert_eq!((r["width"].as_u64(), r["height"].as_u64()), (Some(200), Some(150)));
    let frames = r["frames"].as_u64().unwrap() as usize;
    // Still frames at the end merge into one, so fewer frames than duration × fps.
    assert!(frames > 5 && frames < (total * 10.0).ceil() as usize, "{frames}");
    let dec = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(&gif).unwrap())).unwrap();
    let decoded = dec.into_frames().collect_frames().unwrap();
    assert_eq!(decoded.len(), frames);
    let ms: f64 = decoded.iter().map(|f| f64::from(f.delay().numer_denom_ms().0) / f64::from(f.delay().numer_denom_ms().1)).sum();
    assert!((ms / 1000.0 - total).abs() < 0.05, "{ms} ms vs {total} s");
    // The first frame differs from the last (the page animates in).
    assert_ne!(decoded[0].buffer(), decoded[decoded.len() - 1].buffer());

    let apng = tmp("a.png");
    let r = s.execute("export.apng", &json!({"path": apng, "fps": 10, "scale": 0.5, "loop": false})).unwrap();
    assert_eq!(r["frames"].as_u64().unwrap() as usize, frames);
    let dec = image::codecs::png::PngDecoder::new(std::io::BufReader::new(std::fs::File::open(&apng).unwrap())).unwrap();
    assert!(dec.is_apng().unwrap());
    assert_eq!(dec.apng().unwrap().into_frames().collect_frames().unwrap().len(), frames);
}

#[test]
fn exports_cap_size_and_frames_and_letterbox_pages() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 4000, "height": 4000})).unwrap();
    s.execute("page.setDuration", &json!({"seconds": 3600})).unwrap();
    s.execute("shape.add", &json!({})).unwrap();
    s.execute("animation.set", &json!({"kind": "fade", "loop": "sway"})).unwrap();
    // A second page of another size.
    s.execute("page.add", &json!({"width": 100, "height": 400})).unwrap();
    let r = s.execute("export.gif", &json!({"path": tmp("big.gif"), "all_pages": true, "fps": 5, "scale": 0.1})).unwrap();
    assert_eq!(r["width"], 400);
    assert!(r["frames"].as_u64().unwrap() as usize <= super::MAX_FRAMES);
    // The pixel cap lowers the scale.
    let plan_scale = {
        let r = s.execute("export.apng", &json!({"path": tmp("big.png"), "page": 1, "fps": 5, "scale": 4})).unwrap();
        r["width"].as_u64().unwrap() * r["height"].as_u64().unwrap()
    };
    assert!(plan_scale as f64 <= super::MAX_FRAME_PIXELS * 1.01);
    assert!(s.execute("export.gif", &json!({"path": tmp("x.gif"), "pages": [0, 99]})).is_err());
    assert!(s.execute("export.gif", &json!({"path": tmp("x.gif"), "pages": []})).is_err());
}

#[test]
fn frame_export_shows_a_moment() {
    let mut s = session();
    s.execute("page.setBackground", &json!({"color": "#ffffff"})).unwrap();
    add_rect(&mut s, 10.0);
    s.execute("animation.set", &json!({"kind": "fade", "duration": 1.0})).unwrap();
    let path = tmp("f.png");
    s.execute("export.frame", &json!({"path": path, "t": 0})).unwrap();
    let img = image::open(&path).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(50, 130).0, [255, 255, 255, 255], "hidden at t=0");
    s.execute("export.frame", &json!({"path": path, "t": 5})).unwrap();
    let img = image::open(&path).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(50, 130).0, [255, 0x33, 0x66, 255], "in place at the end");
    assert!(s.execute("export.frame", &json!({"path": path})).is_err());
}
