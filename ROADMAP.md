# GenHa Roadmap

**Stage: pre-alpha** · next: alpha (M2–M3: web build, document styles, version history, crop page) · ~20–25% of the target feature set (estimated)

> **Last updated:** 2026-10-12 · **Change:** major (first version: M0 foundation and M1 editor landed)

## Headline numbers

| Dimension | Value | Measured / estimated |
|---|---|---|
| Feature parity with the target (all areas) | ~20–25% | estimated, see [target-app-parity.md](docs/target-app-parity.md) |
| Commands (UI = CLI = MCP) | 115 | measured (`genha-cli exec engine.commands`) |
| Templates / size presets / icons / text styles / palettes / filters | 17 / 35 / 25 (+10 basic shapes) / 11 / 16 / 14 | measured |
| Tests | 141 | measured (`cargo test --workspace`) |
| Rust lines | ~16k | measured |
| Platforms | Linux, macOS, Windows (desktop); engine builds for wasm32 | Linux measured; others by CI target |

## What's next

1. **M2 — Web & polish:** WASM build of the full app (eframe web), crop page to content, inline text editing on the canvas, multi-select handles, keyboard reference.
2. **M3 — Text:** grid text, document-wide text styles, font files embedded in designs, kerning (HarfRust).
3. **M4 — Pictures & print:** vector PDF (selectable text), print PDF with bleed/crop marks, SVG import, HEIC, vectorize.
4. **M5 — Documents:** charts with several series, present mode, version history, several brands, accessibility (AccessKit), German UI.
5. **M6 — Motion & video:** animation playback, GIF then MP4 export, timeline with scenes, audio.

Full milestone table: [docs/roadmap.md](docs/roadmap.md). Ranked work list: [docs/gaps.md](docs/gaps.md).

## Progress log

| Date | Milestone | Summary |
|---|---|---|
| 2026-10-12 | M2.1 | Charts (bar, horizontal bar, line, area, pie, donut, progress bar/ring), tables and curved text: model, raster + SVG drawing, 13 commands, Elements tab and properties panels |
| 2026-10-12 | M1 | Desktop editor (home, templates, canvas with snapping, panels), smart resize, picture tools and quick actions, exports (PNG/JPEG/SVG/PDF), CLI, MCP, control channel |
| 2026-10-12 | M0 | Workspace and layering after the ArtCraft apps, document model, native format, engine with undo and panic guard, xtask CI gates |
