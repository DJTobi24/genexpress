# Architecture

> **Last updated:** 2026-10-13 · **Change:** minor (vector PDF)

How GenHa is built. Day-to-day rules (clean-room, never crash, quality gates) are in
[`AGENTS.md`](../AGENTS.md) and [development.md](development.md). The layout deliberately mirrors the
ArtCraft apps (VectorCraft and its siblings) so contributors and agents can move between them.

## Shape

A Cargo workspace of library crates under `crates/`, apps under `apps/` and `xtask`, pure Rust (no
handwritten JS/TS). The same engine runs the desktop app, the headless CLI and the MCP server, and is
built to run in the browser (WASM).

## Layers

`cargo xtask layers` (part of `cargo xtask ci`) enforces that a crate depends only on lower layers, and
that nothing below L6 depends on a UI toolkit (egui, eframe, winit, rfd). Table: `xtask/src/layers.rs`.

| Layer | Crates | Role |
|---|---|---|
| L0 | `geom`, `color` | frames (position, size, rotation), fitting, snapping, shape outlines (on `kurbo`); sRGB colours as hex, HSL, WCAG contrast, harmonies |
| L1 | `doc` | the document model: designs with pages of any size, elements (shapes, text, pictures, groups), brand kit, embedded assets, page-size presets; pure data + serde |
| L2 | `text`, `effects`, `layout`, `templates`, `motion` | font discovery and text layout to outlines (`skrifa`); picture decoding, adjustments, filters and quick actions (background removal, resize, convert…); smart resize, align/distribute, collages, snapping guides; templates, shapes/icons, text styles, palettes and QR codes generated in code; animations evaluated at a time (`page_at`) |
| L3 | `render`, `svg`, `pdf`, `format` | page → pixels (`vello_cpu`) and PNG/JPEG; SVG export; vector PDF export (`krilla`: embedded font subsets, pictures, print boxes); the native `.genha` format (versioned, compressed JSON) |
| L4 | `tools` | pointer gestures → commands (select/move/resize/rotate, shape and text tools, snapping) |
| L5 | `engine` | sessions, undo history, selection, the command registry, file I/O, the panic guard |
| L6 | `ui-egui`, `mcp` | the egui front end (home screen, editor, panels, canvas); the MCP server (JSON-RPC over stdio) |
| L7 | `apps/genha`, `apps/genha-web`, `apps/genha-cli`, `xtask` | desktop app (eframe), web app (eframe web runner, wasm32; `cargo xtask web`), CLI (`run`, `export`, `quick`, `info`, `mcp`…), build tooling |

## The document

```
Document ─ title, brand kit, assets (pictures stored once, shared by id)
 └─ Page ─ size (px), background (solid/gradient), duration (video), notes
     └─ Element ─ id, name, frame {x, y, width, height, rotation}, opacity, lock, hide, flip, animation
         ├─ Shape  ─ rect/ellipse/triangle/polygon/star/line/path, fill, border, shadow
         ├─ Text   ─ text, style (family, size, weight, colour, align, spacing…), effect (shadow/outline/highlight), curve
         ├─ Image  ─ asset, crop, fit, corner radius, crop-to-shape, adjustments, filter, border
         ├─ Chart  ─ kind (bar/horizontal-bar/line/area/pie/donut/progress), labelled values, colours, labels/values/legend
         ├─ Table  ─ rows of cell text, header row, fills, grid lines, padding, alignment, column widths
         └─ Group  ─ children (page coordinates)
```

Documents are cheap to clone (asset bytes are reference-counted), which the undo history relies on:
each step keeps the previous `Arc<Document>`.

## Everything is a command

Every user-visible action is a `CommandSpec` in `crates/engine/src/cmd/*` (id, label, menu path,
shortcut, params doc, `enabled`, `run`). Tools emit commands (a preview while dragging, commands on
release). Menus, the command palette, the control channel, the CLI and MCP all reach the same registry,
so anything a user can do an agent can do. `command.batch` runs several as one undo step.

## Rendering

Pages are rasterized on the CPU by `vello_cpu`; the desktop app shows the result as a texture and
draws selection handles, guides and previews on top with egui. Text is laid out by `genha-text`
into glyph outlines so the canvas, PNG/JPEG/PDF exports and SVG-as-paths all match. Curved text
places each glyph's own outline (`genha_text::layout_glyphs`) along an arc.

### Text layout

`genha-text` turns a string and a `TextStyle` into lines of positioned glyphs, per paragraph:

1. **Analysis:** bidi levels (UAX #9, `unicode-bidi`), each character's script (`unicode-script`;
   Common/Inherited take their neighbours'), a face per grapheme cluster (the style's face if it
   has every character, else the first fallback face that does: per-cluster font fallback), and
   line-break opportunities (UAX #14, `unicode-linebreak`).
2. **Shaping:** the paragraph is cut where face, script or level change and at each break
   opportunity (pieces are words with their trailing spaces); each piece is shaped by HarfRust
   (`harfrust` 0.12, on the same `read-fonts` as `skrifa`) and cached per face in font units, so a
   word is shaped once for every size. Ligatures are off for letter-spaced text.
3. **Lines:** greedy filling with shaped advances; over-long words break between clusters.
4. **Placement:** each line is reordered for display (UAX #9 L1/L2) and drawn with `skrifa`
   outlines. `layout_glyphs` also reports per glyph its glyph id, face (`FontBook::face_data` gives
   the font bytes, for exporters that embed fonts), source byte offset (`cluster`, for carets) and
   direction; lines carry their source byte range. Charts and tables are
turned into a `Scene` of filled/stroked paths and laid-out text by `genha_render::{chart_scene,
table_scene}`, which both the rasterizer and the SVG writer draw, so they agree.

The PDF writer (`crates/pdf`) walks pages the same way and draws them as vectors with `krilla`:
paths, gradients, clips and transparency groups; text as real text in the font the layout chose
(`genha_text::layout_glyphs` gives each shaped glyph's face, id and cluster; `FontBook::face_data` the font bytes), embedded as a subset; pictures at
their native resolution. Only blurred shadows are rendered by `genha-render` and placed as pictures,
one element at a time. `raster: true` keeps whole pages as pictures.

## Motion

`genha-motion::page_at(page, t)` is the page as it looks `t` seconds in: each animated element
moved, scaled, turned, faded or (typewriter) cut short. Canvas playback, present mode and the
animated exports (`export.gif`, `export.apng`, `export.frame`) all render that page with
`genha-render`, so they agree. Exports merge frames whose posed model is unchanged before rendering.

## Files

The native `.genha` format is gzip-compressed JSON with a `{"format", "version", "document"}`
header, pictures embedded as base64, saved atomically, refused with a clear message when it comes from
a newer version, and repaired on open (ids, sizes, missing pictures) because files are untrusted.

## Agent control

- **Control channel:** JSON lines on a local TCP port (`--control 7979`), see [control-protocol.md](control-protocol.md).
- **MCP:** `genha-cli mcp`, headless or attached to a running app ([mcp.md](mcp.md)).

## Robustness

No panics in shipped code (workspace clippy lints ban `unwrap`, `expect`, `panic!` and friends outside
tests); `genha_engine::guard` catches a panic at every entry point and rolls the document back.
