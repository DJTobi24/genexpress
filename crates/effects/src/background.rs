//! Classical (non-AI) background removal.
//!
//! This is a heuristic that works well for product shots, logos, scans and portraits on a plain or
//! gently graded backdrop, and poorly on busy photos:
//!
//! 1. the background colours are estimated by clustering the picture's border pixels in CIELAB;
//! 2. starting from every border pixel that matches one of them, a flood fill (4-connected) marks
//!    each pixel whose colour is within `tolerance` of a background colour (CIE76 ΔE, a
//!    perceptual-ish distance) — so background regions not connected to the border, like the
//!    inside of a ring, are kept;
//! 3. the marked pixels become transparent and the foreground's edge is feathered by about a pixel
//!    (a 3×3 box filter over the mask) so it does not look jagged.

use genha_color::Color;
use image::{Rgba, RgbaImage};

/// Pixels at or below this alpha already count as background.
const CLEAR: u8 = 16;
/// Border samples examined at most (evenly spaced).
const MAX_BORDER_SAMPLES: usize = 4096;
/// Distinct background colours tracked at most.
const MAX_CLUSTERS: usize = 6;
/// ΔE within which border samples are merged into one background colour.
const CLUSTER_RADIUS: f32 = 6.0;

type Lab = [f32; 3];

fn srgb_to_linear_table() -> [f32; 256] {
    let mut t = [0.0f32; 256];
    for (i, v) in t.iter_mut().enumerate() {
        let c = i as f32 / 255.0;
        *v = if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    }
    t
}

fn lab(lin: &[f32; 256], p: &Rgba<u8>) -> Lab {
    let [r, g, b] = [lin[usize::from(p[0])], lin[usize::from(p[1])], lin[usize::from(p[2])]];
    // linear sRGB → XYZ (D65), normalised by the white point.
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = (0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / 1.088_83;
    let f = |t: f32| if t > 0.008_856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn delta_e(a: &Lab, b: &Lab) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Maps the user's `0..=100` tolerance to a ΔE threshold.
fn threshold(tolerance: f32) -> f32 {
    let t = if tolerance.is_finite() { tolerance.clamp(0.0, 100.0) } else { 30.0 };
    3.0 + t * 0.5
}

/// Border pixel coordinates, at most [`MAX_BORDER_SAMPLES`] of them, evenly spread.
fn border_coords(w: u32, h: u32) -> Vec<(u32, u32)> {
    let mut v = vec![];
    if w == 0 || h == 0 {
        return v;
    }
    let perimeter = 2 * (u64::from(w) + u64::from(h));
    let step = (perimeter / MAX_BORDER_SAMPLES as u64).max(1) as usize;
    for x in (0..w).step_by(step) {
        v.push((x, 0));
        v.push((x, h - 1));
    }
    for y in (0..h).step_by(step) {
        v.push((0, y));
        v.push((w - 1, y));
    }
    v
}

/// Estimates the background colours from the border (largest cluster first).
fn background_colors(img: &RgbaImage, lin: &[f32; 256]) -> Vec<Lab> {
    let mut clusters: Vec<(Lab, f32)> = vec![];
    let mut total = 0.0f32;
    for (x, y) in border_coords(img.width(), img.height()) {
        let Some(p) = img.get_pixel_checked(x, y) else { continue };
        if p[3] <= CLEAR {
            continue;
        }
        total += 1.0;
        let c = lab(lin, p);
        let nearest = clusters.iter().enumerate().map(|(i, k)| (delta_e(&k.0, &c), i)).min_by(|a, b| a.0.total_cmp(&b.0));
        match nearest {
            Some((d, i)) if d <= CLUSTER_RADIUS => {
                if let Some(k) = clusters.get_mut(i) {
                    k.1 += 1.0;
                    for (m, v) in k.0.iter_mut().zip(c) {
                        *m += (v - *m) / k.1;
                    }
                }
            }
            _ if clusters.len() < MAX_CLUSTERS => clusters.push((c, 1.0)),
            Some((_, i)) => {
                if let Some(k) = clusters.get_mut(i) {
                    k.1 += 1.0;
                }
            }
            None => {}
        }
    }
    clusters.sort_by(|a, b| b.1.total_cmp(&a.1));
    let keep = total * 0.08;
    clusters.into_iter().enumerate().filter(|(i, k)| *i == 0 || k.1 >= keep).map(|(_, k)| k.0).collect()
}

/// Makes the background of `img` transparent (see the module docs for how, and its limits).
/// `tolerance` is `0..=100`: how different from the estimated background colour a pixel may be
/// and still be removed (around 30 suits most pictures).
pub fn remove_background(img: &RgbaImage, tolerance: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (wu, hu) = (w as usize, h as usize);
    if wu == 0 || hu == 0 {
        return img.clone();
    }
    let lin = srgb_to_linear_table();
    let bg = background_colors(img, &lin);
    let thr = threshold(tolerance);
    let is_bg = |p: &Rgba<u8>| p[3] <= CLEAR || bg.iter().any(|b| delta_e(b, &lab(&lin, p)) <= thr);

    // Flood fill from the border. 1 = background.
    let mut mask = vec![0u8; wu * hu];
    let mut stack: Vec<(u32, u32)> = vec![];
    let push = |x: u32, y: u32, mask: &mut [u8], stack: &mut Vec<(u32, u32)>| {
        let i = y as usize * wu + x as usize;
        if let (Some(m), Some(p)) = (mask.get_mut(i), img.get_pixel_checked(x, y))
            && *m == 0
            && is_bg(p)
        {
            *m = 1;
            stack.push((x, y));
        }
    };
    for x in 0..w {
        push(x, 0, &mut mask, &mut stack);
        push(x, h - 1, &mut mask, &mut stack);
    }
    for y in 0..h {
        push(0, y, &mut mask, &mut stack);
        push(w - 1, y, &mut mask, &mut stack);
    }
    while let Some((x, y)) = stack.pop() {
        if x > 0 {
            push(x - 1, y, &mut mask, &mut stack);
        }
        if x + 1 < w {
            push(x + 1, y, &mut mask, &mut stack);
        }
        if y > 0 {
            push(x, y - 1, &mut mask, &mut stack);
        }
        if y + 1 < h {
            push(x, y + 1, &mut mask, &mut stack);
        }
    }

    // Feather: a foreground pixel keeps the share of its 3×3 neighbourhood that is foreground.
    let mut out = img.clone();
    let fg = |x: i64, y: i64| -> u32 {
        if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
            return 1; // outside counts as foreground so pictures touching the border keep their edge
        }
        let i = y as usize * wu + x as usize;
        u32::from(mask.get(i).is_some_and(|m| *m == 0))
    };
    for (x, y, p) in out.enumerate_pixels_mut() {
        let (xi, yi) = (i64::from(x), i64::from(y));
        if fg(xi, yi) == 0 {
            *p = Rgba([p[0], p[1], p[2], 0]);
            continue;
        }
        let mut n = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                n += fg(xi + dx, yi + dy);
            }
        }
        if n < 9 {
            p[3] = ((u32::from(p[3]) * n + 4) / 9) as u8;
        }
    }
    out
}

