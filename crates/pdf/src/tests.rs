use genha_doc::{
    Asset, Color, CropRect, Element, ElementKind, Fill, Frame, GradientStop, GroupEl, ImageEl, Page, Shadow, ShapeEl, ShapeKind, StrokeStyle,
    TextEffect, TextEl, TextStyle,
};

use super::*;

fn doc() -> Document {
    let mut d = Document::new("Flyer (v2) \\ final", 200.0, 100.0);
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(
        id,
        Frame::new(10.0, 10.0, 50.0, 50.0),
        ElementKind::Shape(ShapeEl { shape: ShapeKind::Ellipse, fill: Fill::solid(Color::rgb(255, 0, 0)), stroke: None, shadow: None }),
    ));
    let pid = d.alloc_id();
    d.pages.push(Page::new(pid, "Page 2", 50.0, 120.0));
    d
}

fn shape(d: &mut Document, frame: Frame, shape: ShapeKind, fill: Fill) -> Element {
    let id = d.alloc_id();
    Element::new(id, frame, ElementKind::Shape(ShapeEl { shape, fill, stroke: None, shadow: None }))
}

fn text(d: &mut Document, frame: Frame, s: &str, size: f64) -> Element {
    let id = d.alloc_id();
    Element::new(
        id,
        frame,
        ElementKind::Text(TextEl {
            text: s.into(),
            style: TextStyle { font_size: size, ..TextStyle::default() },
            effect: TextEffect::None,
            curve: 0.0,
        }),
    )
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

fn vector(d: &Document, fonts: &FontBook) -> Vec<u8> {
    export_pdf(d, fonts, &PdfOptions::default()).unwrap()
}

/// The parsed PDF.
fn parse(pdf: &[u8]) -> lopdf::Document {
    lopdf::Document::load_mem(pdf).expect("the PDF parses")
}

/// The text `pdftotext` (poppler) reads from `pdf`, when it is installed.
fn pdftotext(pdf: &[u8]) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("genha-pdf-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("t.pdf");
    std::fs::write(&path, pdf).ok()?;
    let out = std::process::Command::new("pdftotext").arg(&path).arg("-").output().ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The operators in `content`.
fn ops(content: &[u8]) -> Vec<String> {
    lopdf::content::Content::decode(content).map(|c| c.operations.into_iter().map(|o| o.operator).collect()).unwrap_or_default()
}

/// The operators of every page and form XObject.
fn all_ops(pdf: &lopdf::Document) -> Vec<String> {
    let mut out = vec![];
    for (_, id) in pdf.get_pages() {
        out.extend(ops(&pdf.get_page_content(id)));
    }
    for obj in pdf.objects.values() {
        if let Ok(s) = obj.as_stream()
            && s.dict.get(b"Subtype").and_then(|v| v.as_name()).is_ok_and(|n| n == b"Form")
        {
            out.extend(ops(&s.decompressed_content().unwrap_or_else(|_| s.content.clone())));
        }
    }
    out
}

fn has(ops: &[String], op: &str) -> bool {
    ops.iter().any(|o| o == op)
}

/// The `BaseFont` names of the fonts.
fn base_fonts(pdf: &lopdf::Document) -> Vec<String> {
    pdf.objects
        .values()
        .filter_map(|o| o.as_dict().ok())
        .filter(|d| d.get(b"Type").and_then(|v| v.as_name()).is_ok_and(|n| n == b"Font"))
        .filter_map(|d| d.get(b"BaseFont").and_then(|v| v.as_name()).ok().map(|n| String::from_utf8_lossy(n).into_owned()))
        .collect()
}

/// The image XObjects: (width, height, filter).
fn images(pdf: &lopdf::Document) -> Vec<(i64, i64, String)> {
    let mut out = vec![];
    for obj in pdf.objects.values() {
        if let Ok(s) = obj.as_stream()
            && s.dict.get(b"Subtype").and_then(|v| v.as_name()).is_ok_and(|n| n == b"Image")
            // Soft masks are images too; count the pictures.
            && !s.dict.get(b"ColorSpace").and_then(|v| v.as_name()).is_ok_and(|n| n == b"DeviceGray")
        {
            let w = s.dict.get(b"Width").and_then(|v| v.as_i64()).unwrap_or(0);
            let h = s.dict.get(b"Height").and_then(|v| v.as_i64()).unwrap_or(0);
            let f = match s.dict.get(b"Filter") {
                Ok(lopdf::Object::Name(n)) => String::from_utf8_lossy(n).into_owned(),
                Ok(lopdf::Object::Array(a)) => a.iter().filter_map(|o| o.as_name().ok()).map(|n| String::from_utf8_lossy(n).into_owned()).collect(),
                _ => String::new(),
            };
            out.push((w, h, f));
        }
    }
    out.sort();
    out
}

/// A page's box, `[x0 y0 x1 y1]`.
fn page_box(pdf: &lopdf::Document, page: u32, name: &[u8]) -> Option<[f64; 4]> {
    let id = *pdf.get_pages().get(&page)?;
    let dict = pdf.get_dictionary(id).ok()?;
    let arr = dict.get(name).ok()?.as_array().ok()?;
    let v: Vec<f64> = arr.iter().filter_map(|o| o.as_float().ok().map(f64::from).or_else(|| o.as_i64().ok().map(|i| i as f64))).collect();
    Some([*v.first()?, *v.get(1)?, *v.get(2)?, *v.get(3)?])
}

fn close(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05)
}

fn jpeg(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x * 4) as u8, (y * 5) as u8, 128]));
    let mut out = vec![];
    image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Jpeg).unwrap();
    out
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(w, h, |x, _| image::Rgba([0, 200, 0, if x < w / 2 { 255 } else { 128 }]));
    let mut out = vec![];
    image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

