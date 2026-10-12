//! A texture cache for thumbnails (templates, shapes, pages, pictures), filled a few per frame so
//! the UI stays responsive.

use std::collections::HashMap;

use genexpress_render::Raster;

#[derive(Default)]
pub struct Thumbs {
    map: HashMap<String, (u64, egui::TextureHandle)>,
    budget: usize,
}

/// Thumbnails rendered per frame at most (the rest show a placeholder and come next frame).
const PER_FRAME: usize = 3;

impl Thumbs {
    pub fn begin_frame(&mut self) {
        self.budget = PER_FRAME;
    }

    /// The texture for `key`, rendering it with `make` when missing or when `stamp` changed (and the
    /// frame's budget allows; otherwise the stale texture, or `None`, and a repaint is requested).
    pub fn get(&mut self, ctx: &egui::Context, key: &str, stamp: u64, make: impl FnOnce() -> Option<Raster>) -> Option<egui::TextureHandle> {
        let fresh = self.map.get(key).filter(|(s, _)| *s == stamp).map(|(_, t)| t.clone());
        if fresh.is_some() {
            return fresh;
        }
        if self.budget == 0 {
            ctx.request_repaint();
            return self.map.get(key).map(|(_, t)| t.clone());
        }
        self.budget -= 1;
        let raster = make()?;
        let tex = ctx.load_texture(key, to_color_image(&raster), egui::TextureOptions::LINEAR);
        self.map.insert(key.to_string(), (stamp, tex.clone()));
        Some(tex)
    }

    pub fn forget_prefix(&mut self, prefix: &str) {
        self.map.retain(|k, _| !k.starts_with(prefix));
    }
}

pub fn to_color_image(r: &Raster) -> egui::ColorImage {
    let (w, h) = (r.width as usize, r.height as usize);
    if r.rgba.len() != w * h * 4 {
        return egui::ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT]);
    }
    egui::ColorImage::from_rgba_unmultiplied([w, h], &r.rgba)
}

/// Size to show a `w × h` thumbnail within `max` (keeps the aspect ratio).
pub fn fit_size(w: f32, h: f32, max: egui::Vec2) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return max;
    }
    let s = (max.x / w).min(max.y / h);
    egui::vec2(w * s, h * s)
}
