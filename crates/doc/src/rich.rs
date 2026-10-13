//! Style runs: formatting part of a text box (one word bold, another colour, size or font).
//!
//! A [`TextEl`] has its own [`TextStyle`] for the whole box and a list of [`TextSpan`]s, each
//! overriding some of the style's values ([`SpanStyle`]) for a range of characters. Ranges count
//! `char`s of [`TextEl::text`] (as the text editor does), sorted, not overlapping, not empty and
//! inside the text; [`TextEl::normalize_spans`] (run by `Document::repair`) restores that for
//! untrusted input. Paragraph-wide values (alignment, line height, letter spacing, upper case) stay
//! on the box's style.
//!
//! Editing the text moves the spans with it ([`TextEl::replace_range`],
//! [`TextEl::set_text_keeping_styles`]): text typed at the caret takes the style of the character
//! before it.

use std::ops::Range;

use genha_color::Color;
use serde::{Deserialize, Serialize};

use crate::{TextEl, TextStyle};

/// Most style runs a text box keeps (further ones are dropped by repair).
pub const MAX_SPANS: usize = 2000;
/// Longest font family name kept in a span, in bytes.
const MAX_FAMILY: usize = 200;

/// Values that override the box's [`TextStyle`] for part of the text; `None` keeps the box's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpanStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// Pixels (1–2000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

/// The style values a span can override (the JSON field names).
pub const SPAN_FIELDS: [&str; 7] = ["font_family", "font_size", "bold", "italic", "underline", "strikethrough", "color"];

impl SpanStyle {
    pub fn is_empty(&self) -> bool {
        *self == SpanStyle::default()
    }

    /// `base` with these overrides applied.
    pub fn apply(&self, base: &TextStyle) -> TextStyle {
        let mut s = base.clone();
        if let Some(v) = &self.font_family {
            s.font_family = v.clone();
        }
        if let Some(v) = self.font_size {
            s.font_size = v;
        }
        if let Some(v) = self.bold {
            s.bold = v;
        }
        if let Some(v) = self.italic {
            s.italic = v;
        }
        if let Some(v) = self.underline {
            s.underline = v;
        }
        if let Some(v) = self.strikethrough {
            s.strikethrough = v;
        }
        if let Some(v) = self.color {
            s.color = v;
        }
        s
    }

    /// These overrides with `patch`'s on top.
    pub fn merged(&self, patch: &SpanStyle) -> SpanStyle {
        SpanStyle {
            font_family: patch.font_family.clone().or_else(|| self.font_family.clone()),
            font_size: patch.font_size.or(self.font_size),
            bold: patch.bold.or(self.bold),
            italic: patch.italic.or(self.italic),
            underline: patch.underline.or(self.underline),
            strikethrough: patch.strikethrough.or(self.strikethrough),
            color: patch.color.or(self.color),
        }
    }

    /// These overrides without the values `patch` sets (they now come from the box's style).
    pub fn without(&self, patch: &SpanStyle) -> SpanStyle {
        SpanStyle {
            font_family: if patch.font_family.is_some() { None } else { self.font_family.clone() },
            font_size: if patch.font_size.is_some() { None } else { self.font_size },
            bold: if patch.bold.is_some() { None } else { self.bold },
            italic: if patch.italic.is_some() { None } else { self.italic },
            underline: if patch.underline.is_some() { None } else { self.underline },
            strikethrough: if patch.strikethrough.is_some() { None } else { self.strikethrough },
            color: if patch.color.is_some() { None } else { self.color },
        }
    }

    /// Only the overrides that differ from `base`, with unusable values dropped (a size that is
    /// not a number in 1–2000, an empty or very long family name).
    pub fn reduced(&self, base: &TextStyle) -> SpanStyle {
        let family = self.font_family.clone().filter(|f| !f.trim().is_empty() && f.len() <= MAX_FAMILY && *f != base.font_family);
        let size = self.font_size.filter(|v| v.is_finite() && (1.0..=2000.0).contains(v) && *v != base.font_size);
        SpanStyle {
            font_family: family,
            font_size: size,
            bold: self.bold.filter(|v| *v != base.bold),
            italic: self.italic.filter(|v| *v != base.italic),
            underline: self.underline.filter(|v| *v != base.underline),
            strikethrough: self.strikethrough.filter(|v| *v != base.strikethrough),
            color: self.color.filter(|v| *v != base.color),
        }
    }

