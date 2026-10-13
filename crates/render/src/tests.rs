use std::sync::OnceLock;

use genha_doc::{
    Adjustments, Asset, Bytes, CropRect, Element, ElementKind, Fill, GroupEl, ImageEl, NodeId, Shadow, ShapeEl, ShapeKind, StrokeStyle, TextAlign,
    TextEffect, TextEl, TextStyle,
};
use genha_geom::Fit;
use kurbo::Shape;

use super::*;

fn fonts() -> &'static FontBook {
    static BOOK: OnceLock<FontBook> = OnceLock::new();
    BOOK.get_or_init(FontBook::system)
}

fn doc(w: f64, h: f64) -> Document {
    Document::new("Test", w, h)
}

fn shape(doc: &mut Document, frame: Frame, kind: ShapeKind, fill: Fill) -> NodeId {
    let id = doc.alloc_id();
    let el = Element::new(id, frame, ElementKind::Shape(ShapeEl { shape: kind, fill, stroke: None, shadow: None }));
    doc.pages[0].elements.push(el);
    id
}

fn rect() -> ShapeKind {
    ShapeKind::Rect { corner_radius: 0.0 }
}

const RED: Color = Color::rgb(255, 0, 0);

fn close(a: [u8; 4], b: [u8; 4], tol: i16) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (i16::from(*x) - i16::from(*y)).abs() <= tol)
}

fn render(d: &Document) -> Raster {
    render_page(d, 0, &FontBook::new(), &RenderOptions::default()).unwrap()
}

