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
