//! Icons and illustrations for the home screen, drawn in code: icons are GenHa's own library
//! icons (`genha_templates::shape_library`, rendered once in white and tinted when drawn) or simple
//! vector shapes; illustrations are compositions of rounded shapes, lines and icons. Colours come
//! from [`Tokens`].

use std::sync::Arc;

use egui::{Color32, Painter, Pos2, Rect, Stroke, pos2, vec2};

use crate::theme::Tokens;
use crate::thumbs::Thumbs;

/// An icon: one of the library's, or one drawn here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// A library icon by id (`home`, `search`, `settings`, `upload`…).
    Lib(&'static str),
    Plus,
    Folder,
    Grid,
    Brand,
    Dots,
    Close,
    /// A circle, a square and a triangle: shapes and other elements.
    Shapes,
    /// A capital T: text.
    Text,
    /// Three stacked sheets: layers.
    Layers,
    /// A sheet with a folded corner: the page.
    Page,
}

/// Draws icons and illustrations; holds what rendering library icons needs.
pub struct Art<'a> {
    pub thumbs: &'a mut Thumbs,
    pub ctx: egui::Context,
    pub fonts: Arc<genha_text::FontBook>,
}

/// Rendered size of library icons (pixels): crisp at 2× on icons up to 48 points.
const ICON_PX: u32 = 96;

