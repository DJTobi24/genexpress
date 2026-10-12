//! One-click filter presets (built on [`Adjustments`]) and the "palette from photo" colour picker.

use std::collections::BTreeMap;

use genexpress_color::Color;
use genexpress_doc::Adjustments;
use image::RgbaImage;

/// A named look made of photo adjustments.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub adjustments: Adjustments,
}

fn preset(id: &'static str, label: &'static str, adjustments: Adjustments) -> FilterPreset {
    FilterPreset { id, label, adjustments }
}

/// The built-in filter presets, "none" first.
pub fn filter_presets() -> Vec<FilterPreset> {
    let a = Adjustments::default;
    vec![
        preset("none", "None", a()),
        preset("vivid", "Vivid", Adjustments { brightness: 3.0, contrast: 15.0, saturation: 35.0, ..a() }),
        preset("mono", "Mono", Adjustments { contrast: 10.0, grayscale: 100.0, ..a() }),
        preset("noir", "Noir", Adjustments { brightness: -8.0, contrast: 45.0, grayscale: 100.0, sharpen: 15.0, ..a() }),
        preset("warm", "Warm", Adjustments { brightness: 3.0, saturation: 8.0, warmth: 35.0, ..a() }),
        preset("cool", "Cool", Adjustments { saturation: -5.0, warmth: -35.0, ..a() }),
        preset("fade", "Fade", Adjustments { brightness: 8.0, contrast: -30.0, saturation: -25.0, ..a() }),
        preset("sepia", "Sepia", Adjustments { contrast: 5.0, sepia: 80.0, ..a() }),
        preset("dramatic", "Dramatic", Adjustments { brightness: -5.0, contrast: 40.0, saturation: -15.0, sharpen: 25.0, ..a() }),
        preset("soft", "Soft", Adjustments { brightness: 6.0, contrast: -15.0, saturation: -5.0, blur: 2.0, ..a() }),
        preset("retro", "Retro", Adjustments { contrast: -10.0, saturation: -20.0, warmth: 20.0, sepia: 35.0, ..a() }),
        preset("sunny", "Sunny", Adjustments { brightness: 10.0, contrast: -5.0, saturation: 15.0, warmth: 12.0, ..a() }),
        preset("frost", "Frost", Adjustments { brightness: 8.0, saturation: -30.0, warmth: -20.0, ..a() }),
        preset("invert", "Invert", Adjustments { invert: true, ..a() }),
    ]
}

/// The preset with id `id`.
pub fn filter_preset(id: &str) -> Option<FilterPreset> {
    filter_presets().into_iter().find(|p| p.id == id)
}

/// The preset's adjustments scaled by `intensity` (`0..=100`; 0 is "unchanged", 100 the full
/// preset). Inversion switches on at 50 and above.
pub fn blend_preset(p: &FilterPreset, intensity: f32) -> Adjustments {
    let k = if intensity.is_finite() { intensity.clamp(0.0, 100.0) / 100.0 } else { 1.0 };
    let a = &p.adjustments;
    Adjustments {
        brightness: a.brightness * k,
        contrast: a.contrast * k,
        saturation: a.saturation * k,
        warmth: a.warmth * k,
        sharpen: a.sharpen * k,
        blur: a.blur * k,
        grayscale: a.grayscale * k,
        sepia: a.sepia * k,
        invert: a.invert && k >= 0.5,
    }
}

/// Largest side the picture is reduced to before its colours are clustered.
const SAMPLE_SIDE: u32 = 64;
/// Most colours [`dominant_colors`] returns.
pub const MAX_DOMINANT: usize = 16;