fn png_bytes(img: &image::RgbaImage) -> Vec<u8> {
    let mut out = vec![];
    image::DynamicImage::ImageRgba8(img.clone()).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

#[test]
fn red_rect_on_white() {
    let mut d = doc(100.0, 80.0);
    shape(&mut d, Frame::new(10.0, 10.0, 50.0, 30.0), rect(), Fill::solid(RED));
    let r = render(&d);
    assert_eq!((r.width, r.height), (100, 80));
    assert_eq!(r.rgba.len(), 100 * 80 * 4);
    assert_eq!(r.pixel(30, 20), Some([255, 0, 0, 255]));
    assert_eq!(r.pixel(5, 5), Some([255, 255, 255, 255]));
    assert_eq!(r.pixel(70, 50), Some([255, 255, 255, 255]));
    assert_eq!(r.pixel(100, 0), None);
    // Twice the scale, twice the pixels.
    let r2 = render_page(&d, 0, &FontBook::new(), &RenderOptions { scale: 2.0, transparent_background: true }).unwrap();
    assert_eq!((r2.width, r2.height), (200, 160));
    assert_eq!(r2.pixel(60, 40), Some([255, 0, 0, 255]));
    assert_eq!(r2.pixel(2, 2).map(|p| p[3]), Some(0));
}

#[test]
fn background_fills() {
    let mut d = doc(100.0, 50.0);
    d.pages[0].background = Fill::linear(0.0, Color::rgb(0, 0, 0), Color::rgb(255, 255, 255));
    let r = render(&d);
    let left = r.pixel(1, 25).unwrap();
    let right = r.pixel(98, 25).unwrap();
    let mid = r.pixel(50, 25).unwrap();
    assert!(left[0] < 20 && right[0] > 235, "{left:?} {right:?}");
    assert!((100..160).contains(&mid[0]), "{mid:?}");
    // Vertical gradient (90°): top dark, bottom light.
    d.pages[0].background = Fill::linear(90.0, Color::rgb(0, 0, 0), Color::rgb(255, 255, 255));
    let r = render(&d);
    assert!(r.pixel(50, 1).unwrap()[0] < 30 && r.pixel(50, 48).unwrap()[0] > 225);
    // Radial: centre is the first stop.
    d.pages[0].background =
        Fill::Radial { stops: vec![GradientStop { offset: 0.0, color: RED }, GradientStop { offset: 1.0, color: Color::rgb(0, 0, 255) }] };
    let r = render(&d);
    let c = r.pixel(50, 25).unwrap();
    assert!(c[0] > 230 && c[2] < 30, "{c:?}");
    // No background: white, or transparent on request.
    d.pages[0].background = Fill::None;
    assert_eq!(render(&d).pixel(10, 10), Some([255, 255, 255, 255]));
    let t = render_page(&d, 0, &FontBook::new(), &RenderOptions { scale: 1.0, transparent_background: true }).unwrap();
    assert_eq!(t.pixel(10, 10).map(|p| p[3]), Some(0));
}

use genha_doc::GradientStop;

#[test]
fn gradient_shape_fill() {
    let mut d = doc(200.0, 100.0);
    shape(&mut d, Frame::new(0.0, 0.0, 200.0, 100.0), rect(), Fill::linear(0.0, RED, Color::rgb(0, 0, 255)));
    let r = render(&d);
    let (l, rt) = (r.pixel(2, 50).unwrap(), r.pixel(197, 50).unwrap());
    assert!(l[0] > 240 && l[2] < 15, "{l:?}");
    assert!(rt[2] > 240 && rt[0] < 15, "{rt:?}");
}

#[test]
fn opacity_blends() {
    let mut d = doc(40.0, 40.0);
    let id = shape(&mut d, Frame::new(0.0, 0.0, 40.0, 40.0), rect(), Fill::solid(RED));
    d.element_mut(id).unwrap().opacity = 0.5;
    let p = render(&d).pixel(20, 20).unwrap();
    assert!(close(p, [255, 128, 128, 255], 2), "{p:?}");
    d.element_mut(id).unwrap().hidden = true;
    assert_eq!(render(&d).pixel(20, 20), Some([255, 255, 255, 255]));
}

#[test]
fn rotated_element() {
    let mut d = doc(200.0, 200.0);
    let id = shape(&mut d, Frame::new(50.0, 50.0, 100.0, 100.0), rect(), Fill::solid(RED));
    d.element_mut(id).unwrap().frame.rotation = 45.0;
    let r = render(&d);
    // Centre is covered; the unrotated corner is not; the diamond tip beyond the old edge is.
    assert_eq!(r.pixel(100, 100), Some([255, 0, 0, 255]));
    assert_eq!(r.pixel(55, 55), Some([255, 255, 255, 255]));
    assert_eq!(r.pixel(100, 40), Some([255, 0, 0, 255]));
    assert_eq!(r.pixel(100, 25), Some([255, 255, 255, 255]));
    let b = d.element(id).unwrap().frame.bounds();
    assert!(b.y0 < 40.0 && b.y0 > 25.0);
}

#[test]
fn shapes_and_paths() {
    let tri = shape_path(&ShapeKind::Triangle, 100.0, 50.0);
    let bb = tri.bounding_box();
    assert_eq!((bb.x0, bb.y0, bb.x1, bb.y1), (0.0, 0.0, 100.0, 50.0));
    let p = shape_path(&ShapeKind::Path { d: "M0 0 L10 0 L10 10 Z".into(), view_width: 10.0, view_height: 10.0 }, 200.0, 100.0);
    let bb = p.bounding_box();
    assert_eq!((bb.x1, bb.y1), (200.0, 100.0));
    assert!(shape_path(&ShapeKind::Path { d: "M0 0 L oops".into(), view_width: 10.0, view_height: 10.0 }, 10.0, 10.0).is_empty());
    let p = shape_path(&ShapeKind::Path { d: "M5 5 L15 5 L15 25 Z".into(), view_width: 0.0, view_height: f64::NAN }, 100.0, 100.0);
    let bb = p.bounding_box();
    assert!((bb.x0, bb.y0, bb.x1, bb.y1) == (0.0, 0.0, 100.0, 100.0), "{bb:?}");
    let line = shape_path(&ShapeKind::Line, 80.0, 10.0);
    assert_eq!(line.elements().len(), 2);
    assert!(shape_path(&ShapeKind::Ellipse, f64::NAN, 1.0).is_empty());
    for k in [
        ShapeKind::Ellipse,
        ShapeKind::Polygon { sides: 6 },
        ShapeKind::Star { points: 5, inner_ratio: 0.5 },
        ShapeKind::Rect { corner_radius: 10.0 },
    ] {
        let bb = shape_path(&k, 60.0, 40.0).bounding_box();
        assert!(bb.x0 >= -1e-6 && bb.y0 >= -1e-6 && bb.x1 <= 60.0 + 1e-6 && bb.y1 <= 40.0 + 1e-6, "{k:?} {bb:?}");
    }
}

#[test]
fn strokes_lines_and_shadows() {
    let mut d = doc(120.0, 120.0);
    let id = doc_line(&mut d);
    let r = render(&d);
    assert_eq!(r.pixel(60, 60).map(|p| p[0] < 10 && p[3] == 255), Some(true));
    assert_eq!(r.pixel(60, 40), Some([255, 255, 255, 255]));
    d.remove(id);
    let id = shape(&mut d, Frame::new(20.0, 20.0, 40.0, 40.0), rect(), Fill::solid(RED));
    if let ElementKind::Shape(s) = &mut d.element_mut(id).unwrap().kind {
        s.stroke = Some(StrokeStyle { color: Color::rgb(0, 0, 255), width: 4.0, dash: vec![4.0, 2.0] });
        s.shadow = Some(Shadow { color: Color::rgba(0, 0, 0, 255), offset_x: 20.0, offset_y: 20.0, blur: 0.0 });
    }
    let r = render(&d);
    assert_eq!(r.pixel(40, 40), Some([255, 0, 0, 255]));
    // The hard shadow shows below-right of the shape.
    assert_eq!(r.pixel(75, 75), Some([0, 0, 0, 255]));
    let edge = r.pixel(20, 40).unwrap();
    assert!(edge[2] > 100, "{edge:?}");
    // A blurred shadow is soft: grey at its middle edge, darker inside.
    if let ElementKind::Shape(s) = &mut d.element_mut(id).unwrap().kind {
        s.shadow = Some(Shadow { color: Color::rgba(0, 0, 0, 255), offset_x: 20.0, offset_y: 20.0, blur: 8.0 });
    }
    let r = render(&d);
    let inside = r.pixel(70, 70).unwrap();
    let edge = r.pixel(80, 70).unwrap();
    assert!(inside[0] < 60, "{inside:?}");
    assert!(edge[0] > inside[0] && edge[0] < 250, "{edge:?}");
    // Shadows near the page edge and under rotation draw without trouble.
    d.element_mut(id).unwrap().frame = Frame { x: -30.0, y: 100.0, width: 60.0, height: 60.0, rotation: 30.0 };
    let _ = render(&d);
}

fn doc_line(d: &mut Document) -> NodeId {
    let id = d.alloc_id();
    let el = Element::new(
        id,
        Frame::new(10.0, 50.0, 100.0, 20.0),
        ElementKind::Shape(ShapeEl {
            shape: ShapeKind::Line,
            fill: Fill::None,
            stroke: Some(StrokeStyle { color: Color::BLACK, width: 6.0, dash: vec![] }),
            shadow: None,
        }),
    );
    d.pages[0].elements.push(el);
    id
}

fn two_tone() -> Vec<u8> {
    let img = image::RgbaImage::from_fn(40, 20, |x, _| if x < 20 { image::Rgba([0, 0, 255, 255]) } else { image::Rgba([0, 255, 0, 255]) });
    png_bytes(&img)
}

fn add_image(d: &mut Document, bytes: Vec<u8>, frame: Frame, fit: Fit) -> NodeId {
    let asset = d.add_asset(Asset { name: "pic.png".into(), mime: "image/png".into(), width: 40, height: 20, data: Bytes::new(bytes) });
    let id = d.alloc_id();
    let mut im = ImageEl::new(asset);
    im.fit = fit;
    d.pages[0].elements.push(Element::new(id, frame, ElementKind::Image(im)));
    id
}

#[test]
fn pictures_fit_crop_and_adjust() {
    let mut d = doc(100.0, 100.0);
    let id = add_image(&mut d, two_tone(), Frame::new(0.0, 0.0, 100.0, 100.0), Fit::Stretch);
    let mut renderer = Renderer::new();
    let fonts = FontBook::new();
    let r = renderer.render_page(&d, 0, &fonts, &RenderOptions::default()).unwrap();
    assert!(close(r.pixel(10, 50).unwrap(), [0, 0, 255, 255], 3));
    assert!(close(r.pixel(90, 50).unwrap(), [0, 255, 0, 255], 3));
    // Contain leaves bands above and below (page white).
    if let ElementKind::Image(im) = &mut d.element_mut(id).unwrap().kind {
        im.fit = Fit::Contain;
    }
    let r = renderer.render_page(&d, 0, &fonts, &RenderOptions::default()).unwrap();
    assert_eq!(r.pixel(50, 5), Some([255, 255, 255, 255]));
    assert!(close(r.pixel(10, 50).unwrap(), [0, 0, 255, 255], 3));
    // Crop to the right half, then invert it.
    if let ElementKind::Image(im) = &mut d.element_mut(id).unwrap().kind {
        im.fit = Fit::Cover;
        im.crop = CropRect { x: 0.5, y: 0.0, width: 0.5, height: 1.0 };
        im.adjustments = Adjustments { invert: true, ..Default::default() };
    }
    let r = renderer.render_page(&d, 0, &fonts, &RenderOptions::default()).unwrap();
    assert!(close(r.pixel(10, 50).unwrap(), [255, 0, 255, 255], 3), "{:?}", r.pixel(10, 50));
    // Rounded corners clip the picture.
    if let ElementKind::Image(im) = &mut d.element_mut(id).unwrap().kind {
        im.corner_radius = 40.0;
        im.adjustments = Adjustments::default();
    }
    let r = renderer.render_page(&d, 0, &fonts, &RenderOptions::default()).unwrap();
    assert_eq!(r.pixel(1, 1), Some([255, 255, 255, 255]));
    // A circle mask clips to the circle.
    if let ElementKind::Image(im) = &mut d.element_mut(id).unwrap().kind {
        im.corner_radius = 0.0;
        im.mask = Some(ShapeKind::Ellipse);
        im.border = Some(StrokeStyle { color: RED, width: 4.0, dash: vec![] });
    }
    let r = renderer.render_page(&d, 0, &fonts, &RenderOptions::default()).unwrap();
    assert_eq!(r.pixel(3, 3), Some([255, 255, 255, 255]));
    assert!(close(r.pixel(50, 50).unwrap(), [0, 255, 0, 255], 3));
    assert_eq!(r.pixel(50, 1), Some([255, 0, 0, 255]));
    // A thumbnail reduces the cached picture without failing.
    let t = render_thumbnail(&d, 0, &fonts, 16).unwrap();
    assert_eq!((t.width, t.height), (16, 16));
    renderer.clear_cache();
}

#[test]
fn broken_pictures_draw_placeholders() {
    let mut d = doc(60.0, 60.0);
    add_image(&mut d, b"not a picture".to_vec(), Frame::new(10.0, 10.0, 40.0, 40.0), Fit::Cover);
    let id = d.alloc_id();
    // Refers to an asset that does not exist.
    d.pages[0].elements.push(Element::new(id, Frame::new(0.0, 50.0, 10.0, 10.0), ElementKind::Image(ImageEl::new(genha_doc::AssetId(9999)))));
    let r = render(&d);
    let p = PLACEHOLDER;
    assert_eq!(r.pixel(30, 30), Some([p.r, p.g, p.b, 255]));
    assert_eq!(r.pixel(5, 55), Some([p.r, p.g, p.b, 255]));
}

#[test]
fn groups_and_flips() {
    let mut d = doc(100.0, 100.0);
    let a = d.alloc_id();
    let child = Element::new(
        a,
        Frame::new(0.0, 0.0, 50.0, 100.0),
        ElementKind::Shape(ShapeEl { shape: rect(), fill: Fill::solid(RED), stroke: None, shadow: None }),
    );
    let g = d.alloc_id();
    let mut group = Element::new(g, Frame::new(0.0, 0.0, 100.0, 100.0), ElementKind::Group(GroupEl { children: vec![child] }));
    d.pages[0].elements.push(group.clone());
    let r = render(&d);
    assert_eq!(r.pixel(25, 50), Some([255, 0, 0, 255]));
    assert_eq!(r.pixel(75, 50), Some([255, 255, 255, 255]));
    // Flipping the group mirrors its content.
    group.flip_h = true;
    d.pages[0].elements[0] = group;
    let r = render(&d);
    assert_eq!(r.pixel(25, 50), Some([255, 255, 255, 255]));
    assert_eq!(r.pixel(75, 50), Some([255, 0, 0, 255]));
    // A flipped triangle points down.
    let mut d = doc(100.0, 100.0);
    let t = shape(&mut d, Frame::new(0.0, 0.0, 100.0, 100.0), ShapeKind::Triangle, Fill::solid(RED));
    assert_eq!(render(&d).pixel(50, 95), Some([255, 0, 0, 255]));
    assert_eq!(render(&d).pixel(10, 5), Some([255, 255, 255, 255]));
    d.element_mut(t).unwrap().flip_v = true;
    assert_eq!(render(&d).pixel(50, 5), Some([255, 0, 0, 255]));
    assert_eq!(render(&d).pixel(10, 95), Some([255, 255, 255, 255]));
}

fn text_doc(effect: TextEffect) -> Document {
    let mut d = doc(300.0, 100.0);
    let id = d.alloc_id();
    let style = TextStyle { font_size: 60.0, color: Color::BLACK, align: TextAlign::Left, ..TextStyle::default() };
    d.pages[0].elements.push(Element::new(
        id,
        Frame::new(10.0, 10.0, 280.0, 80.0),
        ElementKind::Text(TextEl { text: "HHHH".into(), style, effect, curve: 0.0 }),
    ));
    d
}

fn count(r: &Raster, f: impl Fn([u8; 4]) -> bool) -> usize {
    r.rgba.as_chunks::<4>().0.iter().filter(|p| f(**p)).count()
}

#[test]
fn text_draws_with_effects() {
    let fonts = fonts();
    if fonts.is_empty() {
        return;
    }
    let opts = RenderOptions::default();
    let plain = render_page(&text_doc(TextEffect::None), 0, fonts, &opts).unwrap();
    let dark = count(&plain, |p| p[0] < 50);
    assert!(dark > 500, "{dark}");
    let hl = render_page(&text_doc(TextEffect::Highlight { color: RED, padding: 8.0, corner_radius: 4.0 }), 0, fonts, &opts).unwrap();
    assert!(count(&hl, |p| p == [255, 0, 0, 255]) > 1000);
    let ol = render_page(&text_doc(TextEffect::Outline { color: RED, width: 3.0 }), 0, fonts, &opts).unwrap();
    assert!(count(&ol, |p| p[0] > 200 && p[1] < 50) > 200);
    let sh =
        render_page(&text_doc(TextEffect::Shadow { color: Color::rgba(0, 0, 255, 255), offset_x: 4.0, offset_y: 4.0, blur: 4.0 }), 0, fonts, &opts)
            .unwrap();
    assert!(count(&sh, |p| p[2] > 150 && p[0] < 150) > 100);
}

#[test]
fn limits_and_errors() {
    let d = doc(100.0, 100.0);
    let f = FontBook::new();
    assert!(matches!(render_page(&d, 3, &f, &RenderOptions::default()), Err(RenderError::NoPage(3))));
    assert!(matches!(render_page(&d, 0, &f, &RenderOptions { scale: f64::NAN, ..Default::default() }), Err(RenderError::InvalidScale(_))));
    assert!(matches!(render_page(&d, 0, &f, &RenderOptions { scale: 0.0, ..Default::default() }), Err(RenderError::InvalidScale(_))));
    assert!(matches!(render_page(&d, 0, &f, &RenderOptions { scale: 1000.0, ..Default::default() }), Err(RenderError::TooLarge(..))));
    let big = doc(16_000.0, 16_000.0);
    assert!(matches!(render_page(&big, 0, &f, &RenderOptions::default()), Err(RenderError::TooLarge(..))));
    // Hostile frames and numbers draw nothing rather than failing.
    let mut d = doc(50.0, 50.0);
    let id = shape(&mut d, Frame { x: f64::NAN, y: 0.0, width: f64::INFINITY, height: -3.0, rotation: f64::NAN }, rect(), Fill::solid(RED));
    d.element_mut(id).unwrap().opacity = f32::NAN;
    let _ = render(&d);
}

#[test]
fn thumbnails_keep_aspect() {
    let d = doc(1080.0, 1920.0);
    let t = render_thumbnail(&d, 0, &FontBook::new(), 256).unwrap();
    assert_eq!((t.width, t.height), (144, 256));
    let d = doc(3.0, 1.0);
    let t = render_thumbnail(&d, 0, &FontBook::new(), 300).unwrap();
    assert_eq!((t.width, t.height), (300, 100));
}

#[test]
fn encoders_round_trip() {
    let mut d = doc(64.0, 32.0);
    shape(&mut d, Frame::new(0.0, 0.0, 32.0, 32.0), rect(), Fill::solid(RED));
    let r = render_page(&d, 0, &FontBook::new(), &RenderOptions { scale: 1.0, transparent_background: true }).unwrap();
    let png = encode_png(&r).unwrap();
    let back = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(back.dimensions(), (64, 32));
    assert_eq!(back.as_raw(), &r.rgba);
    let jpg = encode_jpeg(&r, 90).unwrap();
    let back = image::load_from_memory(&jpg).unwrap().to_rgb8();
    assert_eq!(back.dimensions(), (64, 32));
    let p = back.get_pixel(10, 10);
    assert!(p[0] > 230 && p[1] < 30, "{p:?}");
    // Transparent areas become white.
    let p = back.get_pixel(50, 10);
    assert!(p[0] > 240 && p[1] > 240 && p[2] > 240, "{p:?}");
    let bad = Raster { width: 2, height: 2, rgba: vec![0; 3] };
    assert!(encode_png(&bad).is_err());
    assert!(encode_jpeg(&bad, 80).is_err());
}

/// A page with every chart kind, a table and curved text (written to `$GENHA_SAMPLES/data.png`
/// when that variable is set, to look at).
fn data_sample() -> Document {
    let mut d = doc(1500.0, 1300.0);
    let kinds = genha_doc::ChartKind::ALL;
    for (i, k) in kinds.iter().enumerate() {
        let (col, row) = (i % 3, i / 3);
        let mut c = genha_doc::ChartEl::sample(*k);
        if *k == genha_doc::ChartKind::Bar {
            c.data.push(genha_doc::ChartPoint::new("Loss", -12.0));
        }
        if *k == genha_doc::ChartKind::HorizontalBar {
            c.show_legend = true;
        }
        let id = d.alloc_id();
        let frame = Frame::new(20.0 + col as f64 * 490.0, 20.0 + row as f64 * 400.0, 460.0, 360.0);
        d.pages[0].elements.push(Element::new(id, frame, ElementKind::Chart(c)));
    }
    let mut bar = genha_doc::ChartEl::sample(genha_doc::ChartKind::Progress);
    bar.data[0].label = "Funding goal".into();
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(id, Frame::new(510.0, 860.0, 460.0, 90.0), ElementKind::Chart(bar)));
    let mut t = genha_doc::TableEl::blank(4, 3);
    t.rows = vec![
        vec!["Plan".into(), "Price".into(), "Includes".into()],
        vec!["Starter".into(), "$0".into(), "Templates and basic export".into()],
        vec!["Plus".into(), "$9".into(), "Brand kit, more storage, priority support for teams".into()],
        vec!["Team".into(), "$29".into(), "Everything".into()],
    ];
    t.column_widths = vec![0.25, 0.2, 0.55];
    t.font_size = 20.0;
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(id, Frame::new(1000.0, 800.0, 470.0, 200.0), ElementKind::Table(t)));
    for (k, (curve, x, y)) in [(120.0, 510.0, 980.0), (-90.0, 1000.0, 1060.0)].into_iter().enumerate() {
        let style = TextStyle { font_size: 44.0, bold: true, color: Color::rgb(0x5b, 0x3c, 0xf5), ..TextStyle::default() };
        let mut te = TextEl::new(if k == 0 { "CURVED ARCH TEXT" } else { "smiling along the bottom" }, style);
        te.curve = curve;
        if k == 1 {
            te.effect = TextEffect::Outline { color: Color::rgb(0xff, 0xcc, 0x00), width: 3.0 };
        }
        let id = d.alloc_id();
        d.pages[0].elements.push(Element::new(id, Frame::new(x, y, 470.0, 200.0), ElementKind::Text(te)));
    }
    d
}

