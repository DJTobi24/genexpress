# Assets

Every image, icon, font or other non-code file shipped with GenHa has a row here.
`cargo xtask assets` (run by `cargo xtask ci`) fails on any file under `assets/` that is missing.

Allowed sources: original work created for GenHa (licensed MIT OR Apache-2.0 by its author),
OSI open source, public domain / CC0, Creative Commons with redistribution allowed. Never Adobe
icons, templates, artwork, presets or stock content (see AGENTS.md).

Templates, shapes, icons, palettes and filter presets are generated in code (`crates/templates`,
`crates/effects`) and need no row.

## Downloaded on request (not in the repository)

Machine-learning models are never committed or bundled. The app downloads them only when the user
asks (`model.download`, or the prompt behind Remove background), checks the pinned SHA-256 and keeps
them in the user's data folder (`models/`). See docs/architecture.md › Background removal.

| File (in the data folder) | Author | Source | Licence | Notes |
|---|---|---|---|---|
| models/u2netp.onnx | Xuebin Qin et al. (U²-Net); ONNX conversion by the rembg project (Daniel Gatis, MIT) | weights: https://github.com/xuebinqin/U-2-Net · file: https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx | Apache-2.0 | 4,574,861 bytes, SHA-256 309c8469258dda742793dce0ebea8e6dd393174f89934733ecc8b14c76f4ddd8; default background removal model |
| models/u2net.onnx | Xuebin Qin et al. (U²-Net); ONNX conversion by the rembg project (MIT) | weights: https://github.com/xuebinqin/U-2-Net · file: https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx | Apache-2.0 | 175,997,641 bytes, SHA-256 8d10d2f3bb75ae3b6d527c77944fc5e7dcd94b29809d47a739a7a728a912b491; optional "high quality" model |

## In the repository

| Path | Author | Source | Licence | Notes |
|---|---|---|---|---|
| docs/images/editor.png | GenHa contributors | screenshot of GenHa | MIT OR Apache-2.0 | the app showing an original template; rendered with system fonts (Inter, OFL) |
| docs/images/home.png | GenHa contributors | screenshot of GenHa | MIT OR Apache-2.0 | home screen |
| docs/images/web.png | GenHa contributors | screenshot of GenHa in Chromium (web build) | MIT OR Apache-2.0 | the web app showing an original template; text in the built-in fallback face Ubuntu Light (Ubuntu Font Licence 1.0, from the `epaint_default_fonts` crate) |
| docs/images/home-de.png | GenHa contributors | screenshot of GenHa | MIT OR Apache-2.0 | home screen with the German UI |
| docs/images/edit-text.gif | GenHa contributors | screen recording of GenHa (Xvfb, ffmpeg) | MIT OR Apache-2.0 | editing an original template's headline on the canvas |
| docs/images/smart-resize.gif | GenHa contributors | screen recording of GenHa (Xvfb, ffmpeg) | MIT OR Apache-2.0 | smart resize of an original template to a story and a landscape page |
| docs/images/animation.gif | GenHa contributors | screen recording of GenHa (Xvfb, ffmpeg) | MIT OR Apache-2.0 | an original animated template playing on the canvas |
| docs/images/templates.jpg | GenHa contributors | rendered by `genha-cli` | MIT OR Apache-2.0 | 18 of GenHa's original templates; text in system fonts (Inter and others, OFL) |
