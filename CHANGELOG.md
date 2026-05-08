# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-09

### Added
- Homebrew distribution: `brew install iBenjamin/tap/org-roam-toolkit` installs all bins (`ortk-mcp`, `ortk-emacs-eval`, `ortk-fetch`, `ortk-ocr`, `ortk-dashboard`).
- `brew services start org-roam-toolkit` to autostart the dashboard at login.
- `Formula/org-roam-toolkit.rb` source-of-truth formula in this repo (mirrored to `iBenjamin/homebrew-tap`).
- `LICENSE` (MIT) at repo root and `license` field on every package.
- `AGENT.md` — project-level policy authorizing AI co-authorship in commits (vendor-neutral).
- `ortk-dashboard` rewritten in Rust (axum + HTMX, single 2.4 MB static binary). Replaces the previous TypeScript + Svelte/Vite implementation that was hitting npm hoisting bugs on Node 24. The HTTP/JSON contract is unchanged; HTML output now uses HTMX `hx-trigger="every 5s"` for refresh instead of a Svelte SPA.
- `ortk-agent-install` to install the shared Claude Code and Codex plugin from the Homebrew package.
- Codex plugin metadata and installer-managed Codex MCP registration.

### Changed
- All bins renamed with `ortk-` prefix to avoid PATH collisions: `mcp-org-roam` → `ortk-mcp`, `emacs-eval` → `ortk-emacs-eval`, `skill-fetch` → `ortk-fetch`, `skill-ocr` → `ortk-ocr`, `dashboard-serve` → `ortk-dashboard`. The remaining npm package names are unchanged.
- `ortk-mcp` rewritten in Rust as a JSON-RPC stdio MCP adapter. The tool/resource contract is unchanged; Emacs operations still go through `ortk-emacs-eval --pkg=org-roam-skill`.
- Plugin `.mcp.json` and skill scripts now call `ortk-*` bins on PATH. The previous `${CLAUDE_PLUGIN_ROOT}/../../...` path-traversal escape (which only worked inside the monorepo) is gone.
- `packages/emacs/bin/emacs-eval` and `packages/dashboard-server/bin/dashboard-serve` now resolve `$BASH_SOURCE` through symlinks, so brew's `install_symlink` works.
- README rewritten around the brew install flow.
- The plugin install flow now supports Claude Code and Codex with rollback on partial install failures.

### Removed
- `packages/dashboard-server/launchd/io.org-roam-toolkit.dashboard.plist.tmpl` and the `make install-launchd` / `make uninstall-launchd` targets — replaced by `brew services`.
- The TypeScript implementation of `dashboard-server` (Hono backend + Svelte/Vite UI) and the now-unused `packages/emacs/src/probes/*.ts` consumed only by it.
- `dashboard-server` is no longer an npm workspace (it has no `package.json`).
- `@org-roam-toolkit/mcp-org-roam` npm workspace — replaced by the Rust `mcp-servers/org-roam` crate.
