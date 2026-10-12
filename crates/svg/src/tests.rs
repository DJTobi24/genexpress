use std::sync::OnceLock;

use genha_doc::{
    Adjustments, Asset, Bytes, Element, ElementKind, Fill, Frame, GradientStop, GroupEl, ImageEl, Shadow, ShapeEl, ShapeKind, StrokeStyle,
    TextEffect, TextEl, TextStyle,
};

use super::*;

fn fonts() -> &'static FontBook {
    static BOOK: OnceLock<FontBook> = OnceLock::new();
    BOOK.get_or_init(FontBook::system)
}

/// Checks that tags nest properly (a rough well-formedness test) and returns the element names.
fn check_tags(svg: &str) -> Vec<String> {
    let mut stack: Vec<String> = vec![];
    let mut names = vec![];
    let mut rest = svg;
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        let end = rest.find('>').expect("unclosed tag");
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        assert!(!tag.contains('<'), "stray < in {tag}");
        if let Some(name) = tag.strip_prefix('/') {
            assert_eq!(stack.pop().as_deref(), Some(name.trim()), "mismatched </{name}>");
        } else {
            let name: String = tag.chars().take_while(|c| !c.is_whitespace() && *c != '/').collect();
            names.push(name.clone());
            if !tag.ends_with('/') {
                stack.push(name);
            }
        }
        // Attribute quotes balance.
        assert_eq!(tag.matches('"').count() % 2, 0, "unbalanced quotes in <{tag}>");
    }
    assert!(stack.is_empty(), "unclosed: {stack:?}");
    names
}

fn push(d: &mut Document, frame: Frame, kind: ElementKind) -> genha_doc::NodeId {
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(id, frame, kind));
    id
}

fn rect_shape(fill: Fill) -> ElementKind {
    ElementKind::Shape(ShapeEl { shape: ShapeKind::Rect { corner_radius: 8.0 }, fill, stroke: None, shadow: None })
}

