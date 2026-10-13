# GenHa Roadmap

**Stage: pre-alpha** · next: alpha (M2–M3: document styles, embedded fonts) · ~30–35% of the target feature set (estimated)

> **Last updated:** 2026-10-13 · **Change:** minor (M3.1 text shaping, M2.16 text edited on the canvas, M4.1 vector PDF, M2.17 web app follow-ups, M2.19 smart guides, M5.1 multi-series charts and richer tables, M3.2 formatting part of a text box, M6.1 MP4 video export)

## Headline numbers

| Dimension | Value | Measured / estimated |
|---|---|---|
| Feature parity with the target (all areas) | ~20–25% | estimated, see [target-app-parity.md](docs/target-app-parity.md) |
| Commands (UI = CLI = MCP) | 158 | measured (`genha-cli exec engine.commands`) |
| Templates / size presets / icons / text styles / palettes / filters | 65 / 35 / 114 (+10 basic shapes) / 25 / 40 / 14 | measured (counted from `genha_templates`) |
| Tests | 299 | measured (`cargo test --workspace`) |
| Rust lines | ~36k | measured (all `.rs` files, tests included) |
| Platforms | Linux, macOS, Windows (desktop); web (WASM, `cargo xtask web`) | Linux and Chromium measured; others by CI target |

## What's next

1. **M2 — Web & polish:** keyboard reference, preferences; web app: control channel over a WebSocket, WOFF2 fonts. The web build (with fonts, Data Recovery, recent designs and picture paste), crop page, multi-select handles, recent files, SVG import and on-canvas text editing landed.
2. **M3 — Text:** grid text, document-wide text styles, font files embedded in designs, colour emoji (shaping with HarfRust and formatting part of a text box landed).
3. **M4 — Pictures & print:** colour-managed CMYK (ICC output intent), HEIC, vectorize, better background removal. Vector PDF with embedded fonts, print PDF with bleed/crop marks and SVG import landed.
4. **M5 — Documents:** several brands, accessibility (AccessKit), German UI (charts with several series, tables with merged cells, present mode and version history landed).
5. **M6 — Motion & video:** timeline with scenes, audio (animations, playback, present mode, GIF, animated PNG and MP4 video landed).

Full milestone table: [docs/roadmap.md](docs/roadmap.md). Ranked work list: [docs/gaps.md](docs/gaps.md).

## Progress log

| Date | Milestone | Summary |
|---|---|---|
| 2026-10-13 | M2.19 | Smart guides while resizing (moving edges snap to page and object lines, same width/height and equal gaps; corners with Shift, Alt from the centre, several objects), distance hints to neighbours and page edges, equal-spacing markers, Alt-hover distances, Ctrl/Cmd skips snapping, `view.snapping` kept between runs; a click on an object or handle never nudges it (3 px drag threshold, handles follow the pointer's travel) |
| 2026-10-13 | M5.1 | Charts with several series (grouped and stacked bars, horizontal bars, lines, stacked areas), per-series colours, axis titles, gridlines, legend position, number/percent/currency formats with decimals, fixed min/max; series × categories grid and CSV/TSV paste. Tables with merged cells, per-cell fill/text colour/bold/alignment/size, column widths (pixels, fit to text) and row heights, outer/inner lines; `table.mergeCells`, `splitCell`, `setCellStyle`, `setColumnWidth`, `setRowHeight`; one scene for PNG, SVG and vector PDF |
| 2026-10-13 | M3.2 | Formatting part of a text box: style runs (`TextEl::spans`: font, size, bold, italic, underline, strikethrough, colour by character range) that move with edits and are repaired on load; layout shapes each run in its own face and size with breaking, bidi and alignment across runs; per-run drawing on the canvas, in SVG (nested `<tspan>`s) and vector PDF (with synthetic bold/italic); `text.setStyle {range}` / the editing selection, `text.toggleBold/Italic/Underline/Strikethrough` (Ctrl/Cmd+B/I/U), `text.inspect`; mixed values in the panel |
| 2026-10-13 | M6.1 | MP4 video export: our own pure-Rust H.264 encoder (Constrained Baseline, written from the ITU-T specification; skip/inter/intra 16x16 and 4x4 by rate–distortion, deblocking) and MP4 writer in `crates/video`, decoded bit-exactly by FFmpeg in tests; `export.mp4` (every page for its duration, 1–60 fps, quality low/medium/high), Download › MP4 video on the desktop (worker thread, progress, Cancel) and in the browser (where GIF and animated PNG now download too), `genha-cli export … -o out.mp4`. A 17 s 1080 × 1920 story of three animated pages: 2.6 / 4.1 / 5.9 MB (low / medium / high), about 20 s to export (measured, release build) |
| 2026-10-13 | M2.16 | Text edited in place on the canvas: caret and selection from the renderer's shaped layout (zoom, rotation, wrapping, alignment, ligatures, right-to-left runs), mouse and keyboard selection, clipboard, IME; one undo step per edit; `text.beginEdit` / `text.edit` / `text.endEdit`; the floating editor is gone |
| 2026-10-13 | M4.1 | Vector PDF export (krilla): shapes, gradients, clipping and opacity as PDF graphics; real text with embedded, subsetted fonts (curved text as placed glyphs); pictures at native resolution (JPEG as is); charts and tables; only blurred shadows rasterized (300 dpi, per element); `raster: true` and `cmyk: true` options; PDFs 10–50× smaller than the raster ones (measured on templates) |
| 2026-10-13 | M3.1 | Text shaping with HarfRust: kerning, ligatures, mark positioning, Arabic/Hebrew/Devanagari, bidi reordering, UAX #14 line breaks, per-cluster font fallback; glyph ids, faces and source offsets on laid-out glyphs |
| 2026-10-13 | M2.17 | Web app: fonts added from files (IndexedDB) or installed (Local Font Access), synthetic bold/italic everywhere, Data Recovery and recent designs with thumbnails through one `Store` abstraction (folder / IndexedDB), picture paste, Cmd+C/X/V on the desktop, `web` build profile (11.8 → 8.3 MB, 6.7 MB with wasm-opt) |
| 2026-10-12 | M6 (part) | Motion: element animations (12 entrances, 4 loops, exits) in `crates/motion`, canvas playback, present mode (F5), GIF / animated PNG / single-frame export |
| 2026-10-12 | M2.1 | Charts (bar, horizontal bar, line, area, pie, donut, progress bar/ring), tables and curved text: model, raster + SVG drawing, 13 commands, Elements tab and properties panels |
| 2026-10-12 | M2.3 | Content library: 65 original templates (carousels, stories, covers, flyers, posters, cards, invitations, certificates, tickets, decks, resumes, logos, ads), 114 icons in 11 categories, 25 text styles, 40 palettes |
| 2026-10-12 | M2.7 | Web app: the same UI in the browser (`apps/genha-web`, `cargo xtask web`), file picker, drag & drop, downloads, built-in fallback fonts |
| 2026-10-12 | M1 | Desktop editor (home, templates, canvas with snapping, panels), smart resize, picture tools and quick actions, exports (PNG/JPEG/SVG/PDF), CLI, MCP, control channel |
| 2026-10-12 | M0 | Workspace and layering after the ArtCraft apps, document model, native format, engine with undo and panic guard, xtask CI gates |