fn picture(d: &mut Document, bytes: Vec<u8>, mime: &str, frame: Frame) -> Element {
    let asset = d.add_asset(Asset { name: "p".into(), mime: mime.into(), width: 0, height: 0, data: genha_doc::Bytes::new(bytes) });
    let id = d.alloc_id();
    Element::new(id, frame, ElementKind::Image(ImageEl::new(asset)))
}

// ---------- raster PDF (the compatibility option) ----------

/// Checks the xref table: every in-use entry points at `N 0 obj`. Returns the number of objects.
fn check_xref(pdf: &[u8]) -> usize {
    let tail = std::str::from_utf8(&pdf[pdf.len().saturating_sub(64)..]).unwrap();
    let start: usize = tail.split("startxref\n").nth(1).unwrap().lines().next().unwrap().trim().parse().unwrap();
    let xref = std::str::from_utf8(&pdf[start..]).unwrap();
    assert!(xref.starts_with("xref\n"));
    let mut lines = xref.lines().skip(1);
    let header = lines.next().unwrap();
    let size: usize = header.split(' ').nth(1).unwrap().parse().unwrap();
    for (i, line) in lines.take(size).enumerate() {
        assert_eq!(line.len(), 19, "entry {i} has the wrong width: {line:?}");
        if i == 0 {
            assert!(line.ends_with(" f "), "{line:?}");
            continue;
        }
        let off: usize = line[..10].parse().unwrap();
        let want = format!("{i} 0 obj");
        assert_eq!(&pdf[off..off + want.len()], want.as_bytes(), "object {i}");
    }
    size - 1
}

fn raster(dpi: f64) -> PdfOptions {
    PdfOptions { raster: true, dpi, ..PdfOptions::default() }
}

#[test]
fn raster_pdf_is_valid() {
    let d = doc();
    let pdf = export_pdf(&d, &FontBook::new(), &raster(96.0)).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4\n"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    assert_eq!(count(&pdf, b"/Type /Page /Parent"), 2);
    assert!(find(&pdf, b"/Count 2").is_some());
    assert_eq!(check_xref(&pdf), 4 + 2 * 3);
    // Page sizes in points, different per page.
    assert!(find(&pdf, b"/MediaBox [0 0 150 75]").is_some());
    assert!(find(&pdf, b"/MediaBox [0 0 37.5 90]").is_some());
    // Title escaped.
    assert!(find(&pdf, br"/Title (Flyer \(v2\) \\ final)").is_some());
    // The embedded JPEGs decode to the rendered size.
    let start = find(&pdf, b"/Filter /DCTDecode").unwrap();
    let s = start + find(&pdf[start..], b"stream\n").unwrap() + 7;
    let e = s + find(&pdf[s..], b"\nendstream").unwrap();
    let img = image::load_from_memory_with_format(&pdf[s..e], image::ImageFormat::Jpeg).unwrap().to_rgb8();
    assert_eq!(img.dimensions(), (200, 100));
    let p = img.get_pixel(35, 35);
    assert!(p[0] > 200 && p[1] < 60, "{p:?}");
    parse(&pdf);
}

