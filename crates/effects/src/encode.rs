//! Encoding pictures for export ("convert to JPG/PNG…").

use image::{ImageEncoder, RgbImage, RgbaImage};

use crate::{EffectError, Result};

/// A picture file format GenExpress can write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    /// Lossy; `quality` is `1..=100` (out-of-range values are clamped).
    Jpeg {
        quality: u8,
    },
    /// Lossless WebP.
    Webp,
    /// A single-frame GIF (colours are quantized to 256).
    Gif,
    Bmp,
}

impl OutputFormat {
    /// The JPEG quality used when none is given.
    pub const DEFAULT_JPEG_QUALITY: u8 = 90;

    /// The format for a file extension (`png`, `jpg`/`jpeg`, `webp`, `gif`, `bmp`; any case, with
    /// or without the leading dot).
    pub fn from_extension(ext: &str) -> Option<Self> {
        let e = ext.trim().trim_start_matches('.').to_ascii_lowercase();
        match e.as_str() {
            "png" => Some(OutputFormat::Png),
            "jpg" | "jpeg" | "jpe" => Some(OutputFormat::Jpeg { quality: Self::DEFAULT_JPEG_QUALITY }),
            "webp" => Some(OutputFormat::Webp),
            "gif" => Some(OutputFormat::Gif),
            "bmp" => Some(OutputFormat::Bmp),
            _ => None,
        }
    }

    /// The usual file extension, without a dot.
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpeg { .. } => "jpg",
            OutputFormat::Webp => "webp",
            OutputFormat::Gif => "gif",
            OutputFormat::Bmp => "bmp",
        }
    }

    pub fn mime(&self) -> &'static str {
        match self {
            OutputFormat::Png => "image/png",
            OutputFormat::Jpeg { .. } => "image/jpeg",
            OutputFormat::Webp => "image/webp",
            OutputFormat::Gif => "image/gif",
            OutputFormat::Bmp => "image/bmp",
        }
    }

    /// Whether the format keeps transparency.
    pub fn supports_alpha(&self) -> bool {
        !matches!(self, OutputFormat::Jpeg { .. } | OutputFormat::Bmp)
    }
}

/// Composites `img` over opaque white, dropping the alpha channel.
pub fn flatten_on_white(img: &RgbaImage) -> RgbImage {
    RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = u32::from(p[3]);
        let mix = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
        image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])])
    })
}

/// Encodes `img` as `fmt`. JPEG and BMP have no transparency, so the picture is composited on
/// white first.
pub fn encode(img: &RgbaImage, fmt: OutputFormat) -> Result<Vec<u8>> {
    if img.width() == 0 || img.height() == 0 {
        return Err(EffectError::Invalid("cannot encode an empty picture".into()));
    }
    let mut out = std::io::Cursor::new(Vec::new());
    let err = |e: image::ImageError| EffectError::Encode(e.to_string());
    match fmt {
        OutputFormat::Png => image::DynamicImage::ImageRgba8(img.clone()).write_to(&mut out, image::ImageFormat::Png).map_err(err)?,
        OutputFormat::Webp => image::DynamicImage::ImageRgba8(img.clone()).write_to(&mut out, image::ImageFormat::WebP).map_err(err)?,
        OutputFormat::Gif => image::DynamicImage::ImageRgba8(img.clone()).write_to(&mut out, image::ImageFormat::Gif).map_err(err)?,
        OutputFormat::Bmp => image::DynamicImage::ImageRgb8(flatten_on_white(img)).write_to(&mut out, image::ImageFormat::Bmp).map_err(err)?,
        OutputFormat::Jpeg { quality } => {
            let rgb = flatten_on_white(img);
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
            enc.write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8).map_err(err)?;
        }
    }
    Ok(out.into_inner())
}
