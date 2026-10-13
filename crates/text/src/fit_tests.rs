//! Text that shrinks to fit its box ([`crate::fit_scale`]).

use std::sync::OnceLock;

use genha_doc::{SpanStyle, TextAlign, TextEl, TextFit, TextStyle};

use crate::{FitScale, FontBook, MIN_FIT_SCALE, fit_scale, fitted, layout_el, measure_wrapped_el, min_fit_scale};

fn system() -> &'static FontBook {
    static BOOK: OnceLock<FontBook> = OnceLock::new();
    BOOK.get_or_init(FontBook::system)
}

/// A shrinking text box, left-aligned, in a size-`size` style (no fonts: 0.55 em per character,
/// lines 1.2 em).
fn shrinking(text: &str, size: f64) -> TextEl {
    let mut t = TextEl::new(text, TextStyle { font_size: size, align: TextAlign::Left, ..TextStyle::default() });
    t.fit = TextFit::Shrink;
    t
}

#[test]
fn shrinks_to_the_largest_scale_that_fits() {
    let book = FontBook::new();
    // 18 characters at 55 px: 990 px wide, so in a 600 px box it wraps into 2 lines (240 px) and
    // overflows a 130 px box. On one line it fits at 600 / 990.
    let t = shrinking("Grand opening sale", 100.0);
    let r = fit_scale(&book, &t, 600.0, 130.0);
    assert!(r.fits);
    assert!(r.scale <= 600.0 / 990.0 && r.scale > 600.0 / 990.0 - 0.002, "{r:?}");
    let f = fitted(&book, &t, 600.0, 130.0);
    assert_eq!(layout_el(&book, &f, 600.0).lines.len(), 1);
    // A little bigger does not fit.
    let (_, h) = measure_wrapped_el(&book, &t.scaled(r.scale + 0.01), 600.0);
    assert!(h > 130.5, "{h}");
    // The stored sizes are untouched.
    assert_eq!(t.style.font_size, 100.0);
    // Text that fits stays as set (and is not copied).
    assert_eq!(fit_scale(&book, &t, 1000.0, 130.0), FitScale::ONE);
    assert!(matches!(fitted(&book, &t, 1000.0, 130.0), std::borrow::Cow::Borrowed(_)));
    // Text that does not shrink is never scaled.
    let plain = TextEl { fit: TextFit::None, ..t.clone() };
    assert_eq!(fit_scale(&book, &plain, 600.0, 130.0), FitScale::ONE);
}

#[test]
fn bigger_boxes_never_shrink_text_more() {
    let book = FontBook::new();
    let t = shrinking("Summer reading list for the whole family", 80.0);
    for w in [300.0, 500.0, 900.0] {
        let mut last = 0.0;
        for h in (40..600).step_by(20) {
            let r = fit_scale(&book, &t, w, f64::from(h));
            assert!(r.scale + 1e-9 >= last, "w {w} h {h}: {} after {last}", r.scale);
            assert!((min_fit_scale(&t)..=1.0).contains(&r.scale));
            last = r.scale;
        }
    }
}

#[test]
fn never_below_the_minimum() {
    let book = FontBook::new();
    let long = "word ".repeat(400);
    let t = shrinking(&long, 50.0);
    let r = fit_scale(&book, &t, 300.0, 60.0);
    assert_eq!(r, FitScale { scale: MIN_FIT_SCALE, fits: false });
    // Small type keeps at least 6 px; type set smaller than that does not shrink.
    assert!((min_fit_scale(&shrinking("x", 10.0)) - 0.6).abs() < 1e-12);
    assert_eq!(min_fit_scale(&shrinking("x", 5.0)), 1.0);
    let r = fit_scale(&book, &shrinking(&long, 5.0), 300.0, 10.0);
    assert_eq!(r, FitScale { scale: 1.0, fits: false });
}

#[test]
fn words_are_not_broken_when_shrinking_can_avoid_it() {
    let book = FontBook::new();
    // One 13-letter word (715 px) in a tall 400 px box: it would fit by breaking the word; it
    // shrinks until the word fits the width instead.
    let t = shrinking("Extraordinary", 100.0);
    let r = fit_scale(&book, &t, 400.0, 1000.0);
    assert!(r.fits && r.scale <= 400.0 / 715.0 && r.scale > 400.0 / 715.0 - 0.002, "{r:?}");
    // A word too long even at the minimum: broken (only the height counts), not shrunk further.
    let t = shrinking(&"x".repeat(60), 100.0);
    let r = fit_scale(&book, &t, 400.0, 2000.0);
    assert_eq!(r, FitScale::ONE);
}

