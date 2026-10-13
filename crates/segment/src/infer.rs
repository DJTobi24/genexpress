//! Running a model with tract.

use std::sync::{Arc, Mutex};

use image::{GrayImage, RgbaImage};
use tract_onnx::prelude::*;

use crate::{ModelSpec, Result, SegmentError, post, pre, verify};

type Plan = Arc<TypedRunnableModel>;

/// A loaded model, ready to make masks. Cheap to share: the plan is reference-counted and runs
/// with its own state on every call.
#[derive(Clone)]
pub struct Segmenter {
    pub spec: &'static ModelSpec,
    plan: Plan,
}

impl std::fmt::Debug for Segmenter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Segmenter").field("model", &self.spec.name).finish()
    }
}

impl Segmenter {
    /// Loads `spec`'s model from the file's `bytes`, after checking them ([`verify`]).
    pub fn load(spec: &'static ModelSpec, bytes: &[u8]) -> Result<Segmenter> {
        verify(spec, bytes)?;
        let fail = |e: TractError| SegmentError::Load { name: spec.name.into(), msg: format!("{e:#}").chars().take(400).collect() };
        let side = spec.input as usize;
        let plan = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(bytes))
            .map_err(fail)?
            .with_input_fact(0, f32::fact([1, 3, side, side]).into())
            .map_err(fail)?
            .into_optimized()
            .map_err(fail)?
            .into_runnable()
            .map_err(fail)?;
        Ok(Segmenter { spec, plan })
    }

    /// The model's raw answer for `img`: `input × input` values, higher where the subject is.
    pub fn raw(&self, img: &RgbaImage) -> Result<Vec<f32>> {
        let side = self.spec.input as usize;
        let data = pre::tensor(img, self.spec.input, self.spec.mean, self.spec.std);
        let run = |e: TractError| SegmentError::Run(format!("{e:#}").chars().take(400).collect());
        let input = Tensor::from_shape(&[1, 3, side, side], &data).map_err(run)?;
        let outputs = self.plan.run(tvec!(input.into())).map_err(run)?;
        // U²-Net returns its fused map first, then the side outputs.
        let first = outputs.first().ok_or_else(|| SegmentError::Run("the model returned nothing".into()))?;
        let view = first.to_plain_array_view::<f32>().map_err(run)?;
        let values: Vec<f32> = view.iter().copied().collect();
        if values.len() != side * side {
            return Err(SegmentError::Run(format!("unexpected output size {} (expected {})", values.len(), side * side)));
        }
        Ok(values)
    }

    /// The mask for `img`, the picture's size: 255 where the subject is, 0 for the background,
    /// soft at the edges ([`post::refine`]).
    pub fn mask(&self, img: &RgbaImage) -> Result<GrayImage> {
        if img.width() == 0 || img.height() == 0 {
            return Ok(GrayImage::new(img.width(), img.height()));
        }
        let raw = self.raw(img)?;
        Ok(post::refine(img, &raw, self.spec.input))
    }
}

/// The last model loaded, kept so the next picture does not load it again (loading and
/// optimising takes longer than running it).
static CACHE: Mutex<Option<Segmenter>> = Mutex::new(None);

/// `spec`'s model, from the cache or loaded from the bytes `read` gives (`None`: not installed).
pub fn cached(spec: &'static ModelSpec, read: impl FnOnce() -> Option<Vec<u8>>) -> Result<Option<Segmenter>> {
    // A poisoned lock only means an earlier load panicked; the slot is still usable.
    let mut slot = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(s) = slot.as_ref().filter(|s| s.spec.name == spec.name) {
        return Ok(Some(s.clone()));
    }
    let Some(bytes) = read() else { return Ok(None) };
    let s = Segmenter::load(spec, &bytes)?;
    *slot = Some(s.clone());
    Ok(Some(s))
}

/// `spec`'s model when it is the one in the cache.
pub fn peek(spec: &ModelSpec) -> Option<Segmenter> {
    let slot = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    slot.as_ref().filter(|s| s.spec.name == spec.name).cloned()
}

/// Drops the cached model (after it was deleted or replaced).
pub fn forget_cached() {
    let mut slot = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    *slot = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junk_never_reaches_the_runtime() {
        let spec = &crate::MODELS[0];
        let fake = vec![7u8; spec.bytes as usize];
        assert!(matches!(Segmenter::load(spec, &fake), Err(SegmentError::Checksum { .. })));
        assert!(matches!(Segmenter::load(spec, b"onnx?"), Err(SegmentError::Size { .. })));
        assert!(cached(spec, || None).unwrap().is_none());
        assert!(cached(spec, || Some(b"junk".to_vec())).is_err());
        forget_cached();
    }
}