#[test]
fn raster_page_selection_and_resolution() {
    let d = doc();
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions { pages: Some(vec![1]), jpeg_quality: 50, ..raster(192.0) }).unwrap();
    assert_eq!(count(&pdf, b"/Type /Page /Parent"), 1);
    assert!(find(&pdf, b"/Width 100 /Height 240").is_some());
    check_xref(&pdf);
    // Resolution is clamped.
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions { pages: Some(vec![1]), jpeg_quality: 0, ..raster(1e9) }).unwrap();
    assert!(find(&pdf, b"/Width 313 /Height 750").is_some());
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions { pages: Some(vec![1]), jpeg_quality: 200, ..raster(f64::NAN) }).unwrap();
    check_xref(&pdf);
}

#[test]
fn page_selection_errors() {
    let d = doc();
    for r in [false, true] {
        let o = |pages: Vec<usize>| PdfOptions { pages: Some(pages), raster: r, ..Default::default() };
        assert!(matches!(export_pdf(&d, &FontBook::new(), &o(vec![0, 5])), Err(PdfError::NoPage(5))));
        assert!(matches!(export_pdf(&d, &FontBook::new(), &o(vec![])), Err(PdfError::NoPages)));
    }
}

#[test]
fn huge_pages_are_scaled_to_fit() {
    assert!((fitting_scale(16_000.0, 16_000.0, 6.25) * 16_000.0) <= f64::from(MAX_SIDE));
    let s = fitting_scale(10_000.0, 10_000.0, 1.0);
    assert!(s * s * 1e8 <= MAX_PIXELS as f64);
    assert_eq!(fitting_scale(100.0, 100.0, 2.0), 2.0);
}

#[test]
fn strings() {
    assert_eq!(pdf_string("plain"), "(plain)");
    assert_eq!(pdf_string("a\nb"), "(a\\012b)");
    assert_eq!(pdf_string("Café"), "<FEFF00430061006600E9>");
    assert_eq!(num(37.5), "37.5");
    assert_eq!(num(810.0), "810");
    assert_eq!(raster::win_ansi("a(é)"), ("(a\\(\\351\\))".to_string(), 4));
}

#[test]
fn raster_text_layer_and_print_boxes() {
    let mut d = Document::new("Flyer", 400.0, 300.0);
    let t = text(&mut d, Frame::new(20.0, 20.0, 360.0, 60.0), "Hello (world) café", 20.0);
    d.pages[0].elements.push(t);
    let fonts = genha_text::FontBook::system();
    let pdf = export_pdf(&d, &fonts, &raster(40.0)).unwrap();
    assert!(find(&pdf, b"/BaseFont /Helvetica").is_some());
    assert!(find(&pdf, b"3 Tr").is_some(), "invisible text");
    assert!(find(&pdf, b"(Hello \\(world\\) caf\\351)").is_some(), "escaped WinAnsi text");
    assert!(find(&pdf, b"/TrimBox").is_none());
    check_xref(&pdf);

    let print = export_pdf(&d, &fonts, &PdfOptions { dpi: 40.0, raster: true, ..PdfOptions::print() }).unwrap();
    check_xref(&print);
    // 3 mm bleed + 10 mm slug on each side: 400 px = 300 pt wide trim.
    let bleed = 3.0 * 72.0 / 25.4;
    let slug = 10.0 * 72.0 / 25.4;
    let media_w = 300.0 + 2.0 * (bleed + slug);
    let text = String::from_utf8_lossy(&print);
    let mb = text.split("/MediaBox [0 0 ").nth(1).unwrap();
    let w: f64 = mb.split_whitespace().next().unwrap().parse().unwrap();
    assert!((w - media_w).abs() < 0.01, "{w} vs {media_w}");
    assert!(text.contains("/TrimBox") && text.contains("/BleedBox"));
    assert!(count(&print, b" l S") == 8, "eight crop mark lines");
}

#[test]
fn bleed_extends_edge_shapes() {
    let mut d = Document::new("x", 100.0, 100.0);
    let e = shape(&mut d, Frame::new(0.0, 50.0, 100.0, 50.0), ShapeKind::Rect { corner_radius: 0.0 }, Fill::solid(Color::BLACK));
    d.pages[0].elements.push(e);
    let p = with_bleed(&d.pages[0], 10.0);
    assert_eq!((p.width, p.height), (120.0, 120.0));
    let f = p.elements[0].frame;
    assert_eq!((f.x, f.y, f.width, f.height), (0.0, 60.0, 120.0, 60.0));
}

