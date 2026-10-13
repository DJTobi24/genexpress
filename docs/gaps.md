# Gaps — the ranked work list

> **Last updated:** 2026-10-13

Every known shortfall, most valuable first. Pick work from the top unless the user asks for something else.

| # | Gap | Area | Notes |
|---|---|---|---|
| 1 | Web build: installed/web fonts (only the built-in Ubuntu Light, one weight), Data Recovery and recent designs in browser storage, paste of pictures, control channel over a web socket | Platform | the web app runs (`apps/genha-web`, `cargo xtask web`): open, place, drop, quick actions, Save and Download as browser downloads |
| 2 | Edit text directly on the canvas (caret, selection) instead of the floating editor | Editor | |
| 3 | Colour emoji (COLR/bitmap fonts are not drawn), justified text, OpenType feature and language settings, hyphenation, vertical text | Text | shaping landed (M3.1): HarfRust kerning, ligatures, marks, Arabic/Hebrew/Indic, bidi reordering, UAX #14 line breaks, per-cluster font fallback |
| 4 | Vector PDF (krilla) and CMYK print PDF | Export | pages are pictures with an invisible text layer; print PDF has bleed and crop marks |
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
