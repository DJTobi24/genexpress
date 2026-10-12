use genha_doc::{Color, Element, ElementKind, Fill, Frame, Page, ShapeEl, ShapeKind};

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

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

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

#[test]
fn writes_a_valid_pdf() {
    let d = doc();
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions { dpi: 96.0, ..Default::default() }).unwrap();
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
}

#[test]
fn page_selection_and_resolution() {
    let d = doc();
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions { dpi: 192.0, pages: Some(vec![1]), jpeg_quality: 50, ..PdfOptions::default() }).unwrap();
    assert_eq!(count(&pdf, b"/Type /Page /Parent"), 1);
    assert!(find(&pdf, b"/Width 100 /Height 240").is_some());
    check_xref(&pdf);
    // Resolution is clamped.
    let pdf = export_pdf(&d, &FontBook::new(), &PdfOptions { dpi: 1e9, pages: Some(vec![1]), jpeg_quality: 0, ..PdfOptions::default() }).unwrap();
    assert!(find(&pdf, b"/Width 313 /Height 750").is_some());
    let pdf =
        export_pdf(&d, &FontBook::new(), &PdfOptions { dpi: f64::NAN, pages: Some(vec![1]), jpeg_quality: 200, ..PdfOptions::default() }).unwrap();
    check_xref(&pdf);
    assert!(matches!(export_pdf(&d, &FontBook::new(), &PdfOptions { pages: Some(vec![0, 5]), ..Default::default() }), Err(PdfError::NoPage(5))));
    assert!(matches!(export_pdf(&d, &FontBook::new(), &PdfOptions { pages: Some(vec![]), ..Default::default() }), Err(PdfError::NoPages)));
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
}

#[test]
fn text_layer_and_print_boxes() {
    use genha_doc::{TextEffect, TextEl, TextStyle};
    let mut d = Document::new("Flyer", 400.0, 300.0);
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(
        id,
        Frame::new(20.0, 20.0, 360.0, 60.0),
        ElementKind::Text(TextEl {
            text: "Hello (world) café".into(),
            style: TextStyle { font_size: 20.0, ..TextStyle::default() },
            effect: TextEffect::None,
        }),
    ));
    let fonts = genha_text::FontBook::system();
    let pdf = export_pdf(&d, &fonts, &PdfOptions { dpi: 40.0, ..PdfOptions::default() }).unwrap();
    assert!(find(&pdf, b"/BaseFont /Helvetica").is_some());
    assert!(find(&pdf, b"3 Tr").is_some(), "invisible text");
    assert!(find(&pdf, b"(Hello \\(world\\) caf\\351)").is_some(), "escaped WinAnsi text");
    assert!(find(&pdf, b"/TrimBox").is_none());
    check_xref(&pdf);

    let print = export_pdf(&d, &fonts, &PdfOptions { dpi: 40.0, ..PdfOptions::print() }).unwrap();
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
    let id = d.alloc_id();
    d.pages[0].elements.push(Element::new(
        id,
        Frame::new(0.0, 50.0, 100.0, 50.0),
        ElementKind::Shape(ShapeEl { shape: ShapeKind::Rect { corner_radius: 0.0 }, fill: Fill::solid(Color::BLACK), stroke: None, shadow: None }),
    ));
    let p = with_bleed(&d.pages[0], 10.0);
    assert_eq!((p.width, p.height), (120.0, 120.0));
    let f = p.elements[0].frame;
    assert_eq!((f.x, f.y, f.width, f.height), (0.0, 60.0, 120.0, 60.0));
}
