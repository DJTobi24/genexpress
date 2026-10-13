//! Paragraph analysis (bidi levels, scripts, font fallback, line-break opportunities), shaping,
//! line breaking and glyph placement.
//!
//! Each paragraph (text between newlines) goes through these steps:
//! 1. **Analysis.** Bidi embedding levels (UAX #9, `unicode-bidi`), the script of every character
//!    (Common and Inherited characters take their neighbours'), the face of every grapheme cluster
//!    (the style's face when it has all the cluster's characters, else the first fallback face that
//!    does), and the places a line may break (UAX #14, `unicode-linebreak`).
//! 2. **Shaping.** The paragraph is cut wherever face, script or level changes and at every break
//!    opportunity (so pieces are words with their trailing spaces), and each piece is shaped by
//!    HarfRust in its face, direction and script ([`crate::shape`], cached per face). Glyphs are
//!    kept in logical order.
//! 3. **Lines.** Greedy filling at break opportunities using the shaped advances; a word wider than
//!    the box breaks between clusters; spaces may hang past the edge.
//! 4. **Placement.** Each line is reordered for display (UAX #9 rules L1 and L2) and its glyphs
//!    placed left to right from the aligned line start.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use genha_doc::{TextAlign, TextStyle};
use kurbo::{BezPath, Rect, Shape};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider, OutlineGlyphCollection};
use unicode_bidi::ParagraphBidiInfo;
use unicode_linebreak::{BreakOpportunity, linebreaks};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

use crate::shape::ShapeKey;
use crate::{FontBook, GlyphLayout, GlyphLine, LineLayout, PlacedGlyph, TextLayout};

/// Most characters laid out (the rest are dropped).
pub const MAX_TEXT_CHARS: usize = 100_000;
/// Largest font size, in pixels.
pub const MAX_FONT_SIZE: f64 = 4096.0;
/// Advance used for every character when no font is available, in ems.
const APPROX_ADVANCE: f64 = 0.55;
/// Deepest bidi embedding level (UAX #9 max_depth + 1).
const MAX_LEVEL: u8 = 126;

/// A shaped glyph of a paragraph. Paragraph glyphs are kept in logical order (the order of the
/// characters they draw); right-to-left pieces are reversed from HarfRust's visual order.
#[derive(Clone, Copy, Debug)]
struct Glyph {
    /// First character of the glyph's cluster.
    ch: char,
    face: Option<usize>,
    /// The glyph to draw (None: nothing to draw).
    gid: Option<GlyphId>,
    /// Byte offset of the cluster's first character in the paragraph.
    cluster: usize,
    /// Pixels.
    advance: f64,
    /// Drawing offset from the pen position, pixels (`dy` up).
    dx: f64,
    dy: f64,
    /// Bidi embedding level (odd: right to left).
    level: u8,
    /// A line may start at this glyph (UAX #14)…
    can_break: bool,
    /// …and must.
    forced: bool,
}

impl Glyph {
    fn space(&self) -> bool {
        self.ch.is_whitespace()
    }
}

/// A paragraph after newline splitting, uppercasing and dropping control characters.
#[derive(Default)]
struct Para {
    text: String,
    /// (byte offset in `text`, byte offset in the source text) of each character of `text`.
    src: Vec<(usize, usize)>,
    /// Byte offset in the source where the paragraph ends (its newline, or the end of the text).
    src_end: usize,
}

impl Para {
    fn push(&mut self, c: char, src: usize) {
        self.src.push((self.text.len(), src));
        self.text.push(c);
    }

    /// The source offset of byte `at` of the paragraph.
    fn source(&self, at: usize) -> usize {
        if at >= self.text.len() {
            return self.src_end;
        }
        let i = self.src.partition_point(|&(p, _)| p <= at);
        self.src.get(i.saturating_sub(1)).map_or(self.src_end, |&(_, s)| s)
    }
}

/// Splits `text` into paragraphs at newlines. Tabs become spaces; carriage returns and other
/// control characters are dropped.
fn paragraphs(text: &str, upper: bool) -> Vec<Para> {
    let mut out = vec![];
    let mut cur = Para::default();
    let mut end = 0;
    for (i, c) in text.char_indices().take(MAX_TEXT_CHARS) {
        end = i + c.len_utf8();
        match c {
            '\n' => {
                cur.src_end = i;
                out.push(std::mem::take(&mut cur));
            }
            '\t' => cur.push(' ', i),
            c if c.is_control() => {}
            c if upper => c.to_uppercase().for_each(|u| cur.push(u, i)),
            c => cur.push(c, i),
        }
    }
    cur.src_end = end;
    out.push(cur);
    out
}

