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

#[test]
fn glyph_layout_matches_line_layout() {
    for book in [&FontBook::new(), system()] {
        let s = style(30.0);
        let lines = layout(book, "Hi there\nyou", &s, 0.0);
        let glyphs = layout_glyphs(book, "Hi there\nyou", &s, 0.0);
        assert_eq!(glyphs.lines.len(), 2);
        assert_eq!((glyphs.width, glyphs.height), (lines.width, lines.height));
        for (g, l) in glyphs.lines.iter().zip(&lines.lines) {
            // Plain Latin without ligatures: one glyph per character.
            assert_eq!(g.glyphs.len(), l.text.chars().count());
            assert_eq!((g.x, g.width, g.baseline, &g.range), (l.x, l.width, l.baseline, &l.range));
            // Glyphs advance left to right, and together draw the line.
            assert!(g.glyphs.windows(2).all(|w| w[1].x > w[0].x));
            let n: usize = g.glyphs.iter().map(|p| p.path.elements().len()).sum();
            assert_eq!(n, l.path.elements().len());
        }
    }
}

// --- Shaping, bidi and fallback -------------------------------------------------------------

/// A family whose regular face has every character of `chars`: the first of `prefer` that is
/// installed and has them, or (with `prefer` empty) any.
fn family_with(book: &FontBook, chars: &str, prefer: &[&str]) -> Option<String> {
    let has = |i: usize| book.face(i).is_some_and(|f| !f.italic && chars.chars().all(|c| f.covers(c)));
    for fam in prefer {
        if let Some(i) = book.select(fam, false, false)
            && book.face(i).is_some_and(|f| f.family.eq_ignore_ascii_case(fam))
            && has(i)
        {
            return Some(fam.to_string());
        }
    }
    if !prefer.is_empty() {
        return None;
    }
    // A family whose selected regular face has them (not just some odd style of it).
    (0..book.len())
        .filter(|&i| has(i))
        .filter_map(|i| book.face(i).map(|f| f.family.clone()))
        .find(|fam| book.select(fam, false, false).is_some_and(has))
}

fn styled(family: &str, size: f64) -> TextStyle {
    TextStyle { font_family: family.into(), ..style(size) }
}

/// The source clusters of a line's glyphs, left to right.
fn clusters(line: &GlyphLine) -> Vec<usize> {
    line.glyphs.iter().map(|g| g.cluster).collect()
}

#[test]
fn kerning_tightens_pairs() {
    let book = system();
    let Some(fam) = family_with(book, "AVTo", &["DejaVu Sans", "DejaVu Serif", "Liberation Sans", "Inter", "Noto Sans"]) else {
        eprintln!("no font with kerning; skipping");
        return;
    };
    let s = styled(&fam, 100.0);
    let (a, _) = measure(book, "A", &s);
    let (v, _) = measure(book, "V", &s);
    let (av, _) = measure(book, "AV", &s);
    assert!(av < a + v - 1.0, "{fam}: AV {av} vs A {a} + V {v}");
    // The kerned advance shows on the placed glyph too.
    let g = layout_glyphs(book, "AV", &s, 0.0);
    let line = &g.lines[0];
    assert_eq!(line.glyphs.len(), 2);
    assert!(line.glyphs[0].advance < a - 0.5, "{fam}: {:?}", line.glyphs[0]);
    assert!((line.glyphs[1].x - (line.glyphs[0].x + line.glyphs[0].advance)).abs() < 1e-9);
}

#[test]
fn ligatures_join_letters_unless_spaced() {
    let book = system();
    let Some(fam) = family_with(book, "fi\u{FB01}", &["DejaVu Sans", "DejaVu Serif", "Liberation Sans", "Noto Sans", "Inter"]) else {
        eprintln!("no font with an fi ligature; skipping");
        return;
    };
    let mut s = styled(&fam, 60.0);
    let g = layout_glyphs(book, "fit", &s, 0.0);
    let line = &g.lines[0];
    assert_eq!(line.glyphs.len(), 2, "{fam}: {:?}", line.glyphs);
    // The ligature maps back to the f; the t to its own byte.
    assert_eq!(clusters(line), [0, 2]);
    assert_eq!(line.glyphs[0].ch, 'f');
    assert_eq!(layout(book, "fit", &s, 0.0).lines[0].text, "fit");
    // Letter-spaced text keeps its letters apart.
    s.letter_spacing = 200.0;
    let g = layout_glyphs(book, "fit", &s, 0.0);
    assert_eq!(g.lines[0].glyphs.len(), 3);
}