#[test]
fn shapes_and_background() {
    let mut d = Document::new("Shapes", 100.0, 80.0);
    push(&mut d, Frame { x: 10.0, y: 10.0, width: 50.0, height: 30.0, rotation: 15.0 }, rect_shape(Fill::solid(Color::rgba(255, 0, 0, 128))));
    let id = push(
        &mut d,
        Frame::new(0.0, 60.0, 100.0, 10.0),
        ElementKind::Shape(ShapeEl {
            shape: ShapeKind::Line,
            fill: Fill::None,
            stroke: Some(StrokeStyle { color: Color::BLACK, width: 2.0, dash: vec![4.0, 2.0] }),
            shadow: Some(Shadow::default()),
        }),
    );
    d.element_mut(id).unwrap().opacity = 0.5;
    let svg = export_page(&d, 0, &FontBook::new(), &SvgOptions::default()).unwrap();
    let names = check_tags(&svg);
    assert!(svg.starts_with("<?xml"));
    assert!(svg.contains(r#"viewBox="0 0 100 80""#));
    assert!(svg.contains(r##"<rect width="100" height="80" fill="#ffffff"/>"##));
    assert!(svg.contains(r##"fill="#ff0000" fill-opacity="0.502""##), "{svg}");
    assert!(svg.contains(r#"stroke-dasharray="4 2""#));
    assert!(svg.contains(r#"opacity="0.5""#));
    assert!(svg.contains("<feGaussianBlur"));
    assert!(names.iter().filter(|n| *n == "path").count() >= 3);
    assert!(svg.contains("matrix("));
    assert!(matches!(export_page(&d, 7, &FontBook::new(), &SvgOptions::default()), Err(SvgError::NoPage(7))));
}

#[test]
fn gradient_defs_have_unique_ids() {
    let mut d = Document::new("Gradients", 200.0, 100.0);
    d.pages[0].background = Fill::linear(90.0, Color::WHITE, Color::BLACK);
    push(&mut d, Frame::new(0.0, 0.0, 100.0, 100.0), rect_shape(Fill::linear(0.0, Color::rgb(255, 0, 0), Color::rgb(0, 0, 255))));
    push(
        &mut d,
        Frame::new(100.0, 0.0, 100.0, 100.0),
        rect_shape(Fill::Radial {
            stops: vec![GradientStop { offset: 1.0, color: Color::rgba(0, 0, 0, 0) }, GradientStop { offset: 0.0, color: Color::rgb(0, 255, 0) }],
        }),
    );
    let svg = export_page(&d, 0, &FontBook::new(), &SvgOptions::default()).unwrap();
    check_tags(&svg);
    assert_eq!(svg.matches("<linearGradient").count(), 2);
    assert_eq!(svg.matches("<radialGradient").count(), 1);
    let ids: Vec<&str> = svg.match_indices("id=\"ge-grad-").map(|(i, _)| &svg[i + 4..i + 4 + svg[i + 4..].find('"').unwrap()]).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(ids.len(), unique.len());
    for id in ids {
        assert!(svg.contains(&format!("url(#{id})")), "{id} unused");
    }
    // Stops are sorted by offset; the transparent one carries its opacity.
    let radial = &svg[svg.find("<radialGradient").unwrap()..];
    let first = radial.find(r##"stop-color="#00ff00""##).unwrap();
    let second = radial.find(r#"stop-opacity="0""#).unwrap();
    assert!(first < second);
}

#[test]
fn text_is_escaped_and_laid_out() {
    let mut d = Document::new("Fish & <Chips> \"menu\"", 400.0, 200.0);
    let style = TextStyle { font_family: "Sans".into(), font_size: 20.0, underline: true, letter_spacing: 50.0, ..TextStyle::default() };
    push(
        &mut d,
        Frame::new(0.0, 0.0, 400.0, 200.0),
        ElementKind::Text(TextEl {
            text: "a < b & \"c\" {fill}\nsecond line\u{1}".into(),
            style,
            effect: TextEffect::Shadow { color: Color::BLACK, offset_x: 2.0, offset_y: 2.0, blur: 3.0 },
            curve: 0.0,
        }),
    );
    let svg = export_page(&d, 0, fonts(), &SvgOptions::default()).unwrap();
    check_tags(&svg);
    assert!(svg.contains("<title>Fish &amp; &lt;Chips&gt; &quot;menu&quot;</title>"));
    assert!(svg.contains("a &lt; b &amp; &quot;c&quot; {fill}"), "{svg}");
    assert!(!svg.contains('\u{1}'));
    // Two lines, each its own tspan, twice (shadow and text).
    assert_eq!(svg.matches("<tspan").count(), 4);
    assert!(svg.contains(r#"text-anchor="middle""#));
    assert!(svg.contains(r#"text-decoration="underline""#));
    assert!(svg.contains(r#"letter-spacing="1""#));
    assert!(svg.contains("sans-serif"));
    // As outlines: no <text> at all.
    let paths = export_page(&d, 0, fonts(), &SvgOptions { text_as_paths: true }).unwrap();
    check_tags(&paths);
    assert!(!paths.contains("<text"));
    if !fonts().is_empty() {
        assert!(paths.contains("<path d=\"M"));
    }
}

#[test]
fn text_effects() {
    let mut d = Document::new("Effects", 300.0, 100.0);
    for effect in [
        TextEffect::Outline { color: Color::rgb(255, 0, 0), width: 2.0 },
        TextEffect::Highlight { color: Color::rgb(255, 255, 0), padding: 4.0, corner_radius: 3.0 },
    ] {
        push(
            &mut d,
            Frame::new(0.0, 0.0, 300.0, 50.0),
            ElementKind::Text(TextEl { text: "Hi".into(), style: TextStyle::default(), effect, curve: 0.0 }),
        );
    }
    let svg = export_page(&d, 0, &FontBook::new(), &SvgOptions::default()).unwrap();
    check_tags(&svg);
    assert!(svg.contains(r#"stroke-width="4" stroke-linejoin="round""#));
    assert!(svg.contains(r##"fill="#ffff00""##));
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([10, 200, 30, 255]));
    let mut out = vec![];
    image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

#[test]
fn pictures_are_embedded() {
    let mut d = Document::new("Pictures", 200.0, 100.0);
    let bytes = png(40, 20);
    let asset = d.add_asset(Asset { name: "p.png".into(), mime: "image/png".into(), width: 40, height: 20, data: Bytes::new(bytes.clone()) });
    let mut im = ImageEl::new(asset);
    im.corner_radius = 10.0;
    push(&mut d, Frame::new(0.0, 0.0, 100.0, 100.0), ElementKind::Image(im.clone()));
    let mut edited = im.clone();
    edited.adjustments = Adjustments { grayscale: 100.0, ..Default::default() };
    edited.mask = Some(ShapeKind::Ellipse);
    edited.border = Some(StrokeStyle { color: Color::WHITE, width: 3.0, dash: vec![] });
    push(&mut d, Frame::new(100.0, 0.0, 100.0, 100.0), ElementKind::Image(edited));
    // Missing picture: placeholder.
    push(&mut d, Frame::new(0.0, 0.0, 10.0, 10.0), ElementKind::Image(ImageEl::new(genha_doc::AssetId(4242))));
    let svg = export_page(&d, 0, &FontBook::new(), &SvgOptions::default()).unwrap();
    check_tags(&svg);
    let original = base64::engine::general_purpose::STANDARD.encode(&bytes);
    assert!(svg.contains(&format!("href=\"data:image/png;base64,{original}\"")));
    assert_eq!(svg.matches("<image").count(), 2);
    assert_eq!(svg.matches("<clipPath").count(), 3);
    assert!(svg.contains("clip-path=\"url(#ge-clip-"));
    assert!(svg.contains(r##"fill="#d5d5dc""##));
    // Cover: the 2:1 picture is 200 wide in a 100 box, centred.
    assert!(svg.contains(r#"x="-50" y="0" width="200" height="100""#), "{svg}");
    // The edited copy is grey.
    let start = svg.rfind("data:image/png;base64,").unwrap() + "data:image/png;base64,".len();
    let end = start + svg[start..].find('"').unwrap();
    let decoded = base64::engine::general_purpose::STANDARD.decode(&svg[start..end]).unwrap();
    let img = image::load_from_memory(&decoded).unwrap().to_rgba8();
    let p = img.get_pixel(0, 0);
    assert!(p[0] == p[1] && p[1] == p[2]);
}

#[test]
fn groups_and_hidden() {
    let mut d = Document::new("Groups", 100.0, 100.0);
    let a = d.alloc_id();
    let child = Element::new(a, Frame::new(0.0, 0.0, 10.0, 10.0), rect_shape(Fill::solid(Color::BLACK)));
    let b = d.alloc_id();
    let mut hidden = Element::new(b, Frame::new(0.0, 0.0, 10.0, 10.0), rect_shape(Fill::solid(Color::rgb(1, 2, 3))));
    hidden.hidden = true;
    let g = d.alloc_id();
    let mut group = Element::new(g, Frame::new(0.0, 0.0, 100.0, 100.0), ElementKind::Group(GroupEl { children: vec![child, hidden] }));
    group.flip_h = true;
    d.pages[0].elements.push(group);
    d.pages[0].background = Fill::None;
    let svg = export_page(&d, 0, &FontBook::new(), &SvgOptions::default()).unwrap();
    check_tags(&svg);
    assert!(!svg.contains("<rect"));
    assert!(!svg.contains("#010203"));
    // The child is mirrored to the right edge: x scale -1, translated by 100.
    assert!(svg.contains("matrix(-1 0 0 1 100 0)"), "{svg}");
}

#[test]
fn numbers_and_escaping() {
    assert_eq!(num(1.0), "1");
    assert_eq!(num(0.12345), "0.123");
    assert_eq!(num(-0.0001), "0");
    assert_eq!(num(f64::NAN), "0");
    assert_eq!(esc("<a href='x'>&</a>"), "&lt;a href=&apos;x&apos;&gt;&amp;&lt;/a&gt;");
}

#[test]
fn charts_tables_and_curved_text() {
    let mut d = Document::new("Data", 900.0, 600.0);
    for (i, k) in genha_doc::ChartKind::ALL.into_iter().enumerate() {
        let mut c = genha_doc::ChartEl::sample(k);
        c.data[0].label = "R&D <x>".into();
        c.background = Fill::linear(90.0, Color::WHITE, Color::rgb(0xee, 0xee, 0xff));
        push(&mut d, Frame::new((i % 4) as f64 * 220.0, (i / 4) as f64 * 220.0, 200.0, 200.0), ElementKind::Chart(c));
    }
    let mut t = genha_doc::TableEl::blank(3, 2);
    t.rows[1][0] = "a & b".into();
    push(&mut d, Frame::new(660.0, 220.0, 220.0, 150.0), ElementKind::Table(t));
    let mut te = TextEl::new("Round & round", TextStyle::default());
    te.curve = 200.0;
    te.effect = TextEffect::Highlight { color: Color::rgb(255, 255, 0), padding: 4.0, corner_radius: 2.0 };
    push(&mut d, Frame::new(0.0, 450.0, 400.0, 150.0), ElementKind::Text(te));
    for text_as_paths in [false, true] {
        let svg = export_page(&d, 0, fonts(), &SvgOptions { text_as_paths }).unwrap();
        check_tags(&svg);
        assert!(svg.contains("<linearGradient"));
        // Chart and table labels are text unless asked for as outlines; curved text never is.
        assert_eq!(svg.contains("R&amp;D &lt;x&gt;"), !text_as_paths);
        assert_eq!(svg.contains("a &amp; b"), !text_as_paths);
        assert!(!svg.contains("Round &amp; round"));
        // A highlight does not follow the curve: no yellow box.
        assert!(!svg.contains("#ffff00"));
        if !fonts().is_empty() {
            assert!(svg.matches("<path").count() > 40);
        }
    }
}