/// Sanitized style numbers.
struct Params {
    size: f64,
    line_height: f64,
    spacing: f64,
}

fn params(style: &TextStyle) -> Params {
    let size = if style.font_size.is_finite() { style.font_size.clamp(0.0, MAX_FONT_SIZE) } else { 48.0 };
    let lh = if style.line_height.is_finite() { style.line_height.clamp(0.1, 10.0) } else { 1.2 };
    let ls = if style.letter_spacing.is_finite() { style.letter_spacing.clamp(-1000.0, 10_000.0) } else { 0.0 };
    Params { size, line_height: lh * size, spacing: ls / 1000.0 * size }
}

/// Lays out `text` in `style` in a box `box_width` wide (no wrapping when it is not a positive
/// finite number). Origin at the box's top-left, y down.
pub fn layout(book: &FontBook, text: &str, style: &TextStyle, box_width: f64) -> TextLayout {
    layout_impl(book, text, style, box_width, Out::Lines).0
}

/// Like [`layout`], but keeps every glyph's own outline and position, for text that is placed
/// glyph by glyph (curved text) or edited (caret positions from [`PlacedGlyph::cluster`]).
/// Underlines are left out.
pub fn layout_glyphs(book: &FontBook, text: &str, style: &TextStyle, box_width: f64) -> GlyphLayout {
    let (l, lines, size) = layout_impl(book, text, style, box_width, Out::Glyphs);
    GlyphLayout { lines, width: l.width, height: l.height, font_size: size }
}

/// The natural (unwrapped) size of `text` in `style`.
pub fn measure(book: &FontBook, text: &str, style: &TextStyle) -> (f64, f64) {
    let (l, _, _) = layout_impl(book, text, style, f64::INFINITY, Out::Measure);
    (l.width, l.height)
}

/// What [`layout_impl`] produces besides the lines' sizes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Out {
    /// Only sizes: no outlines.
    Measure,
    /// Line outlines.
    Lines,
    /// Line outlines and separate glyphs.
    Glyphs,
}

/// A face opened for shaping and drawing.
struct Open<'a> {
    font: FontRef<'a>,
    outlines: OutlineGlyphCollection<'a>,
    /// Pixels per font unit.
    scale: f64,
}

/// Per-paragraph analysis.
struct Analysis {
    /// Per character of the paragraph: byte offset, face, script, bidi level.
    chars: Vec<CharInfo>,
    /// Paragraph embedding level.
    base_level: u8,
    /// Line-break opportunities inside the paragraph: (byte offset, mandatory), sorted.
    breaks: Vec<(usize, bool)>,
}

#[derive(Clone, Copy)]
struct CharInfo {
    at: usize,
    face: Option<usize>,
    script: Option<[u8; 4]>,
    level: u8,
}

/// Chooses faces for text: the style's face, then fallbacks.
struct FaceChooser<'b> {
    book: &'b FontBook,
    primary: Option<usize>,
    bold: bool,
    italic: bool,
    fallback: Option<Vec<usize>>,
    /// Fallback faces found for single characters.
    memo: HashMap<char, usize>,
}

/// Characters that never decide the face (joiners, variation selectors).
fn ignorable(c: char) -> bool {
    matches!(c, '\u{200C}' | '\u{200D}' | '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}' | '\u{E0020}'..='\u{E007F}')
}

impl FaceChooser<'_> {
    fn covers(&self, face: usize, cluster: &str) -> bool {
        self.book.face(face).is_some_and(|f| cluster.chars().filter(|c| !ignorable(*c)).all(|c| f.covers(c)))
    }

    /// The face for one grapheme cluster; `prev` is the previous cluster's.
    fn pick(&mut self, cluster: &str, prev: Option<usize>) -> Option<usize> {
        let primary = self.primary?;
        if self.covers(primary, cluster) {
            return Some(primary);
        }
        let Some(first) = cluster.chars().find(|c| !ignorable(*c)) else { return prev.or(Some(primary)) };
        // Spaces stay with the text around them (HarfRust substitutes missing space glyphs).
        if first.is_whitespace() {
            return prev.or(Some(primary));
        }
        if let Some(p) = prev
            && p != primary
            && self.covers(p, cluster)
        {
            return Some(p);
        }
        let single = cluster.chars().nth(1).is_none();
        if single && let Some(&f) = self.memo.get(&first) {
            return Some(f);
        }
        let (book, bold, italic) = (self.book, self.bold, self.italic);
        let order = self.fallback.get_or_insert_with(|| book.fallback_order(bold, italic));
        let all = order.iter().copied().find(|&i| book.face(i).is_some_and(|f| cluster.chars().filter(|c| !ignorable(*c)).all(|c| f.covers(c))));
        let face = all.or_else(|| order.iter().copied().find(|&i| book.face(i).is_some_and(|f| f.covers(first)))).unwrap_or(primary);
        if single {
            self.memo.insert(first, face);
        }
        Some(face)
    }
}

