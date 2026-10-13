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
