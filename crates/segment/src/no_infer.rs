//! Builds without the model runtime (the `inference` feature off, e.g. the default browser
//! build): the same API, and every attempt to load a model says this build cannot run it.

use image::{GrayImage, RgbaImage};

use crate::{ModelSpec, Result, SegmentError, verify};

/// A model that can never be loaded in this build.
#[derive(Clone, Debug)]
pub struct Segmenter {
    pub spec: &'static ModelSpec,
}

impl Segmenter {
    /// Checks the file (so a damaged download is still reported as such), then refuses.
    pub fn load(spec: &'static ModelSpec, bytes: &[u8]) -> Result<Segmenter> {
        verify(spec, bytes)?;
        Err(SegmentError::Unavailable)
    }

    pub fn raw(&self, _: &RgbaImage) -> Result<Vec<f32>> {
        Err(SegmentError::Unavailable)
    }

    pub fn mask(&self, _: &RgbaImage) -> Result<GrayImage> {
        Err(SegmentError::Unavailable)
    }
}

/// `None` when the model is not installed, else the refusal.
pub fn cached(spec: &'static ModelSpec, read: impl FnOnce() -> Option<Vec<u8>>) -> Result<Option<Segmenter>> {
    match read() {
        None => Ok(None),
        Some(bytes) => Segmenter::load(spec, &bytes).map(Some),
    }
}

/// Nothing is ever cached.
pub fn peek(_: &ModelSpec) -> Option<Segmenter> {
    None
}

pub fn forget_cached() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_to_run() {
        let spec = &crate::MODELS[0];
        assert!(cached(spec, || None).unwrap().is_none());
        assert!(matches!(cached(spec, || Some(b"x".to_vec())), Err(SegmentError::Size { .. })));
        assert!(peek(spec).is_none());
        assert!(Segmenter { spec }.mask(&RgbaImage::new(2, 2)).is_err());
    }
}
