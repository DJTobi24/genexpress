# Milestones

> **Last updated:** 2026-10-13

| Milestone | Scope | Status |
|---|---|---|
| M0 Foundation | workspace, layers, doc model, format, engine, guard, xtask, agent docs | ✅ done |
| M1 Editor | text/render/svg/pdf, effects/layout/templates, UI (home, editor, canvas, panels), CLI, MCP, control channel | ✅ done |
| M2 Web & polish | WASM app ✅ (`apps/genha-web`; fonts, synthetic bold/italic, Data Recovery, recent designs and picture paste in the browser ✅ M2.17), crop page ✅, recent files ✅, SVG import ✅, multi-select handles ✅, content library ✅ (65 templates, 114 icons, 25 text styles, 40 palettes), on-canvas text editing ✅, smart guides while resizing, distance and equal-spacing hints ✅ M2.19, preferences | 🟡 in progress |
| M3 Text | curved/grid text, document styles, embedded fonts, shaping & kerning, per-word formatting, spellcheck hooks | 🟡 curved text, shaping (HarfRust: kerning, ligatures, bidi, complex scripts, font fallback) and formatting part of a text box (M3.2: style runs) done |
| M4 Pictures & print | vector PDF, print PDF (bleed, marks), SVG import, HEIC, vectorize, better background removal | 🟡 vector PDF ✅ (krilla, embedded fonts), print PDF ✅ (bleed, marks, plain CMYK), SVG import ✅; colour-managed CMYK, HEIC, vectorize, background removal next |
| M5 Documents | charts, tables, present mode, version history, several brands, accessibility, translations | 🟡 charts (several series, stacked, axis titles, number formats), tables (merged cells, cell styles, sizes, lines), version history and present mode done (M5.1); German UI, language setting and UI fonts for every script (M5.2) |
| M6 Motion & video | animation playback, GIF/MP4 export, timeline, scenes, audio | 🟡 animations (in/loop/out), playback, present mode, GIF, animated PNG and MP4 (M6.1, own H.264 encoder) done; timeline, scenes, audio next |
| M7 Collaboration | optional self-hostable sync server: share links, comments, co-editing, template locking | ⬜ |
| M8 AI | pluggable local/remote models: background removal, generative fill/expand, text-to-image, captions, translate | ⬜ |
| M9 Platform | sandboxed WebAssembly add-ons, embed build, PPTX/PDF import, web-page export | ⬜ |
| M10 Social | optional scheduler add-on (user's own platform tokens) | ⬜ |
