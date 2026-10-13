# Control protocol

Start the desktop app with `--control <port>` (e.g. `genha --control 7979`). It listens on
`127.0.0.1:<port>` for newline-delimited JSON: one request per line, one reply per line.

```json
{"id": 1, "method": "engine.execute", "params": {"command": "shape.add", "params": {"shape": "star", "fill": "#ffcc00"}}}
{"id": 1, "ok": true, "result": {"ok": true, "id": 7}}
```

Errors: `{"id": 1, "ok": false, "error": "…"}`.

| Method | Params | Result |
|---|---|---|
| `engine.execute` | `{command, params}` | the command's result |
| `engine.commands` | `{prefix?}` | command list with params docs and enabled state |
| `document.inspect` | `{all_pages?}` | pages, objects, selection, brand kit |
| `session.inspect` | `{}` | open documents, active tool |
| `ui.render` | `{page?, scale?, path?}` | the page as PNG: written to `path`, else `{png_base64}` |
| `ui.state` | `{}` | screen, zoom, left tab, status, `playing`, `presenting` (page index or null) |
| `ui.play` | `{stop?}` | plays the current page's animations on the canvas (or stops) |
| `ui.present` | `{page?, stop?}` | present mode (full window, animations playing) at `page`; `stop` leaves it |

To look at an animation headlessly, use the `export.frame` command (`{path, t}`: one moment as PNG),
`animation.timeline`, or `export.gif` / `export.apng` / `export.mp4`.

Text is edited in place with commands too: `text.beginEdit {id?, selection?}` enters editing (the app shows the caret on the
canvas), `text.edit {text? | insert?, range?, selection?}` types or replaces text and moves the selection (one undo step per
editing session), `text.endEdit` leaves; `session.inspect` reports `text_editing` (`{id, selection}` or null).

Every command (`engine.commands`) is reachable, so an agent can do anything a user can.
The same methods are what [MCP](mcp.md) forwards to.