// ---------- vector PDF ----------

#[test]
fn vector_pdf_pages_title_and_paths() {
    let mut d = doc();
    let stops = vec![GradientStop { offset: 0.0, color: Color::rgb(0, 0, 255) }, GradientStop { offset: 1.0, color: Color::rgba(255, 255, 0, 128) }];
    let lin =
        shape(&mut d, Frame::new(70.0, 10.0, 60.0, 40.0), ShapeKind::Rect { corner_radius: 8.0 }, Fill::Linear { angle: 30.0, stops: stops.clone() });
    let rad = shape(&mut d, Frame::new(140.0, 10.0, 50.0, 50.0), ShapeKind::Ellipse, Fill::Radial { stops });
    let mut star = shape(&mut d, Frame::new(10.0, 60.0, 30.0, 30.0), ShapeKind::Star { points: 5, inner_ratio: 0.5 }, Fill::None);
    if let ElementKind::Shape(s) = &mut star.kind {
        s.stroke = Some(StrokeStyle { color: Color::BLACK, width: 2.0, dash: vec![4.0, 2.0] });
    }
    star.frame.rotation = 30.0;
    star.flip_h = true;
    star.opacity = 0.5;
    let line = shape(&mut d, Frame::new(50.0, 70.0, 80.0, 10.0), ShapeKind::Line, Fill::solid(Color::rgb(0, 128, 0)));
    let gid = d.alloc_id();
    let mut group = Element::new(gid, Frame::new(0.0, 0.0, 200.0, 100.0), ElementKind::Group(GroupEl { children: vec![star, line] }));
    group.flip_v = true;
    d.pages[0].elements.extend([lin, rad, group]);
    let pdf = vector(&d, &FontBook::new());
    let p = parse(&pdf);
    assert_eq!(p.get_pages().len(), 2);
    assert!(close(page_box(&p, 1, b"MediaBox").unwrap(), [0.0, 0.0, 150.0, 75.0]));
    assert!(close(page_box(&p, 2, b"MediaBox").unwrap(), [0.0, 0.0, 37.5, 90.0]));
    assert!(page_box(&p, 1, b"TrimBox").is_none());
    let info = p.trailer.get(b"Info").and_then(|r| r.as_reference()).and_then(|id| p.get_dictionary(id)).unwrap();
    let title = info.get(b"Title").unwrap().as_str().unwrap();
    assert_eq!(String::from_utf8_lossy(title), "Flyer (v2) \\ final");
    // Vector drawing, no pictures.
    assert!(images(&p).is_empty(), "{:?}", images(&p));
    let o = all_ops(&p);
    assert!(o.iter().filter(|o| *o == "c").count() > 4, "curves: {o:?}");
    for op in ["re", "f", "S", "d", "cm", "q", "Q", "W", "gs"] {
        assert!(has(&o, op) || (op == "re" && has(&o, "l")), "{op} in {o:?}");
    }
    let shading = |t: i64| {
        p.objects
            .values()
            .filter_map(|o| o.as_dict().ok().or_else(|| o.as_stream().ok().map(|s| &s.dict)))
            .any(|d| d.get(b"ShadingType").and_then(|v| v.as_i64()).is_ok_and(|v| v == t))
    };
    assert!(shading(2), "axial gradient");
    assert!(shading(3), "radial gradient");
}

