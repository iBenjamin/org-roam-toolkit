# org-roam-toolkit agent plugin

End-to-end org-roam workflows for Claude Code and Codex.

## What's in here

- **Claude Code commands** (`commands/`) — 9 slash commands: `/note`, `/study`, `/deep_note`, `/reference`, `/ref-extract`, `/to-read`, `/read-history`, `/add-toolkit`, `/gen-commit-msg`
- **Agent skills** (`skills/`) — `atomic-notes` (format spec), `org` (agenda + capture), `org-roam` (note management), `fetch` (playwright + OCR), `dashboard` (observability)
- **Claude MCP server registration** (`.mcp.json`) — registers the `org-roam` MCP server backed by the Homebrew-installed `ortk-mcp` bin
- **Codex plugin manifest** (`.codex-plugin/plugin.json`) — points Codex at `./skills/`. The plugin source itself is fetched by `codex plugin marketplace add iBenjamin/org-roam-toolkit`; `ortk-agent-install codex` only writes the `[mcp_servers.org-roam]` and plugin-enable entries into `~/.codex/config.toml`

## Runtime requirements

This plugin **requires** the `iBenjamin/tap/org-roam-toolkit` Homebrew package, which provides the bins the skills, commands, and MCP registrations call on PATH:

| Bin | Purpose |
|---|---|
| `ortk-mcp` | Rust MCP server for org-roam (referenced by Claude `.mcp.json` and Codex config) |
| `ortk-emacs-eval` | Universal emacsclient wrapper (used by `org` and `org-roam` skills) |
| `ortk-fetch` | Playwright-based fetcher (used by `fetch` skill) |
| `ortk-ocr` | OCR helper (used by `fetch` skill) |
| `ortk-dashboard` | Observability dashboard server (used by `dashboard` skill) |

You also need a running Emacs daemon with `org-roam` loaded.

## Install

```bash
# 1. Install the bins (one-time)
brew tap iBenjamin/tap
brew install org-roam-toolkit

# 2. Claude Code plugin (run inside a Claude Code session)
/plugin marketplace add iBenjamin/org-roam-toolkit
/plugin install org-roam-toolkit@org-roam-toolkit

# 3. Codex plugin
codex plugin marketplace add iBenjamin/org-roam-toolkit
ortk-agent-install codex   # writes [mcp_servers.org-roam] to ~/.codex/config.toml
```

See the repo root README for the full setup, including dashboard autostart via `brew services`.

## Notes on the commands

- The slash commands are **Claude-specific, English-language, and opinionated**: they encode a specific atomic-notes / Zettelkasten workflow with English titles, English tags, double-layer References, AI-generation marking, and quarterly file conventions for `read_history/` and `toolkit/`. See `skills/atomic-notes/SKILL.md` for the full format spec.
- `/gen-commit-msg` auto-commits AND auto-pushes when invoked. The slash invocation is treated as explicit authorization.
