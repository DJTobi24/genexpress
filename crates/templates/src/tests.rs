use std::collections::HashSet;

use genha_color::Color;
use genha_doc::{BrandColor, Document, Element, ElementKind, Fill, ShapeKind, TextEffect, preset};

use super::*;
use crate::designs::CATEGORIES;

/// Average advance of a sans character, in ems (a deliberately rough width estimate).
const EM_PER_CHAR: f64 = 0.55;

fn all_docs() -> Vec<(TemplateInfo, Document)> {
    catalog().into_iter().map(|t| (t.clone(), instantiate(t.id, None).unwrap_or_else(|| panic!("{} instantiates", t.id)))).collect()
}

fn fill_colors(f: &Fill, out: &mut Vec<Color>) {
    match f {
        Fill::None => {}
        Fill::Solid { color } => out.push(*color),
        Fill::Linear { stops, .. } | Fill::Radial { stops } => out.extend(stops.iter().map(|s| s.color)),
    }
}

/// Every colour and font family used in `doc`.
fn colors_and_fonts(doc: &Document) -> (Vec<Color>, HashSet<String>) {
    let mut colors = vec![];
    let mut fonts = HashSet::new();
    for p in &doc.pages {
        fill_colors(&p.background, &mut colors);
        p.for_each(&mut |e: &Element| match &e.kind {
            ElementKind::Shape(s) => {
                fill_colors(&s.fill, &mut colors);
                if let Some(st) = &s.stroke {
                    colors.push(st.color);
                }
            }
            ElementKind::Text(t) => {
                colors.push(t.style.color);
                fonts.insert(t.style.font_family.clone());
                for sp in &t.spans {
                    colors.extend(sp.style.color);
                    fonts.extend(sp.style.font_family.clone());
                }
            }
            ElementKind::Chart(c) => {
                colors.extend(c.colors.iter().copied());
                colors.extend(c.series.iter().filter_map(|s| s.color));
                colors.push(c.text_color);
                fonts.insert(c.font_family.clone());
            }
            ElementKind::Table(t) => {
                colors.extend([t.text_color, t.header_fill, t.header_text, t.row_fill]);
                colors.extend(t.alt_row_fill);
                colors.extend(t.cell_styles.iter().flat_map(|c| [c.style.fill, c.style.text_color]).flatten());
                fonts.insert(t.font_family.clone());
            }
            _ => {}
        });
    }
    (colors, fonts)
}

#[test]
fn catalog_is_complete() {
    let cat = catalog();
    assert!(cat.len() >= 110, "{} templates", cat.len());
    // Six categories (the UI translates these names), each with a good number of designs.
    let categories = CATEGORIES;
    for c in categories {
        let n = cat.iter().filter(|t| t.category == c).count();
        assert!(n >= 8, "only {n} templates in {c}");
    }
    let mut ids = HashSet::new();
    for t in &cat {
        assert!(ids.insert(t.id), "duplicate template {}", t.id);
        assert!(!t.name.is_empty() && !t.category.is_empty() && t.tags.len() >= 3, "{}: name, category and tags", t.id);
        assert!(categories.contains(&t.category), "{}: unknown category {}", t.id, t.category);
        assert!(t.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "{}: id", t.id);
        let unique: HashSet<_> = t.tags.iter().collect();
        assert_eq!(unique.len(), t.tags.len(), "{}: repeated tag", t.id);
        for tag in t.tags {
            assert_eq!(*tag, tag.to_lowercase(), "{}: tags are matched in lower case", t.id);
        }
        let p = preset::find(t.preset).unwrap_or_else(|| panic!("{}: unknown preset {}", t.id, t.preset));
        assert!((t.width, t.height) == (p.width, p.height) || (t.width, t.height) == (p.height, p.width), "{}: size does not match its preset", t.id);
    }
    for wanted in [
        "instagram-post",
        "instagram-portrait",
        "instagram-story",
        "tiktok-video",
        "youtube-thumbnail",
        "youtube-banner",
        "x-header",
        "facebook-post",
        "facebook-cover",
        "linkedin-post",
        "linkedin-banner",
        "pinterest-pin",
        "flyer-a4",
        "flyer-a5",
        "flyer-letter",
        "poster-a3",
        "poster-18x24",
        "business-card",
        "business-card-us",
        "invitation-5x7",
        "postcard",
        "logo",
        "banner-leaderboard",
        "banner-rectangle",
        "email-header",
        "presentation",
        "presentation-4x3",
        "resume",
        "x-post",
        "facebook-story",
        "video-landscape",
        "video-portrait",
        "video-square",
        "document-a4",
        "webpage",
    ] {
        assert!(cat.iter().any(|t| t.preset == wanted), "no template for {wanted}");
    }
    assert!(instantiate("no-such-template", None).is_none());
    // Listed category by category.
    let order: Vec<usize> = cat.iter().filter_map(|t| categories.iter().position(|c| *c == t.category)).collect();
    assert!(order.windows(2).all(|w| w[0] <= w[1]), "categories are contiguous");
}

