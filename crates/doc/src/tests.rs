use super::*;

fn rect(doc: &mut Document) -> Element {
    let id = doc.alloc_id();
    Element::new(
        id,
        Frame::new(10.0, 10.0, 100.0, 50.0),
        ElementKind::Shape(ShapeEl {
            shape: ShapeKind::Rect { corner_radius: 4.0 },
            fill: Fill::solid(Color::rgb(255, 0, 0)),
            stroke: None,
            shadow: None,
        }),
    )
}

#[test]
fn find_and_remove_through_groups() {
    let mut doc = Document::new("t", 500.0, 500.0);
    let a = rect(&mut doc);
    let b = rect(&mut doc);
    let (aid, bid) = (a.id, b.id);
    let gid = doc.alloc_id();
    let group = Element::new(gid, Frame::new(0.0, 0.0, 10.0, 10.0), ElementKind::Group(GroupEl { children: vec![a, b] }));
    doc.pages[0].elements.push(group);
    assert_eq!(doc.find(bid).map(|(p, e)| (p, e.id)), Some((0, bid)));
    assert!(doc.remove(aid).is_some());
    assert!(doc.find(aid).is_none());
    assert_eq!(doc.pages[0].ids(), vec![gid, bid]);
}

#[test]
fn json_round_trip_is_readable() {
    let mut doc = Document::new("Round trip", 1080.0, 1080.0);
    let el = rect(&mut doc);
    doc.pages[0].elements.push(el);
    let asset = doc.add_asset(Asset { name: "a.png".into(), mime: "image/png".into(), width: 1, height: 1, data: Bytes::new(vec![1, 2, 3]) });
    let iid = doc.alloc_id();
    doc.pages[0].elements.push(Element::new(iid, Frame::new(0.0, 0.0, 10.0, 10.0), ElementKind::Image(ImageEl::new(asset))));
    let tid = doc.alloc_id();
    doc.pages[0].elements.push(Element::new(
        tid,
        Frame::new(0.0, 0.0, 10.0, 10.0),
        ElementKind::Text(TextEl { text: "Hello".into(), style: TextStyle::default(), effect: TextEffect::None, curve: 0.0, spans: vec![] }),
    ));
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("\"type\": \"shape\""));
    assert!(json.contains("\"kind\": \"rect\""));
    assert!(json.contains("\"#ff0000\""));
    assert!(json.contains("\"AQID\""), "asset bytes are base64");
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(back, doc);
}

#[test]
fn repair_fixes_untrusted_documents() {
    let mut doc = Document::new("t", f64::NAN, -3.0);
    let mut a = rect(&mut doc);
    a.frame.width = f64::INFINITY;
    a.opacity = f32::NAN;
    let mut b = a.clone();
    b.id = a.id; // duplicate id
    doc.pages[0].elements.push(a);
    doc.pages[0].elements.push(b);
    doc.next_id = 0;
    let notes = doc.repair();
    assert!(!notes.is_empty());
    let ids = doc.pages[0].ids();
    assert_ne!(ids[0], ids[1]);
    assert!(doc.next_id > ids.iter().map(|i| i.0).max().unwrap());
    assert!(doc.pages[0].elements.iter().all(|e| e.frame.is_valid() && e.opacity == 1.0));
    assert_eq!(doc.pages[0].height, 1.0);
    assert_eq!(doc.pages[0].width, 1080.0);

    let mut empty = Document { pages: vec![], ..Document::default() };
    empty.repair();
    assert_eq!(empty.pages.len(), 1);
}

#[test]
fn presets_are_sane() {
    let mut seen = std::collections::HashSet::new();
    for p in PRESETS {
        assert!(seen.insert(p.id), "duplicate preset {}", p.id);
        assert!(p.width >= 1.0 && p.height >= 1.0 && p.width <= MAX_PAGE_SIDE && p.height <= MAX_PAGE_SIDE);
    }
    assert!(preset::find("instagram-story").is_some());
    assert_eq!(preset::categories().first(), Some(&"Social"));
}

#[test]
fn unused_assets() {
    let mut doc = Document::default();
    let a = doc.add_asset(Asset { name: "x".into(), mime: "image/png".into(), width: 0, height: 0, data: Bytes::default() });
    assert_eq!(doc.unused_assets(), vec![a]);
}