#[test]
fn vector_text_embeds_fonts() {
    let fonts = FontBook::system();
    let mut d = Document::new("Text", 600.0, 400.0);
    let mut t = text(&mut d, Frame::new(20.0, 20.0, 560.0, 80.0), "Hello GenHa vector", 36.0);
    if let ElementKind::Text(te) = &mut t.kind {
        te.style.underline = true;
        te.style.letter_spacing = 50.0;
    }
    let mut curved = text(&mut d, Frame::new(20.0, 150.0, 400.0, 200.0), "Curved words", 30.0);
    if let ElementKind::Text(te) = &mut curved.kind {
        te.curve = 120.0;
        te.effect = TextEffect::Outline { color: Color::rgb(255, 0, 0), width: 2.0 };
    }
    // Ligatures copy as their letters; upper-cased text as shown.
    let lig = text(&mut d, Frame::new(20.0, 100.0, 560.0, 40.0), "office affinity", 24.0);
    let mut loud = text(&mut d, Frame::new(20.0, 360.0, 560.0, 40.0), "make it loud", 24.0);
    if let ElementKind::Text(te) = &mut loud.kind {
        te.style.uppercase = true;
    }
    d.pages[0].elements.extend([t, curved, lig, loud]);
    let pdf = vector(&d, &fonts);
    let p = parse(&pdf);
    let o = all_ops(&p);
    if fonts.is_empty() {
        // Nothing to embed: the export still works.
        return;
    }
    assert!(has(&o, "Tj") || has(&o, "TJ"), "text operators: {o:?}");
    assert!(has(&o, "Tf") && has(&o, "BT"));
    assert!(find(&pdf, b"/FontFile2").is_some() || find(&pdf, b"/FontFile3").is_some(), "an embedded font program");
    // Subset fonts are named with a six-letter tag: `/BaseFont /ABCDEF+Name`.
    let names = base_fonts(&p);
    assert!(!names.is_empty() && names.iter().all(|n| n.find('+') == Some(6)), "subset tags: {names:?}");
    assert!(find(&pdf, b"/ToUnicode").is_some());
    if let Some(text) = pdftotext(&pdf) {
        for want in ["Hello GenHa vector", "office affinity", "MAKE IT LOUD"] {
            assert!(text.contains(want), "{want} in {text:?}");
        }
    }
    // Outlines (the red outline effect) under the curved text.
    assert!(has(&o, "S"));
    // Letters as outlines on request: no fonts.
    let outlined = export_pdf(&d, &fonts, &PdfOptions { text_as_outlines: true, ..Default::default() }).unwrap();
    assert!(find(&outlined, b"/FontFile").is_none());
    assert!(!has(&all_ops(&parse(&outlined)), "Tf"));
}

#[test]
fn curved_glyphs_follow_the_renderer() {
    let fonts = FontBook::system();
    if fonts.is_empty() {
        return;
    }
    let mut d = Document::new("x", 400.0, 400.0);
    for curve in [90.0, -200.0, 360.0] {
        let mut e = text(&mut d, Frame::new(0.0, 0.0, 300.0, 300.0), "Round and round\nwe go", 28.0);
        let ElementKind::Text(t) = &mut e.kind else { panic!("text") };
        t.curve = curve;
        let want = kurbo::Shape::bounding_box(&genha_render::curved_text_path(&fonts, t, 300.0, 300.0).unwrap());
        let (gl, xfs) = vector::test_curved_glyphs(&fonts, t, 300.0, 300.0).unwrap();
        let mut got: Option<kurbo::Rect> = None;
        for (line, row) in gl.lines.iter().zip(&xfs) {
            for (g, xf) in line.glyphs.iter().zip(row) {
                if !g.path.is_empty() {
                    let b = kurbo::Shape::bounding_box(&(*xf * g.path.clone()));
                    got = Some(got.map_or(b, |r| r.union(b)));
                }
            }
        }
        let got = got.unwrap();
        for (a, b) in [(got.x0, want.x0), (got.y0, want.y0), (got.x1, want.x1), (got.y1, want.y1)] {
            assert!((a - b).abs() < 0.01, "{got:?} vs {want:?}");
        }
    }
}