/// The ISO 15924 tag of a character's script; None for Common, Inherited and Unknown.
fn script_tag(c: char) -> Option<[u8; 4]> {
    match c.script() {
        Script::Common | Script::Inherited | Script::Unknown => None,
        s => <[u8; 4]>::try_from(s.short_name().as_bytes()).ok(),
    }
}

fn analyse(para: &Para, faces: &mut FaceChooser<'_>) -> Analysis {
    let text = para.text.as_str();
    // Bidi levels (by byte); pure ASCII text is all left to right.
    let (levels, base_level): (Vec<u8>, u8) = if text.is_ascii() {
        (vec![], 0)
    } else {
        let bidi = ParagraphBidiInfo::new(text, None);
        if bidi.is_pure_ltr && !bidi.paragraph_level.is_rtl() {
            (vec![], 0)
        } else {
            (bidi.levels.iter().map(|l| l.number().min(MAX_LEVEL)).collect(), bidi.paragraph_level.number().min(MAX_LEVEL))
        }
    };
    let mut chars: Vec<CharInfo> = Vec::with_capacity(text.len());
    let mut prev_face = None;
    for (at, g) in text.grapheme_indices(true) {
        let face = faces.pick(g, prev_face);
        prev_face = face;
        for (i, c) in g.char_indices() {
            let at = at + i;
            chars.push(CharInfo { at, face, script: script_tag(c), level: levels.get(at).copied().unwrap_or(base_level) });
        }
    }
    // Common and Inherited characters take the script before them (or, at the start, after).
    let mut last = None;
    for ch in chars.iter_mut() {
        match ch.script {
            Some(s) => last = Some(s),
            None => ch.script = last,
        }
    }
    let mut next = None;
    for ch in chars.iter_mut().rev() {
        match ch.script {
            Some(s) => next = Some(s),
            None => ch.script = next,
        }
    }
    let breaks =
        linebreaks(text).filter(|&(at, _)| at > 0 && at < text.len()).map(|(at, kind)| (at, matches!(kind, BreakOpportunity::Mandatory))).collect();
    Analysis { chars, base_level, breaks }
}

/// Shapes a paragraph into glyphs in logical order.
fn shape_para(para: &Para, an: &Analysis, open: &HashMap<usize, Open<'_>>, book: &FontBook, p: &Params) -> Vec<Glyph> {
    let text = para.text.as_str();
    let mut glyphs: Vec<Glyph> = Vec::with_capacity(text.len());
    let is_break = |at: usize| an.breaks.binary_search_by_key(&at, |b| b.0).is_ok();
    let mut start = 0;
    while let Some(first) = an.chars.get(start) {
        // The piece: same face, script and level, up to the next break opportunity.
        let mut end = start + 1;
        while let Some(c) = an.chars.get(end) {
            if c.face != first.face || c.script != first.script || c.level != first.level || is_break(c.at) {
                break;
            }
            end += 1;
        }
        let from = first.at;
        let to = an.chars.get(end).map_or(text.len(), |c| c.at);
        let piece = text.get(from..to).unwrap_or("");
        let rtl = first.level % 2 == 1;
        let face = first.face.and_then(|f| open.get(&f).zip(book.face(f)).map(|(o, face)| (f, o, face)));
        let at = glyphs.len();
        match face {
            Some((f, o, face)) => {
                let key = ShapeKey { rtl, script: first.script, ligatures: p.spacing == 0.0 };
                let shaped = face.shaping.shape(&o.font, piece, key);
                let px = |v: i32| {
                    let v = f64::from(v) * o.scale;
                    if v.is_finite() { v } else { 0.0 }
                };
                for g in shaped.iter() {
                    let cluster = from.saturating_add(usize::try_from(g.cluster).unwrap_or(0)).min(to);
                    let ch = text.get(cluster..).and_then(|s| s.chars().next()).unwrap_or(' ');
                    let gid = if g.gid == 0 && ch.is_whitespace() { None } else { Some(GlyphId::new(g.gid)) };
                    glyphs.push(Glyph {
                        ch,
                        face: Some(f),
                        gid,
                        cluster,
                        advance: px(g.advance),
                        dx: px(g.x_offset),
                        dy: px(g.y_offset),
                        level: first.level,
                        can_break: false,
                        forced: false,
                    });
                }
                if rtl && let Some(new) = glyphs.get_mut(at..) {
                    new.reverse();
                }
            }
            // No font: one box-less glyph per character with an approximate advance.
            None => {
                for (i, ch) in piece.char_indices() {
                    glyphs.push(Glyph {
                        ch,
                        face: None,
                        gid: None,
                        cluster: from + i,
                        advance: p.size * APPROX_ADVANCE,
                        dx: 0.0,
                        dy: 0.0,
                        level: first.level,
                        can_break: false,
                        forced: false,
                    });
                }
            }
        }
        start = end;
    }
    // Mark where lines may start: the first glyph of a cluster at a break opportunity.
    let mut prev_cluster = None;
    for g in &mut glyphs {
        if prev_cluster != Some(g.cluster)
            && let Ok(i) = an.breaks.binary_search_by_key(&g.cluster, |b| b.0)
        {
            g.can_break = true;
            g.forced = an.breaks.get(i).is_some_and(|b| b.1);
        }
        prev_cluster = Some(g.cluster);
    }
    glyphs
}