    /// The span-level values of `style`, all set.
    pub fn of(style: &TextStyle) -> SpanStyle {
        SpanStyle {
            font_family: Some(style.font_family.clone()),
            font_size: Some(style.font_size),
            bold: Some(style.bold),
            italic: Some(style.italic),
            underline: Some(style.underline),
            strikethrough: Some(style.strikethrough),
            color: Some(style.color),
        }
    }
}

/// Characters `start..end` of a text box's text set with `style` on top of the box's style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    /// First character (an index in characters, not bytes).
    pub start: usize,
    /// Just past the last character.
    pub end: usize,
    #[serde(flatten)]
    pub style: SpanStyle,
}

impl TextSpan {
    pub fn new(range: Range<usize>, style: SpanStyle) -> Self {
        TextSpan { start: range.start, end: range.end, style }
    }
}

/// The style of a range of text for the UI: the values of its first character and which of the
/// span-level values (as in [`SPAN_FIELDS`]) vary over the range.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeStyle {
    pub style: TextStyle,
    pub mixed: Vec<&'static str>,
}

impl RangeStyle {
    pub fn is_mixed(&self, field: &str) -> bool {
        self.mixed.contains(&field)
    }
}

/// Characters in `s`.
fn chars(s: &str) -> usize {
    s.chars().count()
}

/// Byte offsets of every character boundary of `s` (its length last).
fn boundaries(s: &str) -> Vec<usize> {
    s.char_indices().map(|(b, _)| b).chain(std::iter::once(s.len())).collect()
}

/// Runs (length in characters, overrides) to spans: empty overrides dropped, equal neighbours
/// merged.
fn spans_from_runs(runs: &[(usize, SpanStyle)]) -> Vec<TextSpan> {
    let mut out: Vec<TextSpan> = vec![];
    let mut at = 0usize;
    for (len, st) in runs {
        let (start, end) = (at, at.saturating_add(*len));
        at = end;
        if *len == 0 || st.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.end == start && last.style == *st => last.end = end,
            _ => out.push(TextSpan { start, end, style: st.clone() }),
        }
    }
    out
}

impl TextEl {
    /// Every character's overrides as runs covering the whole text: (length in characters,
    /// overrides), in order. Assumes normalized spans (out-of-range parts are ignored).
    fn runs(&self) -> Vec<(usize, SpanStyle)> {
        let n = chars(&self.text);
        let mut out = vec![];
        let mut at = 0usize;
        for sp in &self.spans {
            let (s, e) = (sp.start.max(at).min(n), sp.end.min(n));
            if e <= s {
                continue;
            }
            if s > at {
                out.push((s - at, SpanStyle::default()));
            }
            out.push((e - s, sp.style.clone()));
            at = e;
        }
        if n > at {
            out.push((n - at, SpanStyle::default()));
        }
        out
    }

    /// Splits `runs` so a run starts at character `i`; returns the index of that run.
    fn split_at(runs: &mut Vec<(usize, SpanStyle)>, i: usize) -> usize {
        let mut at = 0usize;
        for k in 0..runs.len() {
            let Some((len, st)) = runs.get(k).cloned() else { break };
            if i == at {
                return k;
            }
            if i < at + len {
                if let Some(r) = runs.get_mut(k) {
                    r.0 = i - at;
                }
                runs.insert(k + 1, (at + len - i, st));
                return k + 1;
            }
            at += len;
        }
        runs.len()
    }

    /// Whether the spans are in order, inside the text, not empty, not overlapping and not
    /// redundant (what [`normalize_spans`](Self::normalize_spans) makes them).
    pub fn spans_normalized(&self) -> bool {
        let n = chars(&self.text);
        let mut at = 0usize;
        let mut prev: Option<&TextSpan> = None;
        for sp in &self.spans {
            if sp.start < at || sp.end <= sp.start || sp.end > n || sp.style.is_empty() || sp.style.reduced(&self.style) != sp.style {
                return false;
            }
            if prev.is_some_and(|p| p.end == sp.start && p.style == sp.style) {
                return false;
            }
            at = sp.end;
            prev = Some(sp);
        }
        self.spans.len() <= MAX_SPANS
    }

    /// Puts the spans in order, inside the text, without overlaps (a later span's values win),
    /// empty or redundant overrides and equal neighbours. True when anything changed.
    pub fn normalize_spans(&mut self) -> bool {
        if self.spans_normalized() {
            return false;
        }
        let n = chars(&self.text);
        self.spans.truncate(MAX_SPANS);
        let mut cuts: Vec<usize> = vec![0, n];
        for sp in &self.spans {
            cuts.push(sp.start.min(n));
            cuts.push(sp.end.min(n));
        }
        cuts.sort_unstable();
        cuts.dedup();
        let mut runs: Vec<(usize, SpanStyle)> = vec![];
        for w in cuts.windows(2) {
            let (Some(&a), Some(&b)) = (w.first(), w.get(1)) else { continue };
            let mut st = SpanStyle::default();
            for sp in self.spans.iter().filter(|sp| sp.start <= a && b <= sp.end) {
                st = st.merged(&sp.style);
            }
            runs.push((b - a, st.reduced(&self.style)));
        }
        self.spans = spans_from_runs(&runs);
        true
    }

