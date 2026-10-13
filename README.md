# GenHa

**Design anything — open source, made in Rust.**

GenHa is a free, open-source design app for quick content: social posts and stories, flyers,
posters, logos, business cards, presentations, resumes and invitations. It aims at the feature set
of Adobe Express — and beyond — as an **independent, clean-room project**: its own name, its own UI,
its own templates and artwork, 100% Rust, MIT OR Apache-2.0, no account, everything runs locally.

It follows the conventions of the ArtCraft apps (VectorCraft, PhotoCraft, PDFCraft): a layered Cargo
workspace, "everything is a command", a never-crash policy, and first-class automation for AI agents
(control channel, CLI and an MCP server).

![The GenHa editor](docs/images/editor.png)

## Status

**Stage: pre-alpha (M1 done, M2 in progress).** A working desktop and web editor with templates, smart resize, charts, animation, picture tools
and exports; ~30–35% of the target feature set by our own (low) estimate — see
[ROADMAP.md](ROADMAP.md) and [docs/target-app-parity.md](docs/target-app-parity.md).

What works today:

- **Start**: 35 size presets (social, video, print, marketing, documents) or a custom size; 121 original templates (posts, carousels, animated posts and videos, stories, covers, flyers, posters, menus, cards, invitations, letterheads, invoices, decks with charts, resumes, worksheets, infographics, landing pages, logos, ads); open pictures as a design.
- **Edit**: multi-page designs (pages of different sizes), select/move/resize/rotate with smart guides (snapping while moving and resizing, equal spacing, distance hints; also several objects at once), groups, layer order, lock/hide, align & distribute, copy/paste (also style), undo/redo, crop page to content, version history, Data Recovery after a crash, recent designs with thumbnails, paste pictures from the clipboard.
- **Content**: text edited in place on the canvas, with styles (also for single words: font, size, bold, italic, underline, strikethrough, colour), presets and effects (shadow, outline, highlight) and curved text, shaped with kerning and ligatures, right-to-left and complex scripts, and font fallback for mixed scripts; 25 text styles; charts (bar, line, area, pie, donut, progress) with several series, stacking, axis titles and number formats, and tables with merged cells and per-cell styles; shapes and 114 original icons in 11 categories; QR codes; 40 palettes; collages; brand kit (colours, fonts, palettes, colours from a photo).
- **Pictures**: place (also SVG as editable shapes), crop, fit, crop to shape, corner radius, border, adjustments, 14 filters, remove / blur / change background: an AI model finds the subject on busy photos (U²-Net, Apache-2.0, downloaded once on request, runs locally in pure Rust) or a colour-based method for plain backdrops.
- **Smart resize**: re-lay out a page for another size (text reflowed, rows and grids re-arranged, tall designs in columns on wide pages), or add a resized copy; or scale it to fit.
- **Motion**: entrance, loop and exit animations for any object or text, "animate all" with a stagger, playback on the canvas, present mode (F5).
- **Export**: PNG (also transparent and 2×), JPEG, SVG, multi-page vector PDF (real, searchable text with embedded fonts; pictures at full resolution), print PDF (bleed, crop marks, optional CMYK), MP4 video (H.264, every page for its duration; our own pure-Rust encoder, also in the browser), animated GIF and animated PNG. Native `.genha` files.
- **Languages**: the whole interface in English or German (follows the system, or picked on the home screen); UI fields show text in any script the computer has fonts for.
- **Quick actions** (file in, file out): remove / blur / change background, resize, convert PNG/JPEG/WebP, trim, black & white, pixelate.
- **Automation**: 162 commands reachable from the UI, the CLI, a JSON-lines control channel and MCP.

## Run it

```sh
cargo run --release -p genha                       # the desktop app
cargo run --release -p genha-cli -- --help         # the command line
cargo xtask web && python3 -m http.server -d dist/web 8080   # the web app: open http://127.0.0.1:8080
```

The web version is the same app compiled to WebAssembly: files are opened through the browser's
file picker or by dropping them on the page, and Save / Download become browser downloads
(see [docs/development.md › Web](docs/development.md#web)).

Command-line examples:

```sh
genha-cli templates
genha-cli new --template event-night -o party.genha
genha-cli export party.genha -o party.pdf
genha-cli export party.genha -o party.mp4 --quality high   # every page, each for its duration
genha-cli model download                           # the background removal model (4.6 MB, once)
genha-cli quick remove-background photo.jpg -o cutout.png   # AI model when downloaded, else colour-based
genha-cli run script.json --export out.png         # script: [{"command": "...", "params": {...}}]
genha-cli mcp                                      # MCP server for AI assistants
```

## For contributors and agents

- [AGENTS.md](AGENTS.md) — the rules (clean-room, never crash, everything is a command, quality gates).
- [docs/architecture.md](docs/architecture.md) — crates and layers.
- [docs/gaps.md](docs/gaps.md) — the ranked work list.
- `cargo xtask ci` must pass before every commit.

## Licence

Code and original assets: MIT OR Apache-2.0 (see `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`).
GenHa is not affiliated with Adobe Inc.; "Adobe Express" is their trademark, named only to
describe the feature set this project covers.
