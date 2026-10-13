//! Untrusted input: arbitrary bytes and mangled files never panic the reader.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use genha_format::{from_bytes, to_bytes};
use proptest::prelude::*;

proptest! {
    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = from_bytes(&bytes);
    }

    #[test]
    fn mangled_files_never_panic(pos in 0usize..4096, val in any::<u8>(), compress in any::<bool>()) {
        let doc = genha_doc::Document::new("x", 100.0, 100.0);
        let mut bytes = to_bytes(&doc, compress).unwrap();
        let len = bytes.len();
        if let Some(b) = bytes.get_mut(pos % len) { *b = val; }
        let _ = from_bytes(&bytes);
        bytes.truncate(pos % len);
        let _ = from_bytes(&bytes);
    }

    #[test]
    fn mangled_charts_and_tables_never_panic(pos in 0usize..8192, val in any::<u8>(), digits in "[-0-9e.]{1,12}") {
        // A chart with several series and a table with merges and cell styles, then one byte
        // changed, or a number swapped for an arbitrary one.
        let json = String::from_utf8(to_bytes(&data_doc(), false).unwrap()).unwrap();
        let mut bytes = json.clone().into_bytes();
        let len = bytes.len();
        if let Some(b) = bytes.get_mut(pos % len) { *b = val; }
        if let Ok((doc, _)) = from_bytes(&bytes) {
            let _ = to_bytes(&doc, false).unwrap();
        }
        let swapped = json.replacen("\"rows\":2", &format!("\"rows\":{digits}"), 1).replacen("38.0", &digits, 1).replacen("\"row\":1", &format!("\"row\":{digits}"), 1);
        if let Ok((mut doc, _)) = from_bytes(swapped.as_bytes()) {
            // Repaired documents are consistent: repairing again changes nothing.
            assert!(doc.repair().is_empty());
        }
    }
    /// Text with style runs: junk runs (out of range, overlapping, empty, bad values) come back
    /// repaired; mangled bytes never panic.
    #[test]
    fn junk_style_runs_are_repaired(
        text in "[a-zé中 \n]{0,40}",
        spans in proptest::collection::vec((0usize..60, 0usize..60, any::<Option<bool>>(), proptest::option::of(-10.0f64..3000.0), any::<Option<u8>>()), 0..12),
        pos in 0usize..4096,
        val in any::<u8>(),
    ) {
        let doc = formatted(&text, &spans);
        let bytes = to_bytes(&doc, false).unwrap();
        let (back, _) = from_bytes(&bytes).unwrap();
        for e in &back.pages[0].elements {
            if let genha_doc::ElementKind::Text(t) = &e.kind {
                prop_assert!(t.spans_normalized(), "{:?}", t.spans);
            }
        }
        let mut bytes = bytes;
        let len = bytes.len();
        if let Some(b) = bytes.get_mut(pos % len) { *b = val; }
        let _ = from_bytes(&bytes);
    }

    /// The text box's fit mode: written only when set, read back unchanged; a junk value or a
    /// mangled byte never panics the reader.
    #[test]
    fn junk_text_fit_never_panics(junk in "[a-zA-Z0-9\"{}:,. -]{0,16}", pos in 0usize..4096, val in any::<u8>()) {
        let doc = fitted_doc();
        let json = String::from_utf8(to_bytes(&doc, false).unwrap()).unwrap();
        prop_assert!(json.contains("\"fit\":\"shrink\""));
        let (back, _) = from_bytes(json.as_bytes()).unwrap();
        prop_assert_eq!(&back.pages[0].elements, &doc.pages[0].elements);
        let swapped = json.replacen("\"fit\":\"shrink\"", &format!("\"fit\":{junk}"), 1);
        let _ = from_bytes(swapped.as_bytes());
        let mut bytes = json.into_bytes();
        let len = bytes.len();
        if let Some(b) = bytes.get_mut(pos % len) { *b = val; }
        let _ = from_bytes(&bytes);
    }
}

/// A one-page document with a text box that shrinks to fit.
fn fitted_doc() -> genha_doc::Document {
    use genha_doc::{Element, ElementKind, Frame, NodeId, TextEl, TextFit, TextStyle};
    let mut doc = genha_doc::Document::new("x", 100.0, 100.0);
    let mut t = TextEl::new("Grand opening", TextStyle::default());
    t.fit = TextFit::Shrink;
    doc.pages[0].elements.push(Element::new(NodeId(1000), Frame::new(0.0, 0.0, 50.0, 50.0), ElementKind::Text(t)));
    doc
}

