# Gaps — the ranked work list

> **Last updated:** 2026-10-13

Every known shortfall, most valuable first. Pick work from the top unless the user asks for something else.

| # | Gap | Area | Notes |
|---|---|---|---|
| 1 | Web build: control channel over a WebSocket; WOFF/WOFF2 font files; several tabs share one storage (each tab offers the others' live copies as recovered); a picture alone on the desktop clipboard needs Edit › Paste Picture (egui-winit reports Cmd+V only with text) | Platform | M2.17: fonts added in the browser (kept in IndexedDB) and installed fonts (Local Font Access, Chromium), synthetic bold/italic, Data Recovery and recent designs (with thumbnails) in IndexedDB, picture paste, `web` profile (download 11.8 → 8.3 MB, 6.7 MB with wasm-opt) |
| 2 | Text formatting for part of a text box (bold or colour one word); curved text edits straight, not on its arc; caret moves in logical order inside right-to-left runs | Editor | text is edited in place on the canvas (M2.16: caret, selection, clipboard, IME, one undo step); styles still apply to the whole box |
| 3 | Colour emoji (COLR/bitmap fonts are not drawn), justified text, OpenType feature and language settings, hyphenation, vertical text | Text | shaping landed (M3.1): HarfRust kerning, ligatures, marks, Arabic/Hebrew/Indic, bidi reordering, UAX #14 line breaks, per-cluster font fallback |
| 4 | Print PDF with a colour profile (ICC output intent, PDF/X); blurred shadows as vectors | Export | vector PDF landed (M4.1, krilla): real text with embedded font subsets, paths, gradients, pictures at full resolution, print boxes; `cmyk: true` converts colours without a profile; blurred shadows are pictures (300 dpi) |
| 5 | Document-wide text styles; grid text | Text | curved text landed (`text.setCurve`) |
| 6 | Smart resize: reflow text boxes to the new width, re-layout grids | Layout | stacked elements now move as blocks; boxes keep their scaled widths |
| 7 | Background removal for busy photos (ML model, optional download) | Pictures | colour-based flood fill today |
| 8 | Snapping while resizing; distance hints between objects | Editor | multi-select handles and crop page to content landed |
| 9 | Charts with several data series, axis titles; tables with merged cells and per-cell styles | Documents | single-series charts (7 kinds) and tables landed |
| 10 | Version history across devices (sync) | Editor | local version history, recent files and Data Recovery landed |
| 11 | MP4 export, timeline with scenes, audio | Video | animations, playback, present mode and GIF/animated PNG export landed (`crates/motion`); MP4 needs a pure-Rust encoder |
| 12 | PDF import; text inside imported SVGs | Formats | SVG shapes, gradients and borders import (`genha_svg::import`) |
| 13 | Screen reader support (AccessKit) and UI translations (German first) | Accessibility | |
| 14 | Template library growth (target 100+ original templates) | Content | 65 templates with search tags, 114 icons, 25 text styles, 40 palettes (M2.3); still short of the target library |
| 15 | Rendering off the UI thread, tiles for very large pages | Performance | |
