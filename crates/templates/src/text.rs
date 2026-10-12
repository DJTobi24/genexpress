//! One-click text styles for the "Add text" panel.

use genha_color::Color;
use genha_doc::{TextAlign, TextEffect, TextStyle};

use crate::build::hex;

/// A text style with a sample to show in the panel.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub sample: &'static str,
    pub style: TextStyle,
    pub effect: TextEffect,
}

const INK: Color = hex(0x1b1b2f);
const ACCENT: Color = hex(0x5b3cf5);

fn style(family: &str, size: f64, bold: bool) -> TextStyle {
    TextStyle { font_family: family.into(), font_size: size, bold, color: INK, ..TextStyle::default() }
}

/// The built-in text presets, largest first.
pub fn text_presets() -> Vec<TextPreset> {
    let p = |id, label, sample, style, effect| TextPreset { id, label, sample, style, effect };
    vec![
        p("heading", "Heading", "Add a heading", TextStyle { line_height: 1.1, ..style("Sans", 72.0, true) }, TextEffect::None),
        p("subheading", "Subheading", "Add a subheading", TextStyle { line_height: 1.2, ..style("Sans", 44.0, true) }, TextEffect::None),
        p("body", "Body text", "Add a little bit of body text", TextStyle { line_height: 1.45, ..style("Sans", 28.0, false) }, TextEffect::None),
        p(
            "caption",
            "Caption",
            "A short caption",
            TextStyle { color: hex(0x5c5c70), line_height: 1.35, ..style("Sans", 20.0, false) },
            TextEffect::None,
        ),
        p(
            "quote",
            "Quote",
            "“Make it simple, but significant.”",
            TextStyle { italic: true, line_height: 1.3, ..style("Serif", 44.0, false) },
            TextEffect::None,
        ),
        p(
            "label",
            "Label",
            "New",
            TextStyle { uppercase: true, letter_spacing: 160.0, color: Color::WHITE, ..style("Sans", 22.0, true) },
            TextEffect::Highlight { color: ACCENT, padding: 10.0, corner_radius: 8.0 },
        ),
        p(
            "big-number",
            "Big number",
            "42%",
            TextStyle { color: ACCENT, line_height: 1.0, letter_spacing: -20.0, ..style("Display", 160.0, true) },
            TextEffect::None,
        ),
        p(
            "outlined-title",
            "Outlined title",
            "Bold move",
            TextStyle { color: Color::WHITE, uppercase: true, ..style("Display", 80.0, true) },
            TextEffect::Outline { color: INK, width: 4.0 },
        ),
        p(
            "highlighted-title",
            "Highlighted title",
            "Shine bright",
            TextStyle { align: TextAlign::Left, ..style("Sans", 64.0, true) },
            TextEffect::Highlight { color: hex(0xffd94a), padding: 12.0, corner_radius: 6.0 },
        ),
        p(
            "shadow-title",
            "Shadow title",
            "Stand out",
            TextStyle { color: Color::WHITE, ..style("Display", 80.0, true) },
            TextEffect::Shadow { color: Color::rgba(0, 0, 0, 140), offset_x: 4.0, offset_y: 6.0, blur: 10.0 },
        ),
        p(
            "mono-tag",
            "Code tag",
            "v2.0 is here",
            TextStyle { color: hex(0x0f766e), ..style("Mono", 28.0, false) },
            TextEffect::Highlight { color: hex(0xdff7f2), padding: 8.0, corner_radius: 4.0 },
        ),
    ]
}