/// Index just past the cluster starting at glyph `i`.
fn cluster_end(glyphs: &[Glyph], i: usize) -> usize {
    let Some(c) = glyphs.get(i).map(|g| g.cluster) else { return i };
    let mut j = i + 1;
    while glyphs.get(j).is_some_and(|g| g.cluster == c) {
        j += 1;
    }
    j
}

/// Greedy line filling: ranges of `glyphs` per line. Lines end at break opportunities (after
/// spaces, between ideographs…) or, when a word alone is wider than the box, between clusters.
/// Spaces may hang past the edge and stay on the line they follow.
fn break_lines(glyphs: &[Glyph], spacing: f64, wrap: Option<f64>) -> Vec<Range<usize>> {
    let n = glyphs.len();
    if n == 0 {
        return std::iter::once(0..0).collect();
    }
    let mut out = vec![];
    let mut start = 0;
    while start < n {
        let mut width = 0.0;
        let mut last_break: Option<usize> = None;
        let mut forced = false;
        let mut i = start;
        while let Some(g) = glyphs.get(i) {
            if i > start && g.forced {
                forced = true;
                break;
            }
            let j = cluster_end(glyphs, i);
            let adv: f64 = glyphs.get(i..j).unwrap_or(&[]).iter().map(|g| g.advance).sum();
            let w = adv + if i > start { spacing } else { 0.0 };
            if let Some(max) = wrap
                && i > start
                && !g.space()
                && width + w > max + 1e-9
            {
                break;
            }
            if i > start && g.can_break {
                last_break = Some(i);
            }
            width += w;
            i = j;
        }
        let mut end = if i >= n || forced || glyphs.get(i).is_some_and(|g| g.can_break) {
            i
        } else {
            match last_break {
                Some(b) if b > start => b,
                _ => i,
            }
        };
        // Spaces after a line hang on it.
        while glyphs.get(end).is_some_and(|g| g.space() && !g.forced) {
            end += 1;
        }
        let end = end.max(start + 1).min(n);
        out.push(start..end);
        start = end;
    }
    out
}

/// Width of laid-out glyphs: advances plus spacing between clusters.
fn run_width(glyphs: &[Glyph], spacing: f64) -> f64 {
    let mut clusters = 0usize;
    let mut prev = None;
    let mut adv = 0.0;
    for g in glyphs {
        if prev != Some(g.cluster) {
            clusters += 1;
        }
        prev = Some(g.cluster);
        adv += g.advance;
    }
    adv + spacing * clusters.saturating_sub(1) as f64
}

/// Glyph count of a line without its trailing spaces.
fn visible_len(glyphs: &[Glyph]) -> usize {
    glyphs.iter().rposition(|g| !g.space()).map_or(0, |i| i + 1)
}

