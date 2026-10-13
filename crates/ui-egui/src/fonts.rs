//! Fallback fonts for when the system has none (always the case in the browser): egui's built-in
//! faces, which ship inside the `epaint_default_fonts` crate, so this repository holds no font
//! files. Licences: Ubuntu Font Licence 1.0 (Ubuntu Light), MIT + Bitstream Vera (Hack), OFL-1.1
//! (Noto Emoji); see NOTICE.

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

/// "Add fonts…" (font files, kept for next time) and, where the browser can share them,
/// "Use installed fonts", for the home screen and the editor's Text panel.
pub fn add_fonts_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("Add fonts…").on_hover_text("TrueType or OpenType files (.ttf, .otf, .ttc); kept for next time").clicked() {
            add_fonts(app);
        }
        #[cfg(target_arch = "wasm32")]
        if crate::web::local_fonts_supported()
            && ui
                .button("Use installed fonts")
                .on_hover_text("Lets GenHa use the fonts installed on this computer (the browser asks first)")
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
                Err(format!("{name} is too large or can't be read"))
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
            app.set_status(format!("Added {}", added.join(", ")));
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
