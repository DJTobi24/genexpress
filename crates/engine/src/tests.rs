use serde_json::json;

use super::*;
use genha_doc::{ElementKind, Fill};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"preset": "instagram-post"})).unwrap();
    s
}

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

#[test]
fn new_add_undo_redo() {
    let mut s = session();
    let r = s.execute("shape.add", &json!({"shape": "rect", "x": 10, "y": 20, "width": 100, "height": 50, "fill": "#ff0000"})).unwrap();
    let rid = NodeId(id(&r));
    assert_eq!(s.doc().unwrap().selection, vec![rid]);
    let el = s.doc().unwrap().doc.element(rid).unwrap().clone();
    assert_eq!(el.frame, genha_geom::Frame::new(10.0, 20.0, 100.0, 50.0));
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.find(rid).is_none());
    s.execute("edit.redo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.find(rid).is_some());
    assert!(matches!(s.execute("edit.redo", &json!({})), Err(EngineError::Disabled(..))));
}

#[test]
fn unknown_and_bad_params() {
    let mut s = session();
    assert!(matches!(s.execute("nope", &json!({})), Err(EngineError::UnknownCommand(_))));
    assert!(matches!(s.execute("shape.add", &json!({"shape": "blob"})), Err(EngineError::BadParams { .. })));
    assert!(matches!(s.execute("shape.add", &json!({"fill": "#nothex"})), Err(EngineError::BadParams { .. })));
    assert!(matches!(s.execute("select.set", &json!({"ids": "x"})), Err(EngineError::BadParams { .. })));
    // A failed command leaves no undo step.
    assert!(s.doc().unwrap().history.undo.is_empty());
}

#[test]
fn text_and_style() {
    let mut s = session();
    let t = s.execute("text.add", &json!({"text": "Hello", "role": "heading"})).unwrap();
    s.execute("text.setStyle", &json!({"font_size": 99, "align": "left", "color": "#112233"})).unwrap();
    let el = s.doc().unwrap().doc.element(NodeId(id(&t))).unwrap().clone();
    let ElementKind::Text(te) = el.kind else { panic!() };
    assert_eq!(te.style.font_size, 99.0);
    assert!(te.style.bold, "heading role is bold and keeps it");
    assert_eq!(te.style.color.to_hex(), "#112233");
    assert!(s.execute("text.setStyle", &json!({"font_size": -3})).is_err());
    s.execute("text.setEffect", &json!({"effect": {"type": "outline", "color": "#000000", "width": 3}})).unwrap();
    s.execute("style.copy", &json!({})).unwrap();
    let t2 = s.execute("text.add", &json!({"text": "Other"})).unwrap();
    s.execute("style.paste", &json!({})).unwrap();
    let el2 = s.doc().unwrap().doc.element(NodeId(id(&t2))).unwrap().clone();
    let ElementKind::Text(te2) = el2.kind else { panic!() };
    assert_eq!(te2.style.font_size, 99.0);
}

