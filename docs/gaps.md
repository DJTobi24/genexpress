# Gaps — the ranked work list

> **Last updated:** 2026-10-12

Every known shortfall, most valuable first. Pick work from the top unless the user asks for something else.

| # | Gap | Area | Notes |
|---|---|---|---|
| 1 | Web build: installed/web fonts (only the built-in Ubuntu Light, one weight), Data Recovery and recent designs in browser storage, paste of pictures, control channel over a web socket | Platform | the web app runs (`apps/genha-web`, `cargo xtask web`): open, place, drop, quick actions, Save and Download as browser downloads |
| 2 | Edit text directly on the canvas (caret, selection) instead of the floating editor | Editor | |
| 3 | Kerning, ligatures and complex scripts (HarfRust shaping); per-glyph font fallback | Text | `crates/text` uses charmap + advances only |
| 4 | Vector PDF with selectable text (krilla), print PDF with bleed and crop marks | Export | PDF pages are JPEG images today |
| 5 | Curved text and document-wide text styles | Text | |
| 6 | Smart resize keeps multi-part text blocks together | Layout | entries can spread on big aspect changes |
| 7 | Background removal for busy photos (ML model, optional download) | Pictures | colour-based flood fill today |
| 8 | Multi-select resize handles | Editor | crop page to content landed (`page.cropToContent`) |
| 9 | Charts and tables | Documents | |
| 10 | Version history across devices (sync) | Editor | local version history, recent files and Data Recovery landed |
| 11 | Animation playback and GIF/MP4 export | Video | model already stores animations and page durations |
| 12 | PDF import; text inside imported SVGs | Formats | SVG shapes, gradients and borders import (`genha_svg::import`) |
| 13 | Screen reader support (AccessKit) and UI translations (German first) | Accessibility | |
| 14 | Template library growth (target 100+ original templates) and template search tags | Content | |
| 15 | Rendering off the UI thread, tiles for very large pages | Performance | |
