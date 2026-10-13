//! Curved text: glyphs placed one by one along circular arcs.
//!
//! [`TextEl::curve`] is the angle (degrees) the widest line spans, so the arc's radius is that
//! line's width divided by the angle. Each glyph's centre keeps its distance from the line's centre,
//! measured along the arc, and is turned to follow it. Positive curves put the baseline on the top
//! of the circle (an arch, letters pointing outwards); negative ones on the bottom (a smile, letters
//! pointing inwards). Further lines sit on concentric arcs below the first. The bent text is
//! centred in the element's box. Each style run keeps its own size, colour and synthetic style.

use genha_doc::TextEl;
use genha_text::FontBook;
use kurbo::{Affine, BezPath, Point, Rect, Shape, Vec2};

use crate::StyledPath;

/// Curves smaller than this (degrees) draw straight.
const MIN_CURVE: f64 = 0.5;

/// The bent glyphs by style and the box of their line bands, before centring.
fn bend(fonts: &FontBook, t: &TextEl, w: f64) -> Option<(Vec<StyledPath>, Rect)> {
    let deg = t.curve_degrees();
    if deg.abs() < MIN_CURVE {
        return None;
    }
    let gl = genha_text::layout_glyphs_el(fonts, t, w);
    let syns: Vec<crate::Synthetic> = gl.styles.iter().map(|st| crate::synthetic(fonts, st)).collect();
    let mut groups: Vec<StyledPath> =
        gl.styles.iter().zip(&syns).map(|(st, sy)| StyledPath { style: st.clone(), path: BezPath::new(), embolden: sy.embolden }).collect();
    let widest = gl.width;
    let first = gl.lines.first()?;
    if !(widest.is_finite() && widest > 0.0) {
        return None;
    }
    let theta = deg.abs().to_radians();
    let r0 = widest / theta;
    let arch = deg > 0.0;
    let box_w = if w.is_finite() && w > 0.0 { w } else { widest };
    let cx = box_w / 2.0;
    let b0 = first.baseline;
    let center = Point::new(cx, if arch { b0 + r0 } else { b0 - r0 });
    let min_r = t.style.font_size.max(1.0) * 0.2;
    let mut bounds: Option<Rect> = None;
    for line in &gl.lines {
        let dr = line.baseline - b0;
        let r = if arch { r0 - dr } else { r0 + dr }.max(min_r);
        for g in &line.glyphs {
            let gc = g.x + g.advance / 2.0;
            let phi = (gc - cx) / r;
            let (pos, rot) = if arch {
                (center + Vec2::new(r * phi.sin(), -r * phi.cos()), phi)
            } else {
                (center + Vec2::new(r * phi.sin(), r * phi.cos()), -phi)
            };
            let xf = Affine::translate(pos.to_vec2()) * Affine::rotate(rot) * Affine::translate((-gc, -line.baseline));
            let cell = Rect::new(g.x, line.top, g.x + g.advance, line.top + line.height);
            let cb = (xf * cell.to_path(0.1)).bounding_box();
            bounds = Some(bounds.map_or(cb, |b| b.union(cb)));
            if !g.path.is_empty() {
                let slant = syns.get(g.style).map_or(0.0, |s| s.slant);
                let p = xf * crate::slant_about(g.path.clone(), line.baseline, slant);
                let pb = p.bounding_box();
                bounds = Some(bounds.map_or(pb, |b| b.union(pb)));
                if let Some(grp) = groups.get_mut(g.style) {
                    grp.path.extend(p);
                }
            }
        }
    }
    groups.retain(|g| !g.path.is_empty());
    Some((groups, bounds?))
}

/// The glyph outlines of `t` bent along its curve and centred in a `w` × `h` box, by style, or
/// None when the text is straight.
pub fn curved_text_groups(fonts: &FontBook, t: &TextEl, w: f64, h: f64) -> Option<Vec<StyledPath>> {
    let (mut groups, bounds) = bend(fonts, t, w)?;
    let c = bounds.center();
    let target = Point::new(if w.is_finite() { w / 2.0 } else { c.x }, if h.is_finite() { h / 2.0 } else { c.y });
    let xf = Affine::translate(target - c);
    for g in &mut groups {
        g.path.apply_affine(xf);
    }
    Some(groups)
}

/// The glyph outlines of `t` bent along its curve and centred in a `w` × `h` box (every style
/// together), or None when the text is straight.
pub fn curved_text_path(fonts: &FontBook, t: &TextEl, w: f64, h: f64) -> Option<BezPath> {
    let groups = curved_text_groups(fonts, t, w, h)?;
    let mut path = BezPath::new();
    for g in groups {
        path.extend(g.path);
    }
    Some(path)
}

/// The size bent text takes in a box `w` wide (its frame can be fitted to it), or None when
/// the text is straight.
pub fn curved_text_size(fonts: &FontBook, t: &TextEl, w: f64) -> Option<(f64, f64)> {
    bend(fonts, t, w).map(|(_, b)| (b.width(), b.height()))
}