#[test]
fn templates_are_well_formed() {
    for (info, doc) in all_docs() {
        let id = info.id;
        assert!(!doc.pages.is_empty());
        let mut seen = HashSet::new();
        for p in &doc.pages {
            assert_eq!((p.width, p.height), (info.width, info.height), "{id}: page size");
            assert!(seen.insert(p.id), "{id}: duplicate page id");
            assert!(!p.elements.is_empty(), "{id}: empty page");
            let page = genha_geom::Rect::new(-0.5, -0.5, p.width + 0.5, p.height + 0.5);
            p.for_each(&mut |e| {
                assert!(seen.insert(e.id), "{id}: duplicate id {}", e.id);
                assert!(!e.name.is_empty(), "{id}: element {} has no name", e.id);
                assert!(e.frame.is_valid());
                let b = e.frame.bounds();
                assert!(b.x0 >= page.x0 && b.y0 >= page.y0 && b.x1 <= page.x1 && b.y1 <= page.y1, "{id}: {} is off the page: {b:?}", e.name);
                if let ElementKind::Text(t) = &e.kind {
                    let s = &t.style;
                    let spacing = 1.0 + (s.letter_spacing / 1000.0).max(0.0);
                    for line in t.text.lines() {
                        let need = line.chars().count() as f64 * EM_PER_CHAR * s.font_size * spacing;
                        assert!(need <= e.frame.width + 0.5, "{id}: `{line}` needs {need:.0} px but {} has {:.0}", e.name, e.frame.width);
                    }
                    let lines = t.text.lines().count() as f64;
                    assert!(lines * s.font_size * s.line_height <= e.frame.height + 0.5, "{id}: {} is too short", e.name);
                }
                if let ElementKind::Group(g) = &e.kind {
                    for c in &g.children {
                        let (cb, gb) = (c.frame.bounds(), e.frame.bounds());
                        assert!(cb.x0 >= gb.x0 - 1e-6 && cb.y0 >= gb.y0 - 1e-6 && cb.x1 <= gb.x1 + 1e-6 && cb.y1 <= gb.y1 + 1e-6);
                    }
                }
            });
        }
        assert!(seen.iter().all(|i| i.0 < doc.next_id), "{id}: next_id is past every id");
        let json = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc, "{id}: JSON round trip");
        let mut repaired = doc.clone();
        let notes = repaired.repair();
        assert!(notes.is_empty(), "{id}: nothing to repair: {notes:?}");
    }
}

