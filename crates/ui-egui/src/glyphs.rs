//! Line icons for the editor's chrome (top bar, the bar over the selection, zoom), drawn with the
//! painter in one style: 1.6-point strokes, round shapes, scaled to the icon's box. All original.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use crate::theme::Tokens;

/// An icon of the editor's chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Home,
    Undo,
    Redo,
    Cursor,
    Rect,
    Ellipse,
    Line,
    Text,
    Hand,
    Resize,
    Play,
    Stop,
    Present,
    Download,
    Save,
    More,
    Duplicate,
    Trash,
    Lock,
    Unlock,
    Forward,
    Backward,
    Plus,
    Minus,
    Fit,
    Eye,
    EyeOff,
}

impl Glyph {
    /// The icon of a tool.
    pub fn tool(tool: genha_tools::ToolId) -> Glyph {
        use genha_tools::ToolId;
        match tool {
            ToolId::Select => Glyph::Cursor,
            ToolId::Rect => Glyph::Rect,
            ToolId::Ellipse => Glyph::Ellipse,
            ToolId::Line => Glyph::Line,
            ToolId::Text => Glyph::Text,
            ToolId::Hand => Glyph::Hand,
        }
    }
}

/// Points of an arc around `c` of radius `r` from angle `a0` to `a1` (radians, y down).
fn arc(c: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    (0..=16).map(|i| a0 + (a1 - a0) * i as f32 / 16.0).map(|a| c + vec2(a.cos(), a.sin()) * r).collect()
}