/// Display order of a line's glyphs (UAX #9 L1: trailing spaces take the paragraph level; L2:
/// reverse every run at each level from the highest down to the lowest odd one).
fn visual_order(glyphs: &[Glyph], base_level: u8) -> Vec<usize> {
    let n = glyphs.len();
    let visible = visible_len(glyphs);
    let level = |i: usize| if i >= visible { base_level } else { glyphs.get(i).map_or(base_level, |g| g.level) };
    let mut order: Vec<usize> = (0..n).collect();
    let max = (0..n).map(level).max().unwrap_or(0);
    let min_odd = (0..n).map(level).min().unwrap_or(0) | 1;
    let mut lvl = max;
    while lvl >= min_odd && lvl > 0 {
        let mut i = 0;
        while i < n {
            if order.get(i).is_some_and(|&g| level(g) >= lvl) {
                let mut j = i;
                while order.get(j).is_some_and(|&g| level(g) >= lvl) {
                    j += 1;
                }
                if let Some(run) = order.get_mut(i..j) {
                    run.reverse();
                }
                i = j;
            } else {
                i += 1;
            }
        }
        lvl -= 1;
    }
    order
}

/// A line ready to place: its glyphs (logical order) and where it sits in its paragraph.
struct Line<'p> {
    para: usize,
    glyphs: &'p [Glyph],
    /// Byte range in the paragraph.
    range: Range<usize>,
    width: f64,
    /// Width including trailing spaces.
    full: f64,
}