#[test]
fn arabic_joins_and_reads_right_to_left() {
    let book = system();
    let word = "سلام"; // seen, lam, alef, meem
    let Some(fam) = family_with(book, word, &["DejaVu Sans", "Noto Sans Arabic", "Noto Naskh Arabic", "FreeSerif", "FreeSans", "Arial"]) else {
        eprintln!("no Arabic font; skipping");
        return;
    };
    let s = styled(&fam, 60.0);
    let g = layout_glyphs(book, word, &s, 0.0);
    let line = &g.lines[0];
    assert!(line.rtl);
    assert!(line.glyphs.iter().all(|g| g.rtl));
    // Display order is the reverse of reading order: the first letter is rightmost.
    let c = clusters(line);
    assert_eq!(c.first(), Some(&(word.len() - 2)), "{c:?}");
    assert_eq!(c.last(), Some(&0), "{c:?}");
    // Joined: the lam in the middle of the word is not the lam on its own.
    let alone = layout_glyphs(book, "ل", &s, 0.0).lines[0].glyphs[0].glyph_id;
    let lam = line.glyphs.iter().find(|g| g.ch == 'ل').and_then(|g| g.glyph_id);
    assert!(lam.is_some() && lam != alone, "{fam}: {lam:?} {alone:?}");
    // A right-to-left paragraph with numbers: digits stay left to right, left of the word.
    let g = layout_glyphs(book, "سلام 123", &s, 0.0);
    let c = clusters(&g.lines[0]);
    let digits: Vec<usize> = c.iter().copied().filter(|&b| b > word.len()).collect();
    assert_eq!(digits, [word.len() + 1, word.len() + 2, word.len() + 3], "{c:?}");
    assert!(c.iter().position(|&b| b == word.len() + 1) < c.iter().position(|&b| b == 0));
}

#[test]
fn hebrew_inside_latin_is_reordered() {
    let book = system();
    let Some(fam) = family_with(book, "abשלום", &["DejaVu Sans", "Noto Sans Hebrew", "FreeSans", "FreeSerif", "Arial"]) else {
        eprintln!("no Hebrew font; skipping");
        return;
    };
    let text = "ab שלום cd";
    let g = layout_glyphs(book, text, &styled(&fam, 40.0), 0.0);
    let line = &g.lines[0];
    assert!(!line.rtl);
    // a b ␠, then the Hebrew word from its last letter to its first, then ␠ c d.
    let mut want = vec![0, 1, 2];
    want.extend("שלום".char_indices().map(|(i, _)| i + 3).rev());
    want.extend([11, 12, 13]);
    assert_eq!(clusters(line), want);
    assert!(line.glyphs.windows(2).all(|w| w[1].x >= w[0].x));
}

#[test]
fn bidi_without_fonts() {
    // Reordering does not need a font.
    let book = FontBook::new();
    let g = layout_glyphs(&book, "אב cd", &style(10.0), 0.0);
    let line = &g.lines[0];
    assert!(line.rtl);
    // Right-to-left paragraph: "cd" at the left, then the space, then bet, alef.
    assert_eq!(clusters(line), [5, 6, 4, 2, 0]);
    assert!((g.width - 5.0 * 5.5).abs() < 1e-9);
}

#[test]
fn devanagari_forms_conjuncts() {
    let book = system();
    let text = "क्ष";
    let Some(fam) =
        family_with(book, text, &["Noto Sans Devanagari", "Noto Serif Devanagari", "FreeSerif", "FreeSans", "Lohit Devanagari", "Mangal"])
    else {
        eprintln!("no Devanagari font; skipping");
        return;
    };
    let g = layout_glyphs(book, text, &styled(&fam, 60.0), 0.0);
    let n = g.lines[0].glyphs.iter().filter(|g| g.glyph_id.is_some()).count();
    assert!(n < 3, "{fam}: {:?}", g.lines[0].glyphs);
}

