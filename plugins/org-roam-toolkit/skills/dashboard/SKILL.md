---
name: dashboard
description: |
  Local observability dashboard for the org-roam-toolkit monorepo. Shows Emacs
  daemon health, MCP server status, org-roam config and graph stats in a web UI.

  Triggers: dashboard, daemon health, mcp status, org-roam stats, observability,
  状态, 监控, 看板, daemon 死了, server 没反应
---

# Dashboard Skill

Local web dashboard at `http://127.0.0.1:9876` showing four cards:

| Card | What it shows |
|---|---|
| **Emacs Daemon** | pid, uptime, loaded elisp packages |
| **MCP Server** | server name/version, tool count, binary path |
| **org-roam Config** | `org-roam-directory`, db path/size, subdirectories |
| **Graph Stats** | nodes, edges, orphans, tags |

Refreshes every 5 seconds. Each card turns red and shows the underlying error message when its probe fails — handy for diagnosing "daemon died" / "MCP can't start" without leaving the browser.

## Starting the dashboard

```bash
# Foreground (Ctrl-C to stop)
ortk-dashboard --port=9876

# Custom port / host
ortk-dashboard --port=12345 --host=127.0.0.1
```

## Auto-start at login (macOS, optional)

```bash
brew services start org-roam-toolkit       # autostart at login
brew services stop org-roam-toolkit
brew services restart org-roam-toolkit
```

Logs are managed by `brew services` (run `brew services info org-roam-toolkit` for paths).

## REST API (machine-friendly)

```bash
curl -s http://127.0.0.1:9876/api/health                 # all four probes
curl -s http://127.0.0.1:9876/api/health/daemon          # one probe
curl -s http://127.0.0.1:9876/api/health/mcp
curl -s http://127.0.0.1:9876/api/health/roam-config
curl -s http://127.0.0.1:9876/api/health/graph-stats
```

Every response is a `Probe<T>` envelope:

```json
{
  "status": "up",
  "data": { "...": "..." },
  "probedAt": "2026-05-08T10:14:35.271Z"
}
```

…or on failure:

```json
{ "status": "down", "error": "<reason>", "probedAt": "..." }
```

## Same data via MCP resources

The `ortk-mcp` server (registered as `org-roam` in `mcp.json`) also exposes the same probes as MCP **resources**, so when chatting with Claude Desktop/Code you can ask the model to read them directly:

| URI | Same as |
|---|---|
| `health://daemon` | `/api/health/daemon` |
| `health://mcp` | self-metadata (no recursion) |
| `config://org-roam` | `/api/health/roam-config` |
| `stats://graph` | `/api/health/graph-stats` |

## Implementation notes

- Backend: `@org-roam-toolkit/dashboard-server` (hono, ~150 LOC). Loopback only (`127.0.0.1`); no auth.
- UI: Svelte 5 + Vite, single-page, dark theme.
- Probes: `@org-roam-toolkit/emacs` exports `probeDaemon`, `probeRoamConfig`, `probeGraphStats`. The MCP probe spawns and handshakes with `ortk-mcp`.
- Probe results are cached for 5 seconds per name; clients polling at 5s get fresh data on each cycle without thundering-herd risk.

## Permissions

You have permission to call all `/api/health[/...]` endpoints and to read the MCP resources listed above without asking the user first.
