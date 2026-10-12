//! Elements: the things on a page.

use genha_color::Color;
use genha_geom::{Fit, Frame};
use serde::{Deserialize, Serialize};

use crate::{AssetId, ChartEl, NodeId, TableEl};

fn one() -> f32 {
    1.0
}
fn is_one(v: &f32) -> bool {
    *v == 1.0
}
fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: NodeId,
    /// Shown in the Layers panel; empty means "use the kind's name".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub frame: Frame,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub opacity: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_h: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_v: bool,
    /// Entrance animation, for animated exports (stored now, played by a later milestone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<Animation>,
    #[serde(flatten)]
    pub kind: ElementKind,
}

impl Element {
    pub fn new(id: NodeId, frame: Frame, kind: ElementKind) -> Self {
        Element { id, name: String::new(), frame, opacity: 1.0, locked: false, hidden: false, flip_h: false, flip_v: false, animation: None, kind }
    }

    /// The name shown in the UI.
    pub fn display_name(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        match &self.kind {
            ElementKind::Text(t) => {
                let first: String = t.text.lines().next().unwrap_or("").chars().take(24).collect();
                if first.is_empty() { "Text".into() } else { first }
            }
            k => k.type_name().to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ElementKind {
    Shape(ShapeEl),
    Text(TextEl),
    Image(ImageEl),
    Group(GroupEl),
    Chart(ChartEl),
    Table(TableEl),
}

impl ElementKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            ElementKind::Shape(_) => "Shape",
            ElementKind::Text(_) => "Text",
            ElementKind::Image(_) => "Image",
            ElementKind::Group(_) => "Group",
            ElementKind::Chart(_) => "Chart",
            ElementKind::Table(_) => "Table",
        }
    }
}

// ---------- shapes ----------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeEl {
    pub shape: ShapeKind,
    #[serde(default)]
    pub fill: Fill,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<StrokeStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
}

/// The outline of a shape, always fitted to the element's frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ShapeKind {
    Rect {
        #[serde(default)]
        corner_radius: f64,
    },
    Ellipse,
    Triangle,
    Polygon {
        sides: u32,
    },
    Star {
        points: u32,
        #[serde(default = "half")]
        inner_ratio: f64,
    },
    /// A straight line from the frame's left middle to its right middle.
    Line,
    /// SVG path data in a `view_width` × `view_height` box, scaled to the frame.
    Path {
        d: String,
        view_width: f64,
        view_height: f64,
    },
}

