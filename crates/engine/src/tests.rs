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
fn every_command_survives_junk_params() {
    // Untrusted params never panic (the guard would turn a panic into Internal: that's a bug too).
    let junk = [json!(null), json!({}), json!({"ids": [999999]}), json!({"x": "a", "index": -1, "color": 5}), json!([1, 2])];
    let mut s = session();
    s.execute("shape.add", &json!({})).unwrap();
    s.execute("text.add", &json!({})).unwrap();
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
    s.execute("page.resize", &json!({"width": 500, "height": 500})).unwrap();
    assert_eq!(s.doc().unwrap().doc.pages[1].width, 500.0);
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
