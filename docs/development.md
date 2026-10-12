# Development

## Build and run

```sh
cargo run --release -p genexpress                 # desktop app
cargo run --release -p genexpress-cli -- --help   # headless CLI
cargo xtask ci                                    # everything a commit must pass
```

Linux needs the usual windowing libraries for the desktop app (`libxkbcommon`, X11 or Wayland, GTK 3 for
file dialogs). Fonts come from the system; with none installed text still lays out (with approximate
metrics) but draws nothing.

## Quality gates

`cargo xtask ci` runs: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`
(including the no-panic lints), `cargo test --workspace`, `cargo xtask layers`, `cargo xtask assets`,
`cargo xtask cleanroom` and `cargo xtask wasm` (needs `rustup target add wasm32-unknown-unknown`).

## Robustness: GenExpress never crashes

See [`AGENTS.md`](../AGENTS.md) › Never crash. In short: return `Result`, never index with numbers that
came from data, cap sizes, check floats, and fuzz every parser (`proptest`).

## Adding a feature

1. Model: if the document needs new data, add it to `crates/doc` with `#[serde(default)]` so older files
   still open (bump `genexpress_format::VERSION` only for breaking changes, and keep reading old ones).
2. Behaviour: a command in `crates/engine/src/cmd/<area>.rs` with a params doc and tests.
3. Rendering/export: `crates/render`, `crates/svg`, `crates/pdf`.
4. UI: a panel or button in `crates/ui-egui` that calls the command (`app.run(id, params)`).
5. Docs: the feature's row in [target-app-parity.md](target-app-parity.md), [gaps.md](gaps.md), the ROADMAP.
