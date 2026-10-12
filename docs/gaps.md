# Gaps — the ranked work list

> **Last updated:** 2026-10-12

Every known shortfall, most valuable first. Pick work from the top unless the user asks for something else.

| # | Gap | Area | Notes |
|---|---|---|---|
| 1 | Web build of the app (WASM, eframe web, downloads instead of file dialogs) | Platform | engine and below already build for wasm32 |
| 2 | Edit text directly on the canvas (caret, selection) instead of the floating editor | Editor | |
| 3 | Kerning, ligatures and complex scripts (HarfRust shaping); per-glyph font fallback | Text | `crates/text` uses charmap + advances only |
| 4 | Vector PDF with selectable text (krilla), print PDF with bleed and crop marks | Export | PDF pages are JPEG images today |
| 5 | Curved text and document-wide text styles | Text | |
| 6 | Smart resize keeps multi-part text blocks together | Layout | entries can spread on big aspect changes |
| 7 | Background removal for busy photos (ML model, optional download) | Pictures | colour-based flood fill today |
| 8 | Crop page to content; multi-select resize handles | Editor | |
| 9 | Charts and tables | Documents | |
| 10 | Version history (local snapshots), recent files, autosave/recovery | Editor | |
| 11 | MP4 export, timeline with scenes, audio | Video | animations, playback, present mode and GIF/animated PNG export landed (`crates/motion`); MP4 needs a pure-Rust encoder |
| 12 | SVG and PDF import | Formats | |
| 13 | Screen reader support (AccessKit) and UI translations (German first) | Accessibility | |
| 14 | Template library growth (target 100+ original templates) and template search tags | Content | |
| 15 | Rendering off the UI thread, tiles for very large pages | Performance | |