#[test]
fn pictures_are_embedded_natively() {
    let mut d = Document::new("Pics", 400.0, 300.0);
    // An unadjusted JPEG goes in as it is, even cropped (the crop is a clip).
    let mut a = picture(&mut d, jpeg(64, 48), "image/jpeg", Frame::new(10.0, 10.0, 100.0, 75.0));
    if let ElementKind::Image(im) = &mut a.kind {
        im.crop = CropRect { x: 0.25, y: 0.0, width: 0.5, height: 1.0 };
        im.corner_radius = 12.0;
    }
    // A PNG with transparency: lossless pixels and a soft mask, cut to a star.
    let mut b = picture(&mut d, png(40, 30), "image/png", Frame::new(150.0, 10.0, 80.0, 60.0));
    if let ElementKind::Image(im) = &mut b.kind {
        im.mask = Some(ShapeKind::Star { points: 5, inner_ratio: 0.5 });
        im.border = Some(StrokeStyle { color: Color::BLACK, width: 3.0, dash: vec![] });
    }
    // An adjusted JPEG: the cropped, adjusted pixels.
    let mut c = picture(&mut d, jpeg(64, 48), "image/jpeg", Frame::new(10.0, 150.0, 100.0, 75.0));
    if let ElementKind::Image(im) = &mut c.kind {
        im.adjustments.grayscale = 100.0;
        im.crop = CropRect { x: 0.0, y: 0.0, width: 0.5, height: 0.5 };
    }
    // A missing picture: the grey placeholder.
    let id = d.alloc_id();
    let missing = Element::new(id, Frame::new(300.0, 200.0, 50.0, 50.0), ElementKind::Image(ImageEl::new(genha_doc::AssetId(9999))));
    d.pages[0].elements.extend([a, b, c, missing]);
    let pdf = vector(&d, &FontBook::new());
    let p = parse(&pdf);
    let imgs = images(&p);
    assert_eq!(imgs.len(), 3, "{imgs:?}");
    assert!(imgs.contains(&(64, 48, "DCTDecode".into())), "{imgs:?}");
    assert!(imgs.contains(&(40, 30, "FlateDecode".into())), "{imgs:?}");
    assert!(imgs.contains(&(32, 24, "FlateDecode".into())), "{imgs:?}");
    assert!(find(&pdf, b"/SMask").is_some(), "the PNG's transparency");
    let o = all_ops(&p);
    assert!(has(&o, "W") && has(&o, "n") && has(&o, "Do"), "clipped pictures: {o:?}");
    // The same picture twice is embedded once.
    let again = picture(&mut d, jpeg(64, 48), "image/jpeg", Frame::new(200.0, 200.0, 64.0, 48.0));
    d.pages[0].elements.push(again);
    assert_eq!(images(&parse(&vector(&d, &FontBook::new()))).len(), 3);
}

#[test]
fn blurred_shadows_become_pictures_sharp_ones_stay_vectors() {
    let mut d = Document::new("Shadows", 300.0, 200.0);
    let mut sharp = shape(&mut d, Frame::new(20.0, 20.0, 80.0, 60.0), ShapeKind::Rect { corner_radius: 0.0 }, Fill::solid(Color::WHITE));
    if let ElementKind::Shape(s) = &mut sharp.kind {
        s.shadow = Some(Shadow { color: Color::rgba(0, 0, 0, 100), offset_x: 4.0, offset_y: 4.0, blur: 0.0 });
    }
    d.pages[0].elements.push(sharp);
    assert!(images(&parse(&vector(&d, &FontBook::new()))).is_empty());
    let mut soft = shape(&mut d, Frame::new(150.0, 20.0, 80.0, 60.0), ShapeKind::Ellipse, Fill::solid(Color::WHITE));
    if let ElementKind::Shape(s) = &mut soft.kind {
        s.shadow = Some(Shadow { color: Color::rgba(0, 0, 0, 100), offset_x: 4.0, offset_y: 6.0, blur: 12.0 });
    }
    soft.frame.rotation = 20.0;
    d.pages[0].elements.push(soft);
    let pdf = vector(&d, &FontBook::new());
    let imgs = images(&parse(&pdf));
    assert_eq!(imgs.len(), 1, "{imgs:?}");
    // The shadow only (a small picture), not the page.
    assert!(imgs[0].0 < 400 && imgs[0].1 < 400, "{imgs:?}");
    assert!(find(&pdf, b"/SMask").is_some(), "the shadow fades out");
}

