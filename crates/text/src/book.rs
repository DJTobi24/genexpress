//! The font book: the faces text can be set in, family matching and style selection.

use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use skrifa::instance::{LocationRef, Size};
use skrifa::raw::FileRef;
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider, Tag};

use crate::TextError;
use crate::shape::ShapeCache;

/// Largest font file accepted, in bytes.
pub const MAX_FONT_FILE: usize = 50 * 1024 * 1024;
/// Most font files [`FontBook::system`] reads.
pub const MAX_FONT_FILES: usize = 3000;
/// Most faces one book holds.
pub const MAX_FACES: usize = 20_000;
/// Most faces read from one (possibly damaged) font collection.
const MAX_COLLECTION_FACES: u32 = 256;
/// Most character mappings recorded per face (coverage for fallback).
const MAX_MAPPINGS: usize = 200_000;

/// Generic "Sans" families, best first.
pub const SANS_FALLBACKS: &[&str] = &[
    "Inter",
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
    "Arial",
    "Helvetica",
    "Helvetica Neue",
    "Segoe UI",
    "FreeSans",
    "Roboto",
    "Open Sans",
    "Carlito",
    "Ubuntu",
    "Cantarell",
    "Verdana",
];
/// Generic "Serif" families, best first.
pub const SERIF_FALLBACKS: &[&str] =
    &["Noto Serif", "DejaVu Serif", "Liberation Serif", "Times New Roman", "Times", "Georgia", "FreeSerif", "Caladea", "Cambria"];
/// Generic "Mono" families, best first.
pub const MONO_FALLBACKS: &[&str] =
    &["DejaVu Sans Mono", "Noto Sans Mono", "Liberation Mono", "Consolas", "Menlo", "Courier New", "FreeMono", "Cascadia Mono", "Monaco"];

/// Where a font file's bytes come from.
enum Source {
    /// Added with [`FontBook::add_font`].
    Memory(Arc<[u8]>),
    /// A file found by [`FontBook::system`], read on first use (scans read hundreds of files, so
    /// their bytes are not kept around until text is set in them).
    #[cfg(not(target_arch = "wasm32"))]
    File { path: PathBuf, cache: OnceLock<Option<Arc<[u8]>>> },
}

impl Source {
    fn data(&self) -> Option<Arc<[u8]>> {
        match self {
            Source::Memory(d) => Some(d.clone()),
            #[cfg(not(target_arch = "wasm32"))]
            Source::File { path, cache } => cache
                .get_or_init(|| {
                    let len = std::fs::metadata(path).ok()?.len();
                    if usize::try_from(len).map_or(true, |l| l > MAX_FONT_FILE) {
                        return None;
                    }
                    std::fs::read(path).ok().map(Arc::from)
                })
                .clone(),
        }
    }
}

/// One face (a font in a file or collection) with what matching and layout need to know about it
/// without opening it.
pub(crate) struct Face {
    source: Arc<Source>,
    pub(crate) index: u32,
    /// The family shown to users (typographic family, else the legacy family).
    pub(crate) family: String,
    /// Lower-cased names the face answers to.
    names: Vec<String>,
    pub(crate) weight: f32,
    pub(crate) italic: bool,
    pub(crate) stretch: f32,
    /// Sorted, merged ranges of the characters the face maps.
    coverage: Vec<(u32, u32)>,
    /// Ascent above the baseline, in ems.
    pub(crate) ascent: f64,
    /// Descent below the baseline (positive), in ems.
    pub(crate) descent: f64,
    /// Underline position below the baseline (positive), in ems.
    pub(crate) underline_offset: f64,
    /// Underline thickness, in ems.
    pub(crate) underline_thickness: f64,
    /// Strikethrough position above the baseline, in ems.
    pub(crate) strikeout_offset: f64,
    /// Strikethrough thickness, in ems.
    pub(crate) strikeout_thickness: f64,
    /// Font units per em (positive).
    pub(crate) upem: f64,
    /// Has OpenType layout tables (GSUB or GPOS).
    pub(crate) layout: bool,
    /// Shaping state and cached results.
    pub(crate) shaping: ShapeCache,
}

impl Face {
    pub(crate) fn data(&self) -> Option<Arc<[u8]>> {
        self.source.data()
    }

    pub(crate) fn covers(&self, c: char) -> bool {
        let c = u32::from(c);
        let i = self.coverage.partition_point(|r| r.1 < c);
        self.coverage.get(i).is_some_and(|r| r.0 <= c)
    }

    fn answers_to(&self, lower: &str) -> bool {
        self.names.iter().any(|n| n == lower)
    }
}

