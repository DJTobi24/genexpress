# GenExpress — instructions for agents

GenExpress is a clean-room, open-source, Rust-native design app for quick content: social posts, stories, flyers, posters, logos, business cards, presentations, resumes, invitations and simple videos. Its feature target is parity with Adobe Express (and beyond), but it is **its own product**: own name, own UI, own templates, own artwork. It runs natively on macOS, Windows and Linux, and on the web via WASM. It follows the conventions of the ArtCraft apps (VectorCraft, PhotoCraft, PDFCraft): same layering, same rules, same agent tooling.

## Start every session here
1. Read [`ROADMAP.md`](ROADMAP.md) (stage, numbers, what's next) and the ranked work list in [`docs/gaps.md`](docs/gaps.md). Unless the user gives you a task, pick work from that list.
2. Read `plan/STATUS.md` if it exists (local session notes, may lag the ROADMAP) and the relevant section of [`docs/architecture.md`](docs/architecture.md). Behaviour reference: public documentation only (see Clean-room below); keep notes from it in `plan/express/*.md`.
3. Work in this loop: orient → plan → implement + test → verify (look at the result) → record (docs) → commit. Don't stop to ask unless the decision is the user's (licensing, naming, dropping a feature, anything irreversible).

`plan/` is gitignored (local only).

## Non-negotiables
- **Clean-room: never use Adobe software (absolute rule).** Adobe's Terms of Use forbid reverse engineering, including observing a program's inputs and outputs to recreate it. So:
  - Never launch, run, script, automate, screenshot or measure Adobe Express or any other Adobe application, and never compare our output against one. Don't make test files with them.
  - Never read, disassemble or copy anything from an Adobe app or web app bundle (code, templates, presets, fonts, icons, stock content, localisation strings).
  - Never copy Adobe-authored code or text into this repo, tests included. No Adobe template, layout, wording or marketing copy — only feature *names* may be mentioned, to say what we cover.
  - Behaviour comes only from public documentation (help pages, published specs, public forum posts), cited by URL, and from files users send us that they own (used locally, never committed).
  - If a task seems to need any of this, stop and say so instead. `cargo xtask cleanroom` (run by `cargo xtask ci`) fails on Adobe copyright notices and on files made with Adobe applications.
  - Never copy GPL/AGPL code.
- **Assets: no Adobe iconography, templates or images — ever.**
  - Every image, icon, font or other asset must be original work created for GenExpress (licensed MIT OR Apache-2.0 by its contributor), OSI open source, public domain / CC0, or Creative Commons with redistribution allowed.
  - **Every asset file must have a row in [`ASSETS.md`](ASSETS.md)** (path, author, source URL, licence, notes); `cargo xtask assets` fails on any unattributed file under `assets/`.
  - **Prefer content generated in code**: templates (`crates/templates`), shapes and icons (SVG path data written by us), palettes, filter presets. No stock photos in the repo.
  - **Never commit font files.** Fonts come from the system (`genexpress_text::FontBook::system()`) or from fonts the user adds; everything must build, test and run with no fonts at all (tests that need glyphs skip when none are found).
  - Code and original assets are MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`).
- **Never crash: no panics in shipped code.** A crash loses the user's work. Files, pasted data, MCP and control messages and command params are all untrusted input.
  - Fallible work returns `Result` (or `Option`); failures reach the user as an error message (`EngineError` for commands, the crate's own error type below the engine), never as an abort.
  - Workspace clippy lints ban these outside tests, so `cargo xtask ci` fails on them: `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`. Use `?`, `.ok_or(…)?`, `let … else`, `if let`, `unwrap_or*`, `.get(i)`.
  - Don't panic implicitly: use `.get(i)` instead of `v[i]` when the index comes from data, don't divide by something that can be zero, cap sizes and counts read from input, check floats are finite before casting.
  - Don't silence errors to satisfy the lint: `let _ =` / `.ok()` only where ignoring the failure is right, with a comment saying why.
  - Tests may unwrap and panic (`clippy.toml`). Integration-test files start with `#![allow(…)]`.
  - `genexpress_engine::guard` is the last line of defence: every entry point (commands, tool events, MCP, the UI frame, the control channel) catches a panic, rolls the document back and reports an internal error. It exists for bugs and doesn't replace `Result`: wasm aborts on panic.
  - Untrusted input is fuzzed (`crates/format/tests/prop_format.rs`, `crates/engine/src/tests.rs` › `every_command_survives_junk_params`, `crates/mcp/src/tests.rs`). Extend these when you add an importer, a parser or a protocol method.
- **Everything is a command.** User-visible behaviour = a command in `crates/engine/src/cmd/*` (id, label, menu path, shortcut, params doc, `enabled`, `run`) + tests. Tools emit commands (preview while dragging, commands on release). The UI, the CLI, the control channel and MCP reach all of them; anything a user can do, an agent can do.
- **Layering** is enforced by `cargo xtask layers` (table in `xtask/src/layers.rs`). Nothing below L6 depends on egui/eframe/winit/rfd.
- **The UI is thin**: panels read engine state and act through `app.run(id, params)`. Colours come from `theme::Tokens`.
- **Rust only** (no handwritten JS/TS). **Never break wasm** (`cargo xtask wasm`).
- **Quality gates** before every commit: `cargo xtask ci` (fmt, clippy -D warnings including the no-panic lints, tests, layers, assets, cleanroom, wasm). One task id per commit (`M2.3: background remover`).

## Running and looking at the app
- `cargo run --release -p genexpress -- --control 7979 [file.genexpress|picture.png]` (sibling ArtCraft apps use the same default port: if the log says it failed to bind, pick another port).
- Drive it: JSON lines on `127.0.0.1:7979`, e.g. `{"id":1,"method":"engine.execute","params":{"command":"shape.add","params":{"shape":"star","fill":"#ffcc00"}}}` then `{"id":2,"method":"ui.render","params":{"path":"/tmp/page.png"}}`. Methods: [`docs/control-protocol.md`](docs/control-protocol.md).
- **For UI and rendering work, look at the result** (render a page to PNG and read it) and compare with the public documentation of the feature.
- Headless: `genexpress-cli run script.json --export out.png`, `genexpress-cli export design.genexpress -o out.pdf`, `genexpress-cli quick remove-background in.jpg -o out.png`.
- MCP: `genexpress-cli mcp` (headless) or `genexpress-cli mcp --attach 127.0.0.1:7979` (see [`docs/mcp.md`](docs/mcp.md)).
- Parallel agents: separate `CARGO_TARGET_DIR` per agent; edit only the crates you own.

## Roadmap and progress docs
- [`ROADMAP.md`](ROADMAP.md): stage, headline numbers, what's next, progress log.
- [`docs/target-app-parity.md`](docs/target-app-parity.md): the feature inventory of the target app (from public docs) with our status per feature. The authoritative parity assessment.
- [`docs/gaps.md`](docs/gaps.md): every known shortfall, ranked; the work list. [`docs/roadmap.md`](docs/roadmap.md): milestones.
- [`docs/architecture.md`](docs/architecture.md), [`docs/development.md`](docs/development.md).

When a task lands, update in the same commit: its row in `docs/target-app-parity.md`, the gap in `docs/gaps.md` if it closed or shrank, the milestone row in `docs/roadmap.md`, and the ROADMAP when a headline number moves. Grade by behaviour against the public documentation, not by whether a button exists; err low; mark numbers measured or estimated. Keep the README's Status section in step with the ROADMAP headline.
