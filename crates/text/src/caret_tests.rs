//! Caret mapping (`caret`) and text editing (`edit`).

use std::sync::OnceLock;

use genha_doc::{TextAlign, TextStyle};

use crate::caret::CaretMap;
use crate::edit::{self, EditBuffer, EditKind, UndoStack};
use crate::{FontBook, layout, layout_glyphs};

fn system() -> &'static FontBook {
    static BOOK: OnceLock<FontBook> = OnceLock::new();
    BOOK.get_or_init(FontBook::system)
}

/// 10 px text: with no fonts every character is 5.5 px wide and lines are 12 px tall.
fn style(align: TextAlign) -> TextStyle {
    TextStyle { font_size: 10.0, line_height: 1.2, letter_spacing: 0.0, align, ..TextStyle::default() }
}

fn map(text: &str, st: &TextStyle, width: f64) -> CaretMap {
    CaretMap::for_text(&FontBook::new(), text, st, width)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn stops_follow_the_glyphs() {
    let m = map("abc", &style(TextAlign::Left), 1000.0);
    assert_eq!(m.len, 3);
    assert_eq!(m.lines.len(), 1);
    let xs: Vec<(usize, f64)> = m.lines[0].stops.iter().map(|s| (s.index, s.x)).collect();
    assert_eq!(xs.len(), 4);
    for (k, (i, x)) in xs.iter().enumerate() {
        assert_eq!(*i, k);
        assert!(close(*x, k as f64 * 5.5), "{xs:?}");
    }
    let c = m.caret(2, false).unwrap();
    assert!(close(c.x, 11.0) && close(c.top, 0.0) && close(c.height, 12.0));
}

#[test]
fn hard_breaks_wraps_and_empty_lines() {
    // "aaa bbb" wraps in a 40 px box; then a line break and an empty paragraph.
    let text = "aaa bbb\n\nccc";
    let m = map(text, &style(TextAlign::Left), 30.0);
    let ranges: Vec<(usize, usize)> = m.lines.iter().map(|l| (l.start(), l.end())).collect();
    assert_eq!(ranges, [(0, 4), (4, 7), (8, 8), (9, 12)]);
    // Index 4 is shown at the start of the second line, or (upstream) at the end of the first.
    assert_eq!(m.caret(4, false).unwrap().line, 1);
    let up = m.caret(4, true).unwrap();
    assert_eq!(up.line, 0);
    assert!(close(up.x, 22.0));
    // The empty paragraph has one stop on the third line.
    let e = m.caret(8, false).unwrap();
    assert_eq!(e.line, 2);
    assert!(close(e.top, 24.0));
    // Home / End on a wrapped line.
    assert_eq!(m.line_start(5, false), 4);
    assert_eq!(m.line_end(1, false), (4, true));
    assert_eq!(m.line_end(5, false), (7, false));
}

#[test]
fn hit_testing_picks_the_nearest_stop() {
    let m = map("abc\ndef", &style(TextAlign::Left), 1000.0);
    assert_eq!(m.hit(0.0, 5.0), (0, false));
    assert_eq!(m.hit(6.0, 5.0), (1, false));
    assert_eq!(m.hit(8.0, 5.0), (1, false));
    assert_eq!(m.hit(9.0, 5.0), (2, false));
    assert_eq!(m.hit(500.0, 5.0), (3, false));
    // Second line, and points above / below the text.
    assert_eq!(m.hit(6.0, 15.0), (5, false));
    assert_eq!(m.hit(6.0, -50.0), (1, false));
    assert_eq!(m.hit(500.0, 500.0), (7, false));
    // Past the end of a wrapped line: the caret stays at its end.
    let w = map("aaa bbb", &style(TextAlign::Left), 30.0);
    assert_eq!(w.hit(29.0, 5.0), (4, true));
}

#[test]
fn alignment_moves_the_stops() {
    let c = map("ab", &style(TextAlign::Center), 100.0);
    assert!(close(c.lines[0].stops[0].x, (100.0 - 11.0) / 2.0));
    let r = map("ab", &style(TextAlign::Right), 100.0);
    assert!(close(r.lines[0].end() as f64, 2.0));
    assert!(close(r.lines[0].x_of(2), 100.0));
    // An empty centred text puts the caret in the middle of the box.
    let e = map("", &style(TextAlign::Center), 100.0);
    assert!(close(e.caret(0, false).unwrap().x, 50.0));
}

#[test]
fn letter_spacing_and_uppercase_are_followed() {
    let st = TextStyle { letter_spacing: 100.0, ..style(TextAlign::Left) };
    let m = map("ab", &st, 1000.0);
    // 5.5 px advance plus 1 px spacing.
    assert!(close(m.lines[0].x_of(1), 6.5));
    // "ß" becomes "SS": one caret stop for two glyphs.
    let up = TextStyle { uppercase: true, ..style(TextAlign::Left) };
    let m = map("aßb", &up, 1000.0);
    assert_eq!(m.len, 3);
    let xs: Vec<f64> = m.lines[0].stops.iter().map(|s| s.x).collect();
    assert_eq!(xs.len(), 4);
    assert!(close(xs[2], 16.5), "{xs:?}");
}

#[test]
fn vertical_moves_keep_the_column() {
    let m = map("abcdef\nab\nabcdef", &style(TextAlign::Left), 1000.0);
    // From index 4 ("abcd|ef") down: the short line's end, then back to column 4.
    let (i, _) = m.vertical(4, false, 1, m.caret(4, false).unwrap().x);
    assert_eq!(i, 9);
    let (i, _) = m.vertical(9, false, 1, 22.0);
    assert_eq!(i, 14);
    assert_eq!(m.vertical(2, false, -1, 11.0), (0, false));
    assert_eq!(m.vertical(12, false, 1, 0.0), (16, false));
}

#[test]
fn selection_boxes() {
    let m = map("abc\ndef", &style(TextAlign::Left), 1000.0);
    assert!(m.selection_rects(2, 2).is_empty());
    let r = m.selection_rects(1, 2);
    assert_eq!(r.len(), 1);
    assert!(close(r[0].x0, 5.5) && close(r[0].x1, 11.0) && close(r[0].y1, 12.0));
    // Across the line break: the break shows after the first line.
    let r = m.selection_rects(5, 2);
    assert_eq!(r.len(), 2);
    assert!(r[0].x1 > 16.5);
    assert!(close(r[1].x0, 0.0) && close(r[1].x1, 5.5) && close(r[1].y0, 12.0));
    // A wrapped line's end is not drawn twice.
    let w = map("aaa bbb", &style(TextAlign::Left), 30.0);
    assert_eq!(w.selection_rects(0, 7).len(), 2);
    assert_eq!(w.selection_rects(4, 7).len(), 1);
}

#[test]
fn real_fonts_match_the_renderer() {
    let book = system();
    if book.is_empty() {
        return; // no fonts on this machine
    }
    let st = TextStyle { font_size: 40.0, align: TextAlign::Center, ..TextStyle::default() };
    let text = "Hello wide world, office fix wrapping\nNext";
    let gl = layout_glyphs(book, text, &st, 300.0);
    let l = layout(book, text, &st, 300.0);
    let m = CaretMap::new(text, &gl);
    assert_eq!(m.lines.len(), l.lines.len());
    for (cl, ll) in m.lines.iter().zip(&l.lines) {
        // Each line starts where the renderer put it.
        assert!(close(cl.stops[0].x, ll.x), "{} vs {}", cl.stops[0].x, ll.x);
        assert!(close(cl.top, ll.top));
    }
    // Every index has a stop and they run left to right.
    let total: usize = m.lines.iter().map(|l| l.stops.len()).sum();
    assert!(total >= m.len);
    for l in &m.lines {
        assert!(l.stops.windows(2).all(|w| w[0].index < w[1].index && w[0].x <= w[1].x + 1e-9));
    }
}

#[test]
fn junk_never_panics() {
    let book = FontBook::new();
    for text in ["", "\n\n\n", "\r\n\t\u{0}", "ﬁ ß İ 🎉 e\u{301}", &"x ".repeat(3000)] {
        for w in [f64::NAN, -1.0, 0.0, 1.0, 1e9, f64::INFINITY] {
            let m = CaretMap::for_text(&book, text, &style(TextAlign::Right), w);
            for i in [0, 1, 5, m.len, m.len + 10] {
                let _ = m.caret(i, true);
                let _ = m.caret(i, false);
                let _ = m.vertical(i, false, -3, 1.0);
                let _ = m.vertical(i, true, 3, f64::NAN);
                let _ = m.selection_rects(0, i);
                let _ = m.line_end(i, false);
            }
            let _ = m.hit(f64::NAN, f64::INFINITY);
            let _ = m.bounds();
        }
    }
}

#[test]
fn word_and_paragraph_boundaries() {
    let t = "Hello, big world\nnext";
    assert_eq!(edit::next_word(t, 0), 5);
    assert_eq!(edit::next_word(t, 5), 6);
    assert_eq!(edit::next_word(t, 6), 10);
    assert_eq!(edit::prev_word(t, 10), 7);
    assert_eq!(edit::prev_word(t, 7), 5);
    assert_eq!(edit::prev_word(t, 0), 0);
    assert_eq!(edit::next_word(t, 99), 21);
    assert_eq!(edit::word_at(t, 2), 0..5);
    assert_eq!(edit::word_at(t, 5), 0..5);
    assert_eq!(edit::word_at(t, 12), 11..16);
    assert_eq!(edit::word_at(t, 16), 11..16);
    assert_eq!(edit::word_at("a  b", 2), 1..3);
    assert_eq!(edit::word_at("", 0), 0..0);
    assert_eq!(edit::word_at("don't stop", 1), 0..5);
    assert_eq!(edit::paragraph_at(t, 3), 0..16);
    assert_eq!(edit::paragraph_at(t, 18), 17..21);
}

#[test]
fn editing_a_buffer() {
    let mut b = EditBuffer::new("Hello world", 0, 0);
    b.move_to(5, false);
    assert!(b.insert(","));
    assert_eq!((b.text.as_str(), b.caret), ("Hello, world", 6));
    // Typing replaces the selection.
    b.select(7, 12);
    assert_eq!(b.selected(), "world");
    assert!(b.insert("ünïcode"));
    assert_eq!(b.text, "Hello, ünïcode");
    assert_eq!(b.caret, 14);
    assert!(b.backspace(true));
    assert_eq!(b.text, "Hello, ");
    assert!(b.backspace(false));
    assert_eq!(b.text, "Hello,");
    b.move_to(0, false);
    assert!(b.delete_forward(true));
    assert_eq!(b.text, ",");
    assert!(!b.backspace(false), "nothing before the caret");
    // Arrows collapse a selection, Shift extends it.
    let mut b = EditBuffer::new("abc def", 1, 3);
    b.left(false, false);
    assert_eq!((b.anchor, b.caret), (1, 1));
    b.right(true, true);
    assert_eq!((b.anchor, b.caret), (1, 3));
    b.right(true, true);
    assert_eq!(b.caret, 7);
    b.right(false, false);
    assert_eq!((b.anchor, b.caret), (7, 7));
    // Line breaks are normalized, control characters dropped, the size is capped.
    let mut b = EditBuffer::new("", 0, 0);
    assert!(b.insert("a\r\nb\rc\u{7}\td"));
    assert_eq!(b.text, "a\nb\nc\td");
    assert!(!b.insert(&"x".repeat(edit::MAX_EDIT_BYTES)));
    assert_eq!(EditBuffer::new("abc", 9, 99).range(), 3..3);
}

#[test]
fn undo_groups_runs_of_typing() {
    let mut b = EditBuffer::new("", 0, 0);
    let mut u = UndoStack::default();
    for c in ["a", "b", "c"] {
        u.record(&b, EditKind::Typing);
        b.insert(c);
    }
    u.break_run();
    u.record(&b, EditKind::Typing);
    b.insert(" d");
    u.record(&b, EditKind::Deleting);
    b.backspace(false);
    assert_eq!(b.text, "abc ");
    let b1 = u.undo(&b).unwrap();
    assert_eq!(b1.text, "abc d");
    let b2 = u.undo(&b1).unwrap();
    assert_eq!(b2.text, "abc");
    let b3 = u.undo(&b2).unwrap();
    assert_eq!(b3.text, "");
    assert!(u.undo(&b3).is_none());
    let r = u.redo(&b3).unwrap();
    assert_eq!(r.text, "abc");
}

fn glyph(cluster: usize, x: f64, advance: f64, rtl: bool) -> crate::PlacedGlyph {
    crate::PlacedGlyph { cluster, x, advance, rtl, ..Default::default() }
}

fn line(range: std::ops::Range<usize>, glyphs: Vec<crate::PlacedGlyph>) -> crate::GlyphLayout {
    let l = crate::GlyphLine { glyphs, range, height: 12.0, baseline: 9.0, ..Default::default() };
    crate::GlyphLayout { lines: vec![l], ..Default::default() }
}

fn xs(m: &CaretMap) -> Vec<(usize, f64)> {
    m.lines[0].stops.iter().map(|s| (s.index, s.x)).collect()
}

#[test]
fn ligatures_are_split_by_grapheme() {
    // "fix": one glyph draws "fi".
    let m = CaretMap::new("fix", &line(0..3, vec![glyph(0, 0.0, 20.0, false), glyph(2, 20.0, 10.0, false)]));
    assert_eq!(xs(&m), [(0, 0.0), (1, 10.0), (2, 20.0), (3, 30.0)]);
    // "é" as e + combining accent is one grapheme: no caret between the two.
    let text = "e\u{301}x";
    let m = CaretMap::new(text, &line(0..text.len(), vec![glyph(0, 0.0, 10.0, false), glyph(0, 2.0, 0.0, false), glyph(3, 10.0, 10.0, false)]));
    assert_eq!(xs(&m), [(0, 0.0), (2, 10.0), (3, 20.0)]);
}

#[test]
fn right_to_left_runs_mirror_the_carets() {
    // "ab אב": the Hebrew word is drawn reversed, its first letter on the right.
    let text = "ab \u{5d0}\u{5d1}";
    let glyphs = vec![
        glyph(0, 0.0, 10.0, false),
        glyph(1, 10.0, 10.0, false),
        glyph(2, 20.0, 10.0, false),
        glyph(5, 30.0, 10.0, true),
        glyph(3, 40.0, 10.0, true),
    ];
    let m = CaretMap::new(text, &line(0..text.len(), glyphs));
    assert_eq!(xs(&m), [(0, 0.0), (1, 10.0), (2, 20.0), (3, 50.0), (4, 40.0), (5, 30.0)]);
    // Selecting the first Hebrew letter highlights the rightmost box.
    let r = m.selection_rects(3, 4);
    assert_eq!(r.len(), 1);
    assert!(close(r[0].x0, 40.0) && close(r[0].x1, 50.0));
    // "b" and "א" are two separate boxes on screen.
    assert_eq!(m.selection_rects(1, 4).len(), 2);
    assert_eq!(m.hit(48.0, 5.0).0, 3);
    // Without fonts the layout still reorders the Hebrew run.
    let m = map(text, &style(TextAlign::Left), 1000.0);
    let x = |i| m.caret(i, false).unwrap().x;
    assert!(x(3) > x(4) && x(4) > x(5), "{:?}", xs(&m));
}

#[test]
fn moving_by_grapheme() {
    let text = "e\u{301}👍🏽x";
    assert_eq!(edit::next_grapheme(text, 0), 2);
    assert_eq!(edit::next_grapheme(text, 2), 4);
    assert_eq!(edit::prev_grapheme(text, 4), 2);
    assert_eq!(edit::prev_grapheme(text, 0), 0);
    assert_eq!(edit::next_grapheme(text, 5), 5);
    let mut b = EditBuffer::new(text, 4, 4);
    assert!(b.backspace(false));
    assert_eq!(b.text, "e\u{301}x");
    b.left(false, false);
    assert_eq!(b.caret, 0);
}
