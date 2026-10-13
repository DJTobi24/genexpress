//! Subject segmentation for GenHa (layer L2): the optional machine-learning models behind
//! "Remove background" on busy photos.
//!
//! No model ships with GenHa. [`MODELS`] lists the models the app can download (where from, how
//! big, their licence and the SHA-256 every download must match); the engine keeps downloaded
//! models in the app's data folder (`genha_engine::cmd::models`). This crate:
//!
//! * checks a model file against its pinned size and SHA-256 ([`verify`]) — a model is only ever
//!   loaded after that check passes, so the inference runtime never parses an unknown file;
//! * runs it with [`tract`](https://github.com/sonos/tract) (pure Rust ONNX inference, MIT OR
//!   Apache-2.0), on the desktop and in the browser ([`Segmenter`]);
//! * turns a picture into the model's input and the model's output into a mask the size of the
//!   picture, with the edges snapped to the picture's own edges ([`pre`], [`post`]);
//! * downloads a model over HTTPS on the desktop ([`download`]; the browser fetches it itself).
//!
//! Masks are [`GrayImage`]s: 255 keeps a pixel, 0 removes it.
#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
pub mod download;
#[cfg(feature = "inference")]
mod infer;
#[cfg(not(feature = "inference"))]
#[path = "no_infer.rs"]
mod infer;
pub mod post;
pub mod pre;

pub use image::{GrayImage, RgbaImage};
pub use infer::{Segmenter, cached, forget_cached, peek};

/// Whether this build can run models (the `inference` feature: on in the desktop app and the CLI,
/// optional in the browser build).
pub const INFERENCE: bool = cfg!(feature = "inference");

/// What can go wrong; every message is meant for the user.
#[derive(Debug, thiserror::Error)]
pub enum SegmentError {
    #[error("unknown model `{0}` (known: {known})", known = known_names())]
    UnknownModel(String),
    #[error("the downloaded {name} model is damaged or not the expected file (checksum mismatch): delete it and download it again")]
    Checksum { name: String },
    #[error("the {name} model file has the wrong size ({got} bytes, expected {expected}): delete it and download it again")]
    Size { name: String, got: u64, expected: u64 },
    #[error("could not load the {name} model: {msg}")]
    Load { name: String, msg: String },
    #[error("this build of GenHa cannot run the background removal model: use the colour-based method")]
    Unavailable,
    #[error("the background removal model failed: {0}")]
    Run(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("you are offline or the server cannot be reached: {0}")]
    Offline(String),
    #[error("the download was cancelled")]
    Cancelled,
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, SegmentError>;

/// A model GenHa can download and run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelSpec {
    /// Stable id used by commands (`model.download {name}`).
    pub name: &'static str,
    /// Shown to people.
    pub label: &'static str,
    /// One line on what it is good for.
    pub summary: &'static str,
    /// The file name it is kept under (in the data folder's `models/`).
    pub file: &'static str,
    /// Where it is downloaded from (HTTPS).
    pub url: &'static str,
    /// SHA-256 of the file, lower-case hex: a download that does not match is thrown away.
    pub sha256: &'static str,
    /// Exact size in bytes.
    pub bytes: u64,
    /// Side of the square picture the model takes.
    pub input: u32,
    /// Per-channel mean and standard deviation the model was trained with (RGB, 0..1 values).
    pub mean: [f32; 3],
    pub std: [f32; 3],
    /// Licence of the weights (SPDX) and where the original weights and licence are published.
    pub licence: &'static str,
    pub source: &'static str,
}

/// ImageNet statistics, used by the U²-Net family.
const IMAGENET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGENET_STD: [f32; 3] = [0.229, 0.224, 0.225];

/// The models GenHa knows, the default (smallest, fastest) first.
///
/// Both are U²-Net (Qin et al., "U²-Net: Going Deeper with Nested U-Structure for Salient Object
/// Detection", Pattern Recognition 2020), whose code and weights are published under Apache-2.0 at
/// <https://github.com/xuebinqin/U-2-Net>. The ONNX conversions are downloaded from the rembg
/// project's release page (MIT; the weights keep their Apache-2.0 licence).
pub const MODELS: &[ModelSpec] = &[
    ModelSpec {
        name: "u2netp",
        label: "Fast (U²-Netp, 4.6 MB)",
        summary: "small and quick; good on people, animals and products, softer on fine detail",
        file: "u2netp.onnx",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx",
        sha256: "309c8469258dda742793dce0ebea8e6dd393174f89934733ecc8b14c76f4ddd8",
        bytes: 4_574_861,
        input: 320,
        mean: IMAGENET_MEAN,
        std: IMAGENET_STD,
        licence: "Apache-2.0",
        source: "https://github.com/xuebinqin/U-2-Net",
    },
    ModelSpec {
        name: "u2net",
        label: "High quality (U²-Net, 176 MB)",
        summary: "the full model: cleaner outlines and fewer stray patches, slower",
        file: "u2net.onnx",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
        sha256: U2NET_SHA256,
        bytes: 175_997_641,
        input: 320,
        mean: IMAGENET_MEAN,
        std: IMAGENET_STD,
        licence: "Apache-2.0",
        source: "https://github.com/xuebinqin/U-2-Net",
    },
];

const U2NET_SHA256: &str = "8d10d2f3bb75ae3b6d527c77944fc5e7dcd94b29809d47a739a7a728a912b491";

/// The model used when none is named.
pub const DEFAULT_MODEL: &str = "u2netp";

fn known_names() -> String {
    MODELS.iter().map(|m| m.name).collect::<Vec<_>>().join(", ")
}

/// The model called `name`.
pub fn model(name: &str) -> Result<&'static ModelSpec> {
    MODELS.iter().find(|m| m.name == name).ok_or_else(|| SegmentError::UnknownModel(name.chars().take(60).collect()))
}

/// SHA-256 of `bytes` as lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Checks that `bytes` are exactly `spec`'s file (size and SHA-256).
pub fn verify(spec: &ModelSpec, bytes: &[u8]) -> Result<()> {
    let got = bytes.len() as u64;
    if got != spec.bytes {
        return Err(SegmentError::Size { name: spec.name.into(), got, expected: spec.bytes });
    }
    if sha256_hex(bytes) != spec.sha256 {
        return Err(SegmentError::Checksum { name: spec.name.into() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_consistent() {
        assert_eq!(model(DEFAULT_MODEL).unwrap().name, DEFAULT_MODEL);
        for m in MODELS {
            assert_eq!(m.sha256.len(), 64, "{}", m.name);
            assert!(m.sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()), "{}", m.name);
            assert!(m.url.starts_with("https://"), "{}", m.name);
            assert!(m.file.ends_with(".onnx") && !m.file.contains('/'), "{}", m.name);
            assert!(m.input >= 32 && m.std.iter().all(|s| *s > 0.0));
        }
        assert!(matches!(model("rmbg"), Err(SegmentError::UnknownModel(_))));
        assert!(model("rmbg").unwrap_err().to_string().contains("u2netp"));
    }

    #[test]
    fn checksums() {
        // Known SHA-256 test vector.
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let spec = ModelSpec { bytes: 3, sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", ..MODELS[0] };
        assert!(verify(&spec, b"abc").is_ok());
        assert!(matches!(verify(&spec, b"abd"), Err(SegmentError::Checksum { .. })));
        assert!(matches!(verify(&spec, b"abcd"), Err(SegmentError::Size { got: 4, .. })));
        // A fake file never passes for a real model.
        assert!(verify(&MODELS[0], &vec![0u8; MODELS[0].bytes as usize]).is_err());
    }
}
