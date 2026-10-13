//! Text that shrinks to fit its box ([`TextFit::Shrink`]).
//!
//! The stored sizes never change: when a shrinking text box is laid out, [`fit_scale`] finds the
//! largest scale (at most 1) by which all its sizes can be multiplied together (the box's and every
//! style run's; letter spacing, in ems, follows) so that the text fits the box: its lines are no
//! taller than the box and no word has to be broken between letters (when that cannot be avoided,
//! only the height counts). The scale does not go below [`min_fit_scale`] (40% of the set size, and
//! never below [`MIN_FIT_SIZE`] pixels); text that does not fit even then overflows, and the result
//! says so ([`FitScale::fits`]).
//!
//! The search is a bisection on the scale, each step measuring the text with [`crate::layout`]'s
//! line breaking (shaping is size-independent and cached per face, so a step only re-breaks lines).
//! Results are cached in the [`FontBook`] by text, styles and box size, so drawing the same box
//! again costs a hash. Every output (canvas, PNG, SVG, PDF, the text editor, smart resize) lays the
//! text out at the same scale through this function ([`fitted`]); bent text measures its arc with
//! [`fit_scale_by`] (see `genha_render::text_fit`).

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::sync::{Mutex, PoisonError};

use genha_doc::{TextEl, TextFit};

use crate::FontBook;

/// The smallest fraction of its set size shrinking text is set in.
pub const MIN_FIT_SCALE: f64 = 0.4;
/// Shrinking text is never set smaller than this (pixels), unless it was set smaller.
pub const MIN_FIT_SIZE: f64 = 6.0;
/// How close the bisection gets to the largest scale that fits.
const PRECISION: f64 = 1.0 / 1024.0;
/// Text may stick out of its box by this much (pixels) and still fit.
const TOLERANCE: f64 = 0.5;
/// Cached results per font book; the cache starts over when it is full.
const MAX_CACHED: usize = 4096;

/// The scale a text box's sizes are multiplied by to fit its box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitScale {
    /// `min_fit_scale..=1`; 1 when the text fits as set (or does not shrink).
    pub scale: f64,
    /// False when the text overflows its box even at the smallest scale allowed.
    pub fits: bool,
}

impl FitScale {
    /// As set: no scaling.
    pub const ONE: FitScale = FitScale { scale: 1.0, fits: true };
}

/// Results of [`fit_scale`] by key (see [`key`]).
#[derive(Default)]
pub(crate) struct FitCache {
    map: Mutex<HashMap<u64, FitScale>>,
}

impl FitCache {
    fn get(&self, key: u64) -> Option<FitScale> {
        self.map.lock().unwrap_or_else(PoisonError::into_inner).get(&key).copied()
    }

    fn put(&self, key: u64, v: FitScale) {
        let mut m = self.map.lock().unwrap_or_else(PoisonError::into_inner);
        if m.len() >= MAX_CACHED {
            m.clear();
        }
        m.insert(key, v);
    }
}

/// The smallest scale `t` may be shrunk to: [`MIN_FIT_SCALE`], or more so that the box's size
/// stays at least [`MIN_FIT_SIZE`] pixels (1 for text set smaller than that).
pub fn min_fit_scale(t: &TextEl) -> f64 {
    let size = t.style.font_size;
    let size = if size.is_finite() && size > 0.0 { size } else { 48.0 };
    (MIN_FIT_SIZE / size).clamp(MIN_FIT_SCALE, 1.0)
}

/// The scale `t`'s sizes are multiplied by to fit a `width` × `height` box: 1 unless it shrinks
/// to fit ([`TextFit::Shrink`]) and does not fit as set. Straight text is measured as wrapped
/// lines; bent text should go through [`fit_scale_by`] with its arc's size
/// (`genha_render::text_fit` does).
pub fn fit_scale(book: &FontBook, t: &TextEl, width: f64, height: f64) -> FitScale {
    fit_impl(book, t, width, height, None)
}

/// [`fit_scale`] for text whose size is not that of its wrapped lines (bent text): `size` gives
/// the (width, height) `t` scaled takes in a box `width` wide (None: as straight lines). The text
/// fits when that is within the box and, if some scale allows it, no word is broken and no line
/// wraps.
pub fn fit_scale_by(book: &FontBook, t: &TextEl, width: f64, height: f64, size: &dyn Fn(&TextEl) -> Option<(f64, f64)>) -> FitScale {
    fit_impl(book, t, width, height, Some(size))
}

