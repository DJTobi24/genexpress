//! Model output → a mask the size of the picture.
//!
//! The model sees a small square copy of the picture, so its answer is blurry and has the wrong
//! shape. [`refine`] turns it into a mask that fits the picture:
//!
//! 1. **Normalise** the raw output to 0..1 (U²-Net's own post-processing: min–max).
//! 2. **Upscale** it to a working size (the picture, at most [`WORK_SIDE`] pixels across) with
//!    bilinear filtering.
//! 3. **Drop stray patches**: parts of the subject (over 50 %) that are not connected to the main
//!    subject and are much smaller than it ([`ISLAND_SHARE`] of the largest part) — on busy photos
//!    the model sometimes picks up a bright patch of background.
//! 4. **Snap the edges** to the picture's edges with a guided filter (He, Sun and Tang, "Guided
//!    Image Filtering", ECCV 2010): within a small window the mask becomes a linear function of
//!    the picture's brightness, so a soft, blurry outline sharpens where the picture has an edge
//!    and stays soft where it does not (hair, fur). A light alpha-matting step, without the cost.
//! 5. **Contrast**: values near 0 or 1 become exactly 0 or 1 ([`EDGE_LOW`], [`EDGE_HIGH`]); what
//!    is in between stays a soft (feathered) edge.
//! 6. **Upscale** to the picture's size.

use image::{GrayImage, Luma, RgbaImage};

/// Largest side of the working mask; bigger pictures are refined at this size and upscaled.
pub const WORK_SIDE: u32 = 1280;
/// Parts of the subject smaller than this share of the largest part are dropped.
pub const ISLAND_SHARE: f32 = 0.04;
/// Mask values at or below this become fully transparent…
pub const EDGE_LOW: f32 = 0.18;
/// …and at or above this fully opaque.
pub const EDGE_HIGH: f32 = 0.82;
/// Guided filter regularisation (in brightness² units, brightness 0..1): larger is smoother.
pub const GUIDE_EPS: f32 = 2e-3;

/// Scales `v` so its smallest value is 0 and its largest 1 (all zero when it is flat or not finite).
pub fn normalize(v: &mut [f32]) {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for x in v.iter().filter(|x| x.is_finite()) {
        lo = lo.min(*x);
        hi = hi.max(*x);
    }
    let range = hi - lo;
    if !(range.is_finite() && range > 1e-6) {
        v.iter_mut().for_each(|x| *x = 0.0);
        return;
    }
    for x in v.iter_mut() {
        *x = if x.is_finite() { ((*x - lo) / range).clamp(0.0, 1.0) } else { 0.0 };
    }
}