#[test]
fn charts_tables_and_curved_text_draw() {
    let d = data_sample();
    // Without fonts: shapes still draw, nothing fails.
    let r = render(&d);
    let violet = |p: [u8; 4]| close(p, [0x5b, 0x3c, 0xf5, 255], 12);
    assert!(count(&r, violet) > 10_000, "{}", count(&r, violet));
    let fonts = fonts();
    let r = render_page(&d, 0, fonts, &RenderOptions::default()).unwrap();
    if let Ok(dir) = std::env::var("GENHA_SAMPLES") {
        std::fs::write(std::path::Path::new(&dir).join("data.png"), encode_png(&r).unwrap()).unwrap();
    }
    // Every scene is finite, for each kind, with odd sizes and hostile data.
    for k in genha_doc::ChartKind::ALL {
        let mut c = genha_doc::ChartEl::sample(k);
        for (w, h) in [(400.0, 300.0), (1.0, 1.0), (5000.0, 20.0), (0.0, 10.0)] {
            let s = chart_scene(fonts, &genha_doc::BrandKit::default(), &c, w, h);
            for p in &s.prims {
                let path = match p {
                    Prim::Fill { path, .. } | Prim::Stroke { path, .. } => path,
                    Prim::Text(t) => &t.path,
                };
                let b = path.bounding_box();
                assert!(path.is_empty() || (b.x0.is_finite() && b.y1.is_finite()), "{k:?} {w}x{h}");
            }
        }
        c.data = vec![genha_doc::ChartPoint::new("z", 0.0), genha_doc::ChartPoint::new("n", -5.0)];
        c.max = Some(1e-300);
        let _ = chart_scene(fonts, &genha_doc::BrandKit::default(), &c, 300.0, 300.0);
        c.data.clear();
        let _ = chart_scene(fonts, &genha_doc::BrandKit::default(), &c, 300.0, 300.0);
    }
}

