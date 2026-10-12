# Assets

Every image, icon, font or other non-code file shipped with GenHa has a row here.
`cargo xtask assets` (run by `cargo xtask ci`) fails on any file under `assets/` that is missing.

Allowed sources: original work created for GenHa (licensed MIT OR Apache-2.0 by its author),
OSI open source, public domain / CC0, Creative Commons with redistribution allowed. Never Adobe
icons, templates, artwork, presets or stock content (see AGENTS.md).

Templates, shapes, icons, palettes and filter presets are generated in code (`crates/templates`,
`crates/effects`) and need no row.

| Path | Author | Source | Licence | Notes |
|---|---|---|---|---|
| docs/images/editor.png | GenHa contributors | screenshot of GenHa | MIT OR Apache-2.0 | the app showing an original template; rendered with system fonts (Inter, OFL) |
| docs/images/home.png | GenHa contributors | screenshot of GenHa | MIT OR Apache-2.0 | home screen |
