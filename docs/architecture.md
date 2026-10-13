# Architecture

> **Last updated:** 2026-10-13 · **Change:** minor (text editing on the canvas, vector PDF, multi-series charts and merged table cells, MP4 video export: `crates/video`)

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
| L3 | `render`, `svg`, `pdf`, `video`, `format` | page → pixels (`vello_cpu`) and PNG/JPEG; SVG export; vector PDF export (`krilla`: embedded font subsets, pictures, print boxes); MP4 video (an H.264 encoder and MP4 writer of our own, pure Rust); the native `.genha` format (versioned, compressed JSON) |
| L4 | `tools` | pointer gestures → commands (select/move/resize/rotate, shape and text tools); smart guides (`tools::smart`: snapping while moving and resizing, equal spacing, same sizes, distance hints) as pure geometry the canvas only draws |
| L5 | `engine` | sessions, undo history, selection, the command registry, file I/O, the panic guard |
| L6 | `ui-egui`, `mcp` | the egui front end (home screen, editor, panels, canvas); the MCP server (JSON-RPC over stdio) |
| L7 | `apps/genha`, `apps/genha-web`, `apps/genha-cli`, `xtask` | desktop app (eframe), web app (eframe web runner, wasm32; `cargo xtask web`), CLI (`run`, `export`, `quick`, `info`, `mcp`…), build tooling |

## The document

```
Document ─ title, brand kit, assets (pictures stored once, shared by id)
 └─ Page ─ size (px), background (solid/gradient), duration (video), notes
     └─ Element ─ id, name, frame {x, y, width, height, rotation}, opacity, lock, hide, flip, animation
         ├─ Shape  ─ rect/ellipse/triangle/polygon/star/line/path, fill, border, shadow
         ├─ Text   ─ text, style (family, size, weight, colour, align, spacing…), spans (style runs by character range), effect (shadow/outline/highlight), curve
         ├─ Image  ─ asset, crop, fit, corner radius, crop-to-shape, adjustments, filter, border
         ├─ Chart  ─ kind (bar/horizontal-bar/line/area/pie/donut/progress), categories + series (name, values, colour),
         │           stacked, axis titles, gridlines, legend position, number format, min/max, colours
         ├─ Table  ─ rows of cell text, header row, fills, lines (all/outer/inner/rows), padding, alignment,
         │           column widths, row heights, merged cells (ranges), per-cell styles
         └─ Group  ─ children (page coordinates)
```

Documents are cheap to clone (asset bytes are reference-counted), which the undo history relies on:
each step keeps the previous `Arc<Document>`.

## Everything is a command

Every user-visible action is a `CommandSpec` in `crates/engine/src/cmd/*` (id, label, menu path,
shortcut, params doc, `enabled`, `run`). Tools emit commands (a preview while dragging, commands on
release). Menus, the command palette, the control channel, the CLI and MCP all reach the same registry,
so anything a user can do an agent can do. `command.batch` runs several as one undo step.

Text edited in place is engine state too: `Session::text_editing` holds the text box and selection
(`text.beginEdit` / `text.edit` / `text.endEdit`), and every change of one editing session merges into one undo
step. The canvas (`ui-egui/src/textedit.rs`) only turns input into `text.edit` and draws the caret and
selection, placed with `genha_text::caret::CaretMap`, which maps the renderer's own shaped glyph clusters (ligatures split by grapheme, right-to-left runs mirrored) to
character indices; caret movement, word boundaries and in-edit undo are pure code in `genha_text::edit`.

Part of a text box can be formatted (M3.2): `TextEl::spans` are sorted, non-overlapping character ranges
with a `SpanStyle` (font, size, bold, italic, underline, strikethrough, colour) over the box's `TextStyle`;
paragraph values (alignment, line height, letter spacing, upper case) stay on the box. `genha_doc::rich`
keeps them consistent: edits move them with the text (typed text takes the style before the caret),
`normalize_spans` (run by `Document::repair`) fixes junk from files. `text.setStyle` and the
`text.toggleBold`/`Italic`/`Underline`/`Strikethrough` commands apply to an explicit `range`, else to the
selection while editing (with a bare caret they set `TextEditing::typing_style` for the next text typed),
else to whole boxes; `text.inspect` reports the runs and the style at the selection (with mixed values).

## Rendering

Pages are rasterized on the CPU by `vello_cpu`; the desktop app shows the result as a texture and
draws selection handles, guides and previews on top with egui. Text is laid out by `genha-text`
into glyph outlines so the canvas, PNG/JPEG/PDF exports and SVG-as-paths all match. Curved text
places each glyph's own outline (`genha_text::layout_glyphs`) along an arc.

### Text layout

`genha-text` turns a string and a `TextStyle` (or style runs: `layout_rich`, `layout_el`) into lines
of positioned glyphs, per paragraph. Each run is shaped in its own face and size; runs that differ only
in colour or decoration are shaped together (kerning holds across them); breaking, bidi and alignment
work across runs; a line is as tall as its tallest run, all on one baseline. Glyphs carry a style index
(`TextLayout::styles`), lines carry per-style outline pieces and underline/strikethrough bars, so the
renderer, SVG and PDF writers paint each run in its colour with its synthetic bold/italic.

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
animated exports (`export.gif`, `export.apng`, `export.mp4`, `export.frame`) all render that page with
`genha-render`, so they agree. Exports merge frames whose posed model is unchanged before rendering.

