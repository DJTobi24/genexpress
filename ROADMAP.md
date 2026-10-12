# GenHa Roadmap

**Stage: pre-alpha** · next: alpha (M2–M3: on-canvas text editing, curved text, document styles, version history, crop page) · ~20–25% of the target feature set (estimated)

> **Last updated:** 2026-10-12 · **Change:** major (first version: M0 foundation and M1 editor landed)

## Headline numbers

| Dimension | Value | Measured / estimated |
|---|---|---|
| Feature parity with the target (all areas) | ~20–25% | estimated, see [target-app-parity.md](docs/target-app-parity.md) |
| Commands (UI = CLI = MCP) | 102 | measured (`genha-cli exec engine.commands`) |
| Templates / size presets / icons / text styles / palettes / filters | 17 / 35 / 25 (+10 basic shapes) / 11 / 16 / 14 | measured |
| Tests | 126 | measured (`cargo test --workspace`) |
| Rust lines | ~16k | measured |
| Platforms | Linux, macOS, Windows (desktop); web (WASM, `cargo xtask web`) | Linux and Chromium measured; others by CI target |

## What's next

1. **M2 — Web & polish:** inline text editing on the canvas, multi-select handles, keyboard reference; web app follow-ups (fonts, Data Recovery in browser storage). The web build and crop page to content landed.
2. **M3 — Text:** curved and grid text, document-wide text styles, font files embedded in designs, kerning (HarfRust).
3. **M4 — Pictures & print:** vector PDF (selectable text), print PDF with bleed/crop marks, SVG import, HEIC, vectorize.
4. **M5 — Documents:** charts & tables, present mode, version history, several brands, accessibility (AccessKit), German UI.
5. **M6 — Motion & video:** animation playback, GIF then MP4 export, timeline with scenes, audio.

Full milestone table: [docs/roadmap.md](docs/roadmap.md). Ranked work list: [docs/gaps.md](docs/gaps.md).

## Progress log

| Date | Milestone | Summary |
|---|---|---|
| 2026-10-12 | M2.7 | Web app: the same UI in the browser (`apps/genha-web`, `cargo xtask web`), file picker, drag & drop, downloads, built-in fallback fonts |
| 2026-10-12 | M1 | Desktop editor (home, templates, canvas with snapping, panels), smart resize, picture tools and quick actions, exports (PNG/JPEG/SVG/PDF), CLI, MCP, control channel |
| 2026-10-12 | M0 | Workspace and layering after the ArtCraft apps, document model, native format, engine with undo and panic guard, xtask CI gates |