#[test]
fn text_fit_defaults_to_none() {
    // Files written before the fit mode existed (no `fit`) open with text that keeps its size,
    // and such text is written without it.
    let mut doc = fitted_doc();
    let json = String::from_utf8(to_bytes(&doc, false).unwrap()).unwrap();
    let json = json.replacen("\"fit\":\"shrink\",", "", 1).replacen(",\"fit\":\"shrink\"", "", 1);
    assert!(!json.contains("\"fit\""));
    let (back, _) = from_bytes(json.as_bytes()).unwrap();
    let genha_doc::ElementKind::Text(t) = &back.pages[0].elements[0].kind else { panic!() };
    assert_eq!(t.fit, genha_doc::TextFit::None);
    if let genha_doc::ElementKind::Text(t) = &mut doc.pages[0].elements[0].kind {
        t.fit = genha_doc::TextFit::None;
    }
    assert!(!String::from_utf8(to_bytes(&doc, false).unwrap()).unwrap().contains("\"fit\""));
}

fn data_doc() -> genha_doc::Document {
    use genha_doc::{CellRange, ChartEl, ChartKind, ChartSeries, Element, ElementKind, Frame, TableEl};
    let mut doc = genha_doc::Document::new("x", 500.0, 500.0);
    let mut chart = ChartEl::sample(ChartKind::Bar);
    chart.series.push(ChartSeries::new("two", vec![1.0, 2.0, 3.0, 4.0]));
    chart.stacked = true;
    let mut table = TableEl::blank(4, 3);
    table.merge_cells(CellRange::new(1, 0, 2, 2));
    table.style_cells(CellRange::new(0, 1, 2, 2), |s| s.bold = Some(true));
    for kind in [ElementKind::Chart(chart), ElementKind::Table(table)] {
        let id = doc.alloc_id();
        doc.pages[0].elements.push(Element::new(id, Frame::new(0.0, 0.0, 100.0, 100.0), kind));
    }
    doc
}

/// A one-page document with a text box whose spans are written as given (not normalized).
/// A generated span: start, end, bold, size, red channel of a colour.
type JunkSpan = (usize, usize, Option<bool>, Option<f64>, Option<u8>);

fn formatted(text: &str, spans: &[JunkSpan]) -> genha_doc::Document {
    use genha_doc::{Color, Element, ElementKind, Frame, NodeId, SpanStyle, TextEl, TextSpan, TextStyle};
    let mut doc = genha_doc::Document::new("x", 100.0, 100.0);
    let mut t = TextEl::new(text, TextStyle::default());
    t.spans = spans
        .iter()
        .map(|&(a, b, bold, size, c)| {
            TextSpan::new(a..b, SpanStyle { bold, font_size: size, color: c.map(|c| Color::rgb(c, 0, 0)), ..SpanStyle::default() })
        })
        .collect();
    doc.pages[0].elements.push(Element::new(NodeId(1000), Frame::new(0.0, 0.0, 50.0, 50.0), ElementKind::Text(t)));
    doc
}

#[test]
fn formatted_text_round_trips() {
    let mut doc = genha_doc::Document::new("x", 100.0, 100.0);
    let mut t = genha_doc::TextEl::new("Big sale today", genha_doc::TextStyle::default());
    t.set_style_range(4..8, &genha_doc::SpanStyle { bold: Some(true), color: Some(genha_doc::Color::rgb(200, 0, 0)), ..Default::default() });
    doc.pages[0].elements.push(genha_doc::Element::new(
        genha_doc::NodeId(1000),
        genha_doc::Frame::new(0.0, 0.0, 50.0, 50.0),
        genha_doc::ElementKind::Text(t),
    ));
    for compress in [false, true] {
        let (back, notes) = from_bytes(&to_bytes(&doc, compress).unwrap()).unwrap();
        assert_eq!(back.pages[0].elements, doc.pages[0].elements, "{notes:?}");
    }
    // A file written before style runs existed opens unchanged.
    let old = String::from_utf8(to_bytes(&genha_doc::Document::new("x", 10.0, 10.0), false).unwrap()).unwrap();
    assert!(!old.contains("spans"));
}
