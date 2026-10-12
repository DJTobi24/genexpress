//! Picture effects for GenExpress (layer L2): decoding with limits, photo adjustments, filter
//! presets, crop/resize/convert and the image quick actions.
//!
//! Pictures are [`RgbaImage`]s with straight (not premultiplied) alpha.
#![forbid(unsafe_code)]

pub use image::RgbaImage;

use genexpress_doc::{Adjustments, CropRect};

/// Largest picture side accepted when decoding.
pub const MAX_SIDE: u32 = 16_384;
/// Most pixels accepted when decoding (100 megapixels).
pub const MAX_PIXELS: u64 = 100_000_000;

#[derive(Debug, thiserror::Error)]
pub enum EffectError {
    #[error("not a supported picture: {0}")]
    Decode(String),
    #[error("picture too large ({0}×{1} pixels)")]
    TooLarge(u32, u32),
    #[error("could not encode the picture: {0}")]
    Encode(String),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, EffectError>;

/// Size and type of an encoded picture, read without decoding the pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub mime: &'static str,
}

/// Reads an encoded picture's size and type.
pub fn probe(bytes: &[u8]) -> Result<ImageInfo> {
    let fmt = image::guess_format(bytes).map_err(|e| EffectError::Decode(e.to_string()))?;
    let reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), fmt);
    let (width, height) = reader.into_dimensions().map_err(|e| EffectError::Decode(e.to_string()))?;
    Ok(ImageInfo { width, height, mime: fmt.to_mime_type() })
}

/// Decodes PNG, JPEG, WebP, GIF (first frame) or BMP, refusing pictures past [`MAX_SIDE`] /
/// [`MAX_PIXELS`] before allocating them.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage> {
    let info = probe(bytes)?;
    if info.width > MAX_SIDE || info.height > MAX_SIDE || u64::from(info.width) * u64::from(info.height) > MAX_PIXELS {
        return Err(EffectError::TooLarge(info.width, info.height));
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e| EffectError::Decode(e.to_string()))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let img = reader.decode().map_err(|e| EffectError::Decode(e.to_string()))?;
    Ok(img.to_rgba8())
}

/// The part of `img` inside `crop` (fractions of its size).
pub fn crop(img: &RgbaImage, crop: &CropRect) -> RgbaImage {
    if crop.is_full() {
        return img.clone();
    }
    let c = crop.sanitized();
    let (w, h) = (f64::from(img.width()), f64::from(img.height()));
    let x = (c.x * w).floor().max(0.0) as u32;
    let y = (c.y * h).floor().max(0.0) as u32;
    let cw = ((c.width * w).round() as u32).clamp(1, img.width().saturating_sub(x).max(1));
    let ch = ((c.height * h).round() as u32).clamp(1, img.height().saturating_sub(y).max(1));
    image::imageops::crop_imm(img, x, y, cw, ch).to_image()
}

/// Applies photo adjustments in place.
pub fn apply_adjustments(img: &mut RgbaImage, a: &Adjustments) {
    if a.is_identity() {
        return;
    }
    let f = |v: f32| if v.is_finite() { v } else { 0.0 };
    let brightness = f(a.brightness).clamp(-100.0, 100.0) / 100.0;
    let contrast = f(a.contrast).clamp(-100.0, 100.0) / 100.0;
    let saturation = f(a.saturation).clamp(-100.0, 100.0) / 100.0;
    let warmth = f(a.warmth).clamp(-100.0, 100.0) / 100.0;
    let gray = f(a.grayscale).clamp(0.0, 100.0) / 100.0;
    let sepia = f(a.sepia).clamp(0.0, 100.0) / 100.0;
    let cfactor = if contrast >= 0.0 { 1.0 + contrast * 2.0 } else { 1.0 + contrast };
    for p in img.pixels_mut() {
        let [mut r, mut g, mut b] = [p[0], p[1], p[2]].map(|v| f32::from(v) / 255.0);
        // brightness
        r += brightness * 0.5;
        g += brightness * 0.5;
        b += brightness * 0.5;
        // contrast about mid grey
        r = (r - 0.5) * cfactor + 0.5;
        g = (g - 0.5) * cfactor + 0.5;
        b = (b - 0.5) * cfactor + 0.5;
        // saturation
        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let s = 1.0 + saturation;
        r = l + (r - l) * s;
        g = l + (g - l) * s;
        b = l + (b - l) * s;
        // warmth: push red/blue apart
        r += warmth * 0.12;
        b -= warmth * 0.12;
        // grayscale
        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        r += (l - r) * gray;
        g += (l - g) * gray;
        b += (l - b) * gray;
        // sepia
        if sepia > 0.0 {
            let sr = 0.393 * r + 0.769 * g + 0.189 * b;
            let sg = 0.349 * r + 0.686 * g + 0.168 * b;
            let sb = 0.272 * r + 0.534 * g + 0.131 * b;
            r += (sr - r) * sepia;
            g += (sg - g) * sepia;
            b += (sb - b) * sepia;
        }
        if a.invert {
            r = 1.0 - r;
            g = 1.0 - g;
            b = 1.0 - b;
        }
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        p[0] = q(r);
        p[1] = q(g);
        p[2] = q(b);
    }
    let blur = f(a.blur).clamp(0.0, 100.0);
    if blur > 0.0 {
        *img = image::imageops::blur(img, blur / 4.0);
    }
    let sharpen = f(a.sharpen).clamp(0.0, 100.0);
    if sharpen > 0.0 {
        *img = image::imageops::unsharpen(img, 1.5, (100.0 - sharpen) as i32 / 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_pixel(w, h, image::Rgba([200, 100, 50, 255]));
        let mut out = vec![];
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn decode_and_probe() {
        let bytes = png(4, 3);
        assert_eq!(probe(&bytes).unwrap(), ImageInfo { width: 4, height: 3, mime: "image/png" });
        assert_eq!(decode(&bytes).unwrap().dimensions(), (4, 3));
        assert!(decode(b"not a picture").is_err());
        assert!(decode(&bytes[..20]).is_err());
    }

    #[test]
    fn crop_fractions() {
        let img = RgbaImage::new(100, 50);
        let c = crop(&img, &CropRect { x: 0.5, y: 0.0, width: 0.5, height: 0.5 });
        assert_eq!(c.dimensions(), (50, 25));
        let c = crop(&img, &CropRect { x: f64::NAN, y: 2.0, width: -1.0, height: 9.0 });
        assert!(c.width() >= 1 && c.height() >= 1);
    }

    #[test]
    fn adjustments() {
        let mut img = RgbaImage::from_pixel(2, 2, image::Rgba([200, 100, 50, 255]));
        apply_adjustments(&mut img, &Adjustments { grayscale: 100.0, ..Default::default() });
        let p = img.get_pixel(0, 0);
        assert!(p[0] == p[1] && p[1] == p[2]);
        let mut img = RgbaImage::from_pixel(2, 2, image::Rgba([10, 10, 10, 255]));
        apply_adjustments(&mut img, &Adjustments { invert: true, ..Default::default() });
        assert_eq!(img.get_pixel(1, 1)[0], 245);
    }
}
