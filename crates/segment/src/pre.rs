//! Picture → model input.
//!
//! The U²-Net family takes a square RGB picture (`side` × `side`), stretched to that size,
//! scaled so its brightest channel value is 1, then normalised per channel with the training
//! mean and standard deviation, laid out as planes (`[1, 3, side, side]`, "NCHW").

use image::{Rgba, RgbaImage};

/// The model input for `img`: `3 × side × side` values, red plane first.
///
/// Transparent pixels count as what they show over black (their colour times their alpha), so a
/// picture that already has a transparent background does not confuse the model.
pub fn tensor(img: &RgbaImage, side: u32, mean: [f32; 3], std: [f32; 3]) -> Vec<f32> {
    let side = side.max(1);
    let n = (side as usize) * (side as usize);
    if img.width() == 0 || img.height() == 0 {
        return vec![0.0; 3 * n];
    }
    let flat = RgbaImage::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = u16::from(p[3]);
        let m = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
        Rgba([m(p[0]), m(p[1]), m(p[2]), 255])
    });
    let small = image::imageops::resize(&flat, side, side, image::imageops::FilterType::Triangle);
    let max = small.pixels().flat_map(|p| [p[0], p[1], p[2]]).max().unwrap_or(0);
    let scale = 1.0 / f32::from(max.max(1));
    let mut out = vec![0.0f32; 3 * n];
    for (i, p) in small.pixels().enumerate() {
        for c in 0..3 {
            let s = std[c];
            let v = (f32::from(p[c]) * scale - mean[c]) / if s.abs() > 1e-6 { s } else { 1.0 };
            if let Some(slot) = out.get_mut(c * n + i) {
                *slot = v;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_and_normalisation() {
        // Left half red, right half blue.
        let img = RgbaImage::from_fn(20, 10, |x, _| if x < 10 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) });
        let t = tensor(&img, 8, [0.5, 0.5, 0.5], [0.5, 0.5, 0.5]);
        assert_eq!(t.len(), 3 * 64);
        // Red plane: (1 - 0.5) / 0.5 = 1 on the left, (0 - 0.5) / 0.5 = -1 on the right.
        assert!((t[0] - 1.0).abs() < 1e-5, "{}", t[0]);
        assert!((t[7] + 1.0).abs() < 1e-5, "{}", t[7]);
        // Blue plane is the opposite.
        assert!((t[128] + 1.0).abs() < 1e-5);
        assert!((t[128 + 7] - 1.0).abs() < 1e-5);
        assert!(t.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn scales_by_the_brightest_value_and_handles_edge_cases() {
        // A dark picture is brightened to full range, like the model's training data.
        let dark = RgbaImage::from_pixel(4, 4, Rgba([100, 50, 0, 255]));
        let t = tensor(&dark, 2, [0.0; 3], [1.0; 3]);
        assert!((t[0] - 1.0).abs() < 1e-5 && (t[4] - 0.5).abs() < 0.01 && t[8].abs() < 1e-5);
        // Empty, black, transparent and zero-std inputs give finite values of the right size.
        assert_eq!(tensor(&RgbaImage::new(0, 0), 4, [0.0; 3], [1.0; 3]), vec![0.0; 48]);
        let black = tensor(&RgbaImage::from_pixel(3, 3, Rgba([0, 0, 0, 255])), 4, [0.5; 3], [0.0; 3]);
        assert!(black.iter().all(|v| v.is_finite()));
        let clear = tensor(&RgbaImage::from_pixel(3, 3, Rgba([255, 255, 255, 0])), 4, [0.0; 3], [1.0; 3]);
        assert!(clear.iter().all(|v| *v == 0.0));
        assert_eq!(tensor(&dark, 0, [0.0; 3], [1.0; 3]).len(), 3);
    }
}
