use super::model::{El, Node, region};
use super::*;
use genha_color::Color;
use genha_doc::{Fill, GroupEl, NodeId, ShapeEl, TextAlign, TextEl, TextStyle};
use genha_geom::Point;

fn shape(id: u64, f: Frame, kind: ShapeKind) -> Element {
    Element::new(NodeId(id), f, ElementKind::Shape(ShapeEl { shape: kind, fill: Fill::solid(Color::BLACK), stroke: None, shadow: None }))
}

fn rect(id: u64, x: f64, y: f64, w: f64, h: f64) -> Element {
    shape(id, Frame::new(x, y, w, h), ShapeKind::Rect { corner_radius: 10.0 })
}

fn dot(id: u64, x: f64, y: f64, w: f64, h: f64) -> Element {
    shape(id, Frame::new(x, y, w, h), ShapeKind::Ellipse)
}

fn text(id: u64, x: f64, y: f64, w: f64, s: &str, size: f64, align: TextAlign) -> Element {
    let style = TextStyle { font_size: size, align, line_height: 1.2, ..TextStyle::default() };
    let h = (s.lines().count().max(1) as f64 * size * 1.2).ceil();
    Element::new(NodeId(id), Frame::new(x, y, w, h), ElementKind::Text(TextEl::new(s, style)))
}

fn page(w: f64, h: f64, els: Vec<Element>) -> Page {
    let mut p = Page::new(NodeId(1), "P", w, h);
    p.elements = els;
    p
}

fn none() -> FontBook {
    FontBook::new()
}

