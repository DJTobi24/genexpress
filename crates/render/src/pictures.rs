//! Decoded pictures (cropped, adjusted and reduced for the size they're drawn at), cached across
//! renders.

use std::collections::HashMap;
use std::sync::Arc;

use genexpress_doc::{Asset, AssetId, ImageEl};
use genexpress_effects::RgbaImage;
use vello_cpu::Pixmap;
use vello_cpu::color::PremulRgba8;

/// Bytes of pixels the cache keeps before it starts over.
const CACHE_BUDGET: usize = 768 * 1024 * 1024;
/// Most reduction steps (each halves the picture).
const MAX_LEVEL: u32 = 10;

/// What a cached picture was made from.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct PicKey {
    asset: AssetId,
    /// Identity of the asset bytes (address and length), so a different picture under a reused
    /// id is not mistaken for the cached one.
    data: (usize, usize),
    crop: [u64; 4],
    adjust: [u32; 8],
    invert: bool,
    /// Halvings applied (0 = full size).
    level: u32,
}

impl PicKey {
    pub(crate) fn new(id: AssetId, asset: &Asset, im: &ImageEl, level: u32) -> Self {
        let a = &im.adjustments;
        let c = im.crop.sanitized();
        PicKey {
            asset: id,
            data: (Arc::as_ptr(&asset.data.0).cast::<u8>() as usize, asset.data.len()),
            crop: [c.x.to_bits(), c.y.to_bits(), c.width.to_bits(), c.height.to_bits()],
            adjust: [a.brightness, a.contrast, a.saturation, a.warmth, a.sharpen, a.blur, a.grayscale, a.sepia].map(f32::to_bits),
            invert: a.invert,
            level,
        }
    }
}

/// A picture ready to draw: its pixels and its size before reduction (the size it is fitted by).
#[derive(Clone)]
pub(crate) struct Pic {
    pub(crate) pixmap: Arc<Pixmap>,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Default)]
pub(crate) struct PicCache {
    /// `None` remembers pictures that could not be decoded.
    entries: HashMap<PicKey, Option<Pic>>,
    bytes: usize,
}

impl PicCache {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    /// The picture for image `im` drawn so that its full (cropped) width covers `device_width`
    /// pixels.
    pub(crate) fn get(&mut self, id: AssetId, asset: Option<&Asset>, im: &ImageEl, device_width: f64) -> Option<Pic> {
        let asset = asset?;
        // The reduction level depends on the cropped size, known from the header without decoding.
        let full_w = genexpress_effects::probe(&asset.data).ok().map(|i| f64::from(i.width) * im.crop.sanitized().width)?;
        let level = level_for(full_w, device_width);
        let key = PicKey::new(id, asset, im, level);
        if let Some(hit) = self.entries.get(&key) {
            return hit.clone();
        }
        let pic = make(asset, im, level);
        let size = pic.as_ref().map_or(0, |p| usize::from(p.pixmap.width()) * usize::from(p.pixmap.height()) * 4);
        if self.bytes.saturating_add(size) > CACHE_BUDGET {
            self.clear();
        }
        self.bytes = self.bytes.saturating_add(size);
        self.entries.insert(key, pic.clone());
        pic
    }
}

/// How many times to halve a picture `src` pixels wide that is shown `dst` pixels wide.
fn level_for(src: f64, dst: f64) -> u32 {
    if !src.is_finite() || !dst.is_finite() || dst <= 0.0 || src <= 0.0 {
        return 0;
    }
    let ratio = src / dst;
    if ratio < 2.0 {
        return 0;
    }
    let l = ratio.log2().floor();
    if l.is_finite() { (l.max(0.0) as u32).min(MAX_LEVEL) } else { 0 }
}

fn make(asset: &Asset, im: &ImageEl, level: u32) -> Option<Pic> {
    let img = genexpress_effects::decode(&asset.data).ok()?;
    let mut img = genexpress_effects::crop(&img, &im.crop);
    let (width, height) = (f64::from(img.width()), f64::from(img.height()));
    if level > 0 {
        let w = (img.width() >> level).max(1);
        let h = (img.height() >> level).max(1);
        img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    }
    // Blur is measured in picture pixels: fewer of them after reduction.
    let mut adj = im.adjustments.clone();
    if level > 0 && adj.blur.is_finite() {
        adj.blur /= (1u32 << level.min(MAX_LEVEL)) as f32;
    }
    genexpress_effects::apply_adjustments(&mut img, &adj);
    Some(Pic { pixmap: Arc::new(pixmap_of(&img)?), width, height })
}

/// A straight-alpha RGBA image as a premultiplied pixmap.
pub(crate) fn pixmap_of(img: &RgbaImage) -> Option<Pixmap> {
    let w = u16::try_from(img.width()).ok()?;
    let h = u16::try_from(img.height()).ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    let data: Vec<PremulRgba8> = img
        .pixels()
        .map(|p| {
            let a = u16::from(p[3]);
            let m = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
            PremulRgba8 { r: m(p[0]), g: m(p[1]), b: m(p[2]), a: p[3] }
        })
        .collect();
    if data.len() != usize::from(w) * usize::from(h) {
        return None;
    }
    Some(Pixmap::from_parts(data, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduction_levels() {
        assert_eq!(level_for(1000.0, 1000.0), 0);
        assert_eq!(level_for(1000.0, 600.0), 0);
        assert_eq!(level_for(1000.0, 400.0), 1);
        assert_eq!(level_for(4000.0, 100.0), 5);
        assert_eq!(level_for(f64::NAN, 100.0), 0);
        assert_eq!(level_for(1e12, 1e-9), MAX_LEVEL);
    }
}
