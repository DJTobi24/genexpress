# Gaps — the ranked work list

> **Last updated:** 2026-10-13

Every known shortfall, most valuable first. Pick work from the top unless the user asks for something else.

| # | Gap | Area | Notes |
|---|---|---|---|
| 1 | Web build: control channel over a WebSocket; WOFF/WOFF2 font files; several tabs share one storage (each tab offers the others' live copies as recovered); a picture alone on the desktop clipboard needs Edit › Paste Picture (egui-winit reports Cmd+V only with text) | Platform | M2.17: fonts added in the browser (kept in IndexedDB) and installed fonts (Local Font Access, Chromium), synthetic bold/italic, Data Recovery and recent designs (with thumbnails) in IndexedDB, picture paste, `web` profile (download 11.8 → 8.3 MB, 6.7 MB with wasm-opt) |
| 2 | Curved text edits straight, not on its arc; caret moves in logical order inside right-to-left runs; undo inside an edit (Ctrl+Z while editing) restores text but not formatting (Undo after editing does); rich paste (formatting from the clipboard) | Editor | text is edited in place on the canvas (M2.16); part of a text box can be formatted (M3.2: font, size, bold, italic, underline, strikethrough, colour per character range, Ctrl/Cmd+B/I/U, mixed values in the panel) |
| 3 | Colour emoji (COLR/bitmap fonts are not drawn), justified text, OpenType feature and language settings, hyphenation, vertical text | Text | shaping landed (M3.1): HarfRust kerning, ligatures, marks, Arabic/Hebrew/Indic, bidi reordering, UAX #14 line breaks, per-cluster font fallback |
| 4 | Print PDF with a colour profile (ICC output intent, PDF/X); blurred shadows as vectors | Export | vector PDF landed (M4.1, krilla): real text with embedded font subsets, paths, gradients, pictures at full resolution, print boxes; `cmyk: true` converts colours without a profile; blurred shadows are pictures (300 dpi); synthetic bold/italic in vector PDF text landed with M3.2 |
| 5 | Document-wide text styles; grid text | Text | curved text landed (`text.setCurve`) |
| 6 | Smart resize: reflow text boxes to the new width, re-layout grids | Layout | stacked elements now move as blocks; boxes keep their scaled widths |
| 7 | Background removal for busy photos (ML model, optional download) | Pictures | colour-based flood fill today |
| 8 | Rotated objects do not snap while resizing (their distances still show); no ruler guides placed by hand; no rotation snapping without Shift | Editor | M2.19: smart guides while resizing (edges, centre, same width/height, equal gaps; Shift, Alt, several objects), distance and equal-spacing hints, Alt-hover distances, Ctrl/Cmd to skip snapping, `view.snapping` |
| 9 | Picking table cells on the canvas (the panel's grid picks ranges today); formulas in tables; scatter/combo charts and a secondary axis; cell borders per cell | Documents | M5.1: several series per chart (grouped/stacked bars, lines, stacked areas), axis titles, gridlines, legend position, number/currency formats, min/max; tables with merged cells, per-cell styles, column widths and row heights, outer/inner lines |
| 10 | Version history across devices (sync) | Editor | local version history, recent files and Data Recovery landed |
| 11 | Timeline with scenes, audio; MP4 at 4K and smaller files (sub-sample motion, CABAC); MP4 export off the main thread in the browser | Video | MP4 export landed (M6.1: our own pure-Rust H.264 encoder, `crates/video`, desktop and web); animations, playback, present mode and GIF/animated PNG export landed (`crates/motion`) |
| 12 | PDF import; text inside imported SVGs | Formats | SVG shapes, gradients and borders import (`genha_svg::import`) |
| 13 | Screen reader support (AccessKit) and UI translations (German first); UI widgets (property fields, layer names) use egui's own font, so Arabic/CJK text shows as boxes there (the canvas is fine) | Accessibility | |
| 14 | Template library growth (target 100+ original templates) | Content | 65 templates with search tags, 114 icons, 25 text styles, 40 palettes (M2.3); still short of the target library |
| 15 | Rendering off the UI thread, tiles for very large pages | Performance | |