#[test]
fn pie_starts_at_twelve_and_goes_clockwise() {
    let mut d = doc(200.0, 200.0);
    let mut c = genha_doc::ChartEl::sample(genha_doc::ChartKind::Pie);
    c.data = vec![genha_doc::ChartPoint::new("a", 1.0), genha_doc::ChartPoint::new("b", 3.0)];
    c.colors = vec![RED, Color::rgb(0, 0, 255)];
    c.show_legend = false;
    c.show_values = false;
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(id, Frame::new(0.0, 0.0, 200.0, 200.0), ElementKind::Chart(c)));
    let r = render(&d);
    // The first quarter (12 to 3 o'clock) is red, the rest blue.
    assert!(close(r.pixel(140, 60).unwrap(), [255, 0, 0, 255], 10), "{:?}", r.pixel(140, 60));
    assert!(close(r.pixel(60, 60).unwrap(), [0, 0, 255, 255], 10));
    assert!(close(r.pixel(140, 140).unwrap(), [0, 0, 255, 255], 10));
}

#[test]
fn table_cells_fill_and_grow() {
    let mut t = genha_doc::TableEl::blank(3, 2);
    t.header_fill = RED;
    t.border = None;
    let g = table_geometry(&FontBook::new(), &t, 200.0, 0.0);
    assert_eq!((g.xs.len(), g.ys.len()), (3, 4));
    assert!(g.natural_height > 0.0);
    // A long cell wraps and makes its row taller.
    let mut long = t.clone();
    long.rows[1][0] = "many words in one narrow cell that must wrap".into();
    let g2 = table_geometry(&FontBook::new(), &long, 200.0, 0.0);
    assert!(g2.ys[2] - g2.ys[1] > (g.ys[2] - g.ys[1]) * 2.0);
    // A taller box stretches the rows.
    let g3 = table_geometry(&FontBook::new(), &t, 200.0, g.natural_height * 2.0);
    assert!((g3.ys[3] - g.natural_height * 2.0).abs() < 1e-6);
    let mut d = doc(200.0, 200.0);
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(id, Frame::new(0.0, 0.0, 200.0, 200.0), ElementKind::Table(t)));
    let r = render(&d);
    assert!(close(r.pixel(5, 5).unwrap(), [255, 0, 0, 255], 2));
}