/// Replaces the background of `img` (found as in [`remove_background`]) with `color`.
pub fn change_background(img: &RgbaImage, color: Color, tolerance: f32) -> RgbaImage {
    let mut out = remove_background(img, tolerance);
    for p in out.pixels_mut() {
        *p = over(*p, color);
    }
    out
}

/// Blurs the background of `img` (found as in [`remove_background`]) and keeps the subject sharp,
/// like a shallow depth of field. `radius` is the blur in pixels (clamped to `0..=100`).
pub fn blur_background(img: &RgbaImage, tolerance: f32, radius: f32) -> RgbaImage {
    let cut = remove_background(img, tolerance);
    let r = if radius.is_finite() { radius.clamp(0.0, 100.0) } else { 12.0 };
    let blurred = image::imageops::blur(img, r.max(0.1) / 2.0);
    let mut out = blurred;
    for (o, (c, src)) in out.pixels_mut().zip(cut.pixels().zip(img.pixels())) {
        // The cut-out's alpha says how much of the sharp subject shows.
        let a = f32::from(c[3]) / 255.0;
        for k in 0..3 {
            let v = f32::from(src[k]) * a + f32::from(o[k]) * (1.0 - a);
            o[k] = v.round().clamp(0.0, 255.0) as u8;
        }
        o[3] = src[3];
    }
    out
}

/// Pixelates `img` in blocks of `block` pixels (clamped to `2..=256`).
pub fn pixelate(img: &RgbaImage, block: u32) -> RgbaImage {
    let b = block.clamp(2, 256);
    let (w, h) = img.dimensions();
    let small = image::imageops::resize(img, w.div_ceil(b).max(1), h.div_ceil(b).max(1), image::imageops::FilterType::Triangle);
    image::imageops::resize(&small, w.max(1), h.max(1), image::imageops::FilterType::Nearest)
}

/// `top` composited over `bottom` (straight alpha).
fn over(top: Rgba<u8>, bottom: Color) -> Rgba<u8> {
    let ta = f32::from(top[3]) / 255.0;
    let ba = f32::from(bottom.a) / 255.0;
    let oa = ta + ba * (1.0 - ta);
    if oa <= 0.0 {
        return Rgba([0, 0, 0, 0]);
    }
    let mix = |t: u8, b: u8| {
        let v = (f32::from(t) * ta + f32::from(b) * ba * (1.0 - ta)) / oa;
        v.round().clamp(0.0, 255.0) as u8
    };
    Rgba([mix(top[0], bottom.r), mix(top[1], bottom.g), mix(top[2], bottom.b), (oa * 255.0).round().clamp(0.0, 255.0) as u8])
}

