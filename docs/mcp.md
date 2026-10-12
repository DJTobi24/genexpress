# MCP server

GenExpress speaks the [Model Context Protocol](https://modelcontextprotocol.io) over stdio.

```sh
genexpress-cli mcp                          # headless: its own in-process session
genexpress-cli mcp --attach 127.0.0.1:7979  # drive a running app started with --control 7979
```

Example client configuration:

```json
{"mcpServers": {"genexpress": {"command": "genexpress-cli", "args": ["mcp"]}}}
```

## Tools

| Tool | What it does |
|---|---|
| `list_commands` | all commands with their parameter descriptions (`prefix` filters) |
| `run_command` | run one command, e.g. `{command: "template.use", params: {id: "quote-card"}}` |
| `run_commands` | several commands as one undo step |
| `inspect_design` | pages, objects (ids, frames, text, colours), selection, brand kit |
| `look` | the page rendered as a PNG image, to check the result |

A typical session: `run_command file.new {preset: "instagram-post"}` → add text and shapes →
`look` → adjust → `run_command export.png {path: "post.png"}`.