fn half() -> f64 {
    0.5
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// `0..=1` along the gradient.
    pub offset: f32,
    pub color: Color,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fill {
    #[default]
    None,
    Solid {
        color: Color,
    },
    /// A linear gradient across the frame; `angle` in degrees, 0 = left to right, 90 = top to bottom.
    Linear {
        angle: f64,
        stops: Vec<GradientStop>,
    },
    /// A radial gradient from the frame centre to its farthest edge.
    Radial {
        stops: Vec<GradientStop>,
    },
}

impl Fill {
    pub fn solid(color: Color) -> Fill {
        Fill::Solid { color }
    }

    pub fn linear(angle: f64, from: Color, to: Color) -> Fill {
        Fill::Linear { angle, stops: vec![GradientStop { offset: 0.0, color: from }, GradientStop { offset: 1.0, color: to }] }
    }

    /// The colour the UI shows for this fill (first stop of a gradient).
    pub fn primary_color(&self) -> Option<Color> {
        match self {
            Fill::None => None,
            Fill::Solid { color } => Some(*color),
            Fill::Linear { stops, .. } | Fill::Radial { stops } => stops.first().map(|s| s.color),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub color: Color,
    pub width: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dash: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub color: Color,
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
}

impl Default for Shadow {
    fn default() -> Self {
        Shadow { color: Color::rgba(0, 0, 0, 90), offset_x: 4.0, offset_y: 6.0, blur: 12.0 }
    }
}

// ---------- text ----------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextEl {
    pub text: String,
    pub style: TextStyle,
    #[serde(default)]
    pub effect: TextEffect,
    /// Curved text: how far the lines bend, in degrees of arc spanned by the widest line
    /// (`-360..=360`, 0 = straight). Positive bends the text over the top of a circle (an arch,
    /// letters pointing away from the centre); negative wraps it under the bottom (a smile, letters
    /// pointing towards the centre). ±360 closes the widest line into a full circle.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub curve: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Largest curve, in degrees.
pub const MAX_CURVE: f64 = 360.0;

impl TextEl {
    /// Straight text with no effect.
    pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
        TextEl { text: text.into(), style, effect: TextEffect::None, curve: 0.0 }
    }

    /// The curve clamped to `-360..=360` (0 when not finite).
    pub fn curve_degrees(&self) -> f64 {
        if self.curve.is_finite() { self.curve.clamp(-MAX_CURVE, MAX_CURVE) } else { 0.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    /// Font family name; falls back to the best available font when not installed.
    pub font_family: String,
    /// Size in page pixels.
    pub font_size: f64,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub uppercase: bool,
    pub color: Color,
    #[serde(default)]
    pub align: TextAlign,
    /// Line height as a multiple of the font size.
    #[serde(default = "line_height")]
    pub line_height: f64,
    /// Extra space between letters, in thousandths of an em.
    #[serde(default)]
    pub letter_spacing: f64,
}

fn line_height() -> f64 {
    1.2
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle {
            font_family: "Sans".into(),
            font_size: 48.0,
            bold: false,
            italic: false,
            underline: false,
            uppercase: false,
            color: Color::rgb(0x1b, 0x1b, 0x2f),
            align: TextAlign::Center,
            line_height: 1.2,
            letter_spacing: 0.0,
        }
    }
}

/// One-click text looks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TextEffect {
    #[default]
    None,
    Shadow {
        color: Color,
        offset_x: f64,
        offset_y: f64,
        blur: f64,
    },
    /// An outline drawn around the letters.
    Outline {
        color: Color,
        width: f64,
    },
    /// A shape behind the text: a rounded box behind each line.
    Highlight {
        color: Color,
        padding: f64,
        #[serde(default)]
        corner_radius: f64,
    },
}

// ---------- images ----------

/// The visible part of a picture, as fractions of its size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for CropRect {
    fn default() -> Self {
        CropRect { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }
    }
}

impl CropRect {
    pub fn is_full(&self) -> bool {
        *self == CropRect::default()
    }

    /// Clamped into the picture with a minimum size, non-finite values reset.
    pub fn sanitized(&self) -> CropRect {
        let f = |v: f64, d: f64| if v.is_finite() { v } else { d };
        let x = f(self.x, 0.0).clamp(0.0, 0.99);
        let y = f(self.y, 0.0).clamp(0.0, 0.99);
        CropRect { x, y, width: f(self.width, 1.0).clamp(0.01, 1.0 - x), height: f(self.height, 1.0).clamp(0.01, 1.0 - y) }
    }
}

/// Photo adjustments; every value is 0 for "unchanged" and ranges over `-100..=100` unless noted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Adjustments {
    #[serde(default)]
    pub brightness: f32,
    #[serde(default)]
    pub contrast: f32,
    #[serde(default)]
    pub saturation: f32,
    /// Negative is cooler (bluer), positive warmer (more orange).
    #[serde(default)]
    pub warmth: f32,
    /// `0..=100`.
    #[serde(default)]
    pub sharpen: f32,
    /// Blur radius in picture pixels, `0..=100`.
    #[serde(default)]
    pub blur: f32,
    /// `0..=100`: how far towards black and white.
    #[serde(default)]
    pub grayscale: f32,
    /// `0..=100`.
    #[serde(default)]
    pub sepia: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub invert: bool,
}

impl Adjustments {
    pub fn is_identity(&self) -> bool {
        *self == Adjustments::default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageEl {
    pub asset: AssetId,
    #[serde(default, skip_serializing_if = "CropRect::is_full")]
    pub crop: CropRect,
    #[serde(default)]
    pub fit: Fit,
    #[serde(default)]
    pub corner_radius: f64,
    #[serde(default, skip_serializing_if = "Adjustments::is_identity")]
    pub adjustments: Adjustments,
    /// The filter preset the adjustments came from, for the UI (`"vivid"`, `"mono"`…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Crop the picture to a shape (circle, star…) instead of the frame's rectangle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<ShapeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<StrokeStyle>,
}

impl ImageEl {
    pub fn new(asset: AssetId) -> Self {
        ImageEl {
            asset,
            crop: CropRect::default(),
            fit: Fit::Cover,
            corner_radius: 0.0,
            adjustments: Adjustments::default(),
            filter: None,
            mask: None,
            border: None,
        }
    }
}

// ---------- groups ----------

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupEl {
    /// Children in page coordinates, bottom first. The group's frame is their bounds.
    pub children: Vec<Element>,
}

// ---------- animation ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationKind {
    Fade,
    SlideLeft,
    SlideUp,
    Rise,
    Pop,
    Zoom,
    Bounce,
    Typewriter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub kind: AnimationKind,
    /// Seconds.
    pub duration: f64,
    #[serde(default)]
    pub delay: f64,
}
