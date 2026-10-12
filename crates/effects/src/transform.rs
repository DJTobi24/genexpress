//! Resizing, rotating, flipping and trimming pictures.

use genha_geom::Fit;
use image::{Rgba, RgbaImage, imageops::FilterType};

use crate::{EffectError, MAX_PIXELS, MAX_SIDE, Result};

/// Resizes `img` to exactly `width` × `height`.
///
/// - [`Fit::Cover`] fills the new size, cropping the centre of what overflows;
/// - [`Fit::Contain`] shows the whole picture, padding with transparent bands;
/// - [`Fit::Stretch`] distorts the picture to the new size.
///
/// Sizes of 0 or past [`MAX_SIDE`] / [`MAX_PIXELS`] are refused. Resampling is done on
/// premultiplied colours so transparent pixels do not darken the edges.
pub fn resize(img: &RgbaImage, width: u32, height: u32, fit: Fit) -> Result<RgbaImage> {
    let (sw, sh) = img.dimensions();
    if sw == 0 || sh == 0 {
        return Err(EffectError::Invalid("cannot resize an empty picture".into()));
    }
    if width == 0 || height == 0 {
        return Err(EffectError::Invalid(format!("invalid size {width}×{height}")));
    }
    if width > MAX_SIDE || height > MAX_SIDE || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(EffectError::TooLarge(width, height));
    }
    let (fw, fh, tw, th) = (f64::from(sw), f64::from(sh), f64::from(width), f64::from(height));
    match fit {
        Fit::Stretch => Ok(resample(img, width, height)),
        Fit::Cover => {
            let s = (tw / fw).max(th / fh);
            let cw = to_px(tw / s, sw);
            let ch = to_px(th / s, sh);
            let x = (sw - cw) / 2;
            let y = (sh - ch) / 2;
            let part = image::imageops::crop_imm(img, x, y, cw, ch).to_image();
            Ok(resample(&part, width, height))
        }
        Fit::Contain => {
            let s = (tw / fw).min(th / fh);
            let nw = to_px(fw * s, width);
            let nh = to_px(fh * s, height);
            let scaled = resample(img, nw, nh);
            let mut canvas = RgbaImage::new(width, height);
            image::imageops::overlay(&mut canvas, &scaled, i64::from((width - nw) / 2), i64::from((height - nh) / 2));
            Ok(canvas)
        }
    }
}

/// Rounds a finite, positive pixel length into `1..=max`.
fn to_px(v: f64, max: u32) -> u32 {
    if !v.is_finite() {
        return max.max(1);
    }
    (v.round().clamp(1.0, f64::from(max.max(1)))) as u32
}

