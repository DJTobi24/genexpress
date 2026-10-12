//! Built-in content for GenHa (layer L2), all original and generated in code:
//!
//! - [`catalog`] / [`instantiate`]: ready-made designs (social posts, flyers, cards, slides…) built
//!   only from shapes, gradients, library icons and text, optionally re-coloured with a [`BrandKit`];
//! - [`shape_library`]: basic shapes and categorised icons as SVG path data in a 100 × 100 view box;
//! - [`text_presets`]: one-click text styles;
//! - [`palettes`]: curated colour palettes;
//! - [`qr_code`]: QR codes as a vector shape.
#![forbid(unsafe_code)]

mod build;
mod designs;
mod palettes;
mod qr;
mod shapes;
mod text;

pub use designs::{TemplateInfo, catalog, instantiate};
pub use palettes::{Palette, palettes};
pub use qr::{MAX_QR_BYTES, qr_code};
pub use shapes::{LibraryShape, shape_library};
pub use text::{TextPreset, text_presets};

#[doc(no_inline)]
pub use genha_doc::BrandKit;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("nothing to encode")]
    Empty,
    #[error("too much data for a QR code ({0} bytes, at most {MAX_QR_BYTES})")]
    TooLong(usize),
    #[error("the colour is too light to scan on white; pick a darker one")]
    LowContrast,
    #[error("could not make a QR code: {0}")]
    Qr(String),
}

#[cfg(test)]
mod tests;
