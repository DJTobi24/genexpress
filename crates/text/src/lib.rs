//! Text for GenHa (layer L2): a [`FontBook`] of installed or added fonts, family matching
//! with generic fallbacks ("Sans", "Serif", "Mono", "Display"), and simple line layout
//! ([`layout`], [`measure`]) producing glyph outlines as [`kurbo::BezPath`]s.
//!
//! Shaping is deliberately simple: one glyph per character from the font's character map, advance
//! widths, letter spacing, no kerning or ligatures. Characters the chosen face lacks are taken from
//! another face that has them. With an empty book text still lays out, using an approximate
//! advance of 0.55 em, so the UI can size text boxes before any font is available (the paths are
//! then empty).
#![forbid(unsafe_code)]

mod book;
mod layout;

pub use book::{FontBook, MAX_FACES, MAX_FONT_FILE, MAX_FONT_FILES, MONO_FALLBACKS, SANS_FALLBACKS, SERIF_FALLBACKS};
pub use layout::{MAX_FONT_SIZE, MAX_TEXT_CHARS, layout, measure};

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
    /// The characters on the line (after uppercasing).
    pub text: String,
}

/// Laid-out text: its lines and the size they take.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    pub lines: Vec<LineLayout>,
    /// Width of the widest line.
    pub width: f64,
    /// Height of all lines.
    pub height: f64,
}

#[cfg(test)]
mod tests;