#[test]
fn style_runs_scale_together() {
    let book = FontBook::new();
    let mut t = shrinking("Big sale today only", 60.0);
    t.set_style_range(4..8, &SpanStyle { font_size: Some(120.0), bold: Some(true), ..SpanStyle::default() });
    t.style.letter_spacing = 100.0;
    let r = fit_scale(&book, &t, 500.0, 160.0);
    assert!(r.fits && r.scale < 1.0, "{r:?}");
    let f = fitted(&book, &t, 500.0, 160.0);
    assert!((f.style.font_size - 60.0 * r.scale).abs() < 1e-9);
    let span = f.spans.iter().find_map(|s| s.style.font_size).unwrap();
    assert!((span - 120.0 * r.scale).abs() < 1e-9);
    // Letter spacing is in ems: it follows the sizes.
    assert_eq!(f.style.letter_spacing, 100.0);
    let (_, h) = measure_wrapped_el(&book, &f, 500.0);
    assert!(h <= 160.5, "{h}");
}

#[test]
fn empty_huge_and_junk_input() {
    let book = FontBook::new();
    for text in ["", "   ", "\n\n"] {
        assert_eq!(fit_scale(&book, &shrinking(text, 100.0), 10.0, 10.0), FitScale::ONE, "{text:?}");
    }
    // Huge text: bounded work, a usable answer.
    let huge = shrinking(&"lorem ipsum ".repeat(4000), 40.0);
    let r = fit_scale(&book, &huge, 800.0, 600.0);
    assert!(!r.fits && r.scale == MIN_FIT_SCALE);
    // Not-a-number anywhere: no panic, a scale in range.
    let mut t = shrinking("Hello there", f64::NAN);
    t.style.line_height = f64::NAN;
    t.style.letter_spacing = f64::INFINITY;
    t.curve = f64::NAN;
    for (w, h) in [(f64::NAN, 100.0), (100.0, f64::INFINITY), (-5.0, 10.0), (0.0, 0.0), (1e300, 1e-300), (50.0, 20.0)] {
        let r = fit_scale(&book, &t, w, h);
        assert!(r.scale.is_finite() && r.scale > 0.0 && r.scale <= 1.0, "{w} {h}: {r:?}");
    }
    let mut t = shrinking("Hello there", 1e308);
    t.set_style_range(0..2, &SpanStyle { font_size: Some(f64::MAX), ..SpanStyle::default() });
    let r = fit_scale(&book, &t, 100.0, 100.0);
    assert!(r.scale.is_finite());
}

#[test]
fn bent_text_fits_its_arc() {
    let book = FontBook::new();
    let mut t = shrinking("Fresh bread daily", 40.0);
    t.curve = 120.0;
    // A stand-in for the arc's size: the lines' size, 20% taller.
    let size = |el: &TextEl| {
        let (w, h) = measure_wrapped_el(&book, el, 300.0);
        Some((w, h * 1.2))
    };
    let r = crate::fit_scale_by(&book, &t, 300.0, 60.0, &size);
    assert!(r.fits && r.scale < 1.0);
    let f = t.scaled(r.scale);
    // On one arc (it did not wrap), within the box.
    assert_eq!(layout_el(&book, &f, 300.0).lines.len(), 1);
    let (w, h) = size(&f).unwrap();
    assert!(w <= 300.5 && h <= 60.5, "{w}×{h}");
}

#[test]
fn shrunk_text_fits_with_real_fonts() {
    let book = system();
    if book.is_empty() {
        eprintln!("no system fonts: skipped");
        return;
    }
    let mut t = shrinking("Grand opening weekend sale", 120.0);
    t.style.bold = true;
    for (w, h) in [(500.0, 150.0), (900.0, 150.0), (300.0, 400.0)] {
        let r = fit_scale(book, &t, w, h);
        let f = fitted(book, &t, w, h);
        let l = layout_el(book, &f, w);
        assert!(r.fits && l.height <= h + 0.5, "{w}×{h}: {r:?}, {} px", l.height);
        assert!(l.lines.iter().all(|line| line.width <= w + 0.5));
        // Cached: the same answer again.
        assert_eq!(fit_scale(book, &t, w, h), r);
    }
}