    /// The overrides of character `i` (none past the end).
    pub fn span_style_at(&self, i: usize) -> SpanStyle {
        self.spans.iter().find(|sp| sp.start <= i && i < sp.end).map(|sp| sp.style.clone()).unwrap_or_default()
    }

    /// The style character `i` is set in (the box's past the end).
    pub fn style_at(&self, i: usize) -> TextStyle {
        self.span_style_at(i).apply(&self.style)
    }

    /// The overrides text typed at caret `i` takes: those of the character before it (the
    /// first character's at the start).
    pub fn caret_style(&self, i: usize) -> SpanStyle {
        self.span_style_at(if i > 0 { i - 1 } else { 0 })
    }

    /// The style of characters `range` (either order): the first character's (the one before a
    /// caret when the range is empty), and which span-level values vary.
    pub fn range_style(&self, range: Range<usize>) -> RangeStyle {
        let (a, b) = (range.start.min(range.end), range.start.max(range.end));
        if a == b || self.spans.is_empty() {
            let st = if a == b { self.caret_style(a) } else { self.span_style_at(a) };
            return RangeStyle { style: st.apply(&self.style), mixed: vec![] };
        }
        let first = self.span_style_at(a).apply(&self.style);
        let mut mixed: Vec<&'static str> = vec![];
        let mut at = 0usize;
        for (len, st) in self.runs() {
            let (s, e) = (at, at + len);
            at = e;
            if e <= a || s >= b {
                continue;
            }
            let r = st.apply(&self.style);
            let diffs = [
                ("font_family", r.font_family != first.font_family),
                ("font_size", r.font_size != first.font_size),
                ("bold", r.bold != first.bold),
                ("italic", r.italic != first.italic),
                ("underline", r.underline != first.underline),
                ("strikethrough", r.strikethrough != first.strikethrough),
                ("color", r.color != first.color),
            ];
            for (f, d) in diffs {
                if d && !mixed.contains(&f) {
                    mixed.push(f);
                }
            }
        }
        RangeStyle { style: first, mixed }
    }

    /// Sets `patch` on characters `range` (clamped to the text).
    pub fn set_style_range(&mut self, range: Range<usize>, patch: &SpanStyle) {
        let n = chars(&self.text);
        let (a, b) = (range.start.min(n), range.end.min(n));
        if a >= b || patch.is_empty() {
            return;
        }
        let mut runs = self.runs();
        let i = Self::split_at(&mut runs, a);
        let j = Self::split_at(&mut runs, b);
        if let Some(rs) = runs.get_mut(i..j) {
            for r in rs {
                r.1 = r.1.merged(patch);
            }
        }
        let base = self.style.clone();
        for r in &mut runs {
            r.1 = r.1.reduced(&base);
        }
        self.spans = spans_from_runs(&runs);
        self.spans.truncate(MAX_SPANS);
    }

    /// Sets `patch` on the whole box: its style takes the values and no span overrides them.
    pub fn set_style_all(&mut self, patch: &SpanStyle) {
        self.style = patch.apply(&self.style);
        for sp in &mut self.spans {
            sp.style = sp.style.without(patch);
        }
        self.spans.retain(|sp| !sp.style.is_empty());
        self.normalize_spans();
    }

    /// Replaces characters `range` (clamped) with `insert`; the spans move with the text and the
    /// new text takes `style` (by default the style at the caret, see
    /// [`caret_style`](Self::caret_style)).
    pub fn replace_range(&mut self, range: Range<usize>, insert: &str, style: Option<SpanStyle>) {
        let b = boundaries(&self.text);
        let n = b.len().saturating_sub(1);
        let (a, e) = (range.start.min(n), range.end.min(n).max(range.start.min(n)));
        let new_style = style.unwrap_or_else(|| if a > 0 { self.span_style_at(a - 1) } else { self.span_style_at(e) });
        let (ba, be) = (b.get(a).copied().unwrap_or(self.text.len()), b.get(e).copied().unwrap_or(self.text.len()));
        let mut runs = self.runs();
        let i = Self::split_at(&mut runs, a);
        let j = Self::split_at(&mut runs, e);
        let inserted = chars(insert);
        runs.splice(i..j.max(i), std::iter::once((inserted, new_style)));
        self.text.replace_range(ba..be, insert);
        let base = self.style.clone();
        for r in &mut runs {
            r.1 = r.1.reduced(&base);
        }
        self.spans = spans_from_runs(&runs);
        self.spans.truncate(MAX_SPANS);
    }

