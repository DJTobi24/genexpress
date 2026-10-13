//! Fallback fonts for when the system has none (always the case in the browser): egui's built-in
//! faces, which ship inside the `epaint_default_fonts` crate, so this repository holds no font
//! files. Licences: Ubuntu Font Licence 1.0 (Ubuntu Light), MIT + Bitstream Vera (Hack), OFL-1.1
//! (Noto Emoji); see NOTICE.

use genha_i18n::tr;
use genha_text::FontBook;

use crate::App;

/// Font files the user can add (what the font parser reads; WOFF and WOFF2 are not supported).
pub const FONT_FILES: (&str, &[&str]) = ("Fonts", &["ttf", "otf", "ttc", "otc"]);

/// Adds the fallback faces (Ubuntu Light, Hack, Noto Emoji) to `book`; returns how many were added.
pub fn add_fallback_fonts(book: &mut FontBook) -> usize {
    [epaint_default_fonts::UBUNTU_LIGHT, epaint_default_fonts::HACK_REGULAR, epaint_default_fonts::NOTO_EMOJI_REGULAR]
        .iter()
        // A face that doesn't load is simply not offered; the others still are.
        .filter_map(|data| book.add_font(data.to_vec()).ok())
        .sum()
}

/// The fonts installed on this computer, or the fallback faces when there are none (the browser,
/// a system without fonts), so text is never invisible.
pub fn system_or_fallback() -> FontBook {
    let mut book = FontBook::system();
    if book.is_empty() {
        let n = add_fallback_fonts(&mut book);
        log::info!("no system fonts: using {n} built-in fallback faces");
    }
    book
}

/// Scripts egui's own faces (Ubuntu Light, Hack, Noto Emoji: Latin, Greek, Cyrillic, emoji) lack,
/// each with a character a face must have to show it.
pub const UI_SCRIPTS: &[(&str, char)] = &[
    ("Arabic", 'ب'),
    ("Hebrew", 'א'),
    ("Devanagari", 'क'),
    ("Bengali", 'ক'),
    ("Gurmukhi", 'ਕ'),
    ("Gujarati", 'ક'),
    ("Tamil", 'த'),
    ("Telugu", 'క'),
    ("Kannada", 'ಕ'),
    ("Malayalam", 'ക'),
    ("Sinhala", 'ක'),
    ("Thai", 'ก'),
    ("Lao", 'ກ'),
    ("Khmer", 'ក'),
    ("Myanmar", 'က'),
    ("Tibetan", 'ཀ'),
    ("Georgian", 'ა'),
    ("Armenian", 'Ա'),
    ("Ethiopic", 'ሀ'),
    ("Han", '中'),
    ("Kana", 'あ'),
    ("Hangul", '한'),
];

/// Most fallback faces the UI loads, and most bytes (CJK fonts are large).
const MAX_UI_FACES: usize = 10;
const MAX_UI_FONT_BYTES: usize = 120 * 1024 * 1024;

/// Faces drawn as pixel blocks (or placeholders), used only when nothing else has the script.
const LAST_RESORT: &[&str] = &["unifont", "last resort"];

/// A system face egui falls back to.
pub struct UiFace {
    /// Unique name in egui's font definitions.
    pub name: String,
    pub family: String,
    pub data: Vec<u8>,
    pub index: u32,
    /// The [`UI_SCRIPTS`] it was chosen for.
    pub scripts: Vec<&'static str>,
}