/// The fonts text can be set in. Cheap to clone (faces are shared); `Send + Sync`.
#[derive(Clone, Default)]
pub struct FontBook {
    faces: Vec<Arc<Face>>,
    /// Scales found by [`crate::fit_scale`], shared by clones; a new one when faces are added.
    pub(crate) fits: Arc<crate::fit::FitCache>,
}

impl std::fmt::Debug for FontBook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FontBook({} faces)", self.faces.len())
    }
}

impl FontBook {
    /// An empty book: text still lays out (with approximate metrics) but draws nothing.
    pub fn new() -> Self {
        FontBook::default()
    }

    /// The fonts installed on this computer (empty on the web). Font files are read once to learn
    /// their names and coverage, then again when text is first set in them.
    pub fn system() -> Self {
        #[cfg(target_arch = "wasm32")]
        let book = FontBook::new();
        #[cfg(not(target_arch = "wasm32"))]
        let mut book = FontBook::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut files = vec![];
            for dir in system_font_dirs() {
                collect_font_files(&dir, 0, &mut files);
                if files.len() >= MAX_FONT_FILES {
                    break;
                }
            }
            files.truncate(MAX_FONT_FILES);
            for path in files {
                book.add_file(path);
            }
        }
        book
    }

    /// Adds the faces of a font file or collection (TrueType, OpenType, TTC) held in memory;
    /// returns how many were added.
    pub fn add_font(&mut self, data: Vec<u8>) -> Result<usize, TextError> {
        if data.len() > MAX_FONT_FILE {
            return Err(TextError::TooLarge(data.len()));
        }
        let data: Arc<[u8]> = Arc::from(data);
        let source = Arc::new(Source::Memory(data.clone()));
        let faces = parse_faces(&data, &source)?;
        if faces.is_empty() {
            return Err(TextError::NoFaces);
        }
        if self.faces.len().saturating_add(faces.len()) > MAX_FACES {
            return Err(TextError::Full);
        }
        let n = faces.len();
        self.faces.extend(faces.into_iter().map(Arc::new));
        self.fits = Arc::default();
        Ok(n)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn add_file(&mut self, path: PathBuf) {
        let Ok(meta) = std::fs::metadata(&path) else { return };
        if usize::try_from(meta.len()).map_or(true, |l| l > MAX_FONT_FILE) {
            return;
        }
        let Ok(data) = std::fs::read(&path) else { return };
        let source = Arc::new(Source::File { path, cache: OnceLock::new() });
        if let Ok(faces) = parse_faces(&data, &source) {
            let room = MAX_FACES.saturating_sub(self.faces.len());
            self.faces.extend(faces.into_iter().take(room).map(Arc::new));
            self.fits = Arc::default();
        }
    }

    /// The family names in the book, sorted and unique.
    pub fn families(&self) -> Vec<String> {
        let mut v: Vec<String> = self.faces.iter().map(|f| f.family.clone()).collect();
        v.sort_by_key(|a| a.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Number of faces.
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Whether `family` is in the book (case-insensitive), as opposed to being matched by a fallback.
    pub fn has_family(&self, family: &str) -> bool {
        let lower = family.trim().to_lowercase();
        self.faces.iter().any(|f| f.answers_to(&lower))
    }

    pub(crate) fn face(&self, i: usize) -> Option<&Face> {
        self.faces.get(i).map(|f| &**f)
    }

    /// The font file holding face `face` (as in [`PlacedGlyph::face`](crate::PlacedGlyph::face))
    /// and the face's index in it (non-zero only in collections), e.g. to embed the font in an
    /// export. None for an unknown face or a file that can no longer be read.
    pub fn face_data(&self, face: usize) -> Option<(Arc<[u8]>, u32)> {
        let f = self.face(face)?;
        Some((f.data()?, f.index))
    }

    /// The family name of face `face`.
    pub fn face_family(&self, face: usize) -> Option<&str> {
        self.face(face).map(|f| f.family.as_str())
    }

    /// The family the book sets `family` in (after generic and fallback matching), if any.
    pub fn resolve_family(&self, family: &str, bold: bool, italic: bool) -> Option<String> {
        self.select(family, bold, italic).and_then(|i| self.face(i)).map(|f| f.family.clone())
    }

    /// What a synthetic style must add to the face text in `family` is set in: the weight it
    /// lacks (bold, or the heavy "Display" generic, asked for but the face is lighter than
    /// semibold; 0 otherwise) and whether it must be slanted (italic asked for, the face is
    /// upright). Drawing applies it (`genha_render::synthetic`).
    pub fn needs_synthetic(&self, family: &str, bold: bool, italic: bool) -> (f32, bool) {
        let Some(f) = self.select(family, bold, italic).and_then(|i| self.face(i)) else { return (0.0, false) };
        let wanted = match (family.trim().eq_ignore_ascii_case("display"), bold) {
            (true, true) => 900.0,
            (true, false) => 800.0,
            (false, true) => 700.0,
            (false, false) => 0.0,
        };
        let lacks = if wanted > 0.0 && f.weight < 600.0 { wanted - f.weight } else { 0.0 };
        (lacks, italic && !f.italic)
    }

    /// The best face of the family named `lower` (lower case) for the style.
    fn best_in_family(&self, lower: &str, weight: f32, italic: bool) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, f) in self.faces.iter().enumerate() {
            if !f.answers_to(lower) {
                continue;
            }
            let s = style_distance(f, weight, italic);
            if best.is_none_or(|(bs, _)| s < bs) {
                best = Some((s, i));
            }
        }
        best.map(|(_, i)| i)
    }

    /// The face text in `family` is set in: the family itself, else the generic families' (Sans,
    /// Serif, Mono, Display) lists, else the first available sans, else any face.
    pub(crate) fn select(&self, family: &str, bold: bool, italic: bool) -> Option<usize> {
        if self.faces.is_empty() {
            return None;
        }
        let lower = family.trim().to_lowercase();
        let mut weight = if bold { 700.0 } else { 400.0 };
        if let Some(i) = self.best_in_family(&lower, weight, italic) {
            return Some(i);
        }
        let list: &[&str] = match lower.as_str() {
            "serif" => SERIF_FALLBACKS,
            "mono" | "monospace" => MONO_FALLBACKS,
            "display" => {
                weight = if bold { 900.0 } else { 800.0 };
                SANS_FALLBACKS
            }
            _ => SANS_FALLBACKS,
        };
        for fam in list.iter().chain(SANS_FALLBACKS) {
            if let Some(i) = self.best_in_family(&fam.to_lowercase(), weight, italic) {
                return Some(i);
            }
        }
        // Anything, closest in style.
        let mut best: Option<(f32, usize)> = None;
        for (i, f) in self.faces.iter().enumerate() {
            let s = style_distance(f, weight, italic);
            if best.is_none_or(|(bs, _)| s < bs) {
                best = Some((s, i));
            }
        }
        best.map(|(_, i)| i)
    }

    /// Whether face `face` maps character `c`.
    pub fn face_covers(&self, face: usize, c: char) -> bool {
        self.face(face).is_some_and(|f| f.covers(c))
    }

    /// Faces to look in for characters a regular, upright face lacks, best first (as text layout
    /// falls back): e.g. to give another renderer, such as the UI's, fonts for more scripts.
    pub fn fallback_faces(&self) -> Vec<usize> {
        self.fallback_order(false, false)
    }

    /// Faces to look in for characters the chosen face lacks, best first: the generic families in
    /// this style, then every other face, those with OpenType layout tables (which shape complex
    /// scripts) before those without.
    pub(crate) fn fallback_order(&self, bold: bool, italic: bool) -> Vec<usize> {
        let weight = if bold { 700.0 } else { 400.0 };
        let mut order: Vec<usize> = vec![];
        for fam in SANS_FALLBACKS.iter().chain(SERIF_FALLBACKS).chain(MONO_FALLBACKS) {
            if let Some(i) = self.best_in_family(&fam.to_lowercase(), weight, italic)
                && !order.contains(&i)
            {
                order.push(i);
            }
        }
        let mut rest: Vec<(bool, f32, usize)> = (0..self.faces.len())
            .filter(|i| !order.contains(i))
            .filter_map(|i| self.face(i).map(|f| (!f.layout, style_distance(f, weight, italic), i)))
            .collect();
        rest.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
        order.extend(rest.into_iter().map(|(_, _, i)| i));
        order
    }
}

