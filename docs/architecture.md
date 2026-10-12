# Architecture

> **Last updated:** 2026-10-12 · **Change:** major (first version)

How GenExpress is built. Day-to-day rules (clean-room, never crash, quality gates) are in
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
| L2 | `text`, `effects`, `layout`, `templates` | font discovery and text layout to outlines (`skrifa`); picture decoding, adjustments, filters and quick actions (background removal, resize, convert…); smart resize, align/distribute, collages, snapping guides; templates, shapes/icons, text styles, palettes and QR codes generated in code |
| L3 | `render`, `svg`, `pdf`, `format` | page → pixels (`vello_cpu`) and PNG/JPEG; SVG export; PDF export; the native `.genexpress` format (versioned, compressed JSON) |
| L4 | `tools` | pointer gestures → commands (select/move/resize/rotate, shape and text tools, snapping) |
| L5 | `engine` | sessions, undo history, selection, the command registry, file I/O, the panic guard |
| L6 | `ui-egui`, `mcp` | the egui front end (home screen, editor, panels, canvas); the MCP server (JSON-RPC over stdio) |
| L7 | `apps/genexpress`, `apps/genexpress-cli`, `xtask` | desktop app (eframe), CLI (`run`, `export`, `quick`, `info`, `mcp`…), build tooling |

## The document

```
Document ─ title, brand kit, assets (pictures stored once, shared by id)
 └─ Page ─ size (px), background (solid/gradient), duration (video), notes
     └─ Element ─ id, name, frame {x, y, width, height, rotation}, opacity, lock, hide, flip, animation
         ├─ Shape  ─ rect/ellipse/triangle/polygon/star/line/path, fill, border, shadow
         ├─ Text   ─ text, style (family, size, weight, colour, align, spacing…), effect (shadow/outline/highlight)
         ├─ Image  ─ asset, crop, fit, corner radius, crop-to-shape, adjustments, filter, border
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
draws selection handles, guides and previews on top with egui. Text is laid out by `genexpress-text`
into glyph outlines so the canvas, PNG/JPEG/PDF exports and SVG-as-paths all match.

## Files

The native `.genexpress` format is gzip-compressed JSON with a `{"format", "version", "document"}`
header, pictures embedded as base64, saved atomically, refused with a clear message when it comes from
a newer version, and repaired on open (ids, sizes, missing pictures) because files are untrusted.

## Agent control

- **Control channel:** JSON lines on a local TCP port (`--control 7979`), see [control-protocol.md](control-protocol.md).
- **MCP:** `genexpress-cli mcp`, headless or attached to a running app ([mcp.md](mcp.md)).

## Robustness

No panics in shipped code (workspace clippy lints ban `unwrap`, `expect`, `panic!` and friends outside
tests); `genexpress_engine::guard` catches a panic at every entry point and rolls the document back.