#[test]
fn print_pdf_has_boxes_and_marks() {
    let mut d = Document::new("Print", 400.0, 300.0);
    let e = shape(&mut d, Frame::new(0.0, 0.0, 400.0, 150.0), ShapeKind::Rect { corner_radius: 0.0 }, Fill::solid(Color::rgb(200, 30, 30)));
    d.pages[0].elements.push(e);
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions::print()).unwrap();
    let p = parse(&pdf);
    let bleed = 3.0 * 72.0 / 25.4;
    let slug = 10.0 * 72.0 / 25.4;
    let mw = 300.0 + 2.0 * (bleed + slug);
    let mh = 225.0 + 2.0 * (bleed + slug);
    assert!(close(page_box(&p, 1, b"MediaBox").unwrap(), [0.0, 0.0, mw, mh]), "{:?}", page_box(&p, 1, b"MediaBox"));
    let o = bleed + slug;
    assert!(close(page_box(&p, 1, b"TrimBox").unwrap(), [o, o, o + 300.0, o + 225.0]), "{:?}", page_box(&p, 1, b"TrimBox"));
    assert!(close(page_box(&p, 1, b"BleedBox").unwrap(), [slug, slug, mw - slug, mh - slug]), "{:?}", page_box(&p, 1, b"BleedBox"));
    let o = all_ops(&p);
    assert!(o.iter().filter(|o| *o == "m").count() >= 8, "crop marks: {o:?}");
    assert!(has(&o, "rg") || has(&o, "sc") || has(&o, "scn"), "{o:?}");
    // CMYK on request: colours set with `k`/`K` (or a CMYK colour space), none with `rg`.
    let cmyk = export_pdf(&d, &FontBook::new(), &PdfOptions { cmyk: true, ..PdfOptions::print() }).unwrap();
    let o = all_ops(&parse(&cmyk));
    assert!(has(&o, "k") || has(&o, "K"), "{o:?}");
    assert!(!has(&o, "rg") && !has(&o, "RG"), "{o:?}");
}

#[test]
fn cmyk_conversion() {
    assert_eq!(vector::test_to_cmyk(Color::BLACK), [0, 0, 0, 255]);
    assert_eq!(vector::test_to_cmyk(Color::WHITE), [0, 0, 0, 0]);
    assert_eq!(vector::test_to_cmyk(Color::rgb(255, 0, 0)), [0, 255, 255, 0]);
}

#[test]
fn every_template_exports() {
    let system = FontBook::system();
    for fonts in [FontBook::new(), system] {
        for info in genha_templates::catalog() {
            let d = genha_templates::instantiate(info.id, None).unwrap();
            let pdf = export_pdf(&d, &fonts, &PdfOptions::default()).unwrap_or_else(|e| panic!("{}: {e}", info.id));
            let p = parse(&pdf);
            assert_eq!(p.get_pages().len(), d.pages.len(), "{}", info.id);
        }
    }
}

#[test]
fn junk_survives() {
    let fonts = FontBook::system();
    let mut d = Document::new("", 1.0, 1.0);
    let mut els = vec![];
    for (i, f) in [
        Frame::new(f64::NAN, 0.0, 10.0, 10.0),
        Frame::new(0.0, 0.0, f64::INFINITY, 10.0),
        Frame::new(1e300, -1e300, 1e300, 1e300),
        Frame::new(0.0, 0.0, 0.0, 0.0),
        Frame::new(0.0, 0.0, -5.0, 3.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = shape(&mut d, f, ShapeKind::Path { d: "M0 0 L10 10 Z".into(), view_width: 0.0, view_height: 0.0 }, Fill::solid(Color::BLACK));
        e.opacity = [f32::NAN, 2.0, -1.0, 0.5, 1.0][i];
        if let ElementKind::Shape(s) = &mut e.kind {
            s.stroke = Some(StrokeStyle { color: Color::BLACK, width: [f64::NAN, 1e30, -1.0, 0.0, 3.0][i], dash: vec![f64::NAN, 0.0, 1e-9] });
            s.shadow = Some(Shadow { color: Color::BLACK, offset_x: f64::NAN, offset_y: 1e300, blur: [f64::INFINITY, 1e9, 3.0, -2.0, 0.0][i] });
        }
        els.push(e);
        let mut t = text(&mut d, f, "x\u{0}\u{FFFF}é漢字 🙂\n\n", [f64::NAN, 1e9, 0.0, -3.0, 12.0][i]);
        if let ElementKind::Text(te) = &mut t.kind {
            te.curve = [f64::NAN, 1e9, 359.0, -0.1, 45.0][i];
            te.effect = TextEffect::Shadow { color: Color::BLACK, offset_x: 1.0, offset_y: 1.0, blur: 4.0 };
        }
        els.push(t);
        els.push(picture(&mut d, b"not a picture".to_vec(), "image/png", f));
    }
    d.pages[0].elements = els;
    d.pages[0].background = Fill::Linear { angle: f64::NAN, stops: vec![] };
    for opts in
        [PdfOptions::default(), PdfOptions::print(), PdfOptions { cmyk: true, dpi: f64::NAN, bleed_mm: f64::INFINITY, ..PdfOptions::default() }]
    {
        let pdf = export_pdf(&d, &fonts, &opts).unwrap();
        parse(&pdf);
    }
}