impl Art<'_> {
    pub fn new<'a>(thumbs: &'a mut Thumbs, ctx: &egui::Context, fonts: Arc<genha_text::FontBook>) -> Art<'a> {
        Art { thumbs, ctx: ctx.clone(), fonts }
    }

    /// A library shape rendered white on transparent (tinted when drawn).
    fn shape_texture(&mut self, key: &str, shape: impl FnOnce() -> Option<genha_doc::ShapeKind>) -> Option<egui::TextureHandle> {
        let fonts = self.fonts.clone();
        self.thumbs.get_now(&self.ctx, &format!("icon:{key}"), 0, || {
            let shape = shape()?;
            let mut doc = genha_doc::Document::new("icon", 100.0, 100.0);
            if let Some(p) = doc.pages.first_mut() {
                p.background = genha_doc::Fill::None;
            }
            let id = doc.alloc_id();
            let el = genha_doc::Element::new(
                id,
                genha_doc::Frame::new(0.0, 0.0, 100.0, 100.0),
                genha_doc::ElementKind::Shape(genha_doc::ShapeEl {
                    shape,
                    fill: genha_doc::Fill::solid(genha_color::Color::WHITE),
                    stroke: None,
                    shadow: None,
                }),
            );
            doc.pages.first_mut()?.elements.push(el);
            let opts = genha_render::RenderOptions { scale: f64::from(ICON_PX) / 100.0, transparent_background: true };
            genha_render::render_page(&doc, 0, &fonts, &opts).ok()
        })
    }

    /// Draws `icon` in `rect` (square) in `color`.
    pub fn icon(&mut self, p: &Painter, icon: Icon, rect: Rect, color: Color32) {
        let r = Rect::from_center_size(rect.center(), egui::Vec2::splat(rect.width().min(rect.height())));
        let w = r.width();
        match icon {
            Icon::Lib(id) => {
                let tex = self.shape_texture(id, || genha_templates::shape_library().into_iter().find(|s| s.id == id).map(|s| s.shape));
                if let Some(tex) = tex {
                    p.image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), color);
                }
            }
            Icon::Plus => {
                let s = Stroke::new(w * 0.12, color);
                p.line_segment([pos2(r.center().x, r.top() + w * 0.18), pos2(r.center().x, r.bottom() - w * 0.18)], s);
                p.line_segment([pos2(r.left() + w * 0.18, r.center().y), pos2(r.right() - w * 0.18, r.center().y)], s);
            }
            Icon::Close => {
                let s = Stroke::new(w * 0.1, color);
                let a = w * 0.26;
                let c = r.center();
                p.line_segment([c + vec2(-a, -a), c + vec2(a, a)], s);
                p.line_segment([c + vec2(-a, a), c + vec2(a, -a)], s);
            }
            Icon::Folder => {
                let body = Rect::from_min_max(pos2(r.left() + w * 0.08, r.top() + w * 0.3), pos2(r.right() - w * 0.08, r.bottom() - w * 0.14));
                let tab = Rect::from_min_size(pos2(body.left(), r.top() + w * 0.18), vec2(w * 0.38, w * 0.2));
                p.rect_filled(tab, w * 0.06, color);
                p.rect_filled(body, w * 0.08, color);
            }
            Icon::Grid => {
                let g = w * 0.1;
                let cell = (w * 0.8 - g) / 2.0;
                for (i, j) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                    let min = pos2(r.left() + w * 0.1 + i * (cell + g), r.top() + w * 0.1 + j * (cell + g));
                    p.rect_filled(Rect::from_min_size(min, vec2(cell, cell)), w * 0.07, color);
                }
            }
            Icon::Brand => {
                // Three colour dots, the brand's colours.
                let rad = w * 0.2;
                let c = r.center();
                p.circle_filled(c + vec2(-w * 0.17, w * 0.12), rad, color);
                p.circle_filled(c + vec2(w * 0.17, w * 0.12), rad, color.gamma_multiply(0.75));
                p.circle_filled(c + vec2(0.0, -w * 0.17), rad, color.gamma_multiply(0.5));
            }
            Icon::Dots => {
                for dx in [-0.25, 0.0, 0.25] {
                    p.circle_filled(r.center() + vec2(w * dx, 0.0), w * 0.08, color);
                }
            }
            Icon::Shapes => {
                p.circle_filled(pos2(r.left() + w * 0.3, r.top() + w * 0.3), w * 0.2, color);
                p.rect_filled(Rect::from_min_size(pos2(r.left() + w * 0.56, r.top() + w * 0.12), vec2(w * 0.34, w * 0.34)), w * 0.05, color);
                let tri = vec![
                    pos2(r.center().x, r.top() + w * 0.54),
                    pos2(r.right() - w * 0.22, r.bottom() - w * 0.08),
                    pos2(r.left() + w * 0.22, r.bottom() - w * 0.08),
                ];
                p.add(egui::Shape::convex_polygon(tri, color, Stroke::NONE));
            }
            Icon::Text => {
                let bar = w * 0.13;
                p.rect_filled(Rect::from_min_size(pos2(r.left() + w * 0.14, r.top() + w * 0.12), vec2(w * 0.72, bar)), w * 0.03, color);
                p.rect_filled(Rect::from_center_size(pos2(r.center().x, r.center().y + w * 0.06), vec2(bar, w * 0.7)), w * 0.03, color);
            }
            Icon::Page => {
                let fold = w * 0.26;
                let (l, t, rt, b) = (r.left() + w * 0.2, r.top() + w * 0.08, r.right() - w * 0.2, r.bottom() - w * 0.08);
                let body = vec![pos2(l, t), pos2(rt - fold, t), pos2(rt, t + fold), pos2(rt, b), pos2(l, b)];
                p.add(egui::Shape::convex_polygon(body, color, Stroke::NONE));
                let corner = vec![pos2(rt - fold, t), pos2(rt - fold, t + fold), pos2(rt, t + fold)];
                p.add(egui::Shape::convex_polygon(corner, color.gamma_multiply(0.5), Stroke::NONE));
            }
            Icon::Layers => {
                // Three diamonds, the top one solid.
                let c = r.center();
                let diamond = |dy: f32| vec![c + vec2(0.0, dy - w * 0.2), c + vec2(w * 0.4, dy), c + vec2(0.0, dy + w * 0.2), c + vec2(-w * 0.4, dy)];
                for (dy, a) in [(w * 0.24, 0.45), (w * 0.08, 0.7)] {
                    p.add(egui::Shape::convex_polygon(diamond(dy), color.gamma_multiply(a), Stroke::NONE));
                }
                p.add(egui::Shape::convex_polygon(diamond(-w * 0.1), color, Stroke::NONE));
            }
        }
    }

    /// A QR code (of "GenHa") in `color`, for the QR quick action.
    fn qr(&mut self, p: &Painter, rect: Rect, color: Color32) {
        let tex = self.shape_texture("qr-sample", || genha_templates::qr_code("https://genha.app", genha_color::Color::BLACK).ok());
        if let Some(tex) = tex {
            p.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), color);
        }
    }
}

