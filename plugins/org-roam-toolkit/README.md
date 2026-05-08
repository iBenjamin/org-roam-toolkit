# org-roam-toolkit (Claude Code plugin)

End-to-end org-roam workflows for Claude Code.

## What's in here

- **9 slash commands** (`commands/`) — `/note`, `/study`, `/deep_note`, `/reference`, `/ref-extract`, `/to-read`, `/read-history`, `/add-toolkit`, `/gen-commit-msg`
- **5 skills** (`skills/`) — `atomic-notes` (format spec), `org` (agenda + capture), `org-roam` (note management), `fetch` (playwright + OCR), `dashboard` (observability)
- **MCP server registration** (`.mcp.json`) — registers `org-roam` MCP server backed by the Homebrew-installed `ortk-mcp` bin

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

# 2. Install the plugin (this directory) into Claude Code
claude marketplace add github:iBenjamin/org-roam-toolkit
claude plugin install org-roam-toolkit
```

See the repo root README for the full setup, including dashboard autostart via `brew services`.

## Notes on the commands

- The slash commands are **Chinese-language and opinionated**: they encode a specific atomic-notes / Zettelkasten workflow with bilingual titles, `:ZH:` drawers, double-layer References, AI-generation marking, and quarterly file conventions for `read_history/` and `toolkit/`. See `skills/atomic-notes/SKILL.md` for the full format spec.
- `/gen-commit-msg` auto-commits AND auto-pushes when invoked. The slash invocation is treated as explicit authorization.