    /// Changes the text to `new`, moving the spans with the unchanged text before and after the
    /// change. `caret` (where the caret is after the change, when known) places the change where
    /// repeated letters make it ambiguous; the new text takes `style` (default: the style at the
    /// caret).
    pub fn set_text_keeping_styles(&mut self, new: &str, caret: Option<usize>, style: Option<SpanStyle>) {
        if new == self.text {
            return;
        }
        if self.spans.is_empty() && style.as_ref().is_none_or(SpanStyle::is_empty) {
            self.text = new.to_string();
            return;
        }
        let old: Vec<char> = self.text.chars().collect();
        let nw: Vec<char> = new.chars().collect();
        let mut p = old.iter().zip(&nw).take_while(|(x, y)| x == y).count();
        if let Some(c) = caret {
            p = p.min(c);
        }
        let mut s = old.iter().rev().zip(nw.iter().rev()).take_while(|(x, y)| x == y).count();
        s = s.min(old.len() - p).min(nw.len() - p);
        if let Some(c) = caret {
            s = s.min(nw.len().saturating_sub(c));
        }
        let inserted: String = nw.get(p..nw.len() - s).unwrap_or(&[]).iter().collect();
        self.replace_range(p..old.len() - s, &inserted, style);
    }

    /// Scales the font sizes, the box's and its runs', by `k` (at least 1 px; runs at most
    /// 2000 px), as resizing a design does.
    pub fn scale_sizes(&mut self, k: f64) {
        if !k.is_finite() || k <= 0.0 {
            return;
        }
        self.style.font_size = (self.style.font_size * k).max(1.0);
        for sp in &mut self.spans {
            if let Some(v) = &mut sp.style.font_size {
                *v = (*v * k).clamp(1.0, 2000.0);
            }
        }
        self.normalize_spans();
    }