/// A rounded rectangle filled with a diagonal gradient from `a` (top left) to `b` (bottom right),
/// with soft (anti-aliased) edges.
pub fn gradient_rect(p: &Painter, rect: Rect, radius: f32, a: Color32, b: Color32) {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let color = |q: Pos2| {
        let tx = ((q.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        let ty = ((q.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        a.lerp_to_gamma(b, (tx + ty) / 2.0)
    };
    // Outline points with their outward normals, clockwise from the top-left corner's arc.
    let mut ring: Vec<(Pos2, egui::Vec2)> = vec![];
    let corners = [
        (pos2(rect.left() + radius, rect.top() + radius), 180.0f32),
        (pos2(rect.right() - radius, rect.top() + radius), 270.0),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (pos2(rect.left() + radius, rect.bottom() - radius), 90.0),
    ];
    const STEPS: usize = 8;
    for (c, start) in corners {
        for i in 0..=STEPS {
            let ang = (start + 90.0 * i as f32 / STEPS as f32).to_radians();
            let n = vec2(ang.cos(), ang.sin());
            ring.push((c + n * radius, n));
        }
    }
    let mut mesh = egui::epaint::Mesh::default();
    let center = rect.center();
    mesh.colored_vertex(center, color(center));
    let n = ring.len() as u32;
    for (q, _) in &ring {
        mesh.colored_vertex(*q, color(*q));
    }
    // A one-point fade outside the edge.
    for (q, nrm) in &ring {
        let o = *q + *nrm;
        mesh.colored_vertex(o, Color32::TRANSPARENT);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.add_triangle(0, 1 + i, 1 + j);
        mesh.add_triangle(1 + i, 1 + n + i, 1 + n + j);
        mesh.add_triangle(1 + i, 1 + n + j, 1 + j);
    }
    p.add(egui::Shape::mesh(mesh));
}

/// Paper with a soft shadow.
fn paper(p: &Painter, t: &Tokens, r: Rect, radius: f32) {
    p.rect_filled(r.translate(vec2(0.0, 3.0)).expand(1.0), radius + 1.0, t.art_shadow);
    p.rect_filled(r, radius, t.art_paper);
}

/// The five start tiles' illustrations, in `r` with the tile family's `ink`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    OpenFile,
    NewDesign,
    EditPhoto,
    Video,
    Presentation,
}

pub fn scene(art: &mut Art, p: &Painter, t: &Tokens, scene: Scene, r: Rect, ink: Color32) {
    let s = (r.height() / 100.0).min(r.width() / 150.0);
    let deep = if t.dark { ink.lerp_to_gamma(t.bg, 0.5) } else { ink };
    let on_ink = if t.dark { t.bg } else { t.art_paper };
    let c = r.center();
    let at = |x: f32, y: f32| c + vec2(x * s, y * s);
    let size = |w: f32, h: f32| vec2(w * s, h * s);
    match scene {
        Scene::OpenFile => {
            let zone = Rect::from_center_size(c, size(132.0, 84.0));
            let pts = rounded_outline(zone, 12.0 * s);
            for sh in egui::Shape::dashed_line(&pts, Stroke::new(1.6 * s.max(0.8), ink.gamma_multiply(0.7)), 6.0 * s, 4.0 * s) {
                p.add(sh);
            }
            p.circle_filled(at(0.0, -10.0), 20.0 * s, t.art_paper);
            art.icon(p, Icon::Lib("upload"), Rect::from_center_size(at(0.0, -10.0), size(22.0, 22.0)), deep);
            for (i, label) in [".png", ".svg", ".genha"].iter().enumerate() {
                let x = (i as f32 - 1.0) * 38.0;
                let chip = Rect::from_center_size(at(x, 26.0), size(34.0, 14.0));
                p.rect_filled(chip, 7.0 * s, t.art_paper);
                p.text(chip.center(), egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(8.5 * s), deep);
            }
        }
        Scene::NewDesign => {
            let pages = [(-38.0, 4.0, 40.0, 56.0), (8.0, -6.0, 48.0, 48.0), (40.0, 18.0, 58.0, 36.0)];
            for (i, (x, y, w, h)) in pages.iter().enumerate() {
                let pr = Rect::from_center_size(at(*x, *y), size(*w, *h));
                paper(p, t, pr, 5.0 * s);
                // A few lines of "content".
                let a = ink.gamma_multiply(0.25 + 0.2 * i as f32);
                p.rect_filled(Rect::from_min_size(pr.min + size(6.0, 7.0), size(w * 0.55, 5.0)), 2.5 * s, a);
                p.rect_filled(Rect::from_min_size(pr.min + size(6.0, 16.0), size(w * 0.35, 4.0)), 2.0 * s, ink.gamma_multiply(0.18));
            }
            p.circle_filled(at(52.0, -28.0), 14.0 * s, ink);
            art.icon(p, Icon::Plus, Rect::from_center_size(at(52.0, -28.0), size(18.0, 18.0)), on_ink);
        }
        Scene::EditPhoto => {
            let photo = Rect::from_center_size(at(-6.0, 0.0), size(96.0, 70.0));
            paper(p, t, photo, 6.0 * s);
            let pic = photo.shrink(5.0 * s);
            p.rect_filled(pic, 3.0 * s, ink.gamma_multiply(0.18));
            p.circle_filled(pic.min + size(64.0, 14.0), 7.0 * s, ink.gamma_multiply(0.55));
            let base = pic.bottom();
            let m1 = vec![pos2(pic.left(), base), pos2(pic.left() + 30.0 * s, base - 34.0 * s), pos2(pic.left() + 56.0 * s, base)];
            let m2 = vec![pos2(pic.left() + 34.0 * s, base), pos2(pic.left() + 62.0 * s, base - 26.0 * s), pos2(pic.right(), base)];
            p.add(egui::Shape::convex_polygon(m1, ink.gamma_multiply(0.75), Stroke::NONE));
            p.add(egui::Shape::convex_polygon(m2, ink.gamma_multiply(0.45), Stroke::NONE));
            // A slider (adjustments) and a sparkle (magic).
            let y = 44.0;
            p.line_segment([at(-40.0, y), at(28.0, y)], Stroke::new(3.0 * s, t.art_paper));
            p.line_segment([at(-40.0, y), at(4.0, y)], Stroke::new(3.0 * s, ink));
            p.circle_filled(at(4.0, y), 5.0 * s, t.art_paper);
            p.circle_stroke(at(4.0, y), 5.0 * s, Stroke::new(1.5 * s, ink));
            p.circle_filled(at(50.0, -30.0), 15.0 * s, t.art_paper);
            art.icon(p, Icon::Lib("sparkle"), Rect::from_center_size(at(50.0, -30.0), size(18.0, 18.0)), deep);
        }
        Scene::Video => {
            let phone = Rect::from_center_size(at(-26.0, -4.0), size(46.0, 80.0));
            paper(p, t, phone, 7.0 * s);
            let screen = phone.shrink(4.0 * s);
            gradient_rect(p, screen, 4.0 * s, ink.gamma_multiply(0.35), ink.gamma_multiply(0.8));
            p.circle_filled(screen.center(), 11.0 * s, t.art_paper);
            let tri = vec![screen.center() + size(-3.5, -6.0), screen.center() + size(6.5, 0.0), screen.center() + size(-3.5, 6.0)];
            p.add(egui::Shape::convex_polygon(tri, deep, Stroke::NONE));
            // A timeline with keyframes.
            let track = Rect::from_min_size(at(10.0, -2.0), size(56.0, 8.0));
            p.rect_filled(track, 4.0 * s, t.art_paper);
            p.rect_filled(Rect::from_min_size(track.min, size(34.0, 8.0)), 4.0 * s, ink);
            for (i, w) in [40.0, 26.0, 48.0].iter().enumerate() {
                let row = Rect::from_min_size(at(10.0, 14.0 + i as f32 * 11.0), size(*w, 6.0));
                p.rect_filled(row, 3.0 * s, ink.gamma_multiply(0.3 + 0.15 * i as f32));
            }
            for x in [16.0, 34.0, 52.0] {
                p.circle_filled(at(x, -14.0), 3.0 * s, ink.gamma_multiply(0.6));
            }
        }
        Scene::Presentation => {
            let slide = Rect::from_center_size(at(0.0, -8.0), size(112.0, 63.0));
            paper(p, t, slide, 5.0 * s);
            p.rect_filled(Rect::from_min_size(slide.min + size(9.0, 9.0), size(46.0, 6.0)), 3.0 * s, ink);
            p.rect_filled(Rect::from_min_size(slide.min + size(9.0, 19.0), size(30.0, 4.0)), 2.0 * s, ink.gamma_multiply(0.3));
            for (i, h) in [16.0, 26.0, 20.0, 34.0].iter().enumerate() {
                let x = slide.left() + 62.0 * s + i as f32 * 11.0 * s;
                let bar = Rect::from_min_max(pos2(x, slide.bottom() - 8.0 * s - h * s), pos2(x + 7.0 * s, slide.bottom() - 8.0 * s));
                p.rect_filled(bar, 2.0 * s, ink.gamma_multiply(0.45 + 0.15 * i as f32));
            }
            // The stand.
            p.line_segment([at(0.0, 24.0), at(0.0, 40.0)], Stroke::new(3.0 * s, ink.gamma_multiply(0.6)));
            p.line_segment([at(-16.0, 42.0), at(16.0, 42.0)], Stroke::new(3.0 * s, ink.gamma_multiply(0.6)));
        }
    }
}

/// The points around a rounded rectangle (closed).
fn rounded_outline(r: Rect, radius: f32) -> Vec<Pos2> {
    let mut pts = vec![];
    let corners = [
        (pos2(r.left() + radius, r.top() + radius), 180.0f32),
        (pos2(r.right() - radius, r.top() + radius), 270.0),
        (pos2(r.right() - radius, r.bottom() - radius), 0.0),
        (pos2(r.left() + radius, r.bottom() - radius), 90.0),
    ];
    for (c, start) in corners {
        for i in 0..=6 {
            let a = (start + 90.0 * i as f32 / 6.0).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * radius);
        }
    }
    if let Some(first) = pts.first().copied() {
        pts.push(first);
    }
    pts
}

/// A quick action's illustration in `r`, with `ink`.
pub fn action(art: &mut Art, p: &Painter, t: &Tokens, id: &str, r: Rect, ink: Color32) {
    let s = (r.height() / 64.0).min(r.width() / 120.0);
    let deep = if t.dark { ink.lerp_to_gamma(t.bg, 0.5) } else { ink };
    let on_ink = if t.dark { t.bg } else { t.art_paper };
    let c = r.center();
    let at = |x: f32, y: f32| c + vec2(x * s, y * s);
    let size = |w: f32, h: f32| vec2(w * s, h * s);
    let person = |art: &mut Art, center: Pos2, side: f32, color: Color32| {
        art.icon(p, Icon::Lib("user"), Rect::from_center_size(center, egui::Vec2::splat(side)), color);
    };
    match id {
        "remove-background" => {
            let sq = Rect::from_center_size(c, size(54.0, 48.0));
            // Checkerboard (transparency) on the right half, a backdrop on the left.
            let cell = 6.0 * s;
            let (cols, rows) = ((sq.width() / cell).ceil() as i32, (sq.height() / cell).ceil() as i32);
            p.rect_filled(sq, 4.0 * s, t.art_paper);
            for i in cols / 2..cols {
                for j in 0..rows {
                    if (i + j) % 2 == 0 {
                        let cr = Rect::from_min_size(sq.min + vec2(i as f32 * cell, j as f32 * cell), egui::Vec2::splat(cell)).intersect(sq);
                        p.rect_filled(cr, 0.0, ink.gamma_multiply(0.14));
                    }
                }
            }
            let left = Rect::from_min_max(sq.min, pos2(sq.center().x, sq.bottom()));
            p.rect_filled(left, egui::CornerRadius { nw: (4.0 * s) as u8, sw: (4.0 * s) as u8, ne: 0, se: 0 }, ink.gamma_multiply(0.3));
            person(art, sq.center() + size(0.0, 4.0), 36.0 * s, ink);
            p.line_segment([pos2(sq.center().x, sq.top()), pos2(sq.center().x, sq.bottom())], Stroke::new(1.5 * s, t.art_paper));
        }
        "resize" => {
            let big = Rect::from_center_size(at(-6.0, 3.0), size(56.0, 42.0));
            let pts = rounded_outline(big, 4.0 * s);
            for sh in egui::Shape::dashed_line(&pts, Stroke::new(1.5 * s, ink.gamma_multiply(0.6)), 4.0 * s, 3.0 * s) {
                p.add(sh);
            }
            let small = Rect::from_min_size(big.min, size(30.0, 22.0));
            p.rect_filled(small, 4.0 * s, ink.gamma_multiply(0.75));
            art.icon(p, Icon::Lib("arrow-diagonal"), Rect::from_center_size(at(28.0, -16.0), size(20.0, 20.0)), ink);
        }
        "convert-png" | "convert-jpeg" | "convert-webp" => {
            let to = match id {
                "convert-png" => "PNG",
                "convert-jpeg" => "JPG",
                _ => "WEBP",
            };
            let from = if to == "PNG" { "JPG" } else { "PNG" };
            for (x, label, strong) in [(-26.0, from, false), (26.0, to, true)] {
                let card = Rect::from_center_size(at(x, 0.0), size(34.0, 44.0));
                paper(p, t, card, 4.0 * s);
                let band = Rect::from_min_max(pos2(card.left(), card.bottom() - 15.0 * s), card.max);
                p.rect_filled(
                    band,
                    egui::CornerRadius { sw: (4.0 * s) as u8, se: (4.0 * s) as u8, nw: 0, ne: 0 },
                    if strong { ink } else { ink.gamma_multiply(0.35) },
                );
                p.text(band.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(8.0 * s), on_ink);
                p.rect_filled(Rect::from_min_size(card.min + size(6.0, 7.0), size(16.0, 4.0)), 2.0 * s, ink.gamma_multiply(0.25));
            }
            art.icon(p, Icon::Lib("arrow-right"), Rect::from_center_size(c, size(16.0, 16.0)), ink);
        }
        "auto-crop" => {
            let pic = Rect::from_center_size(c, size(50.0, 38.0));
            p.rect_filled(pic, 3.0 * s, ink.gamma_multiply(0.25));
            p.circle_filled(pic.min + size(36.0, 10.0), 5.0 * s, ink.gamma_multiply(0.6));
            let crop = pic.expand(6.0 * s);
            let st = Stroke::new(3.0 * s, ink);
            let l = 12.0 * s;
            for (corner, dx, dy) in
                [(crop.left_top(), 1.0, 1.0), (crop.right_top(), -1.0, 1.0), (crop.left_bottom(), 1.0, -1.0), (crop.right_bottom(), -1.0, -1.0)]
            {
                p.line_segment([corner, corner + vec2(l * dx, 0.0)], st);
                p.line_segment([corner, corner + vec2(0.0, l * dy)], st);
            }
        }
        "blur-background" => {
            let sq = Rect::from_center_size(c, size(56.0, 46.0));
            p.rect_filled(sq, 4.0 * s, ink.gamma_multiply(0.12));
            for (x, y, rad, a) in [(-16.0, -10.0, 14.0, 0.18), (14.0, -12.0, 11.0, 0.22), (18.0, 10.0, 13.0, 0.15), (-18.0, 12.0, 10.0, 0.2)] {
                for k in 0..4 {
                    p.circle_filled(at(x, y), (rad + k as f32 * 2.5) * s, ink.gamma_multiply(a / (k as f32 + 1.0)));
                }
            }
            person(art, at(0.0, 6.0), 34.0 * s, ink);
        }
        "change-background" => {
            let sq = Rect::from_center_size(at(-10.0, 0.0), size(46.0, 46.0));
            paper(p, t, sq, 4.0 * s);
            person(art, sq.center() + size(0.0, 5.0), 34.0 * s, deep);
            for (i, a) in [1.0, 0.6, 0.3].iter().enumerate() {
                p.circle_filled(at(28.0, -14.0 + i as f32 * 14.0), 5.5 * s, if i == 0 { t.art_paper } else { ink.gamma_multiply(*a) });
                p.circle_stroke(at(28.0, -14.0 + i as f32 * 14.0), 5.5 * s, Stroke::new(1.0 * s, ink.gamma_multiply(0.5)));
            }
        }
        "pixelate" => {
            let cell = 8.0 * s;
            let (cols, rows) = (7, 5);
            let origin = c - vec2(cols as f32 * cell / 2.0, rows as f32 * cell / 2.0);
            for i in 0..cols {
                for j in 0..rows {
                    let v = ((i * 7 + j * 13 + (i * j) % 5) % 6) as f32 / 6.0;
                    let cr = Rect::from_min_size(origin + vec2(i as f32 * cell, j as f32 * cell), egui::Vec2::splat(cell - 1.0));
                    p.rect_filled(cr, 1.0, ink.gamma_multiply(0.15 + 0.7 * v));
                }
            }
        }
        "grayscale" => {
            let rad = 22.0 * s;
            p.circle_filled(c, rad, t.text_dim);
            let half = vec![c + vec2(0.0, -rad)]
                .into_iter()
                .chain((0..=16).map(|i| {
                    let a = (-90.0 + 180.0 * i as f32 / 16.0f32).to_radians();
                    c + vec2(-a.cos() * rad, a.sin() * rad)
                }))
                .collect::<Vec<_>>();
            p.add(egui::Shape::convex_polygon(half, ink, Stroke::NONE));
            p.circle_stroke(c, rad, Stroke::new(2.0 * s, t.art_paper));
        }
        "qr-code" => {
            let sq = Rect::from_center_size(c, size(48.0, 48.0));
            paper(p, t, sq.expand(3.0 * s), 4.0 * s);
            art.qr(p, sq, deep);
        }
        "collage" => {
            let g = 3.0 * s;
            let all = Rect::from_center_size(c, size(64.0, 46.0));
            let left = Rect::from_min_size(all.min, vec2(all.width() * 0.55 - g / 2.0, all.height()));
            let tr = Rect::from_min_max(pos2(left.right() + g, all.top()), pos2(all.right(), all.center().y - g / 2.0));
            let br = Rect::from_min_max(pos2(left.right() + g, all.center().y + g / 2.0), all.max);
            p.rect_filled(left, 3.0 * s, ink.gamma_multiply(0.7));
            p.rect_filled(tr, 3.0 * s, ink.gamma_multiply(0.4));
            p.rect_filled(br, 3.0 * s, ink.gamma_multiply(0.25));
            p.circle_filled(left.min + size(10.0, 10.0), 4.0 * s, t.art_paper);
        }
        _ => {
            art.icon(p, Icon::Lib("image"), Rect::from_center_size(c, size(36.0, 36.0)), ink);
        }
    }
}