#[test]
fn presentation_has_two_slides_and_templates_look_different() {
    for (id, pages) in [("team-presentation", 2), ("pitch-deck", 5), ("annual-report-deck", 4)] {
        let deck = instantiate(id, None).unwrap();
        assert_eq!(deck.pages.len(), pages, "{id}");
        assert!(deck.pages.iter().all(|p| !p.notes.is_empty()), "{id}: every slide has notes");
    }
    assert_eq!(instantiate("garden-carousel", None).unwrap().pages.len(), 5);
    assert_eq!(instantiate("business-card-bold", None).unwrap().pages.len(), 2);
    // Every page of a multi-page template has its own name.
    for (info, doc) in all_docs() {
        let names: HashSet<&str> = doc.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), doc.pages.len(), "{}: page names", info.id);
    }
    // Every template has its own primary colour scheme.
    let primaries: HashSet<String> = all_docs()
        .iter()
        .map(|(_, d)| {
            let (c, _) = colors_and_fonts(d);
            c.iter().map(Color::to_hex).collect::<Vec<_>>().join(",")
        })
        .collect();
    assert_eq!(primaries.len(), catalog().len());
}

#[test]
fn brand_kits_recolour_templates() {
    let brand = BrandKit {
        name: "Test brand".into(),
        colors: vec![
            BrandColor { name: "Primary".into(), color: Color::rgb(0x12, 0x34, 0x56) },
            BrandColor { name: "Secondary".into(), color: Color::rgb(0xab, 0xcd, 0x01) },
            BrandColor { name: "Dark".into(), color: Color::rgb(0x0a, 0x0b, 0x0c) },
            BrandColor { name: "Light".into(), color: Color::rgb(0xfe, 0xfd, 0xf0) },
        ],
        heading_font: "Brand Heading".into(),
        body_font: "Brand Body".into(),
        logos: vec![],
    };
    for (info, plain) in all_docs() {
        let branded = instantiate(info.id, Some(&brand)).unwrap();
        assert_eq!(branded.brand, brand);
        assert_eq!(branded.pages[0].ids(), plain.pages[0].ids(), "{}: same structure", info.id);
        let (colors, fonts) = colors_and_fonts(&branded);
        assert!(colors.contains(&brand.colors[0].color), "{}: primary used", info.id);
        assert!(colors.contains(&brand.colors[1].color) || colors.contains(&brand.colors[1].color.with_alpha(210)), "{}: secondary used", info.id);
        assert!(fonts.iter().all(|f| f == "Brand Heading" || f == "Brand Body"), "{}: fonts {fonts:?}", info.id);
        assert!(fonts.contains("Brand Heading"), "{}", info.id);
        // None of the template's own primary colours survive.
        let (plain_colors, _) = colors_and_fonts(&plain);
        let plain_primary = plain_colors.iter().find(|c| !colors.contains(c) && c.a == 255).copied();
        assert!(plain_primary.is_some(), "{}: branding changed nothing", info.id);
    }
    // A partial kit keeps the template's own colours for missing roles.
    let partial = BrandKit { colors: vec![BrandColor { name: "P".into(), color: Color::rgb(1, 2, 3) }], heading_font: String::new(), ..brand };
    let doc = instantiate("quote-card", Some(&partial)).unwrap();
    let (colors, fonts) = colors_and_fonts(&doc);
    assert!(colors.contains(&Color::rgb(1, 2, 3)));
    assert!(fonts.contains("Serif"), "an empty heading font keeps the template's");
}