/// Draws `g` in `rect` (square) in `ink`.
pub fn draw(p: &Painter, g: Glyph, rect: Rect, ink: Color32) {
    let s = rect.width().min(rect.height()) / 20.0;
    let c = rect.center();
    let at = |x: f32, y: f32| c + vec2(x * s, y * s);
    let st = Stroke::new(1.6 * s.max(0.8), ink);
    let line = |a: Pos2, b: Pos2| {
        p.line_segment([a, b], st);
    };
    let poly = |pts: Vec<Pos2>| {
        p.add(Shape::line(pts, st));
    };
    match g {
        Glyph::Home => {
            poly(vec![at(-7.0, -1.0), at(0.0, -7.5), at(7.0, -1.0)]);
            poly(vec![at(-5.0, -2.5), at(-5.0, 7.0), at(5.0, 7.0), at(5.0, -2.5)]);
            poly(vec![at(-1.6, 7.0), at(-1.6, 2.5), at(1.6, 2.5), at(1.6, 7.0)]);
        }
        Glyph::Undo | Glyph::Redo => {
            let flip = if g == Glyph::Undo { 1.0 } else { -1.0 };
            let pts: Vec<Pos2> = arc(at(0.0, 2.0), 6.0 * s, std::f32::consts::PI * 1.0, std::f32::consts::PI * 2.1)
                .into_iter()
                .map(|q| pos2(c.x + (q.x - c.x) * flip, q.y))
                .collect();
            if let Some(first) = pts.first().copied() {
                poly(pts);
                line(first, first + vec2(-3.0 * s * flip, -3.5 * s));
                line(first, first + vec2(3.0 * s * flip, -2.5 * s));
            }
        }
        Glyph::Cursor => {
            let pts = vec![at(-4.5, -7.5), at(-4.5, 6.0), at(-1.0, 2.5), at(1.8, 8.0), at(3.8, 7.0), at(1.2, 1.5), at(6.0, 1.5), at(-4.5, -7.5)];
            poly(pts);
        }
        Glyph::Rect => {
            p.rect_stroke(Rect::from_center_size(c, vec2(14.0 * s, 11.0 * s)), 2.0 * s, st, egui::StrokeKind::Middle);
        }
        Glyph::Ellipse => {
            p.circle_stroke(c, 6.8 * s, st);
        }
        Glyph::Line => line(at(-6.5, 6.5), at(6.5, -6.5)),
        Glyph::Text => {
            line(at(-6.0, -6.5), at(6.0, -6.5));
            line(at(0.0, -6.5), at(0.0, 7.0));
            line(at(-2.5, 7.0), at(2.5, 7.0));
        }
        Glyph::Hand => {
            for (x, top) in [(-3.6, -4.0), (-1.2, -6.5), (1.2, -6.0), (3.6, -3.5)] {
                line(at(x, top), at(x, 1.0));
            }
            poly(vec![at(-3.6, 1.0), at(-6.0, -1.0), at(-6.5, 1.5), at(-3.0, 6.5), at(3.0, 7.5), at(4.8, 3.5), at(4.8, -1.0)]);
        }
        Glyph::Resize => {
            p.rect_stroke(Rect::from_min_max(at(-7.0, -2.0), at(2.0, 7.0)), 1.5 * s, st, egui::StrokeKind::Middle);
            poly(vec![at(-2.0, -7.0), at(7.0, -7.0), at(7.0, 2.0)]);
            line(at(1.5, -2.5), at(6.5, -6.5));
        }
        Glyph::Play => {
            let tri = vec![at(-4.0, -6.5), at(6.5, 0.0), at(-4.0, 6.5), at(-4.0, -6.5)];
            poly(tri);
        }
        Glyph::Stop => {
            p.rect_stroke(Rect::from_center_size(c, vec2(11.0 * s, 11.0 * s)), 2.0 * s, st, egui::StrokeKind::Middle);
        }
        Glyph::Present => {
            p.rect_stroke(Rect::from_min_max(at(-8.0, -6.5), at(8.0, 4.5)), 1.5 * s, st, egui::StrokeKind::Middle);
            line(at(0.0, 4.5), at(0.0, 7.5));
            line(at(-4.0, 7.5), at(4.0, 7.5));
            p.add(Shape::convex_polygon(vec![at(-2.0, -3.5), at(3.0, -1.0), at(-2.0, 1.5)], ink, Stroke::NONE));
        }
        Glyph::Download => {
            line(at(0.0, -7.0), at(0.0, 3.5));
            poly(vec![at(-4.0, -0.5), at(0.0, 3.5), at(4.0, -0.5)]);
            poly(vec![at(-7.0, 3.0), at(-7.0, 7.0), at(7.0, 7.0), at(7.0, 3.0)]);
        }
        Glyph::Save => {
            poly(vec![at(-7.0, -7.0), at(4.5, -7.0), at(7.0, -4.5), at(7.0, 7.0), at(-7.0, 7.0), at(-7.0, -7.0)]);
            p.rect_stroke(Rect::from_min_max(at(-4.0, -7.0), at(3.0, -2.5)), 0.5 * s, st, egui::StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_max(at(-4.0, 1.5), at(4.0, 7.0)), 0.5 * s, st, egui::StrokeKind::Middle);
        }
        Glyph::More => {
            for x in [-5.5, 0.0, 5.5] {
                p.circle_filled(at(x, 0.0), 1.6 * s, ink);
            }
        }
        Glyph::Duplicate => {
            p.rect_stroke(Rect::from_min_max(at(-7.0, -3.0), at(3.0, 7.0)), 2.0 * s, st, egui::StrokeKind::Middle);
            poly(vec![at(-3.0, -3.0), at(-3.0, -7.0), at(7.0, -7.0), at(7.0, 3.0), at(3.0, 3.0)]);
        }
        Glyph::Trash => {
            line(at(-7.0, -4.5), at(7.0, -4.5));
            poly(vec![at(-2.5, -4.5), at(-2.5, -7.0), at(2.5, -7.0), at(2.5, -4.5)]);
            poly(vec![at(-5.5, -4.5), at(-4.5, 7.0), at(4.5, 7.0), at(5.5, -4.5)]);
            line(at(-1.5, -1.0), at(-1.5, 4.0));
            line(at(1.5, -1.0), at(1.5, 4.0));
        }
        Glyph::Lock | Glyph::Unlock => {
            p.rect_stroke(Rect::from_min_max(at(-6.0, -1.0), at(6.0, 7.5)), 2.0 * s, st, egui::StrokeKind::Middle);
            let (a0, a1) = if g == Glyph::Lock {
                (std::f32::consts::PI, std::f32::consts::TAU)
            } else {
                (std::f32::consts::PI * 1.05, std::f32::consts::PI * 1.7)
            };
            let mut shackle = arc(at(0.0, -4.0), 4.0 * s, a0, a1);
            if g == Glyph::Lock {
                shackle.insert(0, at(-4.0, -1.0));
                shackle.push(at(4.0, -1.0));
            } else {
                shackle.insert(0, at(-4.0, -1.0));
            }
            poly(shackle);
        }
        Glyph::Forward | Glyph::Backward => {
            // Two sheets; the one moving drawn solid.
            let back = Rect::from_min_max(at(-7.0, -7.0), at(3.0, 3.0));
            let front = Rect::from_min_max(at(-3.0, -3.0), at(7.0, 7.0));
            let (solid, outline) = if g == Glyph::Forward { (front, back) } else { (back, front) };
            p.rect_stroke(outline, 2.0 * s, st, egui::StrokeKind::Middle);
            p.rect_filled(solid, 2.0 * s, ink);
        }
        Glyph::Plus => {
            line(at(-6.0, 0.0), at(6.0, 0.0));
            line(at(0.0, -6.0), at(0.0, 6.0));
        }
        Glyph::Minus => line(at(-6.0, 0.0), at(6.0, 0.0)),
        Glyph::Eye | Glyph::EyeOff => {
            let top = arc(at(0.0, 5.0), 9.0 * s, std::f32::consts::PI * 1.22, std::f32::consts::PI * 1.78);
            let bottom = arc(at(0.0, -5.0), 9.0 * s, std::f32::consts::PI * 0.22, std::f32::consts::PI * 0.78);
            poly(top);
            poly(bottom);
            p.circle_stroke(c, 2.6 * s, st);
            if g == Glyph::EyeOff {
                line(at(-7.0, -7.0), at(7.0, 7.0));
            }
        }
        Glyph::Fit => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let corner = at(7.0 * x, 7.0 * y);
                line(corner, corner + vec2(-4.0 * s * x, 0.0));
                line(corner, corner + vec2(0.0, -4.0 * s * y));
            }
        }
    }
}