/// Bilinear resize of a `sw × sh` plane to `dw × dh` (pixel centres aligned).
pub fn resize_plane(src: &[f32], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<f32> {
    let (sw_u, sh_u) = (sw as usize, sh as usize);
    let n = (dw as usize) * (dh as usize);
    if sw == 0 || sh == 0 || src.len() < sw_u * sh_u {
        return vec![0.0; n];
    }
    let at = |x: usize, y: usize| src.get(y * sw_u + x).copied().unwrap_or(0.0);
    let axis = |d: u32, s: u32| -> Vec<(usize, usize, f32)> {
        (0..d)
            .map(|i| {
                let f = ((i as f32 + 0.5) * s as f32 / d.max(1) as f32 - 0.5).clamp(0.0, (s - 1) as f32);
                let i0 = f.floor() as usize;
                let i1 = (i0 + 1).min(s as usize - 1);
                (i0, i1, f - i0 as f32)
            })
            .collect()
    };
    let xs = axis(dw, sw);
    let ys = axis(dh, sh);
    let mut out = Vec::with_capacity(n);
    for &(y0, y1, fy) in &ys {
        for &(x0, x1, fx) in &xs {
            let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
            let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
            out.push(top * (1.0 - fy) + bottom * fy);
        }
    }
    out
}

/// The mean of each `(2r + 1)²` window (clipped at the borders) of a `w × h` plane.
pub fn box_mean(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    if w == 0 || h == 0 || src.len() < w * h {
        return vec![0.0; w * h];
    }
    // Horizontal sums, then vertical, with running totals in f64 (no drift on large pictures).
    let mut tmp = vec![0.0f32; w * h];
    let mut row_sum = vec![0.0f64; w + 1];
    for y in 0..h {
        let row = src.get(y * w..(y + 1) * w).unwrap_or_default();
        for (i, v) in row.iter().enumerate() {
            row_sum[i + 1] = row_sum[i] + f64::from(*v);
        }
        for x in 0..w {
            let (a, b) = (x.saturating_sub(r), (x + r + 1).min(w));
            if let Some(t) = tmp.get_mut(y * w + x) {
                *t = ((row_sum[b] - row_sum[a]) / (b - a) as f64) as f32;
            }
        }
    }
    let mut out = vec![0.0f32; w * h];
    let mut col_sum = vec![0.0f64; h + 1];
    for x in 0..w {
        for y in 0..h {
            col_sum[y + 1] = col_sum[y] + f64::from(tmp.get(y * w + x).copied().unwrap_or(0.0));
        }
        for y in 0..h {
            let (a, b) = (y.saturating_sub(r), (y + r + 1).min(h));
            if let Some(o) = out.get_mut(y * w + x) {
                *o = ((col_sum[b] - col_sum[a]) / (b - a) as f64) as f32;
            }
        }
    }
    out
}

/// Guided filter of `p` with guide `guide` (both `w × h`, values 0..1), window radius `r`.
pub fn guided_filter(guide: &[f32], p: &[f32], w: usize, h: usize, r: usize, eps: f32) -> Vec<f32> {
    let n = w * h;
    if guide.len() < n || p.len() < n {
        return p.to_vec();
    }
    let mean_i = box_mean(guide, w, h, r);
    let mean_p = box_mean(p, w, h, r);
    let ip: Vec<f32> = guide.iter().zip(p).map(|(a, b)| a * b).collect();
    let ii: Vec<f32> = guide.iter().map(|a| a * a).collect();
    let corr_ip = box_mean(&ip, w, h, r);
    let corr_ii = box_mean(&ii, w, h, r);
    let eps = if eps.is_finite() && eps > 0.0 { eps } else { GUIDE_EPS };
    let mut a = vec![0.0f32; n];
    let mut b = vec![0.0f32; n];
    for i in 0..n {
        let (mi, mp) = (mean_i[i], mean_p[i]);
        let var = (corr_ii[i] - mi * mi).max(0.0);
        let cov = corr_ip[i] - mi * mp;
        a[i] = cov / (var + eps);
        b[i] = mp - a[i] * mi;
    }
    let mean_a = box_mean(&a, w, h, r);
    let mean_b = box_mean(&b, w, h, r);
    (0..n).map(|i| mean_a[i] * guide[i] + mean_b[i]).collect()
}

/// Zeroes the parts of `mask` (`w × h`) over 0.5 that are not connected (8 neighbours) to a part
/// at least `share` of the largest one's size.
pub fn drop_islands(mask: &mut [f32], w: usize, h: usize, share: f32) {
    let n = w * h;
    if mask.len() < n || n == 0 {
        return;
    }
    let mut label = vec![0u32; n];
    let mut sizes: Vec<usize> = vec![0];
    let mut stack = vec![];
    for start in 0..n {
        if label[start] != 0 || mask[start] <= 0.5 {
            continue;
        }
        let id = sizes.len() as u32;
        let mut size = 0usize;
        label[start] = id;
        stack.push(start);
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i % w, i / w);
            for dy in [-1i64, 0, 1] {
                for dx in [-1i64, 0, 1] {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if label[j] == 0 && mask[j] > 0.5 {
                        label[j] = id;
                        stack.push(j);
                    }
                }
            }
        }
        sizes.push(size);
    }
    let largest = sizes.iter().copied().max().unwrap_or(0);
    let min = (largest as f32 * share.clamp(0.0, 1.0)).ceil() as usize;
    for (m, l) in mask.iter_mut().zip(&label) {
        if *l != 0 && sizes.get(*l as usize).is_some_and(|s| *s < min) {
            *m = 0.0;
        }
    }
}

/// Maps `v` so `lo` and below give 0, `hi` and above 1, linear in between.
pub fn contrast(v: f32, lo: f32, hi: f32) -> f32 {
    if !v.is_finite() {
        return 0.0;
    }
    if hi <= lo {
        return if v >= lo { 1.0 } else { 0.0 };
    }
    ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// Brightness (0..1, Rec. 709 weights) of each pixel of `img`.
fn luminance(img: &RgbaImage) -> Vec<f32> {
    img.pixels().map(|p| (0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])) / 255.0).collect()
}

/// The working size for a `w × h` picture: at most [`WORK_SIDE`] across, same proportions.
pub fn work_size(w: u32, h: u32) -> (u32, u32) {
    let side = w.max(h);
    if side <= WORK_SIDE {
        return (w.max(1), h.max(1));
    }
    let k = f64::from(WORK_SIDE) / f64::from(side);
    (((f64::from(w) * k).round() as u32).max(1), ((f64::from(h) * k).round() as u32).max(1))
}

