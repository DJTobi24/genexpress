//! The last pass of a smart resize: every straight text box is made as tall as its lines (so
//! nothing overflows), no word is broken because its box is too narrow, text stays on the page,
//! and text boxes that would cover each other are moved apart, then set smaller (down to the
//! readable minimum), and reported when they still collide.

use genha_doc::{Element, ElementKind, Page, TextEl};
use genha_geom::Rect;
use genha_text::FontBook;

use super::measure::longest_word;
use super::scale_sizes;

/// Readable minimum type size on a page: 1.6 % of its shorter side, at least 8 px.
pub(crate) fn min_text_size(w: f64, h: f64) -> f64 {
    (0.016 * w.min(h)).max(8.0)
}

/// Rounds of overlap resolution.
const ROUNDS: usize = 64;
/// Space left between text boxes moved apart, share of the page height.
const GAP: f64 = 0.004;
/// Each shrinking step.
const SHRINK: f64 = 0.92;

fn straight(el: &Element) -> Option<&TextEl> {
    match &el.kind {
        ElementKind::Text(t) if el.frame.rotation == 0.0 && t.curve_degrees() == 0.0 => Some(t),
        _ => None,
    }
}

fn font_size(el: &Element) -> f64 {
    match &el.kind {
        ElementKind::Text(t) => t.style.font_size,
        _ => 0.0,
    }
}

fn label(el: &Element) -> String {
    let n = el.display_name();
    if n.is_empty() || n == "Text" {
        match &el.kind {
            ElementKind::Text(t) => format!("“{}”", t.text.chars().take(24).collect::<String>().trim()),
            _ => n,
        }
    } else {
        format!("“{n}”")
    }
}

/// Makes a straight text box fit its text: type shrunk until the longest word fits the width, the
/// height that of the lines. Other elements (and groups' children) are left alone except texts in
/// groups, which are fitted too.
fn fit(el: &mut Element, fonts: &FontBook) {
    if let ElementKind::Group(g) = &mut el.kind {
        g.children.iter_mut().for_each(|c| fit(c, fonts));
        return;
    }
    let Some(t) = straight(el) else { return };
    let w = el.frame.width.max(1.0);
    let word = longest_word(fonts, t);
    if word > w + 0.25 && word > 0.0 {
        scale_sizes(el, (w - 0.25).max(0.5) / word);
    }
    let Some(t) = straight(el) else { return };
    let (_, h) = genha_text::measure_wrapped_el(fonts, t, w);
    if h.is_finite() && h > 0.0 {
        el.frame.height = h.ceil();
    }
}

/// Shrinks a text's type by `k` and fits its box again.
fn shrink(el: &mut Element, fonts: &FontBook, k: f64) {
    scale_sizes(el, k);
    fit(el, fonts);
}

fn bounds(el: &Element) -> Rect {
    el.frame.bounds()
}

/// `floors`: for each element, the type size it should not go below (the readable minimum, or
/// less for text that was already smaller).
pub(crate) fn settle(page: &mut Page, fonts: &FontBook, floors: &[f64], notes: &mut Vec<String>) {
    let (pw, ph) = (page.width, page.height);
    let margin = 0.02 * pw.min(ph);
    for el in &mut page.elements {
        fit(el, fonts);
    }
    // Text stays on the page.
    for el in &mut page.elements {
        if !has_text(el) {
            continue;
        }
        let b = bounds(el);
        if b.width() > pw && straight(el).is_some() {
            el.frame.x = margin;
            el.frame.width = (pw - 2.0 * margin).max(1.0);
            fit(el, fonts);
        }
        for _ in 0..40 {
            let b = bounds(el);
            if b.height() <= ph && b.width() <= pw {
                break;
            }
            let k = (ph / b.height().max(1e-9)).min(pw / b.width().max(1e-9)).min(SHRINK);
            if straight(el).is_some() {
                shrink(el, fonts, k);
            } else {
                let c = el.frame.center();
                super::apply(el, &super::Map::uniform(k, (c.x, c.y), (c.x, c.y)), k);
            }
        }
        let b = bounds(el);
        let dx = (-b.x0).max(0.0) - (b.x1 - pw).max(0.0);
        let dy = (-b.y0).max(0.0) - (b.y1 - ph).max(0.0);
        if dx != 0.0 || dy != 0.0 {
            super::apply(el, &super::Map { sx: 1.0, sy: 1.0, tx: dx, ty: dy }, 1.0);
        }
    }
    // Text boxes kept apart.
    let gap = GAP * ph;
    let texts: Vec<usize> = page.elements.iter().enumerate().filter(|(_, e)| !e.hidden && straight(e).is_some()).map(|(i, _)| i).collect();
    let mut stuck: Vec<(usize, usize)> = vec![];
    for _ in 0..ROUNDS {
        let Some((a, b)) = first_overlap(page, &texts, &stuck) else { break };
        let (Some(ea), Some(eb)) = (page.elements.get(a), page.elements.get(b)) else { break };
        let (ra, rb) = (bounds(ea), bounds(eb));
        // The lower one moves down below the upper one, if the page has room.
        let (upper, lower, ru, rl) = if (rb.y0, b) > (ra.y0, a) { (a, b, ra, rb) } else { (b, a, rb, ra) };
        let dy = ru.y1 + gap - rl.y0;
        if rl.y1 + dy <= ph {
            if let Some(el) = page.elements.get_mut(lower) {
                el.frame.y += dy;
            }
            continue;
        }
        // Otherwise both get smaller, down to the readable minimum.
        let mut shrunk = false;
        for i in [upper, lower] {
            let floor = floors.get(i).copied().unwrap_or(0.0);
            if let Some(el) = page.elements.get_mut(i)
                && font_size(el) * SHRINK >= floor
            {
                let top = el.frame.y;
                shrink(el, fonts, SHRINK);
                el.frame.y = top;
                shrunk = true;
            }
        }
        if !shrunk {
            stuck.push((a.min(b), a.max(b)));
            let (Some(ea), Some(eb)) = (page.elements.get(a), page.elements.get(b)) else { break };
            notes.push(format!("{} and {} overlap: there is not enough room for both at a readable size", label(ea), label(eb)));
        }
    }
    for (el, floor) in page.elements.iter().zip(floors) {
        if !el.hidden && straight(el).is_some() && font_size(el) + 0.5 < *floor {
            notes.push(format!("{} is set at {:.0} px, below the readable {:.0} px, to fit", label(el), font_size(el), floor));
        }
    }
}

fn has_text(el: &Element) -> bool {
    match &el.kind {
        ElementKind::Text(_) => true,
        ElementKind::Group(g) => g.children.iter().any(has_text),
        _ => false,
    }
}

/// The first pair of text boxes (by index) that overlap, skipping pairs known to be stuck.
fn first_overlap(page: &Page, texts: &[usize], stuck: &[(usize, usize)]) -> Option<(usize, usize)> {
    for (n, &a) in texts.iter().enumerate() {
        for &b in texts.iter().skip(n + 1) {
            if stuck.contains(&(a.min(b), a.max(b))) {
                continue;
            }
            let (ea, eb) = (page.elements.get(a)?, page.elements.get(b)?);
            if overlap(bounds(ea), bounds(eb)) {
                return Some((a, b));
            }
        }
    }
    None
}

/// Whether two boxes overlap by more than a hair.
pub(crate) fn overlap(a: Rect, b: Rect) -> bool {
    let i = a.intersect(b);
    i.width() > 0.5 && i.height() > 0.5
}