#[test]
fn curved_text_bends() {
    let fonts = fonts();
    let style = TextStyle { font_size: 40.0, ..TextStyle::default() };
    let mut t = TextEl::new("Curved text here", style);
    assert!(curved_text_path(fonts, &t, 600.0, 100.0).is_none());
    t.curve = 180.0;
    let (w, h) = curved_text_size(fonts, &t, 600.0).unwrap();
    let (sw, sh) = genha_text::measure(fonts, &t.text, &t.style);
    // Half a circle: about as tall as its radius (+ the letters), narrower than the straight line.
    assert!(h > sh * 1.5 && w < sw, "{w}x{h} vs {sw}x{sh}");
    if fonts.is_empty() {
        return;
    }
    let p = curved_text_path(fonts, &t, 600.0, 300.0).unwrap();
    let arch = p.bounding_box();
    assert!((arch.center().x - 300.0).abs() < 10.0 && (arch.center().y - 150.0).abs() < 30.0, "{arch:?}");
    // An arch's middle is at its top; a smile's at its bottom.
    let middle = |p: &kurbo::BezPath| {
        p.segments()
            .map(|s| s.bounding_box())
            .filter(|b| b.x0 < 300.0 && b.x1 > 300.0)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), b| (lo.min(b.y0), hi.max(b.y1)))
    };
    assert!(middle(&p).0 < arch.y0 + 10.0, "{:?} vs {arch:?}", middle(&p));
    t.curve = -180.0;
    let smile = curved_text_path(fonts, &t, 600.0, 300.0).unwrap();
    assert!(middle(&smile).1 > smile.bounding_box().y1 - 10.0);
}

