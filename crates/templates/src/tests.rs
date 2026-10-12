use std::collections::HashSet;

use genha_color::Color;
use genha_doc::{BrandColor, Document, Element, ElementKind, Fill, ShapeKind, preset};

use super::*;

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
            }
            _ => {}
        });
    }
    (colors, fonts)
}

#[test]
fn catalog_is_complete() {
    let cat = catalog();
    assert!(cat.len() >= 16, "{} templates", cat.len());
    let mut ids = HashSet::new();
    for t in &cat {
        assert!(ids.insert(t.id), "duplicate template {}", t.id);
        assert!(!t.name.is_empty() && !t.category.is_empty() && !t.tags.is_empty());
        let p = preset::find(t.preset).unwrap_or_else(|| panic!("{}: unknown preset {}", t.id, t.preset));
        assert!((t.width, t.height) == (p.width, p.height) || (t.width, t.height) == (p.height, p.width), "{}: size does not match its preset", t.id);
    }
    for wanted in
        ["instagram-post", "instagram-story", "youtube-thumbnail", "facebook-post", "linkedin-banner", "flyer-a4", "poster-a3", "business-card"]
    {
        assert!(cat.iter().any(|t| t.preset == wanted), "no template for {wanted}");
    }
    assert!(instantiate("no-such-template", None).is_none());
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
        assert!(repaired.repair().is_empty(), "{id}: nothing to repair");
    }
}

#[test]
fn presentation_has_two_slides_and_templates_look_different() {
    let deck = instantiate("team-presentation", None).unwrap();
    assert_eq!(deck.pages.len(), 2);
    assert!(deck.pages.iter().all(|p| !p.notes.is_empty()));
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
    assert!(icons >= 20, "{icons} icons");
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
}

#[test]
fn palettes_are_distinct() {
    let ps = palettes();
    assert!(ps.len() >= 16);
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