#[test]
fn animation_serde_is_backwards_compatible() {
    // Files written before loops and exits existed still read.
    let a: Animation = serde_json::from_str(r#"{"kind":"slide-left","duration":0.5}"#).unwrap();
    assert_eq!(a, Animation::new(AnimationKind::SlideLeft, 0.5, 0.0));
    let full = Animation { looping: Some(LoopKind::Pulse), exit: Some(AnimationKind::Fade), ..Animation::new(AnimationKind::Pop, 0.6, 0.2) };
    let json = serde_json::to_string(&full).unwrap();
    assert!(json.contains(r#""loop":"pulse""#) && json.contains(r#""exit":"fade""#), "{json}");
    assert_eq!(serde_json::from_str::<Animation>(&json).unwrap(), full);
    // A plain entrance writes no extra fields.
    assert!(!serde_json::to_string(&Animation::new(AnimationKind::Fade, 1.0, 0.0)).unwrap().contains("loop"));
}

#[test]
fn charts_and_tables_round_trip_and_default_missing_fields() {
    let mut doc = Document::new("t", 500.0, 500.0);
    let cid = doc.alloc_id();
    let tid = doc.alloc_id();
    let chart = ChartEl::sample(ChartKind::Donut);
    let table = TableEl::blank(3, 2);
    doc.pages[0].elements.push(Element::new(cid, Frame::new(0.0, 0.0, 100.0, 100.0), ElementKind::Chart(chart.clone())));
    doc.pages[0].elements.push(Element::new(tid, Frame::new(0.0, 0.0, 100.0, 100.0), ElementKind::Table(table.clone())));
    let json = serde_json::to_string(&doc).unwrap();
    assert!(json.contains(r#""type":"chart""#) && json.contains(r#""kind":"donut""#) && json.contains(r#""type":"table""#));
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(back, doc);
    // Minimal hand-written elements open with defaults.
    let el: Element = serde_json::from_str(r#"{"id":5,"frame":{"x":0,"y":0,"width":10,"height":10},"type":"chart"}"#).unwrap();
    assert_eq!(el.kind, ElementKind::Chart(ChartEl::default()));
    let el: Element =
        serde_json::from_str(r#"{"id":6,"frame":{"x":0,"y":0,"width":10,"height":10},"type":"table","rows":[["a","b"],["c"]]}"#).unwrap();
    let ElementKind::Table(t) = el.kind else { panic!() };
    assert_eq!((t.columns(), t.cell(1, 1), t.cell(9, 9)), (2, "", ""));
    // Old text without `curve` is straight.
    let el: Element = serde_json::from_str(
        r##"{"id":7,"frame":{"x":0,"y":0,"width":10,"height":10},"type":"text","text":"Hi","style":{"font_family":"Sans","font_size":20,"color":"#000000"}}"##,
    )
    .unwrap();
    let ElementKind::Text(t) = el.kind else { panic!() };
    assert_eq!(t.curve, 0.0);
    assert_eq!(ChartKind::from_id("column"), Some(ChartKind::Bar));
    for k in ChartKind::ALL {
        assert_eq!(ChartKind::from_id(k.id()), Some(k));
    }
}

#[test]
fn repair_caps_data_elements() {
    let mut doc = Document::new("t", 500.0, 500.0);
    let mut chart = ChartEl::with_points((0..500).map(|i| ChartPoint::new("x", if i == 0 { f64::NAN } else { i as f64 })).collect());
    chart.series.extend((0..40).map(|i| ChartSeries::new(format!("s{i}"), vec![1.0; i])));
    chart.donut_hole = 7.0;
    chart.max = Some(f64::INFINITY);
    chart.min = Some(5.0);
    chart.decimals = Some(200);
    let mut table = TableEl { rows: vec![vec!["a".into(); 50]; 300], ..TableEl::default() };
    table.rows[1].truncate(3);
    table.merges = vec![
        CellRange::new(0, 0, 2, 2),
        CellRange::new(1, 1, 3, 3),
        CellRange::new(99, 29, 5, 5),
        CellRange::new(500, 0, 2, 2),
        CellRange::cell(4, 4),
    ];
    table.cell_styles = vec![
        CellFormat { row: 1, col: 1, style: CellStyle { bold: Some(true), ..CellStyle::default() } },
        CellFormat { row: 1, col: 1, style: CellStyle { font_size: Some(f64::NAN), fill: Some(Color::BLACK), ..CellStyle::default() } },
        CellFormat { row: 9999, col: 0, style: CellStyle { bold: Some(true), ..CellStyle::default() } },
        CellFormat { row: 2, col: 2, style: CellStyle::default() },
    ];
    table.row_heights = vec![f64::NAN; 400];
    let mut text = TextEl::new("Hi", TextStyle::default());
    text.curve = 9999.0;
    for kind in [ElementKind::Chart(chart), ElementKind::Table(table), ElementKind::Text(text)] {
        let id = doc.alloc_id();
        doc.pages[0].elements.push(Element::new(id, Frame::new(0.0, 0.0, 10.0, 10.0), kind));
    }
    let notes = doc.repair();
    assert!(notes.iter().any(|n| n.contains("chart")), "{notes:?}");
    let els = &doc.pages[0].elements;
    let ElementKind::Chart(c) = &els[0].kind else { panic!() };
    assert_eq!((c.categories.len(), c.series.len()), (MAX_CHART_POINTS, MAX_CHART_SERIES));
    assert!(c.series.iter().all(|s| s.values.len() == MAX_CHART_POINTS));
    assert_eq!(c.value(0, 0), 0.0);
    assert!(c.donut_hole <= 0.9 && c.max.is_none() && c.min == Some(5.0) && c.decimals == Some(6));
    let ElementKind::Table(t) = &els[1].kind else { panic!() };
    assert_eq!((t.rows.len(), t.columns()), (MAX_TABLE_ROWS, MAX_TABLE_COLS));
    assert!(t.rows.iter().all(|r| r.len() == MAX_TABLE_COLS));
    // Overlapping, outside and one-cell merges go; a merge past the edge is cut to the table.
    assert_eq!(t.merges, vec![CellRange::new(0, 0, 2, 2)]);
    assert_eq!(t.cell_styles.len(), 1, "{:?}", t.cell_styles);
    assert_eq!(t.cell_styles[0].style, CellStyle { fill: Some(Color::BLACK), ..CellStyle::default() });
    assert!(t.row_heights.is_empty());
    let ElementKind::Text(t) = &els[2].kind else { panic!() };
    assert_eq!(t.curve, 360.0);
}

#[test]
fn single_series_charts_from_before_open_as_one_series() {
    let el: Element = serde_json::from_str(
        r#"{"id":5,"frame":{"x":0,"y":0,"width":10,"height":10},"type":"chart","kind":"bar","data":[{"label":"Q1","value":3},{"label":"Q2","value":5}],"max":10}"#,
    )
    .unwrap();
    let ElementKind::Chart(c) = &el.kind else { panic!() };
    assert_eq!(c.categories, vec!["Q1", "Q2"]);
    assert_eq!(c.series, vec![ChartSeries::new("Series 1", vec![3.0, 5.0])]);
    assert_eq!(c.max, Some(10.0));
    assert_eq!(c.points(), vec![ChartPoint::new("Q1", 3.0), ChartPoint::new("Q2", 5.0)]);
    // Written back without `data`, and read again the same.
    let json = serde_json::to_string(&el).unwrap();
    assert!(!json.contains(r#""data""#) && json.contains(r#""series""#), "{json}");
    assert_eq!(serde_json::from_str::<Element>(&json).unwrap(), el);
    // Series shorter or longer than the categories are squared up.
    let mut c = c.clone();
    c.series.push(ChartSeries::new("B", vec![1.0, 2.0, 3.0]));
    assert!(c.sanitize());
    assert_eq!((c.categories.len(), c.series[0].values.len(), c.categories[2].as_str()), (3, 3, ""));
}

#[test]
fn merged_cells_follow_row_and_column_changes() {
    let mut t = TableEl::blank(4, 4);
    t.rows[1][1] = "a".into();
    t.rows[2][2] = "b".into();
    // Merging joins the texts in the top-left cell.
    assert_eq!(t.merge_cells(CellRange::new(1, 1, 2, 2)), Some(CellRange::new(1, 1, 2, 2)));
    assert_eq!((t.cell(1, 1), t.cell(2, 2)), ("a b", ""));
    assert_eq!(t.merge_cells(CellRange::cell(0, 0)), None);
    // A range overlapping a merge takes it in.
    let mut u = t.clone();
    assert_eq!(u.merge_cells(CellRange::new(2, 0, 1, 2)), Some(CellRange::new(1, 0, 2, 3)));
    assert_eq!(u.merges.len(), 1);
    // Rows and columns inserted inside grow it; before it move it.
    t.insert_row(2);
    assert_eq!(t.merges, vec![CellRange::new(1, 1, 3, 2)]);
    t.insert_col(0);
    assert_eq!(t.merges, vec![CellRange::new(1, 2, 3, 2)]);
    t.style_cells(CellRange::cell(1, 2), |s| s.bold = Some(true));
    // Deleting its top row keeps the text and style in the new top row.
    t.delete_row(1);
    assert_eq!(t.merges, vec![CellRange::new(1, 2, 2, 2)]);
    assert_eq!(t.cell(1, 2), "a b");
    assert_eq!(t.style_at(1, 2).and_then(|s| s.bold), Some(true));
    t.delete_col(3);
    assert_eq!(t.merges, vec![CellRange::new(1, 2, 2, 1)]);
    t.delete_row(2);
    assert!(t.merges.is_empty(), "a merge down to one cell goes");
    assert_eq!(t.spans()[1][2], CellSpan::Own);
    assert!(!t.sanitize(), "the edits keep the table consistent");
    // Spans show anchors and covered cells; splitting clears them.
    let mut v = TableEl::blank(3, 3);
    v.merge_cells(CellRange::new(0, 0, 1, 3));
    let spans = v.spans();
    assert_eq!(spans[0][0], CellSpan::Anchor(CellRange::new(0, 0, 1, 3)));
    assert_eq!(spans[0][2], CellSpan::Covered(CellRange::new(0, 0, 1, 3)));
    assert_eq!(v.split_cells(CellRange::cell(0, 1)), 1);
    assert!(v.merges.is_empty());
    // Styling a range and clearing it again.
    assert_eq!(v.style_cells(CellRange::new(1, 0, 5, 5), |s| s.fill = Some(Color::BLACK)), 6);
    assert_eq!(v.cell_styles.len(), 6);
    v.style_cells(CellRange::new(0, 0, 3, 3), |s| s.fill = None);
    assert!(v.cell_styles.is_empty());
}
