//! Synthetic bold and italic. When a family has no bold (or italic) face, text is set in the
//! regular one and drawn emboldened (its outline also stroked, in the text's paint, so every stem
//! gets thicker) or slanted (sheared about each line's baseline), as browsers and word processors
//! do. `genha_text::FontBook::needs_synthetic` says when; the renderer and the SVG writer apply it
//! through [`layout_text`] and [`Synthetic::embolden`].

use genha_doc::TextStyle;
use genha_text::{FontBook, TextLayout};
use kurbo::{Affine, BezPath};

/// Horizontal shear of synthetic italic (about 12 degrees).
pub const SYNTHETIC_SLANT: f64 = 0.21;
/// Extra stem thickness of synthetic bold for regular → bold (300 weight units), in ems.
pub const SYNTHETIC_BOLD_EM: f64 = 0.04;

/// What drawing must add to a text style's face.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Synthetic {
    /// Width (pixels) of the stroke drawn over the letters in their own paint; 0: none.
    pub embolden: f64,
    /// Shear about the baseline (x moves right by `slant` × height above it); 0: none.
    pub slant: f64,
}

/// The synthetic style text in `style` needs with `fonts`.
pub fn synthetic(fonts: &FontBook, style: &TextStyle) -> Synthetic {
    let (lacks, slant) = fonts.needs_synthetic(&style.font_family, style.bold, style.italic);
    let size = if style.font_size.is_finite() { style.font_size.clamp(0.0, genha_text::MAX_FONT_SIZE) } else { 0.0 };
    let lacks = if lacks.is_finite() { f64::from(lacks).clamp(0.0, 900.0) } else { 0.0 };
    Synthetic {
        embolden: if lacks > 0.0 { size * SYNTHETIC_BOLD_EM * (lacks / 300.0).clamp(0.5, 1.8) } else { 0.0 },
        slant: if slant { SYNTHETIC_SLANT } else { 0.0 },
    }
}

/// `path` sheared by `slant` about the horizontal line `y = baseline` (y down).
pub fn slant_about(path: BezPath, baseline: f64, slant: f64) -> BezPath {
    if slant == 0.0 || !baseline.is_finite() {
        return path;
    }
    Affine::new([1.0, 0.0, -slant, 1.0, slant * baseline, 0.0]) * path
}

/// [`genha_text::layout`] with synthetic italic already applied to the lines' outlines, and the
/// synthetic bold still to draw ([`Synthetic::embolden`]).
pub fn layout_text(fonts: &FontBook, text: &str, style: &TextStyle, box_width: f64) -> (TextLayout, Synthetic) {
    let mut layout = genha_text::layout(fonts, text, style, box_width);
    let syn = synthetic(fonts, style);
    if syn.slant != 0.0 {
        for line in &mut layout.lines {
            line.path = slant_about(std::mem::take(&mut line.path), line.baseline, syn.slant);
        }
    }
    (layout, syn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Shape;

    #[test]
    fn slant_moves_tops_right_and_keeps_the_baseline() {
        let mut p = BezPath::new();
        p.move_to((0.0, 100.0));
        p.line_to((0.0, 0.0));
        let s = slant_about(p, 100.0, 0.2);
        let b = s.bounding_box();
        assert!((b.x0 - 0.0).abs() < 1e-9 && (b.x1 - 20.0).abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn no_fonts_no_synthesis() {
        let st = TextStyle { bold: true, italic: true, ..TextStyle::default() };
        assert_eq!(synthetic(&FontBook::new(), &st), Synthetic::default());
    }
}
