# GenHa Roadmap

**Stage: pre-alpha** · next: alpha (M2–M3: document styles, embedded fonts, per-word text formatting) · ~30–35% of the target feature set (estimated)

> **Last updated:** 2026-10-13 · **Change:** minor (M2.16: text edited on the canvas)

## Headline numbers

| Dimension | Value | Measured / estimated |
|---|---|---|
| Feature parity with the target (all areas) | ~20–25% | estimated, see [target-app-parity.md](docs/target-app-parity.md) |
| Commands (UI = CLI = MCP) | 139 | measured (`genha-cli exec engine.commands`) |
| Templates / size presets / icons / text styles / palettes / filters | 65 / 35 / 114 (+10 basic shapes) / 25 / 40 / 14 | measured (counted from `genha_templates`) |
| Tests | 211 | measured (`cargo test --workspace`) |
| Rust lines | ~16k | measured |
| Platforms | Linux, macOS, Windows (desktop); web (WASM, `cargo xtask web`) | Linux and Chromium measured; others by CI target |

## What's next

1. **M2 — Web & polish:** keyboard reference, preferences; web app follow-ups (bold/italic fonts, Data Recovery in browser storage, smaller download). The web build, crop page, multi-select handles, recent files, SVG import and on-canvas text editing landed.
2. **M3 — Text:** grid text, document-wide text styles, font files embedded in designs, colour emoji (shaping with HarfRust landed).
3. **M4 — Pictures & print:** vector PDF (selectable text), print PDF with bleed/crop marks, SVG import, HEIC, vectorize.
4. **M5 — Documents:** charts with several series, present mode, version history, several brands, accessibility (AccessKit), German UI.
5. **M6 — Motion & video:** MP4 export, timeline with scenes, audio (animations, playback, present mode, GIF and animated PNG landed).

Full milestone table: [docs/roadmap.md](docs/roadmap.md). Ranked work list: [docs/gaps.md](docs/gaps.md).

## Progress log

| Date | Milestone | Summary |
|---|---|---|
| 2026-10-13 | M2.16 | Text edited in place on the canvas: caret and selection from the renderer's shaped layout (zoom, rotation, wrapping, alignment, ligatures, right-to-left runs), mouse and keyboard selection, clipboard, IME; one undo step per edit; `text.beginEdit` / `text.edit` / `text.endEdit`; the floating editor is gone |
| 2026-10-13 | M3.1 | Text shaping with HarfRust: kerning, ligatures, mark positioning, Arabic/Hebrew/Devanagari, bidi reordering, UAX #14 line breaks, per-cluster font fallback; glyph ids, faces and source offsets on laid-out glyphs |
| 2026-10-12 | M6 (part) | Motion: element animations (12 entrances, 4 loops, exits) in `crates/motion`, canvas playback, present mode (F5), GIF / animated PNG / single-frame export |
| 2026-10-12 | M2.1 | Charts (bar, horizontal bar, line, area, pie, donut, progress bar/ring), tables and curved text: model, raster + SVG drawing, 13 commands, Elements tab and properties panels |
| 2026-10-12 | M2.3 | Content library: 65 original templates (carousels, stories, covers, flyers, posters, cards, invitations, certificates, tickets, decks, resumes, logos, ads), 114 icons in 11 categories, 25 text styles, 40 palettes |
| 2026-10-12 | M2.7 | Web app: the same UI in the browser (`apps/genha-web`, `cargo xtask web`), file picker, drag & drop, downloads, built-in fallback fonts |
| 2026-10-12 | M1 | Desktop editor (home, templates, canvas with snapping, panels), smart resize, picture tools and quick actions, exports (PNG/JPEG/SVG/PDF), CLI, MCP, control channel |
| 2026-10-12 | M0 | Workspace and layering after the ArtCraft apps, document model, native format, engine with undo and panic guard, xtask CI gates |