/// The structure found on a page.
fn tree(p: &Page) -> (Vec<El>, model::Region) {
    let fonts = none();
    let m = measure::Measurer::new(&fonts);
    let els = model::elements(p, &m);
    let ids: Vec<usize> = (0..els.len()).collect();
    let r = region(&els, &ids, Rect::new(0.0, 0.0, p.width, p.height), 0);
    (els, r)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn frame(p: &Page, i: usize) -> Frame {
    p.elements[i].frame
}

fn size(p: &Page, i: usize) -> f64 {
    match &p.elements[i].kind {
        ElementKind::Text(t) => t.style.font_size,
        _ => panic!("not text"),
    }
}

fn overlaps(a: Frame, b: Frame) -> bool {
    let i = a.bounds().intersect(b.bounds());
    i.width() > 0.5 && i.height() > 0.5
}

// ---------------------------------------------------------------- detection

#[test]
fn detects_backgrounds_bands_cards_and_decorations() {
    let p = page(
        1000.0,
        1000.0,
        vec![
            rect(2, 0.0, 0.0, 1000.0, 1000.0),                                  // 0 background
            rect(3, 0.0, 800.0, 1000.0, 200.0),                                 // 1 footer band…
            text(4, 100.0, 860.0, 800.0, "Visit us", 40.0, TextAlign::Center),  // 2 …holding text
            text(5, 100.0, 100.0, 800.0, "Big title", 80.0, TextAlign::Center), // 3
            dot(6, 900.0, 350.0, 30.0, 30.0),                                   // 4 a sparkle far from text
            rect(7, 0.0, 0.0, 1000.0, 20.0),                                    // 5 an empty band at the top
        ],
    );
    let (_, r) = tree(&p);
    assert_eq!(r.fills, vec![0, 5]);
    assert_eq!(r.decor, vec![vec![4]]);
    let Some(Node::Stack(v)) = &r.flow else { panic!("{:?}", r.flow) };
    assert_eq!(v[0], Node::Text(3));
    let Node::Section(s) = &v[1] else { panic!("{:?}", v[1]) };
    assert_eq!(s.root, 1);
    assert!(s.span_x && !s.span_y);
    assert_eq!(s.inner.flow, Some(Node::Text(2)));
}

#[test]
fn buttons_are_units_and_icons_join_their_labels() {
    let p = page(
        1000.0,
        1000.0,
        vec![
            rect(2, 300.0, 800.0, 400.0, 100.0),                                  // 0 pill…
            text(3, 300.0, 826.0, 400.0, "Shop now", 40.0, TextAlign::Center),    // 1 …and its label
            dot(4, 100.0, 300.0, 60.0, 60.0),                                     // 2 an icon…
            text(5, 180.0, 306.0, 500.0, "Free delivery", 40.0, TextAlign::Left), // 3 …next to its label
        ],
    );
    let (_, r) = tree(&p);
    let Some(Node::Stack(v)) = &r.flow else { panic!("{:?}", r.flow) };
    assert_eq!(v[0], Node::Row(vec![Node::Rigid(vec![2]), Node::Text(3)]));
    assert_eq!(v[1], Node::Rigid(vec![0, 1]));
    assert!(r.decor.is_empty());
}

#[test]
fn boxes_over_labels_become_a_grid_of_columns() {
    // Three boxes with a number each, and a label under each box (a countdown).
    let mut els = vec![];
    for (k, x) in [90.0, 400.0, 710.0].into_iter().enumerate() {
        let id = 10 * (k as u64 + 1);
        els.push(rect(id, x, 300.0, 280.0, 300.0));
        els.push(text(id + 1, x, 350.0, 280.0, "12", 120.0, TextAlign::Center));
        els.push(text(id + 2, x, 630.0, 280.0, "Hours", 34.0, TextAlign::Center));
    }
    let (_, r) = tree(&page(1080.0, 1080.0, els));
    let Some(Node::Grid(items, cols)) = &r.flow else { panic!("{:?}", r.flow) };
    assert_eq!(*cols, 3);
    assert_eq!(items[0], Node::Stack(vec![Node::Rigid(vec![0, 1]), Node::Text(2)]));
}

#[test]
fn repeated_cards_in_a_column_are_a_grid_and_paragraphs_are_not() {
    let mut els = vec![text(1, 90.0, 60.0, 900.0, "Three tips", 80.0, TextAlign::Left)];
    for k in 0..3u64 {
        let y = 250.0 + 250.0 * k as f64;
        els.push(rect(10 + k, 90.0, y, 900.0, 200.0));
        els.push(text(20 + k, 120.0, y + 30.0, 700.0, "Tip title", 48.0, TextAlign::Left));
        els.push(text(30 + k, 120.0, y + 110.0, 700.0, "Tip text", 30.0, TextAlign::Left));
    }
    let (_, r) = tree(&page(1080.0, 1080.0, els));
    let Some(Node::Stack(v)) = &r.flow else { panic!("{:?}", r.flow) };
    assert_eq!(v[0], Node::Text(0));
    let Node::Grid(items, 1) = &v[1] else { panic!("{:?}", v[1]) };
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|i| matches!(i, Node::Section(_))), "wide cards holding text are laid out inside: {items:?}");
    // Stacked paragraphs of the same style keep their order: no grid.
    let p = page(
        1000.0,
        1000.0,
        vec![text(1, 100.0, 100.0, 800.0, "One", 30.0, TextAlign::Left), text(2, 100.0, 200.0, 800.0, "Two", 30.0, TextAlign::Left)],
    );
    assert_eq!(tree(&p).1.flow, Some(Node::Stack(vec![Node::Text(0), Node::Text(1)])));
}

#[test]
fn touching_shapes_are_one_decoration() {
    let p = page(
        1000.0,
        1000.0,
        vec![
            text(1, 100.0, 450.0, 800.0, "Party", 80.0, TextAlign::Center),
            dot(2, 800.0, 60.0, 80.0, 100.0), // balloon
            dot(3, 832.0, 160.0, 16.0, 12.0), // its knot, touching
            rect(4, 839.0, 172.0, 2.0, 60.0), // its string, touching
        ],
    );
    let (_, r) = tree(&p);
    assert_eq!(r.decor, vec![vec![1, 2, 3]]);
}