#[test]
fn synthetic_bold_and_italic_when_the_family_has_neither() {
    // A book with only a regular face: bold is drawn emboldened, italic slanted.
    let Some(path) =
        ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf"].iter().find(|p| std::path::Path::new(p).exists())
    else {
        return; // no font on this machine
    };
    let mut book = FontBook::new();
    book.add_font(std::fs::read(path).unwrap()).unwrap();
    let draw = |bold: bool, italic: bool| {
        let mut d = doc(400.0, 120.0);
        let style = TextStyle { font_family: "DejaVu Sans".into(), font_size: 60.0, bold, italic, color: Color::BLACK, ..TextStyle::default() };
        let id = d.alloc_id();
        d.pages[0].elements.push(Element::new(id, Frame::new(10.0, 10.0, 380.0, 100.0), ElementKind::Text(TextEl::new("IIII", style))));
        let r = render_page(&d, 0, &book, &RenderOptions { scale: 1.0, transparent_background: true }).unwrap();
        let ink: Vec<usize> = r.rgba.as_chunks::<4>().0.iter().enumerate().filter(|(_, p)| p[3] > 128).map(|(i, _)| i % 400).collect();
        (ink.len(), ink.iter().copied().max().unwrap_or(0))
    };
    let (plain, plain_right) = draw(false, false);
    let (bold, _) = draw(true, false);
    let (italic, italic_right) = draw(false, true);
    assert!(bold as f64 > plain as f64 * 1.1, "bold {bold} vs {plain}");
    assert!((italic as f64 - plain as f64).abs() < plain as f64 * 0.1, "slanting keeps the ink: {italic} vs {plain}");
    assert!(italic_right > plain_right + 5, "italic leans right: {italic_right} vs {plain_right}");
    assert_eq!(synthetic(&book, &TextStyle { font_family: "DejaVu Sans".into(), ..TextStyle::default() }), Synthetic::default());
}
