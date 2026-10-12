//! Fallback fonts for when the system has none (always the case in the browser): egui's built-in
//! faces, which ship inside the `epaint_default_fonts` crate, so this repository holds no font
//! files. Licences: Ubuntu Font Licence 1.0 (Ubuntu Light), MIT + Bitstream Vera (Hack), OFL-1.1
//! (Noto Emoji); see NOTICE.

use genha_text::FontBook;

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
