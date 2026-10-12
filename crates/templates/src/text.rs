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
        p(
            "display-hero",
            "Display hero",
            "Big idea",
            TextStyle { line_height: 1.0, letter_spacing: -20.0, ..style("Display", 120.0, true) },
            TextEffect::None,
        ),
        p("serif-title", "Serif title", "An elegant title", TextStyle { line_height: 1.15, ..style("Serif", 64.0, true) }, TextEffect::None),
        p(
            "serif-subtitle",
            "Serif subtitle",
            "A gentle subtitle",
            TextStyle { italic: true, color: hex(0x4a4a5e), line_height: 1.3, ..style("Serif", 40.0, false) },
            TextEffect::None,
        ),
        p(
            "kicker",
            "Kicker",
            "Section label",
            TextStyle { uppercase: true, letter_spacing: 300.0, color: ACCENT, ..style("Sans", 20.0, true) },
            TextEffect::None,
        ),
        p(
            "tracked-caps",
            "Tracked capitals",
            "Quiet and spaced",
            TextStyle { uppercase: true, letter_spacing: 400.0, color: hex(0x5c5c70), ..style("Sans", 18.0, false) },
            TextEffect::None,
        ),
        p(
            "small-print",
            "Small print",
            "Terms apply. Offer ends Sunday.",
            TextStyle { color: hex(0x6b6b80), line_height: 1.4, ..style("Sans", 14.0, false) },
            TextEffect::None,
        ),
        p(
            "pull-quote",
            "Pull quote",
            "“Good design is quiet.”",
            TextStyle { italic: true, align: TextAlign::Left, color: ACCENT, line_height: 1.25, ..style("Serif", 36.0, true) },
            TextEffect::None,
        ),
        p("big-number-serif", "Big serif number", "No. 1", TextStyle { line_height: 1.0, ..style("Serif", 140.0, true) }, TextEffect::None),
        p(
            "price-tag",
            "Price tag",
            "$19.99",
            TextStyle { ..style("Display", 56.0, true) },
            TextEffect::Highlight { color: hex(0xffd94a), padding: 10.0, corner_radius: 12.0 },
        ),
        p(
            "button-label",
            "Button label",
            "Shop now",
            TextStyle { color: Color::WHITE, ..style("Sans", 24.0, true) },
            TextEffect::Highlight { color: INK, padding: 16.0, corner_radius: 24.0 },
        ),
        p(
            "neon-glow",
            "Neon glow",
            "Glow up",
            TextStyle { color: hex(0xfdf4ff), ..style("Display", 72.0, true) },
            TextEffect::Shadow { color: hex(0xff2bd6), offset_x: 0.0, offset_y: 0.0, blur: 18.0 },
        ),
        p(
            "retro-shadow",
            "Retro drop shadow",
            "Retro",
            TextStyle { color: hex(0xffc93c), uppercase: true, ..style("Display", 80.0, true) },
            TextEffect::Shadow { color: INK, offset_x: 6.0, offset_y: 6.0, blur: 0.0 },
        ),
        p(
            "sticker",
            "Sticker",
            "Hello!",
            TextStyle { color: ACCENT, ..style("Display", 64.0, true) },
            TextEffect::Outline { color: Color::WHITE, width: 8.0 },
        ),
        p(
            "mono-receipt",
            "Monospace receipt",
            "ORDER #0042 · READY",
            TextStyle { uppercase: true, letter_spacing: 60.0, ..style("Mono", 22.0, false) },
            TextEffect::None,
        ),
    ]
}