#[test]
fn shape_library_paths_are_valid() {
    let lib = shape_library();
    let mut ids = HashSet::new();
    let mut icons = 0;
    for s in &lib {
        assert!(ids.insert(s.id), "duplicate shape {}", s.id);
        assert!(!s.label.is_empty() && !s.category.is_empty());
        if let ShapeKind::Path { d, view_width, view_height } = &s.shape {
            icons += 1;
            assert_eq!((*view_width, *view_height), (100.0, 100.0));
            let path = kurbo::BezPath::from_svg(d).unwrap_or_else(|e| panic!("{}: {e}", s.id));
            let b = kurbo::Shape::bounding_box(&path);
            assert!(b.x0 >= -1e-6 && b.y0 >= -1e-6 && b.x1 <= 100.0 + 1e-6 && b.y1 <= 100.0 + 1e-6, "{} leaves the view box: {b:?}", s.id);
            assert!(b.width() > 40.0 && b.height() > 20.0, "{} is tiny: {b:?}", s.id);
        }
    }
    assert!(icons >= 70, "{icons} icons");
    for wanted in ["rect", "rounded-rect", "circle", "triangle", "pentagon", "hexagon", "star", "line", "heart", "arrow-right", "check", "cross"] {
        assert!(ids.contains(wanted), "missing {wanted}");
    }
    for wanted in ["speech-bubble", "location-pin", "lightning", "cloud", "sun", "moon", "music-note", "play-button", "envelope", "phone", "home"] {
        assert!(ids.contains(wanted), "missing {wanted}");
    }
    for wanted in ["bell", "badge", "seal", "plus", "minus", "sparkle", "arrow-left", "arrow-up", "arrow-down"] {
        assert!(ids.contains(wanted), "missing {wanted}");
    }
}

#[test]
fn icons_fill_the_same_with_either_fill_rule() {
    // Parts never overlap and holes run against their outline: every point is covered at most
    // once, always with the same sign, so non-zero and even-odd filling agree.
    let lib = shape_library();
    let mut cats: Vec<&str> = vec![];
    for s in &lib {
        let ShapeKind::Path { d, .. } = &s.shape else { continue };
        if !cats.contains(&s.category) {
            cats.push(s.category);
        }
        let path = kurbo::BezPath::from_svg(d).unwrap();
        let mut sign = 0;
        let mut covered = 0;
        for iy in 0..60 {
            for ix in 0..60 {
                let p = kurbo::Point::new(0.4137 + f64::from(ix) * 99.2 / 60.0, 0.2913 + f64::from(iy) * 99.2 / 60.0);
                let w = kurbo::Shape::winding(&path, p);
                assert!(w.abs() <= 1, "{}: winding {w} at {p:?}", s.id);
                if w != 0 {
                    covered += 1;
                    assert!(sign == 0 || sign == w, "{}: a hole runs the wrong way at {p:?}", s.id);
                    sign = w;
                }
            }
        }
        assert!(covered > 200, "{}: covers only {covered} samples", s.id);
    }
    assert!(cats.len() >= 8, "icon categories: {cats:?}");
    // Categories are contiguous, so the panel shows each once.
    let mut seen: Vec<&str> = vec![];
    for s in &lib {
        if seen.last() != Some(&s.category) {
            assert!(!seen.contains(&s.category), "{} is split", s.category);
            seen.push(s.category);
        }
    }
}

/// Signed area of each closed subpath (positive = clockwise on screen).
fn subpath_areas(d: &str) -> Vec<f64> {
    let path = kurbo::BezPath::from_svg(d).unwrap();
    let mut areas = vec![];
    let mut current = kurbo::BezPath::new();
    for el in path.elements() {
        if let kurbo::PathEl::MoveTo(_) = el
            && !current.elements().is_empty()
        {
            areas.push(kurbo::Shape::area(&current));
            current = kurbo::BezPath::new();
        }
        current.push(*el);
    }
    if !current.elements().is_empty() {
        areas.push(kurbo::Shape::area(&current));
    }
    areas
}

#[test]
fn icon_holes_run_against_their_outline() {
    for id in ["play-button", "location-pin", "envelope"] {
        let Some(ShapeKind::Path { d, .. }) = shape_library().into_iter().find(|s| s.id == id).map(|s| s.shape) else { panic!("{id}") };
        let areas = subpath_areas(&d);
        assert_eq!(areas.len(), 2, "{id}");
        assert!(areas[0].signum() != areas[1].signum(), "{id}: {areas:?}");
        assert!(areas[0].abs() > areas[1].abs(), "{id}: the hole is smaller");
    }
}

