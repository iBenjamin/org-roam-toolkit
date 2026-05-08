# org-roam-toolkit

MCP server, observability dashboard, and Claude Code plugin for an Emacs / org-roam knowledge base. Distributed as a Homebrew tap.

macOS only.

## Install

```bash
# 1. Install the bins via Homebrew
brew tap iwangkaimin/tap
brew install org-roam-toolkit

# 2. Symlink the Claude Code plugin into ~/.claude/plugins/
ln -snf "$(brew --prefix org-roam-toolkit)/libexec/plugins/org-roam-toolkit" \
        ~/.claude/plugins/org-roam-toolkit

# 3. (optional) Auto-start the dashboard at login
brew services start org-roam-toolkit       # http://127.0.0.1:9876
```

After step 2, restart Claude Code to load the plugin.

## What you get

| Bin (on PATH) | What it does |
|---|---|
| `ortk-mcp` | Rust MCP server for org-roam (registered as `org-roam` in the plugin's `.mcp.json`) |
| `ortk-emacs-eval` | Universal `emacsclient --eval` wrapper that auto-loads project elisp packages via `--pkg=NAME` |
| `ortk-fetch` | Playwright headless fetcher with per-site extraction strategies (WeChat, archive.today, …) |
| `ortk-ocr` | Tesseract.js OCR helper |
| `ortk-dashboard` | Local dashboard at `http://127.0.0.1:9876` showing daemon / MCP / org-roam health |

Plus the Claude Code plugin under `plugins/org-roam-toolkit/`:

- **9 slash commands** — `/note`, `/study`, `/deep_note`, `/reference`, `/ref-extract`, `/to-read`, `/read-history`, `/add-toolkit`, `/gen-commit-msg`
- **5 skills** — `atomic-notes` (format spec), `org` (agenda + capture), `org-roam` (note management), `fetch` (web + OCR), `dashboard` (observability)

The slash commands are **Chinese-language and opinionated** — they encode a specific atomic-notes / Zettelkasten workflow. See `plugins/org-roam-toolkit/skills/atomic-notes/SKILL.md` for the format spec.

## Runtime requirements

- macOS (Apple Silicon or Intel)
- Node ≥18 (pulled in by `brew install`)
- A running Emacs daemon (`emacs --daemon`) with `org-roam` loaded and `org-roam-directory` set
- For the `fetch` skill: `npx playwright install chromium` (one-time ~150MB Chromium download)

## Layout

```
org-roam-toolkit/
├── packages/
│   ├── emacs/                      # @org-roam-toolkit/emacs        — emacsclient wrapper + shared elisp
│   ├── web/                        # @org-roam-toolkit/web          — playwright fetchers + OCR
│   └── dashboard-server/           # ortk-dashboard — Rust crate (axum + HTMX, single static binary)
│
├── mcp-servers/
│   └── org-roam/                   # ortk-mcp — Rust MCP server backed by ortk-emacs-eval
│
├── plugins/
│   └── org-roam-toolkit/           # Claude Code plugin (commands + skills + .mcp.json)
│
├── Formula/
│   └── org-roam-toolkit.rb         # Source-of-truth Homebrew formula (mirrored to iwangkaimin/homebrew-tap)
│
└── docs/                           # developer documentation
```

`packages/` are capability libraries — the implementations. `mcp-servers/` and `plugins/org-roam-toolkit/skills/` are adapters — they expose those capabilities to specific consumers (MCP clients, Claude Code).

## Development

End users install via Homebrew — the rest of this section is for working **on** the toolkit.

```bash
make install          # npm install (workspaces)
make build            # tsc -b (all TS packages)
make dashboard        # build + run server in foreground on $DASH_PORT (default 9876)
make test             # vitest + eldev tests (if Eldev present)
make lint             # vitest lint hooks + eldev lint
make install-claude   # symlink plugins/org-roam-toolkit into ~/.claude/plugins/ (dev mode)
make uninstall-claude # undo the above
```

To test the formula locally without publishing a tag:

```bash
brew install --build-from-source ./Formula/org-roam-toolkit.rb
```

(The committed `url` / `sha256` only become valid once a release tag is pushed; before that, prefer `--HEAD` or `--build-from-source` against the local working copy.)

See `docs/conventions.md` and `docs/developing-skills.md` for code-level conventions.

## License

MIT — see `LICENSE`.
