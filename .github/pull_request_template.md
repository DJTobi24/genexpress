## What and why

<!-- What does this change, and why? Link the issue if there is one. -->

## How it was tested

<!-- Tests added, and how you checked the behaviour (CLI, MCP, rendered PNGs, screenshots). -->

## Checklist

- [ ] `cargo xtask ci` passes (fmt, clippy, tests, layers, assets, cleanroom, wasm).
- [ ] No panics in shipped code: no `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!` or
      `unimplemented!` outside tests, and no unchecked indexing on data from files, parameters or
      messages. Errors are returned as `Result` and shown to the user.
- [ ] Clean-room: nothing copied from Adobe products or GPL/AGPL code ([AGENTS.md](../AGENTS.md)).
- [ ] Every new asset has a row in `ASSETS.md`.
- [ ] New user-visible behaviour is a command with tests, and `docs/target-app-parity.md` is updated.
