//! Text for GenHa (layer L2): a [`FontBook`] of installed or added fonts, family matching
//! with generic fallbacks ("Sans", "Serif", "Mono", "Display"), and line layout ([`layout`],
//! [`measure`]) producing glyph outlines as [`kurbo::BezPath`]s, or one outline per glyph with its
//! position, glyph id, face and source offset ([`layout_glyphs`]) for curved text, caret placement
//! and exporters that embed fonts.
//!
//! Text is shaped with HarfRust (the Rust port of HarfBuzz) on the same `read-fonts` parser as
//! `skrifa`, which draws the outlines: kerning, ligatures (turned off for letter-spaced text), mark
//! positioning and complex scripts (Arabic joining, Indic reordering…). Paragraphs are split into
//! bidi runs (UAX #9) and each line is reordered for display; lines break at UAX #14 opportunities.
//! Grapheme clusters the chosen face cannot draw are set in another face that can (per-cluster font
//! fallback). A text can be set in several styles ([`layout_rich`], [`layout_el`]): each style run
//! is shaped in its own font, size and weight, while line breaking, bidi and alignment work across
//! runs and each line is as tall as its tallest run; placed glyphs and line pieces carry their
//! run's style (an index in [`TextLayout::styles`]) so colours, underlines and synthetic styles can
//! be drawn per run. With an empty book text still lays out, using an approximate advance of 0.55 em per
//! character, so the UI can size text boxes before any font is available (the paths are then
//! empty).
#![forbid(unsafe_code)]

mod book;
pub mod caret;
pub mod edit;
mod layout;
mod shape;

pub use book::{FontBook, MAX_FACES, MAX_FONT_FILE, MAX_FONT_FILES, MONO_FALLBACKS, SANS_FALLBACKS, SERIF_FALLBACKS};
pub use layout::{
    MAX_FONT_SIZE, MAX_STYLE_RUNS, MAX_TEXT_CHARS, layout, layout_el, layout_glyphs, layout_glyphs_el, layout_glyphs_rich, layout_rich, measure,
    measure_el, measure_rich,
};

#[derive(Debug, thiserror::Error)]
pub enum TextError {
    #[error("not a font: {0}")]
    Parse(String),
    #[error("font file too large ({0} bytes)")]
    TooLarge(usize),
    #[error("the font holds no usable faces")]
    NoFaces,
    #[error("the font book is full")]
    Full,
}

/// One laid-out line. Coordinates are relative to the text box's top-left, y down.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineLayout {
    /// The glyph outlines (and underline) of the line, positioned in the box.
    pub path: kurbo::BezPath,
    /// Left edge of the line's ink box (after alignment).
    pub x: f64,
    /// Width of the line without trailing spaces.
    pub width: f64,
    /// Top of the line box.
    pub top: f64,
    pub baseline: f64,
    /// Line box height (`line_height` × font size).
    pub height: f64,
    /// The characters on the line, in logical (reading) order, after uppercasing.
    pub text: String,
    /// Byte range of the line in the text passed to [`layout`] (trailing spaces included, the
    /// newline left out).
    pub range: std::ops::Range<usize>,
    /// With several styles ([`TextLayout::styles`] longer than one): `path` cut by style, as
    /// (style index, outlines and decorations in that style). Empty otherwise.
    pub pieces: Vec<(usize, kurbo::BezPath)>,
}

impl LineLayout {
    /// The line's outlines by style: (index in [`TextLayout::styles`], path).
    pub fn styled_paths(&self) -> Vec<(usize, &kurbo::BezPath)> {
        if self.pieces.is_empty() { vec![(0, &self.path)] } else { self.pieces.iter().map(|(s, p)| (*s, p)).collect() }
    }
}

/// Laid-out text: its lines and the size they take.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    pub lines: Vec<LineLayout>,
    /// Width of the widest line.
    pub width: f64,
    /// Height of all lines.
    pub height: f64,
    /// The styles the text is set in; the first is the text's own (see
    /// [`LineLayout::pieces`]).
    pub styles: Vec<genha_doc::TextStyle>,
}

/// One glyph placed by [`layout_glyphs`]. Coordinates as in [`LineLayout`].
///
/// A cluster (a ligature, a letter with its marks, an Indic syllable…) may be drawn by several
/// glyphs, and several characters may be drawn by one glyph: [`cluster`](Self::cluster) maps
/// glyphs back to the text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlacedGlyph {
    /// The first character of the glyph's cluster.
    pub ch: char,
    /// Left edge of the glyph's advance (the pen position).
    pub x: f64,
    /// Advance width (kerning included), letter spacing not included.
    pub advance: f64,
    /// The glyph's outline, already in place on its line's baseline (empty for spaces).
    pub path: kurbo::BezPath,
    /// The glyph in its face (None when nothing is drawn: spaces, or no font).
    pub glyph_id: Option<u32>,
    /// The face that draws it, an index for [`FontBook::face_data`] (None without a font).
    pub face: Option<usize>,
    /// Byte offset, in the text passed to [`layout_glyphs`], of the first character of the
    /// glyph's cluster.
    pub cluster: usize,
    /// Where the glyph is drawn relative to (`x`, baseline): mark positioning and the like,
    /// pixels, y down.
    pub x_offset: f64,
    pub y_offset: f64,
    /// The glyph belongs to right-to-left text (its cluster starts at its right edge).
    pub rtl: bool,
    /// The glyph's style, an index in [`GlyphLayout::styles`].
    pub style: usize,
    /// The font size it is set in, pixels.
    pub size: f64,
}

/// An underline or strikethrough bar under or through glyphs of one style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decoration {
    /// Index in [`GlyphLayout::styles`].
    pub style: usize,
    pub rect: kurbo::Rect,
}

/// A line of separately placed glyphs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlyphLine {
    /// The glyphs in display order, left to right.
    pub glyphs: Vec<PlacedGlyph>,
    /// Left edge of the line (after alignment).
    pub x: f64,
    /// Width without trailing spaces.
    pub width: f64,
    pub top: f64,
    pub baseline: f64,
    pub height: f64,
    /// Byte range of the line in the text (as [`LineLayout::range`]).
    pub range: std::ops::Range<usize>,
    /// The line's paragraph reads right to left.
    pub rtl: bool,
    /// Underlines and strikethroughs, in the glyphs' styles.
    pub decorations: Vec<Decoration>,
}

/// Text laid out glyph by glyph (see [`layout_glyphs`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlyphLayout {
    pub lines: Vec<GlyphLine>,
    pub width: f64,
    pub height: f64,
    /// The font size the glyphs were set in (the style's, made safe), pixels; with several styles
    /// each glyph has its own ([`PlacedGlyph::size`]).
    pub font_size: f64,
    /// The styles the text is set in (as [`TextLayout::styles`]).
    pub styles: Vec<genha_doc::TextStyle>,
}

#[cfg(test)]
mod caret_tests;
#[cfg(test)]
mod tests;
