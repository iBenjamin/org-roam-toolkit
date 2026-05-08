# ortk-dashboard

Local observability dashboard for the org-roam-toolkit. Single static
Rust binary, axum + HTMX, embeds its own JS / CSS at compile time.

## Run

```bash
cargo run --release -- --port 9876
# or, after `brew install iwangkaimin/tap/org-roam-toolkit`:
ortk-dashboard --port 9876
```

Visit <http://127.0.0.1:9876>.

## What it shows

Four cards, each refreshing every 5 seconds via `hx-trigger="every 5s"`:

| Card | Backed by |
|---|---|
| **Emacs Daemon** | `ortk-emacs-eval --pkg=claude-skill-base "(claude-skill-probe-daemon)"` |
| **MCP Server** | spawn `ortk-mcp` + JSON-RPC `initialize` + `tools/list` handshake |
| **org-roam Config** | `ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-probe-config)"` |
| **Graph Stats** | `ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-probe-graph-stats)"` |

A failed probe shows the underlying error message in red without
crashing the page.

## JSON API

For automation / `curl | jq` users, the same probes are exposed as JSON:

```
GET /api/health                 # all four
GET /api/health/daemon
GET /api/health/mcp
GET /api/health/roam-config
GET /api/health/graph-stats
```

Every response is a `Probe` envelope:

```json
{ "status": "up", "data": { … }, "probedAt": "2026-05-08T10:14:35.271Z" }
{ "status": "down", "error": "<reason>", "probedAt": "…" }
```

## Caching

Probe results are cached for 5 seconds per probe name. The cache is
in-memory only; restarting the server starts cold.

## License

MIT.