#[test]
fn text_presets_cover_the_basics() {
    let presets = text_presets();
    let ids: HashSet<&str> = presets.iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), presets.len());
    for wanted in ["heading", "subheading", "body", "caption", "quote", "label", "big-number", "outlined-title", "highlighted-title", "shadow-title"]
    {
        assert!(ids.contains(wanted), "missing {wanted}");
    }
    assert!(presets.iter().all(|p| !p.sample.is_empty() && p.style.font_size > 0.0));
    assert!(presets.len() >= 25, "{} text presets", presets.len());
}

#[test]
fn palettes_are_distinct() {
    let ps = palettes();
    assert!(ps.len() >= 40, "{} palettes", ps.len());
    let mut ids = HashSet::new();
    let mut sets = HashSet::new();
    for p in &ps {
        assert!(ids.insert(p.id));
        assert_eq!(p.colors.len(), 5, "{}", p.id);
        let unique: HashSet<_> = p.colors.iter().collect();
        assert_eq!(unique.len(), 5, "{}", p.id);
        assert!(sets.insert(p.colors.iter().map(Color::to_hex).collect::<Vec<_>>()));
        // The light colour reads under the dark one.
        assert!(genha_color::contrast_ratio(p.colors[3], p.colors[4]) >= 7.0, "{}", p.id);
    }
}

#[test]
fn qr_codes() {
    let shape = qr_code("https://example.com", Color::BLACK).unwrap();
    let ShapeKind::Path { d, view_width, view_height } = shape else { panic!() };
    // 19 bytes at level M need version 2 (25 modules) plus a 4-module quiet zone per side.
    assert_eq!((view_width, view_height), (33.0, 33.0));
    let path = kurbo::BezPath::from_svg(&d).unwrap();
    let b = kurbo::Shape::bounding_box(&path);
    assert_eq!((b.x0, b.y0, b.x1, b.y1), (4.0, 4.0, 29.0, 29.0));
    // The top-left finder pattern starts with a run of 7 dark modules.
    assert!(d.starts_with("M4 4h7v1h-7z"), "{}", &d[..40]);
    assert_eq!(qr_code("", Color::BLACK), Err(TemplateError::Empty));
    assert_eq!(qr_code(&"x".repeat(MAX_QR_BYTES + 1), Color::BLACK), Err(TemplateError::TooLong(MAX_QR_BYTES + 1)));
    assert_eq!(qr_code("hi", Color::rgb(250, 240, 120)), Err(TemplateError::LowContrast));
    assert!(qr_code("hi", Color::rgb(30, 60, 150)).is_ok());
}

/// Every text element (in groups too) with the page it is on.
fn each_element(doc: &Document, mut f: impl FnMut(usize, &Element)) {
    for (i, p) in doc.pages.iter().enumerate() {
        p.for_each(&mut |e: &Element| f(i, e));
    }
}