/// `t` as it is laid out in a `width` × `height` box: scaled by [`fit_scale`] (borrowed when it
/// fits as set).
pub fn fitted<'a>(book: &FontBook, t: &'a TextEl, width: f64, height: f64) -> Cow<'a, TextEl> {
    t.scaled(fit_scale(book, t, width, height).scale)
}

type SizeFn<'f> = &'f dyn Fn(&TextEl) -> Option<(f64, f64)>;

fn fit_impl(book: &FontBook, t: &TextEl, width: f64, height: f64, size: Option<SizeFn<'_>>) -> FitScale {
    if t.fit != TextFit::Shrink || t.text.trim().is_empty() {
        return FitScale::ONE;
    }
    if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
        return FitScale::ONE;
    }
    let key = key(t, width, height, size.is_some());
    if let Some(v) = book.fits.get(key) {
        return v;
    }
    // Bent text should not wrap either (its lines become arcs the design did not plan).
    let paragraphs = t.text.split('\n').count();
    let probe = |s: f64| -> (bool, bool) {
        let el = t.scaled(s);
        let m = crate::layout::measure_fit_el(book, &el, width);
        let (w, h, broke) = match size.and_then(|f| f(&el)) {
            Some((w, h)) => (w, h, m.broke_word || m.lines > paragraphs),
            None => (m.width, m.height, m.broke_word),
        };
        let inside = w.is_finite() && h.is_finite() && w <= width + TOLERANCE && h <= height + TOLERANCE;
        (inside, broke)
    };
    let v = search(min_fit_scale(t), probe);
    book.fits.put(key, v);
    v
}

/// The largest scale in `min..=1` at which `probe` (scale → (inside the box, a word broken))
/// says the text fits, by bisection. A broken word only counts when some scale avoids it.
fn search(min: f64, mut probe: impl FnMut(f64) -> (bool, bool)) -> FitScale {
    let strict = |r: (bool, bool)| r.0 && !r.1;
    let loose = |r: (bool, bool)| r.0;
    let one = probe(1.0);
    if strict(one) {
        return FitScale::ONE;
    }
    if min >= 1.0 {
        return FitScale { scale: 1.0, fits: one.0 };
    }
    let low = probe(min);
    let ok: &dyn Fn((bool, bool)) -> bool = if strict(low) {
        &strict
    } else if loose(low) {
        if loose(one) {
            return FitScale::ONE;
        }
        &loose
    } else {
        return FitScale { scale: min, fits: false };
    };
    let (mut lo, mut hi) = (min, 1.0);
    while hi - lo > PRECISION {
        let mid = (lo + hi) / 2.0;
        if ok(probe(mid)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    FitScale { scale: lo, fits: true }
}

/// What a fit depends on: the text, its styles, its curve, the box and how it is measured.
fn key(t: &TextEl, width: f64, height: f64, bent: bool) -> u64 {
    struct W<'h>(&'h mut DefaultHasher);
    impl std::fmt::Write for W<'_> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.0.write(s.as_bytes());
            Ok(())
        }
    }
    let mut h = DefaultHasher::new();
    h.write(t.text.as_bytes());
    h.write_u8(0xff);
    // Formatting into a hash cannot fail.
    let _ = std::fmt::write(&mut W(&mut h), format_args!("{:?}{:?}", t.style, t.spans));
    h.write_u64(t.curve.to_bits());
    h.write_u64(width.to_bits());
    h.write_u64(height.to_bits());
    h.write_u8(u8::from(bent));
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_the_boundary() {
        // Fits below 0.7 (monotonic).
        let r = search(0.4, |s| (s <= 0.7, false));
        assert!(r.fits && (0.7 - PRECISION..=0.7).contains(&r.scale), "{r:?}");
        assert_eq!(search(0.4, |_| (true, false)), FitScale::ONE);
        // Nothing fits: the minimum, flagged.
        assert_eq!(search(0.4, |_| (false, false)), FitScale { scale: 0.4, fits: false });
        // A word is broken at every scale: only the height counts.
        let r = search(0.4, |s| (s <= 0.5, true));
        assert!(r.fits && r.scale <= 0.5 && r.scale > 0.49, "{r:?}");
        // Words stay whole below 0.6 although the height fits below 0.9.
        let r = search(0.4, |s| (s <= 0.9, s > 0.6));
        assert!(r.scale <= 0.6 && r.scale > 0.59, "{r:?}");
        // No room to shrink.
        assert_eq!(search(1.0, |_| (false, false)), FitScale { scale: 1.0, fits: false });
    }
}