#[test]
fn group_ungroup_and_arrange() {
    let mut s = session();
    let a = id(&s.execute("shape.add", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap());
    let b = id(&s.execute("shape.add", &json!({"x": 100, "y": 100, "width": 10, "height": 10})).unwrap());
    let c = id(&s.execute("shape.add", &json!({"shape": "ellipse"})).unwrap());
    s.execute("select.set", &json!({"ids": [a, b]})).unwrap();
    let g = id(&s.execute("object.group", &json!({})).unwrap());
    let doc = &s.doc().unwrap().doc;
    assert_eq!(doc.pages[0].elements.len(), 2);
    assert_eq!(doc.pages[0].elements[0].id.0, g);
    assert_eq!(doc.pages[0].elements[0].frame, genha_geom::Frame::new(0.0, 0.0, 110.0, 110.0));
    // Moving the group moves its children.
    s.execute("object.move", &json!({"dx": 5, "dy": 0})).unwrap();
    assert_eq!(s.doc().unwrap().doc.element(NodeId(b)).unwrap().frame.x, 105.0);
    s.execute("object.bringToFront", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements[1].id.0, g);
    s.execute("object.ungroup", &json!({})).unwrap();
    let ids: Vec<u64> = s.doc().unwrap().doc.pages[0].elements.iter().map(|e| e.id.0).collect();
    assert_eq!(ids, vec![c, a, b]);
    // Scaling a group scales the children about it.
    s.execute("select.set", &json!({"ids": [a, b]})).unwrap();
    let g = id(&s.execute("object.group", &json!({})).unwrap());
    s.execute("object.setFrame", &json!({"id": g, "width": 220, "height": 220})).unwrap();
    let fb = s.doc().unwrap().doc.element(NodeId(b)).unwrap().frame;
    assert_eq!((fb.x, fb.width), (205.0, 20.0));
}

#[test]
fn clipboard_duplicate_delete_lock() {
    let mut s = session();
    let a = id(&s.execute("shape.add", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap());
    s.execute("edit.copy", &json!({})).unwrap();
    let pasted = s.execute("edit.paste", &json!({})).unwrap();
    let p = pasted["ids"][0].as_u64().unwrap();
    assert_ne!(p, a);
    assert_eq!(s.doc().unwrap().doc.element(NodeId(p)).unwrap().frame.x, 20.0);
    s.execute("edit.duplicate", &json!({"ids": [a]})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements.len(), 3);
    s.execute("object.lock", &json!({"ids": [a]})).unwrap();
    let r = s.execute("edit.delete", &json!({"ids": [a]})).unwrap();
    assert_eq!(r["removed"], 0);
    assert!(s.execute("object.move", &json!({"ids": [a], "dx": 1})).is_err());
    // Paste into another design carries the clipboard.
    s.execute("file.new", &json!({})).unwrap();
    s.execute("edit.paste", &json!({"in_place": true})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements.len(), 1);
}

#[test]
fn pages() {
    let mut s = session();
    s.execute("page.add", &json!({"preset": "instagram-story"})).unwrap();
    assert_eq!(s.doc().unwrap().page, 1);
    assert_eq!(s.doc().unwrap().doc.pages[1].height, 1920.0);
    s.execute("shape.add", &json!({})).unwrap();
    s.execute("page.duplicate", &json!({})).unwrap();
    let d = s.doc().unwrap();
    assert_eq!(d.doc.pages.len(), 3);
    assert_ne!(d.doc.pages[1].elements[0].id, d.doc.pages[2].elements[0].id);
    s.execute("page.move", &json!({"from": 2, "to": 0})).unwrap();
    s.execute("page.setBackground", &json!({"color": "#000000"})).unwrap();
    s.execute("page.delete", &json!({"index": 1})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages.len(), 2);
    s.execute("page.goto", &json!({"index": 1})).unwrap();
    assert!(s.execute("page.goto", &json!({"index": 9})).is_err());
    let d = s.doc().unwrap();
    assert_eq!(d.doc.pages[0].background, Fill::solid(genha_doc::Color::BLACK));
}

#[test]
fn batch_is_one_undo_step_and_rolls_back() {
    let mut s = session();
    s.execute(
        "command.batch",
        &json!({"label": "Two", "commands": [
            {"command": "shape.add", "params": {}},
            {"command": "text.add", "params": {"text": "x"}}
        ]}),
    )
    .unwrap();
    let d = s.doc().unwrap();
    assert_eq!(d.history.undo.len(), 1);
    assert_eq!(d.history.undo[0].label, "Two");
    assert_eq!(d.doc.pages[0].elements.len(), 2);
    let r = s.execute("command.batch", &json!({"commands": [{"command": "shape.add"}, {"command": "shape.add", "params": {"shape": "bogus"}}]}));
    assert!(r.is_err());
    assert_eq!(s.doc().unwrap().doc.pages[0].elements.len(), 2);
    assert_eq!(s.doc().unwrap().history.undo.len(), 1);
}

fn png_bytes() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(40, 20, image::Rgba([0, 128, 255, 255]));
    let mut out = vec![];
    image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

#[test]
fn pictures() {
    use base64::Engine as _;
    let mut s = session();
    let data = base64::engine::general_purpose::STANDARD.encode(png_bytes());
    let r = s.execute("image.place", &json!({"data": data, "name": "blue.png"})).unwrap();
    let el = s.doc().unwrap().doc.element(NodeId(id(&r))).unwrap().clone();
    assert!((el.frame.width / el.frame.height - 2.0).abs() < 1e-9);
    s.execute("image.adjust", &json!({"brightness": 20, "grayscale": 100})).unwrap();
    s.execute("image.crop", &json!({"x": 0.1, "y": 0.1, "width": 0.5, "height": 0.5})).unwrap();
    s.execute("image.mask", &json!({"shape": "circle"})).unwrap();
    s.execute("style.cornerRadius", &json!({"radius": 12})).unwrap();
    // Same picture again shares its data.
    s.execute("image.place", &json!({"data": data})).unwrap();
    assert_eq!(s.doc().unwrap().doc.assets.len(), 1);
    assert!(s.execute("image.place", &json!({"data": "!!!"})).is_err());
    assert!(s.execute("image.place", &json!({"data": base64::engine::general_purpose::STANDARD.encode(b"not a picture")})).is_err());
}

#[test]
fn save_and_open_round_trip() {
    let mut s = session();
    s.execute("text.add", &json!({"text": "Saved"})).unwrap();
    let dir = std::env::temp_dir().join(format!("genha-engine-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t");
    let r = s.execute("file.saveAs", &json!({"path": path.to_str().unwrap()})).unwrap();
    let saved = r["path"].as_str().unwrap().to_string();
    assert!(saved.ends_with("t.genha"));
    assert!(!s.doc().unwrap().dirty);
    s.execute("file.open", &json!({"path": saved})).unwrap();
    assert_eq!(s.docs.len(), 2);
    assert_eq!(s.docs[0].doc, s.docs[1].doc);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn tools_reduce_to_commands() {
    use genha_tools::{PointerEvent, PointerKind, ToolId};
    let mut s = session();
    s.tools.set_tool(ToolId::Rect);
    let ev = |kind, x, y| PointerEvent { kind, pos: genha_geom::Point::new(x, y), shift: false, alt: false };
    s.tool_event(ev(PointerKind::Down, 100.0, 100.0), 1.0, (&[], &[])).unwrap();
    s.tool_event(ev(PointerKind::Move, 200.0, 150.0), 1.0, (&[], &[])).unwrap();
    assert!(s.preview.as_ref().unwrap().creating.is_some());
    s.tool_event(ev(PointerKind::Up, 200.0, 150.0), 1.0, (&[], &[])).unwrap();
    let el = &s.doc().unwrap().doc.pages[0].elements[0];
    assert_eq!(el.frame, genha_geom::Frame::new(100.0, 100.0, 100.0, 50.0));
    // Move it with the select tool; the preview shows in display_document.
    s.tools.set_tool(ToolId::Select);
    s.tool_event(ev(PointerKind::Down, 150.0, 120.0), 1.0, (&[], &[])).unwrap();
    s.tool_event(ev(PointerKind::Move, 160.0, 120.0), 1.0, (&[], &[])).unwrap();
    assert_eq!(s.display_document().unwrap().pages[0].elements[0].frame.x, 110.0);
    assert_eq!(s.doc().unwrap().doc.pages[0].elements[0].frame.x, 100.0);
    s.tool_event(ev(PointerKind::Up, 160.0, 120.0), 1.0, (&[], &[])).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements[0].frame.x, 110.0);
}

#[test]
fn smart_guides_preference_and_temporary_off() {
    use genha_tools::{PointerEvent, PointerKind};
    let mut s = session(); // 1080 × 1080
    s.store = Some(Box::new(crate::store::MemStore::default()));
    let id = id(&s.execute("shape.add", &json!({"x": 100, "y": 100, "width": 200, "height": 100})).unwrap());
    s.execute("select.set", &json!({"ids": [id]})).unwrap();
    let ev = |kind, x, y| PointerEvent { kind, pos: genha_geom::Point::new(x, y), shift: false, alt: false };
    let x_after_drag = |s: &mut Session, snap: bool| {
        // Dragged 337 px right: the centre lands 3 px short of the page centre (540).
        s.tool_event_with(ev(PointerKind::Down, 150.0, 150.0), 1.0, (&[], &[]), snap).unwrap();
        s.tool_event_with(ev(PointerKind::Move, 487.0, 150.0), 1.0, (&[], &[]), snap).unwrap();
        let shown = s.preview.as_ref().map(|p| (p.guides_x.clone(), p.measures.len())).unwrap();
        s.tool_event_with(ev(PointerKind::Up, 487.0, 150.0), 1.0, (&[], &[]), snap).unwrap();
        let x = s.doc().unwrap().doc.pages[0].elements[0].frame.x;
        s.execute("edit.undo", &json!({})).unwrap();
        s.execute("select.set", &json!({"ids": [id]})).unwrap();
        (x, shown)
    };
    let (x, (guides, measures)) = x_after_drag(&mut s, true);
    assert_eq!((x, guides), (440.0, vec![540.0]));
    assert!(measures >= 4, "distances to the page edges");
    // Held Ctrl/Cmd: no snapping, distances still shown.
    let (x, (guides, measures)) = x_after_drag(&mut s, false);
    assert_eq!((x, guides.len()), (437.0, 0));
    assert!(measures >= 4);
    // The preference turns it off (and is kept in the store).
    assert_eq!(s.execute("view.snapping", &json!({})).unwrap()["on"], false);
    assert_eq!(x_after_drag(&mut s, true).0, 437.0);
    let mut next = Session::new();
    next.store = s.store.take();
    crate::cmd::edit::load_preferences(&mut next);
    assert!(!next.snapping);
    assert_eq!(next.execute("view.snapping", &json!({"on": true})).unwrap()["on"], true);
    assert!(next.execute("view.snapping", &json!({"on": "yes"})).is_err());
}

#[test]
fn every_command_survives_junk_params() {
    // Untrusted params never panic (the guard would turn a panic into Internal: that's a bug too).
    let junk = [
        json!(null),
        json!({}),
        json!({"ids": [999999]}),
        json!({"x": "a", "index": -1, "color": 5}),
        json!([1, 2]),
        // Charts and tables: huge or wrong ranges, broken data and styles.
        json!({"row": 1e20, "col": -1, "rows": 999999, "cols": "x", "width": -5, "height": 1e308, "auto": "yes"}),
        json!({"row": 0, "col": 0, "rows": 100, "cols": 30, "fill": "#zz", "font_size": 1e308, "bold": 2, "align": 5, "clear": true}),
        json!({"row": 2, "col": 1, "cols": 2, "width": 1e300, "height": 0}),
        json!({"csv": "\"\n,,\"\"\t", "rows": [[1, {"a": 2}], null], "series": [{"values": [1e308, null, "x"]}], "categories": 5}),
        json!({"series": [{"name": 5, "values": [], "color": "red"}], "categories": [null, {}, 3], "legend": 3, "min": "a", "max": {}, "decimals": 99}),
        json!({"legend": "right", "stacked": true, "min": 1e300, "max": -1e300, "value_format": "currency", "currency": "€€€€€€€€€€€€"}),
    ];
    let mut s = session();
    s.execute("shape.add", &json!({})).unwrap();
    s.execute("text.add", &json!({})).unwrap();
    s.execute("text.setCurve", &json!({"degrees": 90})).unwrap();
    s.execute("chart.add", &json!({"kind": "pie"})).unwrap();
    s.execute("chart.add", &json!({"csv": "x,a,b\n1,2,3\n4,5,6", "stacked": true})).unwrap();
    s.execute("table.add", &json!({})).unwrap();
    s.execute("table.mergeCells", &json!({"row": 1, "col": 0, "rows": 2, "cols": 2})).unwrap();
    for spec in command_specs() {
        if ["file.open", "file.save", "file.saveAs"].contains(&spec.id) {
            continue;
        }
        for j in &junk {
            let r = s.execute(spec.id, j);
            assert!(!matches!(r, Err(EngineError::Internal { .. })), "{} panicked on {j}", spec.id);
            if s.docs.is_empty() {
                s.execute("file.new", &json!({})).unwrap();
            }
        }
    }
}

#[test]
fn text_commands_survive_junk_while_editing_formatted_text() {
    let junk = [
        json!(null),
        json!({}),
        json!({"range": [9_999_999, 2], "bold": true}),
        json!({"range": "x", "font_size": 1e308}),
        json!({"range": [0, 3], "font_size": f64::MAX, "color": "#zz"}),
        json!({"insert": "\u{0}\r\n\u{202e}é", "range": [3, 1]}),
        json!({"text": "x".repeat(300), "selection": [1000, 0]}),
        json!({"selection": [5, 5], "font_family": "\u{0}"}),
        json!({"ids": [999999], "range": [0, 1]}),
        json!({"id": "x", "underline": "yes"}),
    ];
    let mut s = session();
    let id = s.execute("text.add", &json!({"text": "Héllo wörld\nאב 中文"})).unwrap()["id"].as_u64().unwrap();
    s.execute("text.setStyle", &json!({"ids": [id], "range": [0, 5], "bold": true, "color": "#ff0000", "font_size": 90})).unwrap();
    s.execute("text.setStyle", &json!({"ids": [id], "range": [3, 9], "italic": true, "strikethrough": true})).unwrap();
    for spec in command_specs().iter().filter(|c| c.id.starts_with("text.") || c.id.starts_with("style.")) {
        for j in &junk {
            if s.text_editing.is_none() {
                let _ = s.execute("text.beginEdit", &json!({"id": id, "selection": [2, 7]}));
            }
            let r = s.execute(spec.id, j);
            assert!(!matches!(r, Err(EngineError::Internal { .. })), "{} panicked on {j}", spec.id);
            if let Ok(d) = s.doc()
                && let Ok(el) = d.doc.element(genha_doc::NodeId(id))
                && let ElementKind::Text(t) = &el.kind
            {
                assert!(t.spans_normalized(), "{} left bad spans on {j}: {:?}", spec.id, t.spans);
            }
            if s.docs.is_empty() {
                return;
            }
        }
    }
}

#[test]
fn command_ids_are_unique_and_documented() {
    let mut seen = std::collections::HashSet::new();
    for c in command_specs() {
        assert!(seen.insert(c.id), "duplicate command {}", c.id);
        assert!(!c.label.is_empty() && !c.params.is_empty(), "{} needs a label and params doc", c.id);
    }
}

#[test]
fn merge_key_coalesces_steps() {
    let mut s = session();
    s.execute("text.add", &json!({"text": "a"})).unwrap();
    for t in ["ab", "abc", "abcd"] {
        s.execute("text.setText", &json!({"text": t, "merge_key": "typing"})).unwrap();
    }
    assert_eq!(s.doc().unwrap().history.undo.len(), 2);
    s.execute("edit.undo", &json!({})).unwrap();
    let ElementKind::Text(t) = &s.doc().unwrap().doc.pages[0].elements[0].kind else { panic!() };
    assert_eq!(t.text, "a");
}

#[test]
fn exports_write_files() {
    let mut s = session();
    s.execute("shape.add", &json!({"fill": "#ff0000"})).unwrap();
    s.execute("page.add", &json!({})).unwrap();
    let dir = std::env::temp_dir().join(format!("genha-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = |n: &str| dir.join(n).display().to_string();
    s.execute("export.png", &json!({"path": path("a.png"), "page": 0, "scale": 0.25})).unwrap();
    let png = std::fs::read(dir.join("a.png")).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let r = s.execute("export.jpeg", &json!({"path": path("b.jpg"), "all_pages": true, "scale": 0.2})).unwrap();
    assert_eq!(r["files"].as_array().unwrap().len(), 2);
    assert!(dir.join("b-2.jpg").exists());
    s.execute("export.svg", &json!({"path": path("c.svg")})).unwrap();
    assert!(std::fs::read_to_string(dir.join("c.svg")).unwrap().contains("<svg"));
    s.execute("export.pdf", &json!({"path": path("d.pdf"), "dpi": 40})).unwrap();
    assert!(std::fs::read(dir.join("d.pdf")).unwrap().starts_with(b"%PDF"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn templates_and_libraries() {
    let mut s = Session::new();
    let list = s.execute("template.list", &json!({})).unwrap();
    let first = list[0]["id"].as_str().unwrap().to_string();
    assert!(list.as_array().unwrap().len() >= 10);
    s.execute("template.use", &json!({"id": first})).unwrap();
    let pages = s.doc().unwrap().doc.pages.len();
    s.execute("template.use", &json!({"id": first, "as_page": true})).unwrap();
    assert_eq!(s.docs.len(), 1);
    assert!(s.doc().unwrap().doc.pages.len() > pages);
    assert!(s.execute("template.use", &json!({"id": "nope"})).is_err());
    let lib = s.execute("library.list", &json!({})).unwrap();
    let shape = lib[0]["id"].as_str().unwrap().to_string();
    s.execute("library.addShape", &json!({"id": shape})).unwrap();
    let presets = s.execute("textPresets.list", &json!({})).unwrap();
    s.execute("text.addPreset", &json!({"id": presets[0]["id"]})).unwrap();
    let pal = s.execute("palettes.list", &json!({})).unwrap();
    s.execute("brand.usePalette", &json!({"id": pal[0]["id"]})).unwrap();
    assert_eq!(s.doc().unwrap().doc.brand.colors[0].color.to_hex(), pal[0]["colors"][0].as_str().unwrap());
    s.execute("element.qrCode", &json!({"text": "https://example.com"})).unwrap();
    assert!(s.execute("element.qrCode", &json!({"text": ""})).is_err());
}

#[test]
fn smart_resize_align_distribute() {
    let mut s = session();
    s.execute("text.add", &json!({"text": "Title", "role": "heading"})).unwrap();
    s.execute("page.resize", &json!({"preset": "instagram-story", "copy": true})).unwrap();
    let d = s.doc().unwrap();
    assert_eq!(d.doc.pages.len(), 2);
    assert_eq!((d.doc.pages[1].width, d.doc.pages[1].height), (1080.0, 1920.0));
    assert_eq!(d.page, 1);
    let r = s.execute("page.resize", &json!({"width": 500, "height": 500})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[1].width, 500.0);
    assert!(r["notes"].is_array());
    for mode in ["scale", "none", "smart"] {
        s.execute("page.resize", &json!({"width": 600, "height": 400, "mode": mode})).unwrap();
        assert_eq!(s.doc().unwrap().doc.pages[1].height, 400.0);
    }
    assert!(s.execute("page.resize", &json!({"width": 600, "height": 400, "mode": "stretch"})).is_err());
    s.execute("page.resize", &json!({"width": 500, "height": 500})).unwrap();
    let a = id(&s.execute("shape.add", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap());
    s.execute("object.align", &json!({"how": "right"})).unwrap();
    assert_eq!(s.doc().unwrap().doc.element(NodeId(a)).unwrap().frame.x, 490.0);
    let b = id(&s.execute("shape.add", &json!({"x": 100, "y": 0, "width": 10, "height": 10})).unwrap());
    let c = id(&s.execute("shape.add", &json!({"x": 130, "y": 0, "width": 10, "height": 10})).unwrap());
    s.execute("select.set", &json!({"ids": [a, b, c]})).unwrap();
    s.execute("object.distribute", &json!({"axis": "horizontal"})).unwrap();
    s.execute("object.align", &json!({"how": "top"})).unwrap();
    assert!(s.execute("object.align", &json!({"how": "diagonal"})).is_err());
}

#[test]
fn picture_features() {
    use base64::Engine as _;
    let mut s = session();
    // A red square on white: the background remover keeps the square.
    let mut img = image::RgbaImage::from_pixel(60, 60, image::Rgba([255, 255, 255, 255]));
    for y in 20..40 {
        for x in 20..40 {
            img.put_pixel(x, y, image::Rgba([220, 20, 20, 255]));
        }
    }
    let mut png = vec![];
    image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    let data = base64::engine::general_purpose::STANDARD.encode(&png);
    let pic = id(&s.execute("image.place", &json!({"data": data, "name": "sq.png"})).unwrap());
    s.execute("image.filter", &json!({"preset": "mono", "intensity": 50})).unwrap();
    assert!(s.execute("image.filter", &json!({"preset": "nope"})).is_err());
    s.execute("image.blurBackground", &json!({"amount": 8})).unwrap();
    s.execute("image.changeBackground", &json!({"color": "#00ff00"})).unwrap();
    assert!(s.execute("image.changeBackground", &json!({})).is_err(), "needs a colour");
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("image.removeBackground", &json!({})).unwrap();
    let d = s.doc().unwrap();
    let ElementKind::Image(im) = &d.doc.element(NodeId(pic)).unwrap().kind else { panic!() };
    let cut = d.doc.asset(im.asset).unwrap();
    assert!(cut.name.ends_with("-cutout.png"));
    let decoded = genha_effects::decode(&cut.data).unwrap();
    assert_eq!(decoded.get_pixel(2, 2)[3], 0, "the white backdrop is gone");
    assert_eq!(decoded.get_pixel(30, 30)[3], 255, "the subject stays");
    let asset = im.asset.0;
    s.execute("brand.fromPicture", &json!({"ids": [pic]})).unwrap();
    s.execute("image.placeAsset", &json!({"asset": asset})).unwrap();
    // Collage of two pictures.
    let p2 = id(&s.execute("image.place", &json!({"data": data})).unwrap());
    s.execute("select.set", &json!({"ids": [pic, p2]})).unwrap();
    let layout = genha_layout::collage_layouts(2)[0].id;
    s.execute("layout.collage", &json!({"layout": layout})).unwrap();
    // Quick action file in, file out.
    let dir = std::env::temp_dir().join(format!("genha-quick-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("in.png"), &png).unwrap();
    let r = s
        .execute("quick.image", &json!({"action": "resize", "width": 30, "input": dir.join("in.png").display().to_string(), "output": dir.join("out.jpg").display().to_string()}))
        .unwrap();
    assert_eq!((r["width"].as_u64(), r["height"].as_u64()), (Some(30), Some(30)));
    assert!(std::fs::read(dir.join("out.jpg")).unwrap().starts_with(&[0xff, 0xd8]));
    std::fs::remove_dir_all(&dir).unwrap();
    // The same on bytes, without files (the web build).
    let (jpg, w, _) = crate::cmd::content::quick_image_bytes(&png, "jpg", &json!({"action": "resize", "width": 30})).unwrap();
    assert_eq!(w, 30);
    assert!(jpg.starts_with(&[0xff, 0xd8]));
    assert!(crate::cmd::content::quick_image_bytes(&png, "tiff", &json!({"action": "convert"})).is_err());
    assert!(crate::cmd::content::quick_image_bytes(b"junk", "png", &json!({"action": "convert"})).is_err());
    // A design as bytes (the web build's Save downloads them): marks it saved, reopens the same.
    assert!(s.doc().unwrap().dirty);
    let bytes = crate::cmd::file::design_bytes(&mut s).unwrap();
    assert!(!s.doc().unwrap().dirty);
    let (doc, _) = genha_format::from_bytes(&bytes).unwrap();
    assert_eq!(doc.pages.len(), s.doc().unwrap().doc.pages.len());
}

#[test]
fn crop_page_to_content() {
    let mut s = session();
    s.execute("shape.add", &json!({"x": 100, "y": 200, "width": 50, "height": 40})).unwrap();
    s.execute("shape.add", &json!({"x": 300, "y": 220, "width": 50, "height": 100})).unwrap();
    let r = s.execute("page.cropToContent", &json!({"padding": 10})).unwrap();
    assert_eq!((r["width"].as_f64(), r["height"].as_f64()), (Some(270.0), Some(140.0)));
    assert_eq!(s.doc().unwrap().doc.pages[0].elements[0].frame.x, 10.0);
    s.execute("page.add", &json!({})).unwrap();
    assert!(s.execute("page.cropToContent", &json!({})).is_err(), "an empty page cannot be cropped");
}

#[test]
fn version_history() {
    let mut s = session();
    s.execute("text.add", &json!({"text": "First"})).unwrap();
    let v1 = s.execute("version.save", &json!({"name": "Draft 1"})).unwrap()["id"].as_u64().unwrap();
    s.execute("text.add", &json!({"text": "Second"})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements.len(), 2);
    let list = s.execute("version.list", &json!({})).unwrap();
    assert_eq!(list[0]["name"], "Draft 1");
    s.execute("version.restore", &json!({"id": v1})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements.len(), 1);
    // The state before restoring was kept, and the restore is undoable.
    assert_eq!(s.doc().unwrap().doc.versions.len(), 2);
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].elements.len(), 2);
    s.execute("version.copy", &json!({"id": v1})).unwrap();
    assert_eq!(s.docs.len(), 2);
    assert!(s.doc().unwrap().doc.versions.is_empty());
    s.execute("document.switch", &json!({"index": 0})).unwrap();
    s.execute("version.rename", &json!({"id": v1, "name": "Final"})).unwrap();
    s.execute("version.delete", &json!({"id": v1})).unwrap();
    assert!(s.execute("version.restore", &json!({"id": v1})).is_err());
    // Versions survive saving and opening, and saving adds an automatic one.
    let dir = std::env::temp_dir().join(format!("genha-versions-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v.genha").display().to_string();
    s.execute("file.saveAs", &json!({"path": path})).unwrap();
    let n = s.doc().unwrap().doc.versions.len();
    s.execute("file.save", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().doc.versions.len(), n, "no new version when nothing changed");
    s.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(s.doc().unwrap().doc.versions.len(), n);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn data_recovery() {
    let root = std::env::temp_dir().join(format!("genha-recovery-{}", std::process::id()));
    let dir = root.join("recovery");
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = session();
    s.store = Some(Box::new(crate::store::DirStore::new(&root)));
    s.execute("text.add", &json!({"text": "Unsaved"})).unwrap();
    let r = s.execute("recovery.save", &json!({})).unwrap();
    assert_eq!(r["written"].as_array().unwrap().len(), 1);
    // This session's own copies are not offered; a copy from "another" process is.
    assert_eq!(s.execute("recovery.list", &json!({})).unwrap().as_array().unwrap().len(), 0);
    let own = std::path::PathBuf::from(r["written"][0].as_str().unwrap());
    assert_eq!(own.parent().unwrap(), dir);
    let other = dir.join("999999999-1.genha");
    std::fs::copy(&own, &other).unwrap();
    let list = s.execute("recovery.list", &json!({})).unwrap();
    assert_eq!(list[0]["path"], other.display().to_string());
    assert_eq!(list[0]["key"], "recovery/999999999-1.genha");
    s.execute("recovery.open", &json!({"path": other.display().to_string()})).unwrap();
    assert!(!other.exists());
    assert!(s.doc().unwrap().dirty);
    // Paths outside the folder are refused.
    assert!(s.execute("recovery.discard", &json!({"path": "/etc/passwd"})).is_err());
    assert!(s.execute("recovery.discard", &json!({"key": "recovery/../../x.genha"})).is_err());
    // Saving removes the copy.
    s.execute("file.saveAs", &json!({"path": root.join("saved").display().to_string()})).unwrap();
    s.execute("file.close", &json!({})).unwrap();
    s.execute("file.close", &json!({})).unwrap();
    s.execute("file.new", &json!({})).unwrap();
    s.execute("recovery.save", &json!({})).unwrap();
    let left: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains('-')).collect();
    assert!(left.is_empty(), "{left:?}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn data_recovery_in_memory_store() {
    // The browser's store: another page load's copy is offered, by key.
    let mut s = session();
    s.instance = "tab1".into();
    s.store = Some(Box::new(crate::store::MemStore::default()));
    s.execute("text.add", &json!({"text": "Unsaved"})).unwrap();
    s.execute("recovery.save", &json!({})).unwrap();
    assert_eq!(s.execute("recovery.list", &json!({})).unwrap().as_array().unwrap().len(), 0);
    // The page is reloaded: a new run with the same store.
    let store = s.store.take();
    let mut s2 = Session::new();
    s2.instance = "tab2".into();
    s2.store = store;
    let list = s2.execute("recovery.list", &json!({})).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "Instagram post (square)");
    let key = list[0]["key"].as_str().unwrap().to_string();
    assert!(key.starts_with("recovery/tab1-"), "{key}");
    s2.execute("recovery.open", &json!({"key": key})).unwrap();
    assert_eq!(s2.docs.len(), 1);
    assert!(s2.execute("recovery.list", &json!({})).unwrap().as_array().unwrap().is_empty());
    // Its own copy replaced the old one.
    assert_eq!(s2.store.as_ref().unwrap().list("recovery").len(), 1);
    // Without a store Data Recovery is off, quietly for the list.
    let mut s3 = session();
    assert!(s3.execute("recovery.save", &json!({})).is_err());
    assert_eq!(s3.execute("recovery.list", &json!({})).unwrap(), json!([]));
}

#[test]
fn recent_designs_in_the_browser_keep_a_copy() {
    let mut s = session();
    s.store = Some(Box::new(crate::store::MemStore::default()));
    s.execute("document.rename", &json!({"title": "Poster"})).unwrap();
    s.execute("text.add", &json!({"text": "Hello"})).unwrap();
    // A download (the browser's Save) records the design with a copy and a thumbnail.
    let bytes = crate::cmd::file::design_bytes(&mut s).unwrap();
    let list = s.execute("recent.list", &json!({})).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "Poster");
    assert!(list[0]["path"].is_null());
    assert!(list[0]["thumbnail"].is_string());
    let id = list[0]["id"].as_str().unwrap().to_string();
    let png = crate::cmd::recent::thumbnail(&s, &id).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    // Downloading it again keeps one entry; opening the same file again too.
    crate::cmd::file::design_bytes(&mut s).unwrap();
    crate::cmd::file::open_bytes(&mut s, "Poster.genha", &bytes).unwrap();
    assert_eq!(s.execute("recent.list", &json!({})).unwrap().as_array().unwrap().len(), 1);
    // Opening the entry brings the design back.
    s.execute("file.close", &json!({})).unwrap();
    s.execute("file.close", &json!({})).unwrap();
    s.execute("recent.open", &json!({"id": id})).unwrap();
    assert_eq!(s.doc().unwrap().doc.title, "Poster");
    // Pictures opened as designs are not recent designs (until saved).
    let png = genha_render::encode_png(&genha_render::Raster { width: 2, height: 2, rgba: vec![255; 16] }).unwrap();
    crate::cmd::file::open_bytes(&mut s, "a.png", &png).unwrap();
    assert_eq!(s.execute("recent.list", &json!({})).unwrap().as_array().unwrap().len(), 1);
    s.execute("recent.remove", &json!({"id": id})).unwrap();
    assert!(s.execute("recent.list", &json!({})).unwrap().as_array().unwrap().is_empty());
    assert!(s.store.as_ref().unwrap().list("recent").iter().all(|(k, _)| k == "recent/list.json"));
    assert!(s.execute("recent.open", &json!({"id": "nope"})).is_err());
}

#[test]
fn recent_designs_on_the_desktop_point_at_files() {
    let root = std::env::temp_dir().join(format!("genha-recent-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    // The list the app kept before is carried over.
    let old = root.join("old.genha");
    std::fs::write(root.join("recent.json"), serde_json::to_string(&[old.display().to_string()]).unwrap()).unwrap();
    let mut s = session();
    s.store = Some(Box::new(crate::store::DirStore::new(&root)));
    assert_eq!(s.execute("recent.list", &json!({})).unwrap()[0]["title"], "old");
    let path = root.join("a.genha");
    s.execute("file.saveAs", &json!({"path": path.display().to_string()})).unwrap();
    let list = s.execute("recent.list", &json!({})).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[0]["path"], path.display().to_string());
    assert!(!root.join("recent.json").exists());
    s.execute("file.close", &json!({})).unwrap();
    let id = list[0]["id"].as_str().unwrap();
    s.execute("recent.open", &json!({"id": id})).unwrap();
    assert_eq!(s.doc().unwrap().path.as_deref(), Some(path.as_path()));
    assert_eq!(s.execute("recent.list", &json!({})).unwrap().as_array().unwrap().len(), 2);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn added_fonts_are_kept_for_next_time() {
    use base64::Engine as _;
    let mut s = session();
    s.store = Some(Box::new(crate::store::MemStore::default()));
    assert!(s.execute("font.add", &json!({"data": "bm90IGEgZm9udA=="})).is_err());
    assert!(s.execute("font.list", &json!({})).unwrap()["added"].as_array().unwrap().is_empty());
    // A real font, when this machine has one.
    let Some(path) =
        ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf"].iter().find(|p| std::path::Path::new(p).exists())
    else {
        return;
    };
    let data = base64::engine::general_purpose::STANDARD.encode(std::fs::read(path).unwrap());
    let r = s.execute("font.add", &json!({"data": data, "name": "DejaVuSans.ttf"})).unwrap();
    assert_eq!(r["faces"], 1);
    assert!(s.fonts.has_family("DejaVu Sans"));
    let added = s.execute("font.list", &json!({})).unwrap()["added"].clone();
    assert_eq!(added[0]["key"], "fonts/DejaVuSans.ttf");
    let mut next = Session::new();
    next.store = s.store.take();
    assert_eq!(crate::cmd::fonts::load_stored_fonts(&mut next), 1);
    assert!(next.fonts.has_family("DejaVu Sans"));
    next.execute("font.remove", &json!({"key": "fonts/DejaVuSans.ttf"})).unwrap();
    assert!(next.execute("font.remove", &json!({"key": "fonts/../x"})).is_err());
}

#[test]
fn svg_import_places_editable_shapes() {
    use base64::Engine as _;
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="40" height="50" fill="#f00"/><circle cx="75" cy="25" r="25" fill="#00f"/></svg>"##;
    let mut s = session();
    let r = s.execute("image.place", &json!({"data": base64::engine::general_purpose::STANDARD.encode(svg), "name": "logo.svg"})).unwrap();
    let el = s.doc().unwrap().doc.element(NodeId(id(&r))).unwrap().clone();
    let ElementKind::Group(g) = &el.kind else { panic!("{el:?}") };
    assert_eq!(g.children.len(), 2);
    assert!((el.frame.width - 1080.0 * 0.6).abs() < 1e-6, "fitted to 60% of the page");
    // Ungrouping gives editable shapes.
    s.execute("object.ungroup", &json!({})).unwrap();
    s.execute("style.fill", &json!({"color": "#00ff00"})).unwrap();
    // Opening an SVG starts a design of its size.
    let dir = std::env::temp_dir().join(format!("genha-svgimport-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.svg"), svg).unwrap();
    s.execute("file.open", &json!({"path": dir.join("a.svg").display().to_string()})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[0].width, 100.0);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn chart_of(s: &Session, id: u64) -> genha_doc::ChartEl {
    let ElementKind::Chart(c) = &s.doc().unwrap().doc.element(NodeId(id)).unwrap().kind else { panic!() };
    c.clone()
}

fn table_of(s: &Session, id: u64) -> (genha_doc::TableEl, f64) {
    let el = s.doc().unwrap().doc.element(NodeId(id)).unwrap();
    let ElementKind::Table(t) = &el.kind else { panic!() };
    (t.clone(), el.frame.height)
}

#[test]
fn charts_from_commands() {
    let mut s = session();
    let c = id(&s.execute("chart.add", &json!({})).unwrap());
    let chart = chart_of(&s, c);
    assert_eq!(chart.kind, genha_doc::ChartKind::Bar);
    assert!(!chart.categories.is_empty() && chart.series.len() == 1);
    assert_eq!(chart.colors, vec![s.doc().unwrap().doc.brand.colors[0].color]);
    s.execute("chart.setData", &json!({"data": "Mon: 3, Tue: 5, Wed: 8"})).unwrap();
    assert_eq!(chart_of(&s, c).points().len(), 3);
    s.execute("chart.setKind", &json!({"kind": "donut"})).unwrap();
    let chart = chart_of(&s, c);
    assert_eq!((chart.kind, chart.value_format, chart.show_legend), (genha_doc::ChartKind::Donut, genha_doc::ValueFormat::Percent, true));
    s.execute("chart.setStyle", &json!({"show_values": false, "colors": ["#ff0000", "#00ff00"], "donut_hole": 5, "kind": "column"})).unwrap();
    let chart = chart_of(&s, c);
    assert!(!chart.show_values && chart.donut_hole <= 0.9 && chart.kind == genha_doc::ChartKind::Bar);
    assert_eq!(chart.colors.len(), 2);
    assert!(s.execute("chart.setStyle", &json!({"font_size": "big"})).is_err());
    assert!(s.execute("chart.setKind", &json!({"kind": "radar"})).is_err());
    s.execute("style.fill", &json!({"color": "#123456"})).unwrap();
    assert_eq!(chart_of(&s, c).colors[0], genha_doc::Color::rgb(0x12, 0x34, 0x56));
    // Pie colours come from the brand; data as a list; the summary shows it.
    let p = id(&s.execute("chart.add", &json!({"kind": "pie", "data": [{"label": "a", "value": 1}, ["b", 2]]})).unwrap());
    assert!(chart_of(&s, p).colors.len() >= 4);
    let summary = inspect::element_summary(s.doc().unwrap().doc.element(NodeId(p)).unwrap());
    assert_eq!(summary["chart"], "pie");
    assert_eq!(summary["data"][1]["value"], 2.0);
    // A ring is a square progress chart.
    let r = id(&s.execute("chart.add", &json!({"kind": "ring"})).unwrap());
    let f = s.doc().unwrap().doc.element(NodeId(r)).unwrap().frame;
    assert_eq!(f.width, f.height);
    // Copy a chart's look onto another; its data stays.
    s.execute("style.copy", &json!({"id": c})).unwrap();
    s.execute("style.paste", &json!({"ids": [p]})).unwrap();
    let pasted = chart_of(&s, p);
    assert_eq!((pasted.kind, pasted.categories.len(), pasted.show_values), (genha_doc::ChartKind::Pie, 2, false));
    // Saved and opened again.
    let bytes = genha_format::to_bytes(&s.doc().unwrap().doc, true).unwrap();
    let (back, notes) = genha_format::from_bytes(&bytes).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(&back, &*s.doc().unwrap().doc);
}

#[test]
fn tables_from_commands() {
    let mut s = session();
    let t = id(&s.execute("table.add", &json!({})).unwrap());
    let (table, h0) = table_of(&s, t);
    assert_eq!((table.rows.len(), table.columns()), (3, 3));
    assert!(h0 > 0.0);
    s.execute("table.setCell", &json!({"row": 1, "col": 2, "text": "hello"})).unwrap();
    assert_eq!(table_of(&s, t).0.cell(1, 2), "hello");
    assert!(s.execute("table.setCell", &json!({"row": 9, "col": 0, "text": "x"})).is_err());
    s.execute("table.insertRow", &json!({"index": 1})).unwrap();
    let (table, h1) = table_of(&s, t);
    assert_eq!((table.rows.len(), table.cell(2, 2)), (4, "hello"));
    assert!(h1 > h0, "the frame grows with the rows");
    s.execute("table.insertColumn", &json!({})).unwrap();
    assert_eq!(table_of(&s, t).0.columns(), 4);
    s.execute("table.deleteColumn", &json!({"index": 0})).unwrap();
    s.execute("table.deleteRow", &json!({})).unwrap();
    let (table, _) = table_of(&s, t);
    assert_eq!((table.rows.len(), table.columns(), table.cell(2, 1)), (3, 3, "hello"));
    s.execute("table.setRows", &json!({"csv": "a,b\nc,d"})).unwrap();
    assert_eq!(table_of(&s, t).0.rows, vec![vec!["a", "b"], vec!["c", "d"]]);
    s.execute("table.setStyle", &json!({"header_fill": "#ff0000", "alt_row_fill": null, "align": "center"})).unwrap();
    let (table, _) = table_of(&s, t);
    assert_eq!((table.header_fill, table.alt_row_fill, table.align), (genha_doc::Color::rgb(255, 0, 0), None, genha_doc::TextAlign::Center));
    s.execute("style.stroke", &json!({"none": true})).unwrap();
    assert!(table_of(&s, t).0.border.is_none());
    for _ in 0..2 {
        let _ = s.execute("table.deleteRow", &json!({}));
    }
    assert_eq!(table_of(&s, t).0.rows.len(), 1, "the last row stays");
    assert!(s.execute("table.add", &json!({"rows": 0})).is_err());
    let big = id(&s.execute("table.add", &json!({"rows": [["x", "y"], ["1", "2"], ["3"]], "header": false})).unwrap());
    let (table, _) = table_of(&s, big);
    assert!(!table.header_row && table.rows[2].len() == 2);
    assert_eq!(inspect::element_summary(s.doc().unwrap().doc.element(NodeId(big)).unwrap())["size"], json!([3, 2]));
}

#[test]
fn charts_with_several_series() {
    let mut s = session();
    let c = id(&s
        .execute(
            "chart.add",
            &json!({"csv": "Quarter,North,South\nQ1,10,12\nQ2,14,9\nQ3,8,\"1,200\"", "stacked": true, "x_title": "Quarter", "legend": "top"}),
        )
        .unwrap());
    let chart = chart_of(&s, c);
    assert_eq!(chart.categories, vec!["Q1", "Q2", "Q3"]);
    assert_eq!(chart.series.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["North", "South"]);
    assert_eq!(chart.value(1, 2), 1200.0);
    assert!(chart.stacked && chart.show_legend && chart.legend_position == genha_doc::LegendPosition::Top && chart.x_title == "Quarter");
    assert!(chart.colors.len() > 1, "several series get several colours");
    // New labels keep the values; new series keep the labels.
    s.execute("chart.setData", &json!({"categories": ["Jan", "Feb"]})).unwrap();
    let chart = chart_of(&s, c);
    assert_eq!((chart.categories.len(), chart.series[0].values.clone()), (2, vec![10.0, 14.0]));
    s.execute("chart.setData", &json!({"series": [{"name": "A", "values": [1, 2, 3], "color": "#ff0000"}, {"name": "B", "values": [4]}]})).unwrap();
    let chart = chart_of(&s, c);
    assert_eq!((chart.categories.clone(), chart.series[1].values.clone()), (vec!["Jan".to_string(), "Feb".into(), "3".into()], vec![4.0, 0.0, 0.0]));
    assert_eq!(chart.series[0].color, Some(genha_doc::Color::rgb(255, 0, 0)));
    // Rows of cells, like a pasted table; the simple form makes one series again.
    s.execute("chart.setData", &json!({"rows": [["", "X", "Y", "Z"], ["a", 1, 2, 3]]})).unwrap();
    assert_eq!(chart_of(&s, c).series.len(), 3);
    s.execute("chart.setData", &json!({"data": "a: 1, b: 2"})).unwrap();
    assert_eq!(chart_of(&s, c).series.len(), 1);
    assert!(s.execute("chart.setData", &json!({})).is_err());
    assert!(s.execute("chart.setData", &json!({"csv": "a,b\nc,lots"})).is_err());
    // Style: number format, axis range, legend off.
    s.execute("chart.setStyle", &json!({"value_format": "currency", "currency": "€", "decimals": 2, "min": 1, "max": 10, "legend": "none", "gridlines": false, "y_title": "Sales"})).unwrap();
    let chart = chart_of(&s, c);
    assert_eq!(
        (chart.value_format, chart.currency.as_str(), chart.decimals, chart.min, chart.max),
        (genha_doc::ValueFormat::Currency, "€", Some(2), Some(1.0), Some(10.0))
    );
    assert!(!chart.show_legend && !chart.gridlines && chart.y_title == "Sales");
    assert!(s.execute("chart.setStyle", &json!({"legend": "left"})).is_err());
    // A minimum over the maximum is dropped.
    s.execute("chart.setStyle", &json!({"min": 50})).unwrap();
    assert_eq!(chart_of(&s, c).min, None);
    // The summary shows the series; files keep them.
    let summary = inspect::element_summary(s.doc().unwrap().doc.element(NodeId(c)).unwrap());
    assert_eq!(summary["series"][0]["name"], "X", "the one-series form keeps the first name");
    let bytes = genha_format::to_bytes(&s.doc().unwrap().doc, true).unwrap();
    let (back, notes) = genha_format::from_bytes(&bytes).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(&back, &*s.doc().unwrap().doc);
}

#[test]
fn table_merges_styles_and_sizes() {
    let mut s = session();
    let t = id(&s.execute("table.add", &json!({"rows": 4, "columns": 3})).unwrap());
    s.execute("table.setCell", &json!({"row": 1, "col": 1, "text": "wide"})).unwrap();
    s.execute("table.mergeCells", &json!({"row": 1, "col": 0, "cols": 3})).unwrap();
    let (table, _) = table_of(&s, t);
    assert_eq!((table.merges.clone(), table.cell(1, 0)), (vec![genha_doc::CellRange::new(1, 0, 1, 3)], "wide"));
    // Editing a cell under the merge edits the merge.
    s.execute("table.setCell", &json!({"row": 1, "col": 2, "text": "merged"})).unwrap();
    assert_eq!(table_of(&s, t).0.cell(1, 0), "merged");
    assert!(s.execute("table.mergeCells", &json!({"row": 0, "col": 0})).is_err(), "one cell");
    assert!(s.execute("table.mergeCells", &json!({"row": 0, "col": 0, "rows": 0})).is_err());
    assert!(s.execute("table.mergeCells", &json!({"row": 50, "col": 0, "rows": 2})).is_err());
    // Inserting a column inside the merge widens it; deleting rows keeps it consistent.
    s.execute("table.insertColumn", &json!({"index": 1})).unwrap();
    assert_eq!(table_of(&s, t).0.merges, vec![genha_doc::CellRange::new(1, 0, 1, 4)]);
    s.execute("table.insertRow", &json!({"index": 0})).unwrap();
    s.execute("table.deleteRow", &json!({"index": 2})).unwrap();
    assert!(table_of(&s, t).0.merges.is_empty());
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(table_of(&s, t).0.merges, vec![genha_doc::CellRange::new(2, 0, 1, 4)]);
    assert!(s.execute("table.splitCell", &json!({"row": 0, "col": 0})).is_err(), "nothing merged there");
    s.execute("table.splitCell", &json!({"row": 2, "col": 3})).unwrap();
    assert!(table_of(&s, t).0.merges.is_empty());
    // Cell styles for a range; null goes back to the table's look.
    s.execute(
        "table.setCellStyle",
        &json!({"row": 1, "col": 0, "rows": 2, "cols": 2, "fill": "#ff0000", "bold": true, "align": "right", "font_size": 40}),
    )
    .unwrap();
    let (table, h_big) = table_of(&s, t);
    assert_eq!(table.cell_styles.len(), 4);
    let st = table.style_at(2, 1).unwrap();
    assert_eq!(
        (st.fill, st.bold, st.align, st.font_size),
        (Some(genha_doc::Color::rgb(255, 0, 0)), Some(true), Some(genha_doc::TextAlign::Right), Some(40.0))
    );
    s.execute("table.setCellStyle", &json!({"row": 1, "col": 0, "rows": 2, "cols": 2, "fill": null, "bold": null, "align": null})).unwrap();
    assert_eq!(table_of(&s, t).0.cell_styles.len(), 4, "the size stays");
    s.execute("table.setCellStyle", &json!({"row": 0, "col": 0, "rows": 9, "cols": 9, "clear": true})).unwrap();
    let (table, h) = table_of(&s, t);
    assert!(table.cell_styles.is_empty());
    let _ = h_big;
    assert!(s.execute("table.setCellStyle", &json!({"row": 0, "col": 0})).is_err(), "nothing to change");
    assert!(s.execute("table.setCellStyle", &json!({"row": 0, "col": 0, "align": "up"})).is_err());
    // Column widths in pixels; fit to the text; equal again.
    let w = s.doc().unwrap().doc.element(NodeId(t)).unwrap().frame.width;
    s.execute("table.setColumnWidth", &json!({"col": 0, "width": w / 2.0})).unwrap();
    let f = table_of(&s, t).0.fractions();
    assert!((f[0] - 0.5).abs() < 1e-9 && (f.iter().sum::<f64>() - 1.0).abs() < 1e-9, "{f:?}");
    s.execute("table.setColumnWidth", &json!({"auto": true})).unwrap();
    assert_eq!(table_of(&s, t).0.column_widths.len(), 4);
    s.execute("table.setColumnWidth", &json!({"equal": true})).unwrap();
    assert!(table_of(&s, t).0.column_widths.is_empty());
    assert!(s.execute("table.setColumnWidth", &json!({"col": 0})).is_err());
    assert!(s.execute("table.setColumnWidth", &json!({"col": 9, "width": 10})).is_err());
    // Row heights: a tall row makes the frame taller; auto again.
    s.execute("table.setRowHeight", &json!({"row": 1, "height": 300})).unwrap();
    let (table, h2) = table_of(&s, t);
    assert!(h2 >= h + 200.0, "{h} {h2}");
    assert_eq!(table.row_height(1), 300.0);
    s.execute("table.setRowHeight", &json!({"auto": true})).unwrap();
    assert!(table_of(&s, t).0.row_heights.is_empty());
    assert!(s.execute("table.setRowHeight", &json!({"row": 99, "height": 10})).is_err());
    // Borders: only the outside, in another colour.
    s.execute("table.setStyle", &json!({"borders": "outer", "outer_border": {"color": "#000000", "width": 3}})).unwrap();
    let (table, _) = table_of(&s, t);
    assert_eq!((table.borders, table.outer_border.map(|b| b.width)), (genha_doc::BorderLines::Outer, Some(3.0)));
    // No lines and no bands survive saving (they used to come back as the defaults).
    s.execute("table.setStyle", &json!({"alt_row_fill": null, "border": null})).unwrap();
    let bytes = genha_format::to_bytes(&s.doc().unwrap().doc, true).unwrap();
    let (back, notes) = genha_format::from_bytes(&bytes).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(&back, &*s.doc().unwrap().doc);
}

#[test]
fn text_curve_command() {
    let mut s = session();
    let t = id(&s.execute("text.add", &json!({"text": "Around we go"})).unwrap());
    let h0 = s.doc().unwrap().doc.element(NodeId(t)).unwrap().frame.height;
    s.execute("text.setCurve", &json!({"degrees": 720})).unwrap();
    let el = s.doc().unwrap().doc.element(NodeId(t)).unwrap().clone();
    let ElementKind::Text(te) = &el.kind else { panic!() };
    assert_eq!(te.curve, 360.0);
    assert!(el.frame.height > h0 * 1.5, "a circle of text is taller: {} vs {h0}", el.frame.height);
    s.execute("text.setCurve", &json!({"degrees": 0})).unwrap();
    let el = s.doc().unwrap().doc.element(NodeId(t)).unwrap();
    assert!((el.frame.height - h0).abs() < 1.0);
    assert!(s.execute("text.setCurve", &json!({"degrees": "a lot"})).is_err());
    let shape = id(&s.execute("shape.add", &json!({})).unwrap());
    assert!(s.execute("text.setCurve", &json!({"ids": [shape], "degrees": 90})).is_err());
}