#[test]
fn text_fits_its_box_with_and_without_fonts() {
    let system = genha_text::FontBook::system();
    let books = [("no fonts", genha_text::FontBook::new()), ("system fonts", system)];
    let mut problems = vec![];
    for (info, doc) in all_docs() {
        for (book_name, book) in &books {
            each_element(&doc, |page, e| {
                let f = e.frame;
                match &e.kind {
                    ElementKind::Text(t) if t.curve_degrees().abs() >= 0.5 => {
                        // Bent text is laid out in lines first: it must not wrap either.
                        let lines = genha_text::layout_el(book, t, f.width).lines.len();
                        if lines > t.text.lines().count().max(1) {
                            problems.push(format!("{} p{page} {} ({book_name}): curved text wraps into {lines} lines", info.id, e.name));
                        }
                        if let Some((w, h)) = genha_render::curved_text_size(book, t, f.width)
                            && (w > f.width + 2.0 || h > f.height + 2.0)
                        {
                            problems.push(format!(
                                "{} p{page} {} ({book_name}): curved text needs {w:.0}×{h:.0}, has {:.0}×{:.0}",
                                info.id, e.name, f.width, f.height
                            ));
                        }
                    }
                    ElementKind::Text(t) => {
                        let l = genha_text::layout_el(book, t, f.width);
                        let lines = t.text.lines().count().max(1);
                        if l.lines.len() > lines || l.height > f.height + 1.0 {
                            problems.push(format!(
                                "{} p{page} {} ({book_name}): {} lines, {:.0} px tall in a {:.0} × {:.0} box",
                                info.id,
                                e.name,
                                l.lines.len(),
                                l.height,
                                f.width,
                                f.height
                            ));
                        }
                        for line in &l.lines {
                            if line.width > f.width + 1.0 {
                                problems.push(format!(
                                    "{} p{page} {} ({book_name}): `{}` is {:.0} px wide in {:.0}",
                                    info.id, e.name, line.text, line.width, f.width
                                ));
                            }
                        }
                    }
                    ElementKind::Table(t) => {
                        let g = genha_render::table_geometry(book, t, f.width, f.height);
                        if g.natural_height > f.height + 1.0 {
                            problems.push(format!(
                                "{} p{page} {} ({book_name}): table needs {:.0} px, has {:.0}",
                                info.id, e.name, g.natural_height, f.height
                            ));
                        }
                    }
                    _ => {}
                }
            });
        }
    }
    assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.join("\n"));
}

/// The colours behind the middle of `frame`: the topmost opaque shape drawn before element `upto`
/// that covers it (a rectangle, an ellipse, or the middle of any other shape), else the page
/// background (every stop of a gradient).
fn backdrop(page: &genha_doc::Page, upto: usize, frame: genha_geom::Frame) -> Vec<Color> {
    let b = frame.bounds();
    let (cx, cy) = ((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0);
    let probe = [(cx - b.width() * 0.25, cy), (cx + b.width() * 0.25, cy), (cx, cy - b.height() * 0.25), (cx, cy + b.height() * 0.25)];
    for e in page.elements.iter().take(upto).rev() {
        let ElementKind::Shape(s) = &e.kind else { continue };
        if e.opacity < 1.0 || e.frame.rotation.abs() > 20.0 {
            continue;
        }
        let r = e.frame.bounds();
        let (ex, ey, rx, ry) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0, r.width() / 2.0, r.height() / 2.0);
        let inside = |&(x, y): &(f64, f64)| match s.shape {
            ShapeKind::Rect { .. } => x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1,
            ShapeKind::Ellipse => rx > 0.0 && ry > 0.0 && ((x - ex) / rx).powi(2) + ((y - ey) / ry).powi(2) <= 1.0,
            ShapeKind::Line => false,
            // Path data: the point is inside the outline.
            ShapeKind::Path { ref d, view_width, view_height } => kurbo::BezPath::from_svg(d).is_ok_and(|p| {
                let v = kurbo::Point::new((x - r.x0) / r.width() * view_width, (y - r.y0) / r.height() * view_height);
                kurbo::Shape::winding(&p, v) != 0
            }),
            // Polygons and stars: their middle.
            _ => (x - ex).abs() <= rx * 0.6 && (y - ey).abs() <= ry * 0.6,
        };
        if !probe.iter().all(inside) {
            continue;
        }
        let mut c = vec![];
        fill_colors(&s.fill, &mut c);
        if !c.is_empty() && c.iter().all(|c| c.a == 255) {
            return c;
        }
    }
    let mut c = vec![];
    fill_colors(&page.background, &mut c);
    if c.is_empty() { vec![Color::WHITE] } else { c }
}