fn resample(img: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if img.dimensions() == (width, height) {
        return img.clone();
    }
    let mut pre = img.clone();
    for p in pre.pixels_mut() {
        let a = u16::from(p[3]);
        for c in p.0.iter_mut().take(3) {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
    let mut out = image::imageops::resize(&pre, width, height, FilterType::CatmullRom);
    for p in out.pixels_mut() {
        let a = u16::from(p[3]);
        if a == 0 {
            *p = Rgba([0, 0, 0, 0]);
            continue;
        }
        for c in p.0.iter_mut().take(3) {
            *c = ((u16::from(*c) * 255 + a / 2) / a).min(255) as u8;
        }
    }
    out
}

/// Rotates by `quarter_turns` × 90° clockwise (negative turns go anticlockwise).
pub fn rotate(img: &RgbaImage, quarter_turns: i32) -> RgbaImage {
    match quarter_turns.rem_euclid(4) {
        1 => image::imageops::rotate90(img),
        2 => image::imageops::rotate180(img),
        3 => image::imageops::rotate270(img),
        _ => img.clone(),
    }
}

/// Mirrors left↔right when `horizontal`, else top↔bottom.
pub fn flip(img: &RgbaImage, horizontal: bool) -> RgbaImage {
    if horizontal { image::imageops::flip_horizontal(img) } else { image::imageops::flip_vertical(img) }
}

/// Alpha at or below which a pixel counts as transparent when trimming.
const TRIM_ALPHA: u8 = 8;
/// Largest per-channel difference from the border colour that still counts as border.
const TRIM_TOLERANCE: u8 = 10;

/// Trims a transparent or uniformly coloured border. The border colour is the top-left pixel's
/// (transparent when that pixel is). A picture that is all border is returned unchanged.
pub fn auto_crop(img: &RgbaImage) -> RgbaImage {
    let Some(&corner) = img.get_pixel_checked(0, 0) else {
        return img.clone();
    };
    let is_border = |p: &Rgba<u8>| {
        if corner[3] <= TRIM_ALPHA { p[3] <= TRIM_ALPHA } else { p.0.iter().zip(corner.0).all(|(a, b)| a.abs_diff(b) <= TRIM_TOLERANCE) }
    };
    let (w, h) = img.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0;
    let mut max_y = 0;
    for (x, y, p) in img.enumerate_pixels() {
        if !is_border(p) {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x > max_x || min_y > max_y {
        return img.clone();
    }
    image::imageops::crop_imm(img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_halves(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, _| if x < w / 2 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) })
    }

    #[test]
    fn resize_modes() {
        let img = two_halves(200, 100);
        let s = resize(&img, 50, 50, Fit::Stretch).unwrap();
        assert_eq!(s.dimensions(), (50, 50));
        // Cover keeps the centre: the left edge is still red, the right still blue.
        let c = resize(&img, 50, 50, Fit::Cover).unwrap();
        assert_eq!(c.dimensions(), (50, 50));
        assert!(c.get_pixel(0, 25)[0] > 200 && c.get_pixel(49, 25)[2] > 200);
        // Contain pads with transparent bands top and bottom.
        let k = resize(&img, 100, 100, Fit::Contain).unwrap();
        assert_eq!(k.dimensions(), (100, 100));
        assert_eq!(k.get_pixel(50, 2)[3], 0);
        assert_eq!(k.get_pixel(10, 50)[3], 255);
        assert!(resize(&img, 0, 10, Fit::Cover).is_err());
        assert!(matches!(resize(&img, MAX_SIDE + 1, 10, Fit::Cover), Err(EffectError::TooLarge(..))));
        assert!(resize(&RgbaImage::new(0, 0), 10, 10, Fit::Cover).is_err());
    }

    #[test]
    fn resize_does_not_darken_transparent_edges() {
        let img = RgbaImage::from_fn(8, 8, |x, _| if x < 4 { Rgba([255, 255, 255, 255]) } else { Rgba([0, 0, 0, 0]) });
        let out = resize(&img, 16, 16, Fit::Stretch).unwrap();
        for p in out.pixels().filter(|p| p[3] > 20) {
            assert!(p[0] > 230, "{p:?}");
        }
    }

    #[test]
    fn rotate_and_flip() {
        let img = two_halves(4, 2);
        let r = rotate(&img, 1);
        assert_eq!(r.dimensions(), (2, 4));
        // After a clockwise quarter turn the left (red) half is on top.
        assert_eq!(r.get_pixel(0, 0)[0], 255);
        assert_eq!(rotate(&img, -3), r);
        assert_eq!(rotate(&img, 4), img);
        assert_eq!(rotate(&img, 2).get_pixel(0, 0)[2], 255);
        assert_eq!(flip(&img, true).get_pixel(0, 0)[2], 255);
        assert_eq!(flip(&img, false), img);
    }

    #[test]
    fn auto_crop_trims_borders() {
        let mut img = RgbaImage::new(20, 10);
        for y in 3..6 {
            for x in 5..9 {
                img.put_pixel(x, y, Rgba([9, 9, 9, 255]));
            }
        }
        assert_eq!(auto_crop(&img).dimensions(), (4, 3));
        let mut white = RgbaImage::from_pixel(10, 10, Rgba([250, 250, 250, 255]));
        white.put_pixel(2, 7, Rgba([0, 0, 0, 255]));
        assert_eq!(auto_crop(&white).dimensions(), (1, 1));
        let plain = RgbaImage::from_pixel(3, 3, Rgba([1, 2, 3, 255]));
        assert_eq!(auto_crop(&plain), plain);
        assert_eq!(auto_crop(&RgbaImage::new(0, 0)).dimensions(), (0, 0));
    }
}