    /// The text cut into runs of one style: (byte range in [`text`](Self::text), style), covering
    /// it all, in order. One run in the box's style when nothing is formatted.
    pub fn style_runs(&self) -> Vec<(Range<usize>, TextStyle)> {
        if self.spans.is_empty() {
            return vec![(0..self.text.len(), self.style.clone())];
        }
        let b = boundaries(&self.text);
        let mut out = vec![];
        let mut at = 0usize;
        for (len, st) in self.runs() {
            let (s, e) = (at, at + len);
            at = e;
            let (Some(&bs), Some(&be)) = (b.get(s), b.get(e)) else { continue };
            out.push((bs..be, st.apply(&self.style)));
        }
        if out.is_empty() {
            out.push((0..self.text.len(), self.style.clone()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(text: &str) -> TextEl {
        TextEl::new(text, TextStyle::default())
    }

    fn bold() -> SpanStyle {
        SpanStyle { bold: Some(true), ..SpanStyle::default() }
    }

    fn red() -> SpanStyle {
        SpanStyle { color: Some(Color::rgb(255, 0, 0)), ..SpanStyle::default() }
    }

    fn ranges(t: &TextEl) -> Vec<(usize, usize)> {
        t.spans.iter().map(|s| (s.start, s.end)).collect()
    }

    #[test]
    fn style_one_word_and_merge_neighbours() {
        let mut t = el("Grand opening today");
        t.set_style_range(6..13, &bold());
        assert_eq!(ranges(&t), vec![(6, 13)]);
        assert!(t.style_at(6).bold && !t.style_at(5).bold && !t.style_at(13).bold);
        t.set_style_range(13..19, &bold());
        assert_eq!(ranges(&t), vec![(6, 19)], "equal neighbours merge");
        t.set_style_range(0..19, &SpanStyle { bold: Some(false), ..SpanStyle::default() });
        assert!(t.spans.is_empty(), "back to the box's style: {:?}", t.spans);
        assert!(t.spans_normalized());
    }

    #[test]
    fn overlapping_patches_keep_both_values() {
        let mut t = el("abcdef");
        t.set_style_range(0..4, &bold());
        t.set_style_range(2..6, &red());
        assert_eq!(ranges(&t), vec![(0, 2), (2, 4), (4, 6)]);
        assert_eq!(t.spans[1].style, bold().merged(&red()));
        let rs = t.range_style(0..6);
        assert!(rs.style.bold && rs.is_mixed("bold") && rs.is_mixed("color") && !rs.is_mixed("italic"));
        assert!(t.range_style(2..4).mixed.is_empty());
    }

    #[test]
    fn typing_takes_the_style_before_the_caret() {
        let mut t = el("Hello world");
        t.set_style_range(0..5, &bold());
        t.replace_range(5..5, "!!", None);
        assert_eq!(t.text, "Hello!! world");
        assert_eq!(ranges(&t), vec![(0, 7)]);
        t.replace_range(0..0, ">", None);
        assert_eq!(ranges(&t), vec![(0, 8)], "at the start: the first character's style");
        t.replace_range(14..14, ".", None);
        assert_eq!(ranges(&t), vec![(0, 8)]);
        assert_eq!(t.text, ">Hello!! world.");
        // An explicit style (pending bold on a caret) wins.
        t.replace_range(15..15, "X", Some(red()));
        assert_eq!(t.spans.last().map(|s| (s.start, s.end)), Some((15, 16)));
    }

    #[test]
    fn deleting_shifts_and_clips_spans() {
        let mut t = el("one two three");
        t.set_style_range(4..7, &bold());
        t.set_style_range(8..13, &red());
        t.replace_range(0..4, "", None);
        assert_eq!(t.text, "two three");
        assert_eq!(ranges(&t), vec![(0, 3), (4, 9)]);
        t.replace_range(2..6, "", None);
        assert_eq!(t.text, "twree");
        assert_eq!(ranges(&t), vec![(0, 2), (2, 5)]);
        t.replace_range(0..5, "", None);
        assert!(t.spans.is_empty() && t.text.is_empty());
    }

    #[test]
    fn whole_text_changes_follow_the_caret() {
        let mut t = el("hello");
        t.set_style_range(3..5, &bold());
        // Typing an `l` after "hel": "hell" + "lo" is ambiguous; the caret (4) says where.
        t.set_text_keeping_styles("helllo", Some(4), None);
        assert_eq!(ranges(&t), vec![(3, 6)]);
        t.set_text_keeping_styles("helo", Some(2), None);
        assert_eq!(t.text, "helo");
        assert_eq!(ranges(&t), vec![(2, 4)]);
        // Unicode: indices are characters.
        let mut u = el("héllo wörld");
        u.set_style_range(6..11, &red());
        u.set_text_keeping_styles("héllo, wörld", None, None);
        assert_eq!(ranges(&u), vec![(7, 12)]);
        let runs = u.style_runs();
        assert_eq!(runs.len(), 2);
        assert_eq!(&u.text[runs[1].0.clone()], "wörld");
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn repair_fixes_junk_spans() {
        let mut t = el("abc");
        t.spans = vec![
            TextSpan::new(5..9, bold()),
            TextSpan::new(2..1, bold()),
            TextSpan::new(1..3, SpanStyle::default()),
            TextSpan::new(0..2, red()),
            TextSpan::new(1..99, bold()),
            TextSpan::new(0..1, SpanStyle { font_size: Some(f64::NAN), font_family: Some(String::new()), ..SpanStyle::default() }),
        ];
        assert!(!t.spans_normalized());
        assert!(t.normalize_spans());
        assert!(t.spans_normalized(), "{:?}", t.spans);
        assert_eq!(ranges(&t), vec![(0, 1), (1, 2), (2, 3)]);
        assert!(!t.normalize_spans(), "normalizing again changes nothing");
    }

    #[test]
    fn whole_box_style_clears_overrides() {
        let mut t = el("abc def");
        t.set_style_range(0..3, &red().merged(&bold()));
        t.set_style_all(&red());
        assert_eq!(t.style.color, Color::rgb(255, 0, 0));
        assert_eq!(t.spans.len(), 1);
        assert_eq!(t.spans[0].style, bold());
        t.set_style_all(&bold());
        assert!(t.spans.is_empty() && t.style.bold);
    }

    #[test]
    fn spans_serialize_flat_and_old_files_have_none() {
        let mut t = el("Hi there");
        t.set_style_range(3..8, &red());
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["spans"][0]["start"], 3);
        assert_eq!(v["spans"][0]["color"], "#ff0000");
        assert!(v["spans"][0].get("bold").is_none());
        let back: TextEl = serde_json::from_value(v).unwrap();
        assert_eq!(back, t);
        let plain = serde_json::to_value(el("x")).unwrap();
        assert!(plain.get("spans").is_none());
        let old: TextEl = serde_json::from_value(serde_json::json!({"text": "x", "style": TextStyle::default()})).unwrap();
        assert!(old.spans.is_empty());
    }
}