## Video

`export.mp4` plays every page in turn for its duration at a constant frame rate (30 fps unless
`fps` says otherwise) and writes an H.264 video in an MP4 file. Frames whose posed page equals the
previous one are not rendered again: the encoder repeats the previous picture, which costs a few bytes.
Pages of another size are letterboxed into the first page's frame; frames are capped at 1920 × 1080
pixels' worth (1080 × 1920 stories fit exactly) and videos at 3600 frames.

**Encoder choice.** The requirements: a licence compatible with MIT/Apache (no GPL, so no x264 or
FFmpeg), H.264 because it plays everywhere (browsers, QuickTime, Windows, Android, iOS), and the
browser build must keep working. Options weighed:

| Option | Licence | Builds for wasm32 | Verdict |
|---|---|---|---|
| `openh264` crate (Cisco OpenH264 compiled from source by `cc`) | BSD-2 (attribution) | no (`wasm32-unknown-unknown` unsupported) | good compression, but ~50k lines of C++ behind `unsafe` FFI (a crash there cannot be caught), needs a C++ toolchain on every platform, desktop only; Cisco's patent licence covers only Cisco's own binaries, not builds from source |
| `less-avc` (pure Rust) | MIT/Apache | yes | I_PCM only (uncompressed): ~1.5 bytes per pixel per frame, 500 MB for 10 s of 1080 × 1080 |
| `rav1e` (pure Rust AV1) + an MP4 muxer | BSD-2 | untested (without its assembly) | royalty-free codec, but slow and AV1 does not play on many phones, older Macs and Windows without an extension |
| **Our own H.264 encoder** (`crates/video`) | MIT OR Apache-2.0 | **yes** | chosen: written from the public specification (Rec. ITU-T H.264 (08/2024)), no `unsafe`, no C, same code on desktop and web |

MP4 muxers (`mp4`, `muxide`, `shiguredo_mp4`, `mp4e`) would work, but one video track of
constant-rate samples is ~300 lines of box writing, so `crates/video/src/mp4.rs` writes it (and
`genha_video::inspect` reads it back for tests).

**What the encoder does** (`crates/video/src/h264`): Constrained Baseline profile (CAVLC, progressive,
one reference frame, no B frames), so every decoder plays it; level chosen from the frame size and
rate (3.0–4.2); BT.709 limited-range Y′CbCr 4:2:0 signalled in the VUI and an MP4 `colr` box; an IDR
picture at each page start and at least every 2 s; one slice per picture; constant QP per quality
(`low` 30, `medium` 25, `high` 20); the in-loop deblocking filter. Each macroblock is coded the way
with the lowest `distortion + λ · bits` (bits counted exactly) among P_Skip, P_L0_16x16 with a
full-sample motion vector (candidates from the neighbours and the previous picture, then a pattern
search), Intra_16x16 (4 modes) and Intra_4x4 (9 modes per block); intra coding is only tried where
motion prediction is worse than intra prediction. Unchanged macroblocks are skipped without a search.
Not done (yet): sub-sample motion vectors, partitions smaller than 16 × 16, CABAC and B frames, which
is why files are larger than a mature encoder's at the same quality.

**Exactness.** The encoder reconstructs each picture exactly as a decoder will (prediction, inverse
transform, deblocking from clauses 8.3–8.7). `crates/video` checks this against an independent
decoder: where `ffmpeg` is installed (CI installs it) the test decodes test videos and requires every
sample to match the encoder's reconstruction; the code tables are checked for prefix-freeness and
were transcribed from the specification's text.

**Patents.** H.264 is covered by patents licensed through a pool (Via LA, formerly MPEG LA); some have
expired, others have not. GenHa's licence covers its code only; distributors of binaries in countries
that enforce such patents should check what applies to them (Cisco's OpenH264 patent coverage would not
have helped: it applies only to Cisco-built binaries). AV1 would avoid this, at the cost of playback on
many devices.

**Threads.** The desktop app runs the export on a worker thread (`genha_engine::cmd::motion::video_file`
on a copy of the design) with progress and Cancel in the status bar. The CLI, control channel and MCP
run `export.mp4` like any command. The browser has no threads: the page waits while the video is made
(a native release build takes about 20 s for a 17 s, 1080 × 1920 story of three animated pages, most
of it encoding; the browser is slower).

## Files

The native `.genha` format is gzip-compressed JSON with a `{"format", "version", "document"}`
header, pictures embedded as base64, saved atomically, refused with a clear message when it comes from
a newer version, and repaired on open (ids, sizes, missing pictures) because files are untrusted.

What the app keeps between runs (Data Recovery copies, recent designs with thumbnails, fonts the
user added) goes through `genha_engine::store::Store`, a small key-value trait: a folder on the
desktop (`DirStore`, the per-user data folder), IndexedDB in the browser (`genha_ui_egui` ›
`web::IdbStore`). The `recovery.*`, `recent.*` and `font.*` commands only see the trait.

## Agent control

- **Control channel:** JSON lines on a local TCP port (`--control 7979`), see [control-protocol.md](control-protocol.md).
- **MCP:** `genha-cli mcp`, headless or attached to a running app ([mcp.md](mcp.md)).

## Robustness

No panics in shipped code (workspace clippy lints ban `unwrap`, `expect`, `panic!` and friends outside
tests); `genha_engine::guard` catches a panic at every entry point and rolls the document back.
