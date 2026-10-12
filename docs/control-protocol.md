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

Every command (`engine.commands`) is reachable, so an agent can do anything a user can.
The same methods are what [MCP](mcp.md) forwards to.