/// A square icon button (`side` points) with a soft background when hovered or `on`.
pub fn button(ui: &mut egui::Ui, t: &Tokens, g: Glyph, on: bool, side: f32) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(vec2(side, side), egui::Sense::click());
    let enabled = ui.is_enabled();
    if on {
        ui.painter().rect_filled(rect, 9, t.accent_soft);
    } else if r.hovered() && enabled {
        ui.painter().rect_filled(rect, 9, t.panel_alt);
    }
    let ink = if !enabled {
        t.text_dim.gamma_multiply(0.5)
    } else if on {
        t.accent
    } else {
        t.text
    };
    draw(ui.painter(), g, Rect::from_center_size(rect.center(), vec2(side * 0.55, side * 0.55)), ink);
    r
}

/// A button with an icon and a label (`accent`: filled with the accent colour).
pub fn labelled(ui: &mut egui::Ui, t: &Tokens, g: Glyph, text: &str, accent: bool) -> egui::Response {
    let font = egui::FontId::proportional(13.5);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, if accent { t.accent_text } else { t.text });
    let size = vec2(galley.size().x + 46.0, 34.0);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = match (accent, r.hovered()) {
        (true, true) => t.accent.gamma_multiply(0.9),
        (true, false) => t.accent,
        (false, true) => t.accent_soft,
        (false, false) => t.panel_alt,
    };
    ui.painter().rect_filled(rect, 10, fill);
    let ink = if accent { t.accent_text } else { t.text };
    draw(ui.painter(), g, Rect::from_center_size(rect.left_center() + vec2(19.0, 0.0), vec2(17.0, 17.0)), ink);
    ui.painter().galley(pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0), galley, ink);
    r
}
