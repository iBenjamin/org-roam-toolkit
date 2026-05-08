# org-roam-toolkit agent plugin

End-to-end org-roam workflows for Claude Code and Codex.

## What's in here

- **Claude Code commands** (`commands/`) — 9 slash commands: `/note`, `/study`, `/deep_note`, `/reference`, `/ref-extract`, `/to-read`, `/read-history`, `/add-toolkit`, `/gen-commit-msg`
- **Agent skills** (`skills/`) — `atomic-notes` (format spec), `org` (agenda + capture), `org-roam` (note management), `fetch` (playwright + OCR), `dashboard` (observability)
- **MCP server registration** (`.mcp.json`) — registers the `org-roam` MCP server backed by the Homebrew-installed `ortk-mcp` bin
- **Codex plugin manifest** (`.codex-plugin/plugin.json`) — points Codex at `./skills/` and `./.mcp.json`

## Runtime requirements

This plugin **requires** the `iBenjamin/tap/org-roam-toolkit` Homebrew package, which provides the bins the skills/commands and `.mcp.json` call on PATH:

| Bin | Purpose |
|---|---|
| `ortk-mcp` | Rust MCP server for org-roam (referenced by `.mcp.json`) |
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

# 2. Install the plugin for Claude Code and Codex
ortk-agent-install all

# Optional: install only one agent integration
ortk-agent-install claude
ortk-agent-install codex
```

See the repo root README for the full setup, including dashboard autostart via `brew services`.

## Notes on the commands

- The slash commands are **Claude-specific, Chinese-language, and opinionated**: they encode a specific atomic-notes / Zettelkasten workflow with bilingual titles, `:ZH:` drawers, double-layer References, AI-generation marking, and quarterly file conventions for `read_history/` and `toolkit/`. See `skills/atomic-notes/SKILL.md` for the full format spec.
- `/gen-commit-msg` auto-commits AND auto-pushes when invoked. The slash invocation is treated as explicit authorization.