/// Lays out text, returning the lines (with outlines unless measuring), the separate glyphs when
/// asked for, and the font size used.
fn layout_impl(book: &FontBook, text: &str, style: &TextStyle, box_width: f64, out: Out) -> (TextLayout, Vec<GlyphLine>, f64) {
    let p = params(style);
    let wrap = if box_width.is_finite() && box_width > 0.0 { Some(box_width) } else { None };
    let paras = paragraphs(text, style.uppercase);
    let primary = book.select(&style.font_family, style.bold, style.italic);
    let (ascent, descent, ul_off, ul_th) = match primary.and_then(|i| book.face(i)) {
        Some(f) => (f.ascent, f.descent, f.underline_offset, f.underline_thickness),
        None => (0.8, 0.2, 0.1, 0.05),
    };

    // Which face draws each character, and the paragraphs' bidi levels and break opportunities.
    let mut chooser = FaceChooser { book, primary, bold: style.bold, italic: style.italic, fallback: None, memo: HashMap::new() };
    let analyses: Vec<Analysis> = paras.iter().map(|para| analyse(para, &mut chooser)).collect();
    let mut faces_used: Vec<usize> = primary.into_iter().collect();
    for c in analyses.iter().flat_map(|a| &a.chars) {
        if let Some(f) = c.face
            && !faces_used.contains(&f)
        {
            faces_used.push(f);
        }
    }

    // Open the faces used.
    let datas: Vec<(usize, Arc<[u8]>, u32, f64)> =
        faces_used.iter().filter_map(|&i| book.face(i).and_then(|f| f.data().map(|d| (i, d, f.index, f.upem)))).collect();
    let open: HashMap<usize, Open<'_>> = datas
        .iter()
        .filter_map(|(i, d, idx, upem)| {
            let font = FontRef::from_index(d, *idx).ok()?;
            let scale = p.size / upem;
            let scale = if scale.is_finite() { scale } else { 0.0 };
            Some((*i, Open { outlines: font.outline_glyphs(), font, scale }))
        })
        .collect();

    // Shape and break into lines.
    let shaped: Vec<Vec<Glyph>> = paras.iter().zip(&analyses).map(|(para, an)| shape_para(para, an, &open, book, &p)).collect();
    let mut lines: Vec<Line<'_>> = vec![];
    let mut widest: f64 = 0.0;
    for (pi, (glyphs, para)) in shaped.iter().zip(&paras).enumerate() {
        let ranges = break_lines(glyphs, p.spacing, wrap);
        let starts: Vec<usize> = ranges.iter().map(|r| glyphs.get(r.start).map_or(0, |g| g.cluster)).collect();
        for (k, r) in ranges.iter().enumerate() {
            let g = glyphs.get(r.clone()).unwrap_or(&[]);
            let a = starts.get(k).copied().unwrap_or(0);
            let b = starts.get(k + 1).copied().unwrap_or(para.text.len()).max(a);
            let width = run_width(g.get(..visible_len(g)).unwrap_or(&[]), p.spacing);
            let full = run_width(g, p.spacing);
            widest = widest.max(width);
            lines.push(Line { para: pi, glyphs: g, range: a..b, width, full });
        }
    }

    // Place.
    let align_width = wrap.unwrap_or(widest);
    let content = (ascent + descent) * p.size;
    let size = Size::new(p.size as f32);
    let settings = || DrawSettings::unhinted(size, LocationRef::default());
    let mut text_lines: Vec<LineLayout> = Vec::with_capacity(lines.len());
    let mut glyph_lines: Vec<GlyphLine> = vec![];
    for (i, line) in lines.iter().enumerate() {
        let para = paras.get(line.para);
        let base_level = analyses.get(line.para).map_or(0, |a| a.base_level);
        let rtl = base_level % 2 == 1;
        let top = i as f64 * p.line_height;
        let baseline = top + (p.line_height - content) / 2.0 + ascent * p.size;
        let w = line.width;
        let x = match style.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => (align_width - w) / 2.0,
            TextAlign::Right => align_width - w,
        };
        let src_range = para.map_or(0..0, |pa| pa.source(line.range.start)..pa.source(line.range.end));
        let line_text = para.and_then(|pa| pa.text.get(line.range.clone())).unwrap_or("").to_string();
        let mut path = BezPath::new();
        let mut placed: Vec<PlacedGlyph> = vec![];
        if out != Out::Measure {
            // Trailing spaces of a right-to-left paragraph sit left of the text.
            let mut pen = if rtl { x - (line.full - line.width) } else { x };
            let mut prev_cluster = None;
            for k in visual_order(line.glyphs, base_level) {
                let Some(g) = line.glyphs.get(k) else { continue };
                if prev_cluster.is_some_and(|c| c != g.cluster) {
                    pen += p.spacing;
                }
                prev_cluster = Some(g.cluster);
                let mut own = BezPath::new();
                if let (Some(f), Some(gid)) = (g.face, g.gid)
                    && let Some(o) = open.get(&f)
                    && let Some(outline) = o.outlines.get(gid)
                {
                    let mut pen = PathPen { path: &mut own, x: pen + g.dx, y: baseline - g.dy };
                    // A glyph that fails to draw is left out.
                    let _ = outline.draw(settings(), &mut pen);
                }
                if out == Out::Glyphs {
                    path.extend(own.iter());
                    placed.push(PlacedGlyph {
                        ch: g.ch,
                        x: pen,
                        advance: g.advance,
                        path: own,
                        glyph_id: g.gid.map(|id| id.to_u32()),
                        face: g.face.filter(|f| open.contains_key(f)),
                        cluster: para.map_or(0, |pa| pa.source(g.cluster)),
                        x_offset: g.dx,
                        y_offset: -g.dy,
                        rtl: g.level % 2 == 1,
                    });
                } else {
                    path.extend(own);
                }
                pen += g.advance;
            }
        }
        if out == Out::Glyphs {
            glyph_lines.push(GlyphLine { glyphs: placed, x, width: w, top, baseline, height: p.line_height, range: src_range.clone(), rtl });
        }
        if out != Out::Measure && style.underline && w > 0.0 {
            let y = baseline + ul_off * p.size;
            let th = (ul_th * p.size).max(0.5);
            let mut bar = Rect::new(x, y - th / 2.0, x + w, y + th / 2.0).to_path(0.1);
            // Same winding as the letters so where they overlap stays filled.
            if path.area().signum() != bar.area().signum() && path.area() != 0.0 {
                bar = bar.reverse_subpaths();
            }
            path.extend(bar);
        }
        text_lines.push(LineLayout { path, x, width: w, top, baseline, height: p.line_height, text: line_text, range: src_range });
    }
    let height = text_lines.len() as f64 * p.line_height;
    (TextLayout { lines: text_lines, width: widest, height }, glyph_lines, p.size)
}

/// Draws font outlines (y up, at the glyph origin) into a path (y down, at `x`, `y`).
struct PathPen<'p> {
    path: &'p mut BezPath,
    x: f64,
    y: f64,
}

impl PathPen<'_> {
    fn pt(&self, x: f32, y: f32) -> (f64, f64) {
        (self.x + f64::from(x), self.y - f64::from(y))
    }
}

impl OutlinePen for PathPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.move_to(p);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.line_to(p);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (c, p) = (self.pt(cx0, cy0), self.pt(x, y));
        self.path.quad_to(c, p);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (c0, c1, p) = (self.pt(cx0, cy0), self.pt(cx1, cy1), self.pt(x, y));
        self.path.curve_to(c0, c1, p);
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}