#[cfg(test)]
mod blur_tests {
    use super::*;

    #[test]
    fn blur_background_keeps_the_subject() {
        // A white backdrop with fine black stripes (blurring greys them) and a red square.
        let mut img = RgbaImage::from_fn(60, 60, |x, _| if x % 4 == 0 { Rgba([200, 200, 200, 255]) } else { Rgba([255, 255, 255, 255]) });
        for y in 20..40 {
            for x in 20..40 {
                img.put_pixel(x, y, Rgba([220, 20, 20, 255]));
            }
        }
        let out = blur_background(&img, 30.0, 10.0);
        assert_eq!(out.get_pixel(30, 30), &Rgba([220, 20, 20, 255]), "the subject stays sharp");
        assert_eq!(out.get_pixel(4, 50)[3], 255);
        let p = pixelate(&img, 10);
        assert_eq!(p.dimensions(), (60, 60));
        assert_eq!(p.get_pixel(0, 0), p.get_pixel(5, 5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A red disc on a light grey backdrop with a slight gradient.
    fn product_shot() -> RgbaImage {
        RgbaImage::from_fn(60, 40, |x, y| {
            let (dx, dy) = (f64::from(x) - 30.0, f64::from(y) - 20.0);
            if dx * dx + dy * dy < 100.0 { Rgba([200, 30, 40, 255]) } else { Rgba([235 - (x / 10) as u8, 235, 238, 255]) }
        })
    }

    #[test]
    fn removes_plain_backdrop() {
        let out = remove_background(&product_shot(), 30.0);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(59, 39)[3], 0);
        assert_eq!(out.get_pixel(30, 20)[3], 255);
        assert_eq!(out.get_pixel(30, 20)[0], 200);
        // The edge is feathered: some pixels are partly transparent.
        assert!(out.pixels().any(|p| p[3] > 0 && p[3] < 255));
    }

    #[test]
    fn keeps_enclosed_regions_and_handles_edge_cases() {
        // A dark ring on white: the white inside the ring is not connected to the border.
        let img = RgbaImage::from_fn(30, 30, |x, y| {
            let d = (f64::from(x) - 15.0).hypot(f64::from(y) - 15.0);
            if (6.0..10.0).contains(&d) { Rgba([10, 10, 10, 255]) } else { Rgba([255, 255, 255, 255]) }
        });
        let out = remove_background(&img, 20.0);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(15, 15)[3], 255);
        assert_eq!(remove_background(&RgbaImage::new(0, 0), 30.0).dimensions(), (0, 0));
        let _ = remove_background(&img, f32::NAN);
        let plain = RgbaImage::from_pixel(5, 5, Rgba([10, 200, 10, 255]));
        assert!(remove_background(&plain, 10.0).pixels().all(|p| p[3] == 0));
    }

    #[test]
    fn tolerance_matters() {
        // A pale grey square on white touching nothing but the backdrop.
        let img = RgbaImage::from_fn(40, 40, |x, y| {
            if (15..25).contains(&x) && (15..25).contains(&y) { Rgba([225, 225, 225, 255]) } else { Rgba([255, 255, 255, 255]) }
        });
        let strict = remove_background(&img, 0.0);
        let loose = remove_background(&img, 60.0);
        assert_eq!(strict.get_pixel(0, 0)[3], 0);
        assert_eq!(strict.get_pixel(20, 20)[3], 255);
        assert_eq!(loose.get_pixel(20, 20)[3], 0);
    }

    #[test]
    fn two_tone_backdrops_are_both_removed() {
        let img = RgbaImage::from_fn(40, 40, |x, y| {
            if (15..25).contains(&x) && (15..25).contains(&y) {
                Rgba([20, 20, 120, 255])
            } else if y < 20 {
                Rgba([240, 240, 240, 255])
            } else {
                Rgba([200, 225, 200, 255])
            }
        });
        let out = remove_background(&img, 10.0);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(0, 39)[3], 0);
        assert_eq!(out.get_pixel(20, 20)[3], 255);
    }

    #[test]
    fn change_background_fills_colour() {
        let out = change_background(&product_shot(), Color::rgb(0, 0, 255), 30.0);
        assert_eq!(*out.get_pixel(0, 0), Rgba([0, 0, 255, 255]));
        assert_eq!(*out.get_pixel(30, 20), Rgba([200, 30, 40, 255]));
        assert!(out.pixels().all(|p| p[3] == 255));
    }
}