#[test]
fn text_contrasts_with_what_is_behind_it() {
    let mut problems = vec![];
    for (info, doc) in all_docs() {
        for (pi, p) in doc.pages.iter().enumerate() {
            for (i, e) in p.elements.iter().enumerate() {
                let ElementKind::Text(t) = &e.kind else { continue };
                // Skip see-through and decorated text, and decorative marks (no letters or digits).
                if t.style.color.a < 255 || !matches!(t.effect, TextEffect::None) || !t.text.chars().any(char::is_alphanumeric) {
                    continue;
                }
                // Body-sized text (small for its page) needs AA contrast (4.5:1), large text 3:1;
                // bold text counts as large from a smaller size, as in WCAG.
                let large_from = p.width.min(p.height) * if t.style.bold { 0.027 } else { 0.035 };
                let small = t.style.font_size < large_from;
                let need = if small { 4.5 } else { 3.0 };
                let inks = std::iter::once(t.style.color).chain(t.spans.iter().filter_map(|s| s.style.color));
                for ink in inks {
                    for bg in backdrop(p, i, e.frame) {
                        let ratio = genha_color::contrast_ratio(ink, bg);
                        if ratio + 0.05 < need {
                            problems.push(format!(
                                "{} p{pi} {}: {} on {} has contrast {ratio:.2} < {need}",
                                info.id,
                                e.name,
                                ink.to_hex(),
                                bg.to_hex()
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn every_template_renders_and_exports() {
    let fonts = genha_text::FontBook::system();
    let none = genha_text::FontBook::new();
    for info in catalog() {
        let doc = instantiate(info.id, None).unwrap();
        for i in 0..doc.pages.len() {
            let r = genha_render::render_thumbnail(&doc, i, &fonts, 160).unwrap_or_else(|e| panic!("{}: {e}", info.id));
            assert!(r.width > 0 && r.height > 0);
            genha_render::encode_png(&r).unwrap();
            let svg = genha_svg::export_page(&doc, i, &fonts, &genha_svg::SvgOptions::default()).unwrap_or_else(|e| panic!("{}: {e}", info.id));
            assert!(svg.contains("<svg"), "{}", info.id);
        }
        let pdf = genha_pdf::export_pdf(&doc, &none, &genha_pdf::PdfOptions::default()).unwrap_or_else(|e| panic!("{}: {e}", info.id));
        assert!(pdf.starts_with(b"%PDF"), "{}", info.id);
    }
}

#[test]
fn new_document_features_are_used() {
    let docs = all_docs();
    let count = |f: &dyn Fn(&Element) -> bool| {
        docs.iter()
            .filter(|(_, d)| {
                let mut any = false;
                each_element(d, |_, e| any |= f(e));
                any
            })
            .count()
    };
    let spans = count(&|e| matches!(&e.kind, ElementKind::Text(t) if !t.spans.is_empty()));
    let curved = count(&|e| matches!(&e.kind, ElementKind::Text(t) if t.curve != 0.0));
    let multi_series = count(&|e| matches!(&e.kind, ElementKind::Chart(c) if c.series.len() > 1));
    let merged = count(&|e| matches!(&e.kind, ElementKind::Table(t) if !t.merges.is_empty() && !t.cell_styles.is_empty()));
    let animated = count(&|e| e.animation.is_some());
    assert!(spans >= 10 && curved >= 4 && multi_series >= 3 && merged >= 3 && animated >= 4, "{spans} {curved} {multi_series} {merged} {animated}");
    for (info, doc) in &docs {
        let mut anim = false;
        each_element(doc, |_, e| anim |= e.animation.is_some());
        if anim {
            assert!(doc.pages.iter().all(|p| p.duration.is_some_and(|d| d > 0.5 && d <= 15.0)), "{}: animated pages have a duration", info.id);
            assert!(info.tags.contains(&"animated"), "{}: tagged animated", info.id);
        }
        // Every slide of every deck has speaker notes.
        if info.preset.starts_with("presentation") {
            assert!(doc.pages.iter().all(|p| !p.notes.trim().is_empty()), "{}: speaker notes", info.id);
        }
    }
}