/// The `count` (at most [`MAX_DOMINANT`]) most representative colours of `img`, most common first,
/// for "palette from photo" and brand-colour extraction. Mostly transparent pixels are ignored.
///
/// Works on a ≤ 64 px downsample: colours are bucketed at 5 bits per channel, then the buckets are
/// clustered with a weighted k-means (deterministic farthest-point seeding). Fewer colours are
/// returned when the picture has fewer distinct ones.
pub fn dominant_colors(img: &RgbaImage, count: usize) -> Vec<Color> {
    let count = count.min(MAX_DOMINANT);
    let (w, h) = img.dimensions();
    if count == 0 || w == 0 || h == 0 {
        return vec![];
    }
    let small = if w > SAMPLE_SIDE || h > SAMPLE_SIDE {
        let s = f64::from(SAMPLE_SIDE) / f64::from(w.max(h));
        let nw = ((f64::from(w) * s).round() as u32).clamp(1, SAMPLE_SIDE);
        let nh = ((f64::from(h) * s).round() as u32).clamp(1, SAMPLE_SIDE);
        image::imageops::thumbnail(img, nw, nh)
    } else {
        img.clone()
    };
    // Buckets: key → (sum r, g, b, count).
    let mut buckets: BTreeMap<u16, ([f64; 3], f64)> = BTreeMap::new();
    for p in small.pixels().filter(|p| p[3] >= 128) {
        let key = (u16::from(p[0] >> 3) << 10) | (u16::from(p[1] >> 3) << 5) | u16::from(p[2] >> 3);
        let e = buckets.entry(key).or_insert(([0.0; 3], 0.0));
        e.0[0] += f64::from(p[0]);
        e.0[1] += f64::from(p[1]);
        e.0[2] += f64::from(p[2]);
        e.1 += 1.0;
    }
    let points: Vec<([f64; 3], f64)> = buckets.values().map(|(s, n)| ([s[0] / n, s[1] / n, s[2] / n], *n)).collect();
    let dist = |a: &[f64; 3], b: &[f64; 3]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2);

    // Farthest-point seeding, weighted by bucket size.
    let mut centres: Vec<[f64; 3]> = vec![];
    if let Some(first) = points.iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
        centres.push(first.0);
    }
    while centres.len() < count {
        let best = points
            .iter()
            .map(|(c, n)| (centres.iter().map(|k| dist(c, k)).fold(f64::INFINITY, f64::min) * n.sqrt(), c))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        match best {
            Some((d, c)) if d > 1.0 => centres.push(*c),
            _ => break,
        }
    }

    // Weighted k-means.
    let mut weights = vec![0.0f64; centres.len()];
    for _ in 0..12 {
        let mut sums = vec![([0.0f64; 3], 0.0f64); centres.len()];
        for (c, n) in &points {
            let nearest = centres.iter().enumerate().map(|(i, k)| (dist(c, k), i)).min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, i)) = nearest
                && let Some(s) = sums.get_mut(i)
            {
                for (acc, v) in s.0.iter_mut().zip(c) {
                    *acc += v * n;
                }
                s.1 += n;
            }
        }
        for ((k, w), (s, n)) in centres.iter_mut().zip(weights.iter_mut()).zip(&sums) {
            *w = *n;
            if *n > 0.0 {
                *k = [s[0] / n, s[1] / n, s[2] / n];
            }
        }
    }
    let mut out: Vec<(f64, [f64; 3])> = weights.into_iter().zip(centres).filter(|(w, _)| *w > 0.0).collect();
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    let q = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    out.into_iter().map(|(_, c)| Color::rgb(q(c[0]), q(c[1]), q(c[2]))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn presets_are_unique_and_scale() {
        let all = filter_presets();
        assert!(all.len() >= 12);
        let mut ids = std::collections::HashSet::new();
        for p in &all {
            assert!(ids.insert(p.id), "duplicate {}", p.id);
            assert!(!p.label.is_empty());
        }
        assert!(filter_preset("none").is_some_and(|p| p.adjustments.is_identity()));
        assert!(filter_preset("nope").is_none());
        let vivid = filter_preset("vivid").unwrap();
        assert_eq!(blend_preset(&vivid, 100.0), vivid.adjustments);
        assert!(blend_preset(&vivid, 0.0).is_identity());
        assert!((blend_preset(&vivid, 50.0).saturation - vivid.adjustments.saturation / 2.0).abs() < 1e-6);
        assert_eq!(blend_preset(&vivid, 500.0), vivid.adjustments);
        let inv = filter_preset("invert").unwrap();
        assert!(blend_preset(&inv, 60.0).invert && !blend_preset(&inv, 40.0).invert);
    }

    #[test]
    fn presets_apply_without_panicking() {
        for p in filter_presets() {
            let mut img = RgbaImage::from_pixel(4, 4, Rgba([120, 80, 200, 255]));
            crate::apply_adjustments(&mut img, &p.adjustments);
        }
    }

    #[test]
    fn dominant_colours_find_the_big_areas() {
        // 60 % red, 30 % blue, 10 % white.
        let img = RgbaImage::from_fn(100, 100, |x, _| match x {
            0..60 => Rgba([220, 20, 30, 255]),
            60..90 => Rgba([20, 40, 200, 255]),
            _ => Rgba([255, 255, 255, 255]),
        });
        let c = dominant_colors(&img, 3);
        assert_eq!(c.len(), 3);
        assert!(c[0].r > 180 && c[0].b < 80, "{c:?}");
        assert!(c[1].b > 160 && c[1].r < 60, "{c:?}");
        assert!(c[2].r > 230 && c[2].g > 230, "{c:?}");
        // Asking for more colours than exist returns fewer.
        assert_eq!(dominant_colors(&RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 255])), 5).len(), 1);
        assert!(dominant_colors(&img, 0).is_empty());
        assert!(dominant_colors(&RgbaImage::new(10, 10), 3).is_empty(), "transparent pixels are ignored");
        assert!(dominant_colors(&img, 1000).len() <= MAX_DOMINANT);
    }
}