/// How far a face is from the wanted style (lower is closer).
fn style_distance(f: &Face, weight: f32, italic: bool) -> f32 {
    let w = (f.weight - weight).abs();
    // Lighter than asked for bold, or heavier than asked for regular, is a little worse.
    let w = if (weight >= 600.0 && f.weight < weight) || (weight < 600.0 && f.weight > weight) { w * 1.2 } else { w };
    let slant = if f.italic != italic { 1000.0 } else { 0.0 };
    w + slant + (f.stretch - 1.0).abs() * 2000.0
}

/// Every face in `data` (a font or a collection).
fn parse_faces(data: &[u8], source: &Arc<Source>) -> Result<Vec<Face>, TextError> {
    let count = match FileRef::new(data) {
        Ok(FileRef::Font(_)) => 1,
        Ok(FileRef::Collection(c)) => c.len().min(MAX_COLLECTION_FACES),
        Err(e) => return Err(TextError::Parse(e.to_string())),
    };
    Ok((0..count).filter_map(|i| FontRef::from_index(data, i).ok().and_then(|f| face_info(&f, i, source))).collect())
}

fn name(font: &FontRef<'_>, id: StringId) -> Option<String> {
    font.localized_strings(id).english_or_first().map(|s| s.to_string().trim().to_string()).filter(|s| !s.is_empty())
}

