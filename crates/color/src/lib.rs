//! Colour for GenHa (layer L0).
//!
//! [`Color`] is 8-bit sRGB with straight (not premultiplied) alpha and serializes as a hex string
//! (`"#1e88e5"`, `"#1e88e580"`) so documents and agent messages stay readable. Also: HSL, WCAG
//! contrast ([`contrast_ratio`]) and colour harmonies for palette suggestions ([`harmony`]).
#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// Parses `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` (the `#` is optional), or a CSS colour
    /// keyword from [`NAMED`].
    pub fn parse(s: &str) -> Option<Color> {
        let t = s.trim();
        if let Some((_, c)) = NAMED.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)) {
            return Some(*c);
        }
        let h = t.strip_prefix('#').unwrap_or(t);
        if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let nib = |i: usize| h.get(i..=i).and_then(|d| u8::from_str_radix(d, 16).ok()).map(|v| v * 17);
        let byte = |i: usize| h.get(i..i + 2).and_then(|d| u8::from_str_radix(d, 16).ok());
        match h.len() {
            3 => Some(Color::rgb(nib(0)?, nib(1)?, nib(2)?)),
            4 => Some(Color::rgba(nib(0)?, nib(1)?, nib(2)?, nib(3)?)),
            6 => Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// `#rrggbb`, or `#rrggbbaa` when not opaque.
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    pub fn with_alpha(self, a: u8) -> Color {
        Color { a, ..self }
    }

    /// Components as `0.0..=1.0`.
    pub fn to_f32(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a].map(|v| f32::from(v) / 255.0)
    }

    /// From `0.0..=1.0` components (clamped; NaN becomes 0).
    pub fn from_f32(c: [f32; 4]) -> Color {
        let q = |v: f32| if v.is_finite() { (v.clamp(0.0, 1.0) * 255.0).round() as u8 } else { 0 };
        Color::rgba(q(c[0]), q(c[1]), q(c[2]), q(c[3]))
    }

    /// Hue (degrees), saturation and lightness (`0..=1`).
    pub fn to_hsl(&self) -> (f64, f64, f64) {
        let [r, g, b] = [self.r, self.g, self.b].map(|v| f64::from(v) / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        if d == 0.0 {
            return (0.0, 0.0, l);
        }
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == r {
            60.0 * (((g - b) / d).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        (h, s, l)
    }

    /// From hue (degrees, any value), saturation and lightness (clamped to `0..=1`), opaque.
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Color {
        let fin = |v: f64| if v.is_finite() { v } else { 0.0 };
        let (h, s, l) = (fin(h).rem_euclid(360.0), fin(s).clamp(0.0, 1.0), fin(l).clamp(0.0, 1.0));
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let q = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::rgb(q(r), q(g), q(b))
    }

    /// WCAG relative luminance (`0..=1`).
    pub fn luminance(&self) -> f64 {
        let lin = |v: u8| {
            let c = f64::from(v) / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// Black or white, whichever reads better on this colour.
    pub fn readable_text(&self) -> Color {
        if contrast_ratio(*self, Color::BLACK) >= contrast_ratio(*self, Color::WHITE) { Color::BLACK } else { Color::WHITE }
    }

    /// Linear interpolation in sRGB (`t` clamped to `0..=1`).
    pub fn lerp(self, other: Color, t: f64) -> Color {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
        let m = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Color::rgba(m(self.r, other.r), m(self.g, other.g), m(self.b, other.b), m(self.a, other.a))
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Color::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid colour `{s}` (expected #rrggbb or #rrggbbaa)")))
    }
}

/// WCAG 2 contrast ratio between two colours (1 to 21). 4.5 is the minimum for body text.
pub fn contrast_ratio(a: Color, b: Color) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Colour harmony rules for palette suggestions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Harmony {
    Complementary,
    Analogous,
    Triadic,
    SplitComplementary,
    Tetradic,
    Monochromatic,
}

impl Harmony {
    pub const ALL: [Harmony; 6] =
        [Harmony::Complementary, Harmony::Analogous, Harmony::Triadic, Harmony::SplitComplementary, Harmony::Tetradic, Harmony::Monochromatic];

    pub fn id(self) -> &'static str {
        match self {
            Harmony::Complementary => "complementary",
            Harmony::Analogous => "analogous",
            Harmony::Triadic => "triadic",
            Harmony::SplitComplementary => "split-complementary",
            Harmony::Tetradic => "tetradic",
            Harmony::Monochromatic => "monochromatic",
        }
    }

    pub fn from_id(id: &str) -> Option<Harmony> {
        Harmony::ALL.into_iter().find(|h| h.id() == id)
    }
}

/// A palette built around `base` (which comes first).
pub fn harmony(base: Color, rule: Harmony) -> Vec<Color> {
    let (h, s, l) = base.to_hsl();
    let rot = |d: f64| Color::from_hsl(h + d, s, l);
    match rule {
        Harmony::Complementary => vec![base, rot(180.0)],
        Harmony::Analogous => vec![base, rot(-30.0), rot(30.0)],
        Harmony::Triadic => vec![base, rot(120.0), rot(240.0)],
        Harmony::SplitComplementary => vec![base, rot(150.0), rot(210.0)],
        Harmony::Tetradic => vec![base, rot(90.0), rot(180.0), rot(270.0)],
        Harmony::Monochromatic => {
            vec![base, Color::from_hsl(h, s, (l * 0.55).clamp(0.0, 1.0)), Color::from_hsl(h, s, l + (1.0 - l) * 0.45), Color::from_hsl(h, s * 0.4, l)]
        }
    }
}

/// CSS colour keywords accepted by [`Color::parse`] (a small, common subset).
pub const NAMED: &[(&str, Color)] = &[
    ("black", Color::BLACK),
    ("white", Color::WHITE),
    ("transparent", Color::TRANSPARENT),
    ("red", Color::rgb(255, 0, 0)),
    ("green", Color::rgb(0, 128, 0)),
    ("blue", Color::rgb(0, 0, 255)),
    ("yellow", Color::rgb(255, 255, 0)),
    ("orange", Color::rgb(255, 165, 0)),
    ("purple", Color::rgb(128, 0, 128)),
    ("pink", Color::rgb(255, 192, 203)),
    ("gray", Color::rgb(128, 128, 128)),
    ("grey", Color::rgb(128, 128, 128)),
    ("navy", Color::rgb(0, 0, 128)),
    ("teal", Color::rgb(0, 128, 128)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        assert_eq!(Color::parse("#1e88e5"), Some(Color::rgb(0x1e, 0x88, 0xe5)));
        assert_eq!(Color::parse("fff"), Some(Color::WHITE));
        assert_eq!(Color::parse("#0008"), Some(Color::rgba(0, 0, 0, 0x88)));
        assert_eq!(Color::parse("Red"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(Color::parse("#12345"), None);
        assert_eq!(Color::parse("#zzzzzz"), None);
        assert_eq!(Color::parse("#é12345"), None);
        assert_eq!(Color::rgba(1, 2, 3, 4).to_hex(), "#01020304");
    }

    #[test]
    fn serde_as_hex() {
        let s = serde_json::to_string(&Color::rgb(255, 0, 128)).unwrap();
        assert_eq!(s, "\"#ff0080\"");
        let c: Color = serde_json::from_str("\"#ff008080\"").unwrap();
        assert_eq!(c.a, 0x80);
        assert!(serde_json::from_str::<Color>("\"nope\"").is_err());
    }

    #[test]
    fn hsl_round_trip() {
        for c in [Color::rgb(30, 136, 229), Color::rgb(255, 0, 0), Color::rgb(12, 200, 90), Color::rgb(128, 128, 128)] {
            let (h, s, l) = c.to_hsl();
            let back = Color::from_hsl(h, s, l);
            assert!((i16::from(back.r) - i16::from(c.r)).abs() <= 1, "{c:?} -> {back:?}");
            assert!((i16::from(back.g) - i16::from(c.g)).abs() <= 1);
            assert!((i16::from(back.b) - i16::from(c.b)).abs() <= 1);
        }
        assert_eq!(Color::from_hsl(f64::NAN, 2.0, -1.0), Color::BLACK);
    }

    #[test]
    fn contrast() {
        assert!((contrast_ratio(Color::BLACK, Color::WHITE) - 21.0).abs() < 1e-9);
        assert_eq!(Color::rgb(250, 240, 10).readable_text(), Color::BLACK);
        assert_eq!(Color::rgb(20, 20, 80).readable_text(), Color::WHITE);
    }

    #[test]
    fn harmonies() {
        let base = Color::rgb(255, 0, 0);
        let comp = harmony(base, Harmony::Complementary);
        assert_eq!(comp[1], Color::rgb(0, 255, 255));
        assert_eq!(harmony(base, Harmony::Tetradic).len(), 4);
        assert_eq!(Harmony::from_id("triadic"), Some(Harmony::Triadic));
    }
}
