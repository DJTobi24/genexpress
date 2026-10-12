use std::sync::OnceLock;

use genha_doc::{TextAlign, TextStyle};
use kurbo::Shape;

use super::*;

/// The system fonts, scanned once for all tests.
fn system() -> &'static FontBook {
    static BOOK: OnceLock<FontBook> = OnceLock::new();
    BOOK.get_or_init(FontBook::system)
}

fn style(size: f64) -> TextStyle {
    TextStyle { font_size: size, align: TextAlign::Left, ..TextStyle::default() }
}

#[test]
fn empty_book_gives_approximate_metrics() {
    let book = FontBook::new();
    assert!(book.is_empty());
    assert!(book.families().is_empty());
    let (w, h) = measure(&book, "abcd", &style(100.0));
    assert!((w - 4.0 * 55.0).abs() < 1e-9, "{w}");
    assert!((h - 120.0).abs() < 1e-9);
    let l = layout(&book, "abcd", &style(100.0), 1000.0);
    assert_eq!(l.lines.len(), 1);
    assert!(l.lines[0].path.is_empty());
    let line = &l.lines[0];
    assert!(line.baseline > line.top && line.baseline < line.top + line.height);
}

#[test]
fn wraps_words_and_honours_newlines() {
    let book = FontBook::new();
    // 0.55 em at 10 px = 5.5 px per character.
    let s = style(10.0);
    let l = layout(&book, "aaa bbb ccc", &s, 40.0);
    let texts: Vec<&str> = l.lines.iter().map(|l| l.text.trim_end()).collect();
    assert_eq!(texts, ["aaa bbb", "ccc"]);
    let l = layout(&book, "one\ntwo\n", &s, 1000.0);
    assert_eq!(l.lines.len(), 3);
    assert!((l.height - 36.0).abs() < 1e-9);
    assert!((l.lines[1].top - 12.0).abs() < 1e-9);
    // A word longer than the box breaks between letters.
    let l = layout(&book, "abcdefghij", &s, 20.0);
    assert!(l.lines.len() >= 3);
    assert!(l.lines.iter().all(|line| line.width <= 20.0 + 1e-9));
    let joined: String = l.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(joined, "abcdefghij");
}

#[test]
fn alignment_and_spacing() {
    let book = FontBook::new();
    let mut s = style(10.0);
    s.align = TextAlign::Center;
    let l = layout(&book, "ab", &s, 100.0);
    assert!((l.lines[0].x - (100.0 - 11.0) / 2.0).abs() < 1e-9);
    s.align = TextAlign::Right;
    let l = layout(&book, "ab", &s, 100.0);
    assert!((l.lines[0].x - 89.0).abs() < 1e-9);
    s.align = TextAlign::Left;
    s.letter_spacing = 100.0; // 0.1 em = 1 px between letters
    let (w, _) = measure(&book, "abc", &s);
    assert!((w - (16.5 + 2.0)).abs() < 1e-9, "{w}");
    s.uppercase = true;
    assert_eq!(layout(&book, "abc", &s, 0.0).lines[0].text, "ABC");
}

#[test]
fn hostile_numbers_do_not_break_layout() {
    let book = FontBook::new();
    let mut s = style(f64::NAN);
    s.line_height = f64::INFINITY;
    s.letter_spacing = f64::NAN;
    let l = layout(&book, "hello world", &s, f64::NAN);
    assert!(l.width.is_finite() && l.height.is_finite());
    let l = layout(&book, "", &style(10.0), 100.0);
    assert_eq!(l.lines.len(), 1);
    assert_eq!(l.lines[0].width, 0.0);
}

#[test]
fn rejects_non_fonts() {
    let mut book = FontBook::new();
    assert!(book.add_font(b"definitely not a font".to_vec()).is_err());
    assert!(book.add_font(vec![]).is_err());
    assert!(book.is_empty());
}

#[test]
fn system_fonts_draw_glyphs() {
    let book = system();
    if book.is_empty() {
        eprintln!("no system fonts; skipping glyph checks");
        return;
    }
    let fams = book.families();
    assert!(!fams.is_empty());
    let mut sorted = fams.clone();
    sorted.sort_by_key(|a| a.to_lowercase());
    assert_eq!(fams, sorted);
    let l = layout(book, "Hello", &style(64.0), 2000.0);
    let line = &l.lines[0];
    assert!(!line.path.is_empty());
    let b = line.path.bounding_box();
    // Ink sits on the baseline and above it, within the line box.
    assert!(b.y1 <= line.baseline + 2.0, "{b:?} {}", line.baseline);
    assert!(b.y0 >= line.top - 1.0 && b.y0 < line.baseline - 30.0, "{b:?}");
    assert!(b.width() > 100.0 && b.width() <= line.width + 5.0, "{b:?} {}", line.width);
    assert!(line.path.area().abs() > 100.0);
}

#[test]
fn system_font_matching() {
    let book = system();
    if book.is_empty() {
        return;
    }
    // Generic families resolve to something.
    for g in ["Sans", "Serif", "Mono", "Display", "No Such Font 123"] {
        assert!(book.resolve_family(g, false, false).is_some(), "{g}");
    }
    if book.has_family("DejaVu Sans") {
        assert_eq!(book.resolve_family("dejavu sans", false, false).as_deref(), Some("DejaVu Sans"));
        // Bold is wider than regular.
        let mut s = style(40.0);
        s.font_family = "DejaVu Sans".into();
        let (regular, _) = measure(book, "Bold text", &s);
        s.bold = true;
        let (bold, _) = measure(book, "Bold text", &s);
        assert!(bold > regular, "{bold} {regular}");
    }
    if book.has_family("Inter") {
        assert_eq!(book.resolve_family("Sans", false, false).as_deref(), Some("Inter"));
    }
    if book.has_family("DejaVu Sans Mono") && !book.has_family("Noto Sans Mono") {
        assert_eq!(book.resolve_family("Mono", false, false).as_deref(), Some("DejaVu Sans Mono"));
    }
}

#[test]
fn underline_and_fallback_glyphs() {
    let book = system();
    if book.is_empty() {
        return;
    }
    let mut s = style(40.0);
    let plain = layout(book, "go", &s, 0.0);
    s.underline = true;
    let under = layout(book, "go", &s, 0.0);
    let (a, b) = (plain.lines[0].path.bounding_box(), under.lines[0].path.bounding_box());
    assert!(under.lines[0].path.elements().len() > plain.lines[0].path.elements().len());
    assert!(b.y1 >= a.y1 - 1.0);
    // A CJK character comes from another face when the main one lacks it.
    let l = layout(book, "中", &style(40.0), 0.0);
    assert!(l.lines[0].width > 0.0);
}

#[test]
fn add_font_from_memory() {
    let path = ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists());
    let Some(path) = path else { return };
    let mut book = FontBook::new();
    let n = book.add_font(std::fs::read(path).unwrap()).unwrap();
    assert_eq!(n, 1);
    assert_eq!(book.families().len(), 1);
    let fam = book.families()[0].clone();
    let mut s = style(30.0);
    s.font_family = fam;
    let l = layout(&book, "Abc", &s, 0.0);
    assert!(!l.lines[0].path.is_empty());
    // A clone shares the faces and is Send + Sync.
    fn assert_send_sync<T: Send + Sync + Clone>(_: &T) {}
    assert_send_sync(&book);
}