/// The faces in `book` that give the UI the scripts its own faces lack: per script the face text
/// layout would fall back to (avoiding pixel-style last-resort faces), at most [`MAX_UI_FACES`].
pub fn ui_fallback_faces(book: &FontBook) -> Vec<UiFace> {
    let order = book.fallback_faces();
    let last_resort = |i: usize| book.face_family(i).is_some_and(|f| LAST_RESORT.iter().any(|l| f.to_lowercase().starts_with(l)));
    let mut picked: Vec<(usize, Vec<&'static str>)> = vec![];
    for (script, c) in UI_SCRIPTS {
        let mut covering = order.iter().copied().filter(|&i| book.face_covers(i, *c));
        let first = covering.next();
        let best = first.filter(|&i| !last_resort(i)).or_else(|| covering.find(|&i| !last_resort(i))).or(first);
        let Some(face) = best else { continue };
        let room = picked.len() < MAX_UI_FACES;
        match picked.iter_mut().find(|(f, _)| *f == face) {
            Some((_, scripts)) => scripts.push(script),
            None if room => picked.push((face, vec![script])),
            None => {}
        }
    }
    let mut total = 0usize;
    let mut out = vec![];
    for (face, scripts) in picked {
        let Some((data, index)) = book.face_data(face) else { continue };
        if total.saturating_add(data.len()) > MAX_UI_FONT_BYTES {
            continue;
        }
        total += data.len();
        let family = book.face_family(face).unwrap_or("").to_string();
        out.push(UiFace { name: format!("genha-ui-fallback-{face}-{family}"), family, data: data.to_vec(), index, scripts });
    }
    out
}

/// egui's font definitions: its own faces, then `faces` as fallbacks in every family.
pub fn ui_font_definitions(faces: &[UiFace]) -> egui::FontDefinitions {
    let mut defs = egui::FontDefinitions::default();
    for f in faces {
        let data = egui::FontData { font: std::borrow::Cow::Owned(f.data.clone()), index: f.index, tweak: Default::default() };
        defs.font_data.insert(f.name.clone(), std::sync::Arc::new(data));
        for list in defs.families.values_mut() {
            list.push(f.name.clone());
        }
    }
    defs
}

/// The UI's fallback fonts: which are installed, and the search for them (a worker thread on the
/// desktop, so a slow disk never delays the first frame).
#[derive(Default)]
pub struct UiFonts {
    /// The faces should be looked for again (at start, after fonts were added).
    stale: bool,
    searched: bool,
    /// Names of the faces egui has now.
    pub installed: Vec<String>,
    /// Scripts the installed faces cover.
    pub scripts: Vec<&'static str>,
    #[cfg(not(target_arch = "wasm32"))]
    pending: Option<std::sync::mpsc::Receiver<(Vec<UiFace>, f64)>>,
}

impl UiFonts {
    /// Look for faces again (fonts were added).
    pub fn refresh(&mut self) {
        self.stale = true;
    }
}

/// Starts or finishes the search for the UI's fallback fonts and hands new ones to egui.
pub fn update_ui_fonts(app: &mut crate::App, ctx: &egui::Context) {
    let st = &mut app.ui_fonts;
    if !st.searched || st.stale {
        st.searched = true;
        st.stale = false;
        let book = app.session.fonts.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let repaint = ctx.clone();
            let spawned = std::thread::Builder::new().name("ui fonts".into()).spawn(move || {
                let start = std::time::Instant::now();
                let faces = ui_fallback_faces(&book);
                // The app may have gone away meanwhile: nothing waits for the faces then.
                tx.send((faces, start.elapsed().as_secs_f64() * 1000.0)).ok();
                repaint.request_repaint();
            });
            match spawned {
                Ok(_) => st.pending = Some(rx),
                Err(e) => log::warn!("UI fonts: {e}"),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let start = crate::now();
            let faces = ui_fallback_faces(&book);
            install_ui_fonts(st, ctx, faces, (crate::now() - start) * 1000.0);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(rx) = &st.pending {
        match rx.try_recv() {
            Ok((faces, ms)) => {
                st.pending = None;
                install_ui_fonts(st, ctx, faces, ms);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => st.pending = None,
        }
    }
}

fn install_ui_fonts(st: &mut UiFonts, ctx: &egui::Context, faces: Vec<UiFace>, ms: f64) {
    let names: Vec<String> = faces.iter().map(|f| f.name.clone()).collect();
    if names == st.installed {
        return;
    }
    let bytes: usize = faces.iter().map(|f| f.data.len()).sum();
    log::info!(
        "UI fallback fonts ({ms:.0} ms, {:.1} MB): {}",
        bytes as f64 / 1_048_576.0,
        faces.iter().map(|f| format!("{} ({})", f.family, f.scripts.join(", "))).collect::<Vec<_>>().join("; ")
    );
    ctx.set_fonts(ui_font_definitions(&faces));
    st.installed = names;
    st.scripts = faces.iter().flat_map(|f| f.scripts.iter().copied()).collect();
}

/// "Add fonts…" (font files, kept for next time) and, where the browser can share them,
/// "Use installed fonts", for the home screen and the editor's Text panel.
pub fn add_fonts_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Add fonts…")).on_hover_text(tr("TrueType or OpenType files (.ttf, .otf, .ttc); kept for next time")).clicked() {
            add_fonts(app);
        }
        #[cfg(target_arch = "wasm32")]
        if crate::web::local_fonts_supported()
            && ui
                .button(tr("Use installed fonts"))
                .on_hover_text(tr("Lets GenHa use the fonts installed on this computer (the browser asks first)"))
                .clicked()
        {
            crate::web::load_local_fonts(true);
        }
    });
}

/// Asks for font files and adds them (`font.add`): the desktop's file dialog, the browser's
/// file picker (the files arrive a frame later, `web::poll`).
pub fn add_fonts(app: &mut App) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = &app;
        crate::web::pick(crate::web::Purpose::Font, &[FONT_FILES]);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let files = rfd::FileDialog::new().add_filter(FONT_FILES.0, FONT_FILES.1).pick_files().unwrap_or_default();
        let mut added = vec![];
        for path in files {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "font.ttf".into());
            let too_large = std::fs::metadata(&path).map_or(true, |m| m.len() > genha_text::MAX_FONT_FILE as u64);
            let read = if too_large {
                Err(genha_i18n::tr!("{name} is too large or can't be read", name = name))
            } else {
                std::fs::read(&path).map_err(|e| format!("{name}: {e}"))
            };
            match read.and_then(|bytes| genha_engine::cmd::fonts::add_font_bytes(&mut app.session, &name, bytes, true).map_err(|e| e.to_string())) {
                Ok((_, families)) => added.extend(families),
                Err(e) => app.set_status(e),
            }
        }
        if !added.is_empty() {
            app.refresh_fonts();
            app.set_status(genha_i18n::tr!("Added {fonts}", fonts = added.join(", ")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_fonts_set_text() {
        let mut book = FontBook::new();
        assert_eq!(add_fallback_fonts(&mut book), 3);
        assert!(book.has_family("Ubuntu"));
        // Generic families (what templates use) land on the fallback sans.
        assert_eq!(book.resolve_family("Sans", true, false).as_deref(), Some("Ubuntu"));
        assert_eq!(book.resolve_family("Some Missing Font", false, false).as_deref(), Some("Ubuntu"));
    }
}
