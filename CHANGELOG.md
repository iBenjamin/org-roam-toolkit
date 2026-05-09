# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.8] - 2026-05-09

### Fixed
- Dashboard probe no longer leaks `emacsclient` processes when the Emacs daemon's eval queue hangs. Two combined bugs caused 3 orphaned `emacsclient` processes per probe cycle (~18/min):
  - Wrapper `emacs_daemon_running` had no timeout, so it sat forever on a hung daemon. Replaced with portable bash 3.2 `_emacs_run` helper carrying `EMACS_PROBE_TIMEOUT` (default 2s) and `EMACS_EVAL_TIMEOUT` (default 30s); watcher subshell I/O is redirected to `/dev/null` so its orphaned `sleep` cannot keep the parent's stdout pipe open past wrapper exit.
  - Dashboard sent `SIGKILL` only to the direct child wrapper; `SIGKILL` is uncatchable and doesn't propagate to grandchildren. The dashboard now spawns the wrapper into its own process group and `kill(-pgid, SIGKILL)` on timeout so wrapper and `emacsclient` die atomically.
- `bin/emacs-eval` installs `INT/TERM/EXIT` traps that propagate clean termination to the in-flight `emacsclient` subprocess.

## [0.2.7] - 2026-05-09

### Fixed
- `roam_create_link` now appends each inserted link beneath an explicit `* Links` top-level heading instead of as a bare `[[id:UUID][Title]]` paragraph at end of file. The heading is created on the first insertion and reused thereafter, idempotently. Existing notes with floating id-link paragraphs need a one-off rewrite to add the heading.

## [0.2.6] - 2026-05-09

### Changed
- **Plugin distribution model**: each agent's plugin manager is now responsible for fetching the plugin source from GitHub. Homebrew only ships the runtime binaries.
  - Claude Code: install with `/plugin marketplace add iBenjamin/org-roam-toolkit` and `/plugin install org-roam-toolkit@org-roam-toolkit` inside a session.
  - Codex: install with `codex plugin marketplace add iBenjamin/org-roam-toolkit`, enable from `codex /plugins`, then run `ortk-agent-install codex` to write `[mcp_servers.org-roam]` and `[plugins."org-roam-toolkit@org-roam-toolkit"].enabled = true` into `~/.codex/config.toml`.
- `ortk-agent-install claude` no longer writes a plugin cache or any JSON metadata into `~/.claude/plugins/` — Claude Code's `/plugin install` is the canonical entrypoint and it manages those files itself. The subcommand now only cleans up the legacy `~/.claude/plugins/org-roam-toolkit` symlink (left by 0.2.0–0.2.4) and prints the slash-command instructions.
- `ortk-agent-install codex` no longer copies the plugin into `~/.codex/plugins/cache/` — `codex plugin marketplace add` does that. The subcommand now only edits `~/.codex/config.toml`.
- The `--plugin-dir` flag is gone; the installer no longer needs to know where the plugin source lives.

### Fixed
- `/plugin marketplace add iBenjamin/org-roam-toolkit` no longer hits the schema error (`source.source: Invalid input`) that 0.2.5's hand-written `known_marketplaces.json` produced. We never write that file.

## [0.2.5] - 2026-05-09

### Fixed
- `ortk-agent-install claude` now copies the plugin into `~/.claude/plugins/cache/<marketplace>/<plugin>/local/` and registers it in `installed_plugins.json` and `known_marketplaces.json`, which is what Claude Code actually scans. The previous symlink at `~/.claude/plugins/<name>` was never discovered by Claude Code, so the plugin silently failed to load. The installer also cleans up that legacy symlink, and rolls back every Claude-side write if a later step (or the Codex install) fails.

## [0.2.4] - 2026-05-09

### Added
- Added a root `VERSION` file so release tooling and humans can track the current project version without parsing package manifests.

### Changed
- Converted the org-roam toolkit skills and Claude slash commands from Chinese-facing copy to English-facing copy for broader international use.
- Updated the atomic-note workflow to default to English titles, English tags, and English prose, while keeping translated/local-language drawers optional when explicitly requested.
- Reading-history entries now use English `original` / `archive` link labels.

## [0.2.3] - 2026-05-09

### Fixed
- `ortk-agent-install codex` now installs the plugin into Codex's plugin cache and enables `[plugins."org-roam-toolkit@org-roam-toolkit"]`, which matches Codex's actual plugin discovery path after restart.
- Added repo-level Codex marketplace metadata for `codex plugin marketplace add <repo>`.

## [0.2.2] - 2026-05-09

### Fixed
- Plugin manifests now report the published package version.

## [0.2.1] - 2026-05-09

### Fixed
- `ortk-agent-install` now resolves Homebrew's `bin/ortk-agent-install` symlink before inferring the bundled plugin directory, so `ortk-agent-install all` installs the Homebrew plugin instead of falling back to a source checkout.

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