// ---------------------------------------------------------------- layout

fn banner_page() -> Page {
    let mut els = vec![rect(2, 0.0, 0.0, 1000.0, 1000.0)];
    let mut title = text(3, 200.0, 450.0, 600.0, "Title", 80.0, TextAlign::Center);
    if let ElementKind::Text(t) = &mut title.kind {
        t.effect = TextEffect::Outline { color: Color::WHITE, width: 4.0 };
    }
    els.push(title);
    els.push(rect(4, 860.0, 860.0, 100.0, 100.0)); // bottom-right logo
    els.push(rect(5, 40.0, 40.0, 100.0, 50.0)); // top-left badge
    let children = vec![rect(7, 40.0, 800.0, 50.0, 50.0), rect(8, 100.0, 800.0, 50.0, 50.0)];
    els.push(Element::new(NodeId(6), Frame::new(40.0, 800.0, 110.0, 50.0), ElementKind::Group(GroupEl { children })));
    page(1000.0, 1000.0, els)
}

#[test]
fn square_to_wide_banner() {
    let src = banner_page();
    let r = smart_resize(&src, 2000.0, 500.0, &none());
    let p = &r.page;
    assert_eq!((p.width, p.height), (2000.0, 500.0));
    assert_eq!(p.ids(), src.ids());
    // Background stretched to the new page.
    assert_eq!(frame(p, 0), Frame::new(0.0, 0.0, 2000.0, 500.0));
    // The title stays centred across, its type and outline scaled with the page's shorter side.
    let t = frame(p, 1);
    assert!((t.center().x - 1000.0).abs() < 2.0, "{t:?}");
    assert!(close(size(p, 1), 40.0));
    let ElementKind::Text(tx) = &p.elements[1].kind else { panic!() };
    assert_eq!(tx.effect, TextEffect::Outline { color: Color::WHITE, width: 2.0 });
    // The logo keeps the bottom-right corner, the badge the top-left one.
    let l = frame(p, 2);
    assert!(l.x + l.width > 1900.0 && l.y + l.height > 450.0, "{l:?}");
    let b = frame(p, 3);
    assert!(b.x < 100.0 && b.y < 50.0, "{b:?}");
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

#[test]
fn groups_move_their_children() {
    let p = smart_resize(&banner_page(), 1000.0, 2000.0, &none()).page;
    let g = &p.elements[4];
    let ElementKind::Group(gr) = &g.kind else { panic!() };
    assert!(close(gr.children[0].frame.x, g.frame.x) && close(gr.children[0].frame.y, g.frame.y));
    assert!(close(gr.children[1].frame.x - gr.children[0].frame.x, 60.0 * g.frame.width / 110.0));
}

#[test]
fn everything_stays_inside_and_sizes_are_clamped() {
    for (w, h) in [(1e9, -5.0), (f64::NAN, f64::NAN)] {
        let p = smart_resize(&banner_page(), w, h, &none()).page;
        assert!(p.width >= 1.0 && p.width <= MAX_PAGE_SIDE && p.height >= 1.0 && p.height <= MAX_PAGE_SIDE);
        p.for_each(&mut |e| assert!(e.frame.x.is_finite() && e.frame.width.is_finite() && e.frame.height.is_finite()));
    }
    for (w, h) in [(300.0, 1200.0), (1584.0, 396.0), (f64::NAN, 50.0), (728.0, 90.0)] {
        let p = smart_resize(&banner_page(), w, h, &none()).page;
        let r = Rect::new(-1e-6, -1e-6, p.width + 1e-6, p.height + 1e-6);
        p.for_each(&mut |e| {
            let b = e.frame.bounds();
            assert!(r.contains(b.origin()) && r.contains(Point::new(b.x1, b.y1)), "{w}x{h}: {:?} outside", e.frame);
        });
    }
}

#[test]
fn labels_stay_on_their_buttons() {
    let p = page(1000.0, 1000.0, vec![rect(2, 380.0, 880.0, 300.0, 80.0), text(3, 380.0, 896.0, 300.0, "Go", 40.0, TextAlign::Center)]);
    for (w, h) in [(2000.0, 1000.0), (1080.0, 1920.0), (500.0, 500.0)] {
        let r = smart_resize(&p, w, h, &none()).page;
        let (pill, label) = (frame(&r, 0), frame(&r, 1));
        let k = pill.width / 300.0;
        assert!(close(label.x - pill.x, 0.0) && close(label.y - pill.y, 16.0 * k), "{w}x{h}: {pill:?} {label:?}");
    }
}

#[test]
fn text_reflows_to_the_new_width_and_its_box_fits_its_lines() {
    let body = "Most herbs and vegetables want six hours of sun, so watch your balcony for a day before you buy a single pot.";
    let p = page(
        1920.0,
        1080.0,
        vec![
            text(1, 100.0, 100.0, 1700.0, "Start a balcony garden", 100.0, TextAlign::Left),
            text(2, 100.0, 260.0, 1700.0, body, 40.0, TextAlign::Left),
        ],
    );
    let fonts = none();
    let r = smart_resize(&p, 1080.0, 1920.0, &fonts).page;
    for i in 0..2 {
        let f = frame(&r, i);
        assert!(f.x >= 0.0 && f.x + f.width <= 1080.0, "{f:?}");
        let ElementKind::Text(t) = &r.elements[i].kind else { panic!() };
        let (w, h) = genha_text::measure_wrapped_el(&fonts, t, f.width);
        assert!(w <= f.width + 0.5 && h <= f.height + 0.5, "{i}: needs {w}×{h}, has {f:?}");
    }
    // Same type size (the shorter side did not change), more lines.
    assert!(close(size(&r, 1), 40.0));
    assert!(frame(&r, 1).height > 2.0 * 48.0);
    // The body stays right below the title.
    let (a, b) = (frame(&r, 0), frame(&r, 1));
    assert!(b.y >= a.y + a.height && b.y - (a.y + a.height) < 200.0, "{a:?} {b:?}");
}

#[test]
fn type_is_never_set_below_the_readable_minimum_while_it_fits() {
    let p = page(1080.0, 1080.0, vec![text(1, 100.0, 500.0, 880.0, "Fine print here", 20.0, TextAlign::Center)]);
    let r = smart_resize(&p, 300.0, 250.0, &none());
    // Plain scaling would give 20 × 250/1080 ≈ 4.6 px; the minimum on a 250 px page is 8 px.
    assert!(size(&r.page, 0) >= 8.0 - 1e-9, "{}", size(&r.page, 0));
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

#[test]
fn a_row_that_does_not_fit_becomes_a_stack() {
    // A picture-like block and a text column side by side on a landscape page.
    let p = page(
        1920.0,
        1080.0,
        vec![rect(2, 100.0, 140.0, 800.0, 800.0), text(3, 1000.0, 400.0, 820.0, "Fresh drops every Friday", 90.0, TextAlign::Left)],
    );
    let r = smart_resize(&p, 1080.0, 1920.0, &none()).page;
    let (pic, title) = (frame(&r, 0), frame(&r, 1));
    assert!(title.y >= pic.y + pic.height, "stacked: {pic:?} {title:?}");
    assert!(close(pic.width, pic.height), "the picture block keeps its aspect: {pic:?}");
}

#[test]
fn a_tall_design_goes_into_columns_on_a_wide_page() {
    let mut els = vec![text(1, 90.0, 100.0, 900.0, "A guide in steps", 100.0, TextAlign::Left)];
    for k in 0..4u64 {
        let y = 500.0 + 340.0 * k as f64;
        els.push(dot(10 + k, 90.0, y, 120.0, 120.0));
        els.push(text(20 + k, 250.0, y + 20.0, 740.0, "Step title and a little text", 44.0, TextAlign::Left));
    }
    let p = page(1080.0, 1920.0, els);
    let r = smart_resize(&p, 1920.0, 1080.0, &none()).page;
    // Plain scaling would set the title at 100 × 1080/1920 = 56 px; columns keep it larger.
    assert!(size(&r, 0) > 70.0, "{}", size(&r, 0));
    let xs: Vec<f64> = (1..=4).map(|k| frame(&r, 2 * k).x).collect();
    assert!(xs.iter().any(|x| *x > 900.0), "something moved to a second column: {xs:?}");
}

#[test]
fn texts_never_overlap_and_notes_report_what_could_not_fit() {
    let words = "So many words that they cannot possibly fit on a tiny page at a readable size, ".repeat(30);
    let p = page(
        1080.0,
        1080.0,
        vec![text(1, 100.0, 100.0, 880.0, &words, 40.0, TextAlign::Left), text(2, 100.0, 900.0, 880.0, &words, 40.0, TextAlign::Left)],
    );
    let r = smart_resize(&p, 300.0, 120.0, &none());
    assert!(!overlaps(frame(&r.page, 0), frame(&r.page, 1)) || !r.notes.is_empty());
    assert!(!r.notes.is_empty());
}

#[test]
fn scale_and_none_modes() {
    let src = banner_page();
    let none_page = resize_page(&src, 2000.0, 500.0, ResizeMode::None, &none()).page;
    assert_eq!((none_page.width, none_page.height), (2000.0, 500.0));
    assert_eq!(none_page.elements, src.elements);
    let scaled = resize_page(&src, 2000.0, 500.0, ResizeMode::Scale, &none()).page;
    // Background covers the page; the rest scaled by ½ and centred.
    assert_eq!(frame(&scaled, 0), Frame::new(0.0, 0.0, 2000.0, 500.0));
    assert_eq!(frame(&scaled, 1), Frame::new(750.0 + 100.0, 225.0, 300.0, frame(&src, 1).height * 0.5));
    assert!(close(size(&scaled, 1), 40.0));
    assert_eq!(ResizeMode::parse("smart"), Some(ResizeMode::Smart));
    assert_eq!(ResizeMode::parse("scale"), Some(ResizeMode::Scale));
    assert_eq!(ResizeMode::parse("none"), Some(ResizeMode::None));
    assert_eq!(ResizeMode::parse("stretch"), None);
}

#[test]
fn deterministic() {
    let src = banner_page();
    let a = smart_resize(&src, 1080.0, 1920.0, &none());
    let b = smart_resize(&src, 1080.0, 1920.0, &none());
    assert_eq!(a, b);
}

#[test]
fn junk_survives() {
    let mut p = banner_page();
    p.elements.push(Element::new(
        NodeId(50),
        Frame::new(f64::NAN, f64::INFINITY, -5.0, 1e300),
        ElementKind::Text(TextEl::new("", TextStyle::default())),
    ));
    p.elements.push(text(51, 0.0, 0.0, 0.0, "x", f64::NAN, TextAlign::Right));
    p.elements.push(rect(52, 1e12, -1e12, 1e-12, 0.0));
    p.width = f64::NAN;
    for (w, h) in [(1080.0, 1920.0), (1.0, 1.0), (1e6, 3.0)] {
        let r = smart_resize(&p, w, h, &none());
        assert_eq!(r.page.elements.len(), p.elements.len());
    }
    // An empty page, and a page of decorations only.
    let r = smart_resize(&page(100.0, 100.0, vec![]), 50.0, 80.0, &none());
    assert!(r.page.elements.is_empty());
    let r = smart_resize(&page(100.0, 100.0, vec![dot(2, 10.0, 10.0, 10.0, 10.0)]), 50.0, 80.0, &none());
    assert_eq!(r.page.elements.len(), 1);
}