/// The mask for `img` from the model's raw output `raw` (`side × side`, any range): see the
/// module docs for the steps.
pub fn refine(img: &RgbaImage, raw: &[f32], side: u32) -> GrayImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return GrayImage::new(w, h);
    }
    let mut low = raw.to_vec();
    low.resize((side as usize) * (side as usize), 0.0);
    normalize(&mut low);
    let (ww, wh) = work_size(w, h);
    let (wu, hu) = (ww as usize, wh as usize);
    let mut mask = resize_plane(&low, side, side, ww, wh);
    drop_islands(&mut mask, wu, hu, ISLAND_SHARE);
    let guide_img = if (ww, wh) == (w, h) { img.clone() } else { image::imageops::resize(img, ww, wh, image::imageops::FilterType::Triangle) };
    let guide = luminance(&guide_img);
    // About two of the model's pixels: the blur its upscaled answer has.
    let r = ((f64::from(ww.max(wh)) / f64::from(side.max(1))) * 2.0).round().clamp(2.0, 32.0) as usize;
    let snapped = guided_filter(&guide, &mask, wu, hu, r, GUIDE_EPS);
    // Keep the model's answer where it is sure; take the snapped edge in the uncertain band.
    let work: Vec<f32> = mask
        .iter()
        .zip(&snapped)
        .map(|(m, s)| {
            let band = 1.0 - (2.0 * m - 1.0).abs(); // 1 at 0.5, 0 at 0 and 1
            let v = m + (s - m) * band.sqrt();
            contrast(v, EDGE_LOW, EDGE_HIGH)
        })
        .collect();
    let full = if (ww, wh) == (w, h) { work } else { resize_plane(&work, ww, wh, w, h) };
    GrayImage::from_fn(w, h, |x, y| {
        let v = full.get(y as usize * w as usize + x as usize).copied().unwrap_or(0.0);
        Luma([(v.clamp(0.0, 1.0) * 255.0).round() as u8])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn normalize_stretches_and_survives_junk() {
        let mut v = vec![2.0, 4.0, 3.0];
        normalize(&mut v);
        assert_eq!(v, vec![0.0, 1.0, 0.5]);
        let mut flat = vec![0.7; 4];
        normalize(&mut flat);
        assert!(flat.iter().all(|x| *x == 0.0));
        let mut junk = vec![f32::NAN, 1.0, f32::INFINITY, 3.0];
        normalize(&mut junk);
        assert_eq!(junk, vec![0.0, 0.0, 0.0, 1.0]);
        normalize(&mut []);
    }

    #[test]
    fn resize_plane_interpolates() {
        let src = [0.0, 1.0, 0.0, 1.0];
        let up = resize_plane(&src, 2, 2, 4, 2);
        assert_eq!(up.len(), 8);
        assert_eq!(up[0], 0.0);
        assert_eq!(up[3], 1.0);
        assert!(up[1] > 0.0 && up[1] < up[2] && up[2] < 1.0);
        assert_eq!(resize_plane(&src, 2, 2, 1, 1), vec![0.5]);
        assert_eq!(resize_plane(&[], 2, 2, 3, 3), vec![0.0; 9]);
        assert_eq!(resize_plane(&src, 2, 2, 0, 5), Vec::<f32>::new());
    }

    #[test]
    fn box_mean_matches_brute_force() {
        let (w, h, r) = (7, 5, 2);
        let src: Vec<f32> = (0..w * h).map(|i| ((i * 37) % 11) as f32).collect();
        let fast = box_mean(&src, w, h, r);
        for y in 0..h {
            for x in 0..w {
                let (mut sum, mut n) = (0.0, 0);
                for yy in y.saturating_sub(r)..(y + r + 1).min(h) {
                    for xx in x.saturating_sub(r)..(x + r + 1).min(w) {
                        sum += src[yy * w + xx];
                        n += 1;
                    }
                }
                assert!((fast[y * w + x] - sum / n as f32).abs() < 1e-4);
            }
        }
        assert!(box_mean(&[], 0, 0, 3).is_empty());
    }

    #[test]
    fn guided_filter_snaps_a_blurry_edge_to_the_picture_edge() {
        // The picture has a sharp edge at x = 10; the mask is a soft ramp around it.
        let (w, h) = (20usize, 6usize);
        let guide: Vec<f32> = (0..w * h).map(|i| if i % w < 10 { 0.1 } else { 0.9 }).collect();
        let mask: Vec<f32> = (0..w * h).map(|i| ((i % w) as f32 - 5.0) / 10.0).map(|v| v.clamp(0.0, 1.0)).collect();
        let out = guided_filter(&guide, &mask, w, h, 3, 1e-3);
        let row = |v: &[f32], x: usize| v[2 * w + x];
        // Just either side of the edge the result is much closer to 0 and 1 than the ramp.
        assert!(row(&out, 8) < row(&mask, 8) - 0.05, "{} vs {}", row(&out, 8), row(&mask, 8));
        assert!(row(&out, 11) > row(&mask, 11) + 0.05, "{} vs {}", row(&out, 11), row(&mask, 11));
        // A flat guide leaves the mask a smoothed version of itself.
        let flat = vec![0.5; w * h];
        let same = guided_filter(&flat, &mask, w, h, 1, 1e-3);
        assert!((row(&same, 10) - row(&mask, 10)).abs() < 0.05);
        assert_eq!(guided_filter(&[], &mask, w, h, 1, 1e-3), mask);
    }

    #[test]
    fn islands_are_dropped_but_large_parts_kept() {
        let (w, h) = (30usize, 30usize);
        let mut m = vec![0.0f32; w * h];
        for y in 5..25 {
            for x in 5..15 {
                m[y * w + x] = 1.0; // main subject: 200 px
            }
        }
        for y in 5..20 {
            for x in 18..26 {
                m[y * w + x] = 0.9; // second subject: 120 px, kept
            }
        }
        m[27 * w + 27] = 0.8; // a stray pixel
        m[28 * w + 2] = 0.4; // below 0.5: not a part
        drop_islands(&mut m, w, h, 0.04);
        assert_eq!(m[10 * w + 10], 1.0);
        assert_eq!(m[10 * w + 20], 0.9);
        assert_eq!(m[27 * w + 27], 0.0);
        assert_eq!(m[28 * w + 2], 0.4);
        drop_islands(&mut [], 0, 0, 0.5);
    }

    #[test]
    fn contrast_and_work_size() {
        assert_eq!(contrast(0.1, 0.2, 0.8), 0.0);
        assert_eq!(contrast(0.9, 0.2, 0.8), 1.0);
        assert!((contrast(0.5, 0.2, 0.8) - 0.5).abs() < 1e-6);
        assert_eq!(contrast(f32::NAN, 0.2, 0.8), 0.0);
        assert_eq!(contrast(0.5, 0.5, 0.5), 1.0);
        assert_eq!(work_size(640, 480), (640, 480));
        assert_eq!(work_size(4000, 2000), (1280, 640));
        assert_eq!(work_size(10, 40_000), (1, 1280));
    }

    #[test]
    fn refine_follows_the_picture() {
        // A dark disc on a light backdrop, and a model answer that is a blurry disc slightly off.
        let img = RgbaImage::from_fn(200, 160, |x, y| {
            let d = (f64::from(x) - 100.0).hypot(f64::from(y) - 80.0);
            if d < 50.0 { Rgba([40, 60, 90, 255]) } else { Rgba([230, 225, 210, 255]) }
        });
        let side = 32u32;
        let raw: Vec<f32> = (0..side * side)
            .map(|i| {
                let (x, y) = ((i % side) as f64 + 0.5, (i / side) as f64 + 0.5);
                let d = ((x / 32.0 * 200.0) - 100.0).hypot((y / 32.0 * 160.0) - 80.0);
                (1.0 / (1.0 + ((d - 50.0) / 8.0).exp())) as f32 * 7.0 - 3.0 // logits-ish range
            })
            .collect();
        let m = refine(&img, &raw, side);
        assert_eq!(m.dimensions(), (200, 160));
        assert_eq!(m.get_pixel(100, 80)[0], 255);
        assert_eq!(m.get_pixel(5, 5)[0], 0);
        // Just inside and outside the disc's edge the mask is decided, not half-way.
        assert!(m.get_pixel(100 + 45, 80)[0] > 200, "{}", m.get_pixel(145, 80)[0]);
        assert!(m.get_pixel(100 + 56, 80)[0] < 40, "{}", m.get_pixel(156, 80)[0]);
        // Degenerate inputs.
        assert_eq!(refine(&RgbaImage::new(0, 4), &raw, side).dimensions(), (0, 4));
        assert!(refine(&img, &[], side).pixels().all(|p| p[0] == 0));
        assert!(refine(&img, &[f32::NAN; 4], 2).pixels().all(|p| p[0] == 0));
    }
}
