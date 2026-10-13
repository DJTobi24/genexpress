//! Text sizes for the re-layout, from the real text layout (`genha_text`), memoised by
//! (element, type scale, width).
//!
//! Text is measured as it will be drawn: scaled first, then laid out (shaping rounds positions,
//! so a scaled measurement of the original can be off by a pixel on a long line).

use std::cell::RefCell;
use std::collections::HashMap;

use genha_doc::TextEl;
use genha_text::FontBook;

/// Widths are cached to a quarter pixel.
const QUANTUM: f64 = 4.0;

type Key = (usize, u64);

pub(crate) struct Measurer<'a> {
    fonts: &'a FontBook,
    wrapped: RefCell<HashMap<(Key, i64), (f64, f64)>>,
    words: RefCell<HashMap<Key, f64>>,
}

/// `t` with its sizes scaled by `k` (itself when `k` is 1).
fn scaled(t: &TextEl, k: f64) -> std::borrow::Cow<'_, TextEl> {
    if k == 1.0 {
        return std::borrow::Cow::Borrowed(t);
    }
    let mut c = t.clone();
    c.scale_sizes(k);
    std::borrow::Cow::Owned(c)
}

impl<'a> Measurer<'a> {
    pub fn new(fonts: &'a FontBook) -> Self {
        Measurer { fonts, wrapped: RefCell::default(), words: RefCell::default() }
    }

    /// (widest line, height) of `t` (cache key `key`) with its sizes scaled by `k`, wrapped in a
    /// box `width` wide (not wrapped when `width` is infinite).
    pub fn wrapped(&self, key: usize, t: &TextEl, k: f64, width: f64) -> (f64, f64) {
        let k = if k.is_finite() && k > 0.0 { k } else { 1.0 };
        // Rounded down: never measure wider than the box will be.
        let q = if width.is_finite() { (width * QUANTUM + 1e-6).floor().clamp(-1e15, 1e15) as i64 } else { i64::MAX };
        let id = ((key, k.to_bits()), q);
        if let Some(v) = self.wrapped.borrow().get(&id).copied() {
            return v;
        }
        let wrap = if q == i64::MAX { f64::INFINITY } else { q as f64 / QUANTUM };
        let v = genha_text::measure_wrapped_el(self.fonts, &scaled(t, k), wrap);
        self.wrapped.borrow_mut().insert(id, v);
        v
    }

    /// The widest single word of `t` (key `key`) with its sizes scaled by `k`: the narrowest box
    /// the text fits without breaking a word.
    pub fn longest_word(&self, key: usize, t: &TextEl, k: f64) -> f64 {
        let k = if k.is_finite() && k > 0.0 { k } else { 1.0 };
        let id = (key, k.to_bits());
        if let Some(v) = self.words.borrow().get(&id).copied() {
            return v;
        }
        let v = longest_word(self.fonts, &scaled(t, k));
        self.words.borrow_mut().insert(id, v);
        v
    }
}

/// Words are measured one by one up to this many (long texts: the rest are estimated from the
/// widest so far, which is plenty for a box width).
const MAX_WORDS: usize = 400;

/// The widest whitespace-separated word of `t`, in its own sizes and styles.
pub(crate) fn longest_word(fonts: &FontBook, t: &TextEl) -> f64 {
    let runs = if t.spans.is_empty() { vec![] } else { t.style_runs() };
    let mut widest: f64 = 0.0;
    let mut seen = 0usize;
    let mut start: Option<usize> = None;
    let measure = |a: usize, b: usize, widest: &mut f64| {
        let Some(word) = t.text.get(a..b) else { return };
        let local: Vec<_> =
            runs.iter().filter(|(r, _)| r.start < b && r.end > a).map(|(r, st)| (r.start.max(a) - a..r.end.min(b) - a, st.clone())).collect();
        let (w, _) = genha_text::measure_rich(fonts, word, &t.style, &local);
        if w.is_finite() {
            *widest = widest.max(w);
        }
    };
    for (i, c) in t.text.char_indices() {
        if c.is_whitespace() {
            if let Some(a) = start.take() {
                seen += 1;
                if seen <= MAX_WORDS {
                    measure(a, i, &mut widest);
                }
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(a) = start
        && seen < MAX_WORDS
    {
        measure(a, t.text.len(), &mut widest);
    }
    widest
}

#[cfg(test)]
mod tests {
    use super::*;
    use genha_doc::{TextAlign, TextStyle};

    #[test]
    fn scaled_measurements_match_scaled_text() {
        let book = FontBook::new();
        let t = TextEl::new("Fresh drops every Friday", TextStyle { font_size: 40.0, align: TextAlign::Left, ..TextStyle::default() });
        let m = Measurer::new(&book);
        let mut big = t.clone();
        big.scale_sizes(2.0);
        let direct = genha_text::measure_wrapped_el(&book, &big, 500.0);
        let (w, h) = m.wrapped(0, &t, 2.0, 500.0);
        assert!((w - direct.0).abs() < 1e-6 && (h - direct.1).abs() < 1e-6, "{w} {h} {direct:?}");
        // Cached: the same answer again.
        assert_eq!(m.wrapped(0, &t, 2.0, 500.0), (w, h));
        // "Friday" and "drops" are the widest words at 0.55 em per character.
        assert!((m.longest_word(0, &t, 1.0) - 6.0 * 0.55 * 40.0).abs() < 1e-6);
        assert!((m.longest_word(0, &t, 0.5) - 3.0 * 0.55 * 40.0).abs() < 1e-6);
    }
}
