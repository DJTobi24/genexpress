//! Shaping with HarfRust: a run of text in one face, direction and script becomes positioned glyphs
//! (substitutions such as ligatures and Arabic joining forms, kerning, mark positioning).
//!
//! Results are in font units, so they hold for every size, and are cached per face: text is shaped
//! in short segments (a word and its trailing spaces), which repeat a lot across a design and
//! between frames. Shape plans (the compiled feature lookups for a script and direction) are cached
//! per face as well.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use harfrust::{Direction, Feature, FontRef, Script, ShapeOptions, ShapePlan, ShaperData, Tag, UnicodeBuffer};

/// Longest segment (bytes) whose shaping is cached; longer ones are shaped every time.
const MAX_CACHED_SEGMENT: usize = 256;
/// Cached segments per face; the cache starts over when it is full.
const MAX_CACHED_SEGMENTS: usize = 8192;
/// Cached shape plans per face.
const MAX_PLANS: usize = 64;

/// One shaped glyph, in font units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawGlyph {
    pub(crate) gid: u32,
    /// Byte offset, in the shaped text, of the first character of the glyph's cluster.
    pub(crate) cluster: u32,
    pub(crate) advance: i32,
    pub(crate) x_offset: i32,
    /// Upwards.
    pub(crate) y_offset: i32,
}

/// What, besides the text, the result of shaping depends on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ShapeKey {
    pub(crate) rtl: bool,
    /// ISO 15924 tag of the script, if the text has one.
    pub(crate) script: Option<[u8; 4]>,
    /// Standard and contextual ligatures on (they are turned off for letter-spaced text).
    pub(crate) ligatures: bool,
}

impl ShapeKey {
    fn direction(&self) -> Direction {
        if self.rtl { Direction::RightToLeft } else { Direction::LeftToRight }
    }

    fn script(&self) -> Option<Script> {
        self.script.and_then(|t| Script::from_iso15924_tag(Tag::new(&t)))
    }
}

/// Per-face shaping state, shared by every clone of the book.
#[derive(Default)]
pub(crate) struct ShapeCache {
    data: OnceLock<ShaperData>,
    plans: Mutex<HashMap<ShapeKey, Arc<ShapePlan>>>,
    segments: Mutex<Segments>,
}

/// Shaped segments by key and text.
#[derive(Default)]
struct Segments {
    by_key: HashMap<ShapeKey, HashMap<Box<str>, Arc<[RawGlyph]>>>,
    count: usize,
}

/// Locks a cache. A poisoned lock (a panic elsewhere while it was held) only means the cache may be
/// incomplete, which is fine for a cache.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

const NO_LIGATURES: [Feature; 2] =
    [Feature { tag: Tag::new(b"liga"), value: 0, start: 0, end: u32::MAX }, Feature { tag: Tag::new(b"clig"), value: 0, start: 0, end: u32::MAX }];

impl ShapeCache {
    /// Shapes `text` set in `font` (the face this cache belongs to). Glyphs come in visual order
    /// (right to left text reversed), as HarfBuzz returns them.
    pub(crate) fn shape(&self, font: &FontRef<'_>, text: &str, key: ShapeKey) -> Arc<[RawGlyph]> {
        let cacheable = text.len() <= MAX_CACHED_SEGMENT;
        if cacheable && let Some(hit) = lock(&self.segments).by_key.get(&key).and_then(|m| m.get(text)) {
            return hit.clone();
        }
        let glyphs: Arc<[RawGlyph]> = self.shape_uncached(font, text, key).into();
        if cacheable {
            let mut segments = lock(&self.segments);
            if segments.count >= MAX_CACHED_SEGMENTS {
                *segments = Segments::default();
            }
            if segments.by_key.entry(key).or_default().insert(Box::from(text), glyphs.clone()).is_none() {
                segments.count += 1;
            }
        }
        glyphs
    }

    fn shape_uncached(&self, font: &FontRef<'_>, text: &str, key: ShapeKey) -> Vec<RawGlyph> {
        if text.is_empty() {
            return vec![];
        }
        let data = self.data.get_or_init(|| ShaperData::new(font));
        let shaper = data.shaper(font).build();
        let features: &[Feature] = if key.ligatures { &[] } else { &NO_LIGATURES };
        let (direction, script) = (key.direction(), key.script());
        let plan = {
            let mut plans = lock(&self.plans);
            match plans.get(&key) {
                Some(p) => p.clone(),
                None => {
                    let p = Arc::new(ShapePlan::new(&shaper, direction, script, None, features));
                    if plans.len() >= MAX_PLANS {
                        plans.clear();
                    }
                    plans.insert(key, p.clone());
                    p
                }
            }
        };
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        // The buffer must agree with the plan (HarfRust checks).
        buffer.set_direction(direction);
        if let Some(s) = script {
            buffer.set_script(s);
        }
        let out = shaper.shape(buffer, ShapeOptions::new().plan(Some(&plan)).features(features));
        out.glyph_infos()
            .iter()
            .zip(out.glyph_positions())
            .map(|(i, p)| RawGlyph { gid: i.glyph_id, cluster: i.cluster, advance: p.x_advance, x_offset: p.x_offset, y_offset: p.y_offset })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<ShapeCache>();
    }
}
