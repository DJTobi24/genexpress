//! SVG import: an SVG file becomes editable shapes (one path element per SVG path, with its fill,
//! gradient and border). Text inside the SVG is not imported yet; embedded pictures are skipped.

use genha_color::Color;
use genha_doc::{Element, ElementKind, Fill, GradientStop, NodeId, ShapeEl, ShapeKind, StrokeStyle};
use genha_geom::Frame;
use kurbo::{Affine, BezPath, Point, Shape};

/// Largest SVG file accepted.
pub const MAX_BYTES: usize = 20 << 20;
/// Most paths imported from one file.
pub const MAX_PATHS: usize = 5000;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("not a readable SVG: {0}")]
    Parse(String),
    #[error("the SVG is larger than {} MB", MAX_BYTES >> 20)]
    TooLarge,
    #[error("the SVG has nothing to import (only paths and shapes are imported)")]
    Empty,
}

/// The imported drawing: its size and its shapes (ids are 0; the caller assigns them).
#[derive(Clone, Debug)]
pub struct ImportedSvg {
    pub width: f64,
    pub height: f64,
    pub elements: Vec<Element>,
    /// Paths dropped past [`MAX_PATHS`] and embedded pictures skipped (text never reaches us).
    pub skipped: usize,
}

/// Whether `bytes` look like an SVG document.
pub fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start_matches('\u{feff}').trim_start();
    t.starts_with("<svg") || ((t.starts_with("<?xml") || t.starts_with("<!--") || t.starts_with("<!DOCTYPE")) && t.contains("<svg"))
}

pub fn import(bytes: &[u8]) -> Result<ImportedSvg, ImportError> {
    if bytes.len() > MAX_BYTES {
        return Err(ImportError::TooLarge);
    }
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).map_err(|e| ImportError::Parse(e.to_string()))?;
    let size = tree.size();
    let mut out = ImportedSvg { width: f64::from(size.width()), height: f64::from(size.height()), elements: vec![], skipped: 0 };
    walk(tree.root(), 1.0, &mut out);
    if out.elements.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(out)
}

fn walk(group: &usvg::Group, opacity: f32, out: &mut ImportedSvg) {
    let opacity = opacity * group.opacity().get();
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(g, opacity, out),
            usvg::Node::Path(p) => {
                if out.elements.len() >= MAX_PATHS {
                    out.skipped += 1;
                } else if let Some(el) = path_element(p, opacity) {
                    out.elements.push(el);
                }
            }
            usvg::Node::Image(_) | usvg::Node::Text(_) => out.skipped += 1,
        }
    }
}

fn affine(t: usvg::Transform) -> Affine {
    Affine::new([f64::from(t.sx), f64::from(t.ky), f64::from(t.kx), f64::from(t.sy), f64::from(t.tx), f64::from(t.ty)])
}