#[test]
fn missing_characters_fall_back_to_another_face() {
    let book = system();
    let Some(latin) = family_with(book, "Ab", &["DejaVu Sans", "Liberation Sans", "Inter", "Noto Sans"]) else { return };
    let primary = book.select(&latin, false, false).unwrap();
    // A character the primary face lacks, which some other face has.
    let candidates = ['中', 'あ', 'ש', 'س', '\u{0915}', 'ক', 'ก', '\u{263A}'];
    let Some(c) =
        candidates.into_iter().find(|&c| !book.face(primary).unwrap().covers(c) && (0..book.len()).any(|i| book.face(i).unwrap().covers(c)))
    else {
        eprintln!("no character to fall back for; skipping");
        return;
    };
    let text = format!("A{c}b");
    let g = layout_glyphs(book, &text, &styled(&latin, 40.0), 0.0);
    let glyphs = &g.lines[0].glyphs;
    let a = glyphs.iter().find(|g| g.ch == 'A').unwrap();
    let other = glyphs.iter().find(|g| g.ch == c).unwrap();
    assert_eq!(a.face, Some(primary));
    assert!(other.face.is_some() && other.face != a.face, "{c}: {:?}", other.face);
    assert!(other.glyph_id.is_some_and(|id| id != 0), "{c} drawn as a missing glyph");
    assert!(!other.path.is_empty());
    // The face's font file is there for exporters.
    let (data, _) = book.face_data(other.face.unwrap()).unwrap();
    assert!(!data.is_empty());
    assert!(book.face_family(other.face.unwrap()).is_some());
}

#[test]
fn clusters_map_to_source_bytes() {
    for book in [&FontBook::new(), system()] {
        let mut s = style(20.0);
        s.uppercase = true;
        let text = "a\tß é\r\nz";
        let g = layout_glyphs(book, text, &s, 0.0);
        assert_eq!(g.lines.len(), 2);
        let l = layout(book, text, &s, 0.0);
        assert_eq!(l.lines[0].text, "A SS É");
        assert_eq!(l.lines[0].range, 0..text.find('\n').unwrap());
        assert_eq!(g.lines[1].range, text.find('z').unwrap()..text.len());
        for line in &g.lines {
            for gl in &line.glyphs {
                assert!(text.is_char_boundary(gl.cluster) && line.range.contains(&gl.cluster), "{gl:?}");
            }
        }
        // Both S of the uppercased ß point at the ß.
        let ss: Vec<usize> = g.lines[0].glyphs.iter().filter(|g| g.ch == 'S').map(|g| g.cluster).collect();
        assert_eq!(ss, [2, 2]);
        assert!((g.font_size - 20.0).abs() < 1e-9);
    }
}

#[test]
fn breaks_between_ideographs_and_at_line_separators() {
    let book = FontBook::new();
    let s = style(10.0); // 5.5 px per character
    let l = layout(&book, "中文中文中文", &s, 12.0);
    assert_eq!(l.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["中文", "中文", "中文"]);
    let l = layout(&book, "one\u{2028}two", &s, 0.0);
    assert_eq!(l.lines.len(), 2);
    assert_eq!(l.lines[1].text, "two");
    // A hyphen is a break opportunity.
    let l = layout(&book, "well-known", &s, 40.0);
    assert_eq!(l.lines[0].text, "well-");
}

#[test]
fn junk_text_lays_out() {
    // A small deterministic generator over characters from many scripts, marks and controls.
    let pool: Vec<char> = "aZ 9.,-\u{0301}\u{0308}\u{200D}\u{200C}\u{FE0F}\u{202E}\u{202B}\u{2067}\u{2069}\u{200F}\u{061C}\u{2028}\u{2029}\t\r\n\u{0}\u{7}שלוםسلام١٢٣क्षত্রไทย中文😀👍🏽\u{1F1E9}\u{1F1EA}\u{E0100}\u{10FFFF}\u{FFFD}"
        .chars()
        .collect();
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    for book in [&FontBook::new(), system()] {
        for round in 0..200 {
            let len = (round * 7) % 60;
            let text: String = (0..len)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    pool[(seed % pool.len() as u64) as usize]
                })
                .collect();
            let mut s = style(12.0 + (round % 5) as f64 * 10.0);
            s.letter_spacing = if round % 3 == 0 { 50.0 } else { 0.0 };
            s.uppercase = round % 4 == 0;
            s.align = [TextAlign::Left, TextAlign::Center, TextAlign::Right][round % 3];
            let w = [0.0, 30.0, 200.0, f64::NAN][round % 4];
            let g = layout_glyphs(book, &text, &s, w);
            let l = layout(book, &text, &s, w);
            assert_eq!(g.lines.len(), l.lines.len());
            assert!(g.width.is_finite() && g.height.is_finite());
            for line in &g.lines {
                assert!(line.range.start <= line.range.end && line.range.end <= text.len());
                for gl in &line.glyphs {
                    assert!(gl.x.is_finite() && gl.advance.is_finite() && text.is_char_boundary(gl.cluster), "{text:?} {gl:?}");
                }
            }
        }
    }
}