fn face_info(font: &FontRef<'_>, index: u32, source: &Arc<Source>) -> Option<Face> {
    // Only faces with outlines the engine draws (not bitmap-only fonts).
    let outlines = [b"glyf", b"CFF ", b"CFF2"].iter().any(|t| font.table_data(Tag::new(t)).is_some());
    if !outlines {
        return None;
    }
    let typo = name(font, StringId::TYPOGRAPHIC_FAMILY_NAME);
    let legacy = name(font, StringId::FAMILY_NAME);
    let family = typo.clone().or_else(|| legacy.clone())?;
    let mut names: Vec<String> = [typo, legacy].into_iter().flatten().map(|n| n.to_lowercase()).collect();
    names.dedup();
    let attrs = font.attributes();
    let italic = !matches!(attrs.style, skrifa::attribute::Style::Normal);
    let weight = attrs.weight.value();
    let stretch = attrs.stretch.ratio();
    let m = font.metrics(Size::unscaled(), LocationRef::default());
    let upem = f64::from(m.units_per_em);
    if upem <= 0.0 {
        return None;
    }
    let em = |v: f32| {
        let v = f64::from(v) / upem;
        if v.is_finite() { v } else { 0.0 }
    };
    let (mut ascent, mut descent) = (em(m.ascent), -em(m.descent));
    if ascent <= 0.0 || ascent > 4.0 || !(-1.0..=4.0).contains(&descent) {
        (ascent, descent) = (0.8, 0.2);
    }
    let (underline_offset, underline_thickness) = match m.underline {
        Some(d) if d.thickness > 0.0 => (-em(d.offset), em(d.thickness)),
        _ => (0.1, 0.05),
    };
    let (strikeout_offset, strikeout_thickness) = match m.strikeout {
        Some(d) if d.thickness > 0.0 && d.offset > 0.0 => (em(d.offset), em(d.thickness)),
        _ => (0.3, underline_thickness),
    };
    let mut cps: Vec<u32> = font.charmap().mappings().take(MAX_MAPPINGS).map(|(c, _)| c).collect();
    cps.sort_unstable();
    let mut coverage: Vec<(u32, u32)> = vec![];
    for c in cps {
        match coverage.last_mut() {
            Some(r) if c <= r.1.saturating_add(1) => r.1 = r.1.max(c),
            _ => coverage.push((c, c)),
        }
    }
    Some(Face {
        source: source.clone(),
        index,
        family,
        names,
        weight: if weight.is_finite() { weight } else { 400.0 },
        italic,
        stretch: if stretch.is_finite() { stretch } else { 1.0 },
        coverage,
        ascent,
        descent,
        underline_offset: underline_offset.clamp(-1.0, 1.0),
        underline_thickness: underline_thickness.clamp(0.0, 0.5),
        strikeout_offset: strikeout_offset.clamp(0.0, 1.0),
        strikeout_thickness: strikeout_thickness.clamp(0.0, 0.5),
        upem,
        layout: [b"GSUB", b"GPOS"].iter().any(|t| font.table_data(Tag::new(t)).is_some()),
        shaping: ShapeCache::default(),
    })
}

/// The folders fonts are installed in on this operating system.
#[cfg(not(target_arch = "wasm32"))]
fn system_font_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = vec![];
    if cfg!(target_os = "windows") {
        let win =
            std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        dirs.push(win.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft").join("Windows").join("Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Some(h) = &home {
            dirs.push(h.join("Library").join("Fonts"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(h) = &home {
            dirs.push(h.join(".local").join("share").join("fonts"));
            dirs.push(h.join(".fonts"));
        }
    }
    dirs
}

/// Font files under `dir` (sorted, recursing a few levels, not following linked folders).
#[cfg(not(target_arch = "wasm32"))]
fn collect_font_files(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 8 || out.len() >= MAX_FONT_FILES {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<(PathBuf, bool)> = rd.filter_map(|e| e.ok()).filter_map(|e| Some((e.path(), e.file_type().ok()?.is_dir()))).collect();
    entries.sort();
    for (path, is_dir) in entries {
        if out.len() >= MAX_FONT_FILES {
            return;
        }
        if is_dir {
            collect_font_files(&path, depth + 1, out);
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
            if matches!(ext.as_deref(), Some("ttf" | "otf" | "ttc" | "otc")) {
                out.push(path);
            }
        }
    }
}
