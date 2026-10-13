#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Every page of every built-in template, smart-resized to a story, a square post, a landscape
//! slide and an A4 page, with the system fonts and with none: no text overflows its box, leaves
//! the page or overlaps other text.

use genha_doc::{Element, ElementKind, Page, preset};
use genha_geom::Rect;
use genha_layout::smart_resize;
use genha_text::FontBook;

fn targets() -> Vec<(f64, f64, &'static str)> {
    let a4 = preset::find("flyer-a4").map_or((793.7, 1122.5), |p| (p.width, p.height));
    vec![(1080.0, 1920.0, "story"), (1080.0, 1080.0, "square"), (1920.0, 1080.0, "landscape"), (a4.0, a4.1, "A4")]
}

/// Visible text elements, groups' children included.
fn texts<'a>(els: &'a [Element], out: &mut Vec<&'a Element>) {
    for e in els {
        if e.hidden {
            continue;
        }
        match &e.kind {
            ElementKind::Text(_) => out.push(e),
            ElementKind::Group(g) => texts(&g.children, out),
            _ => {}
        }
    }
}

fn problems(page: &Page, fonts: &FontBook) -> Vec<String> {
    let mut out = vec![];
    let mut ts = vec![];
    texts(&page.elements, &mut ts);
    let area = Rect::new(-0.5, -0.5, page.width + 0.5, page.height + 0.5);
    for e in &ts {
        let b = e.frame.bounds();
        if b.x0 < area.x0 || b.y0 < area.y0 || b.x1 > area.x1 || b.y1 > area.y1 {
            out.push(format!("{:?} leaves the page: {b:?}", e.name));
        }
        let ElementKind::Text(t) = &e.kind else { continue };
        if e.frame.rotation == 0.0 && t.curve_degrees() == 0.0 {
            // As drawn: text that shrinks to fit at the scale it is drawn at.
            let t = genha_text::fitted(fonts, t, e.frame.width, e.frame.height);
            let (w, h) = genha_text::measure_wrapped_el(fonts, &t, e.frame.width);
            if h > e.frame.height + 0.5 || w > e.frame.width + 0.5 {
                out.push(format!("{:?} overflows its box: needs {w:.1} × {h:.1}, has {:.1} × {:.1}", e.name, e.frame.width, e.frame.height));
            }
        }
    }
    for (i, a) in ts.iter().enumerate() {
        for b in ts.iter().skip(i + 1) {
            let r = a.frame.bounds().intersect(b.frame.bounds());
            if r.width() > 0.5 && r.height() > 0.5 {
                out.push(format!("{:?} and {:?} overlap", a.name, b.name));
            }
        }
    }
    out
}

fn run(fonts: &FontBook, label: &str) {
    let mut failures = vec![];
    let mut pages = 0;
    for info in genha_templates::catalog() {
        let doc = genha_templates::instantiate(info.id, None).expect(info.id);
        for (pi, page) in doc.pages.iter().enumerate() {
            for (w, h, target) in targets() {
                let r = smart_resize(page, w, h, fonts);
                assert_eq!((r.page.width, r.page.height), (w, h));
                assert_eq!(r.page.ids(), page.ids(), "{}: ids kept", info.id);
                pages += 1;
                for p in problems(&r.page, fonts) {
                    failures.push(format!("{} page {} → {target} ({label}): {p}", info.id, pi + 1));
                }
            }
        }
    }
    assert!(pages > 0);
    assert!(failures.is_empty(), "{} problems in {pages} resized pages:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn every_template_resizes_cleanly_with_system_fonts() {
    run(&FontBook::system(), "system fonts");
}

#[test]
fn every_template_resizes_cleanly_without_fonts() {
    run(&FontBook::new(), "no fonts");
}

/// With only DejaVu Sans (much wider than Inter, the templates' design font), text that shrinks to
/// fit its box keeps fitting in every new size.
#[test]
fn every_template_resizes_cleanly_with_a_wide_font() {
    let mut book = FontBook::new();
    for dir in ["/usr/share/fonts/truetype/dejavu", "/usr/share/fonts/dejavu", "/usr/share/fonts/TTF"] {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for f in rd.flatten() {
            let name = f.file_name().to_string_lossy().to_string();
            if name.starts_with("DejaVu") && name.ends_with(".ttf") && !name.contains("Mono") && !name.contains("Condensed") {
                let _ = book.add_font(std::fs::read(f.path()).unwrap());
            }
        }
    }
    if !book.has_family("DejaVu Sans") {
        eprintln!("DejaVu Sans not installed: skipped");
        return;
    }
    run(&book, "DejaVu only");
}