fn color(c: usvg::Color, alpha: f32) -> Color {
    Color::rgba(c.red, c.green, c.blue, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn paint(p: &usvg::Paint, alpha: f32) -> Fill {
    let stops = |s: &[usvg::Stop]| -> Vec<GradientStop> {
        s.iter().map(|st| GradientStop { offset: st.offset().get(), color: color(st.color(), st.opacity().get() * alpha) }).collect()
    };
    match p {
        usvg::Paint::Color(c) => Fill::solid(color(*c, alpha)),
        usvg::Paint::LinearGradient(g) => {
            let angle = f64::from(g.y2() - g.y1()).atan2(f64::from(g.x2() - g.x1())).to_degrees();
            Fill::Linear { angle: if angle.is_finite() { angle } else { 0.0 }, stops: stops(g.stops()) }
        }
        usvg::Paint::RadialGradient(g) => Fill::Radial { stops: stops(g.stops()) },
        usvg::Paint::Pattern(_) => Fill::solid(Color::rgba(128, 128, 128, (alpha * 255.0) as u8)),
    }
}

fn path_element(p: &usvg::Path, opacity: f32) -> Option<Element> {
    if !p.is_visible() {
        return None;
    }
    let m = affine(p.abs_transform());
    let mut path = BezPath::new();
    let pt = |q: usvg::tiny_skia_path::Point| m * Point::new(f64::from(q.x), f64::from(q.y));
    for seg in p.data().segments() {
        use usvg::tiny_skia_path::PathSegment as S;
        match seg {
            S::MoveTo(a) => path.move_to(pt(a)),
            S::LineTo(a) => path.line_to(pt(a)),
            S::QuadTo(a, b) => path.quad_to(pt(a), pt(b)),
            S::CubicTo(a, b, c) => path.curve_to(pt(a), pt(b), pt(c)),
            S::Close => path.close_path(),
        }
    }
    let b = path.bounding_box();
    if ![b.x0, b.y0, b.x1, b.y1].iter().all(|v| v.is_finite()) {
        return None;
    }
    let (w, h) = (b.width().max(1e-3), b.height().max(1e-3));
    let local = Affine::translate((-b.x0, -b.y0)) * path;
    let fill = p.fill().map(|f| paint(f.paint(), f.opacity().get())).unwrap_or(Fill::None);
    let scale = m.determinant().abs().sqrt();
    let stroke = p.stroke().and_then(|s| {
        let c = paint(s.paint(), s.opacity().get()).primary_color()?;
        let width = f64::from(s.width().get()) * if scale.is_finite() { scale } else { 1.0 };
        Some(StrokeStyle {
            color: c,
            width,
            dash: s.dasharray().map(|d| d.iter().map(|v| f64::from(*v) * scale).take(16).collect()).unwrap_or_default(),
        })
    });
    if fill == Fill::None && stroke.is_none() {
        return None;
    }
    let shape = ShapeKind::Path { d: local.to_svg(), view_width: w, view_height: h };
    let mut el = Element::new(NodeId(0), Frame::new(b.x0, b.y0, w, h), ElementKind::Shape(ShapeEl { shape, fill, stroke, shadow: None }));
    el.opacity = opacity.clamp(0.0, 1.0);
    if !p.id().is_empty() {
        el.name = p.id().chars().take(60).collect();
    }
    Some(el)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">
  <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs>
  <rect id="box" x="10" y="10" width="50" height="40" fill="#00ff00"/>
  <g transform="translate(100 0)" opacity="0.5">
    <circle cx="25" cy="50" r="20" fill="url(#g)" stroke="#000" stroke-width="2"/>
  </g>
  <text x="0" y="90">ignored</text>
</svg>"##;

    #[test]
    fn imports_shapes_with_paint_and_position() {
        assert!(is_svg(SAMPLE.as_bytes()));
        assert!(!is_svg(b"\x89PNG...."));
        let r = import(SAMPLE.as_bytes()).unwrap();
        assert_eq!((r.width, r.height), (200.0, 100.0));
        assert_eq!(r.elements.len(), 2);
        let rect = &r.elements[0];
        assert_eq!(rect.name, "box");
        assert!((rect.frame.x - 10.0).abs() < 1e-6 && (rect.frame.width - 50.0).abs() < 1e-6);
        let ElementKind::Shape(s) = &rect.kind else { panic!() };
        assert_eq!(s.fill, Fill::solid(Color::rgb(0, 255, 0)));
        let circle = &r.elements[1];
        assert!((circle.frame.x - 105.0).abs() < 1e-6, "{:?}", circle.frame);
        assert_eq!(circle.opacity, 0.5);
        let ElementKind::Shape(s) = &circle.kind else { panic!() };
        assert!(matches!(s.fill, Fill::Linear { .. }));
        assert!(s.stroke.is_some());
        // Text is dropped by the parser (usvg is built without its font support).
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(import(b"<svg").is_err());
        assert!(import(b"not svg").is_err());
        assert!(matches!(import(br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"></svg>"#), Err(ImportError::Empty)));
    }
}
