# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Homebrew distribution: `brew install iwangkaimin/tap/org-roam-toolkit` installs all bins (`ortk-mcp`, `ortk-emacs-eval`, `ortk-fetch`, `ortk-ocr`, `ortk-dashboard`).
- `brew services start org-roam-toolkit` to autostart the dashboard at login.
- `Formula/org-roam-toolkit.rb` source-of-truth formula in this repo (mirrored to `iwangkaimin/homebrew-tap`).
- `LICENSE` (MIT) at repo root and `license` field on every package.

### Changed
- All bins renamed with `ortk-` prefix to avoid PATH collisions: `mcp-org-roam` → `ortk-mcp`, `emacs-eval` → `ortk-emacs-eval`, `skill-fetch` → `ortk-fetch`, `skill-ocr` → `ortk-ocr`, `dashboard-serve` → `ortk-dashboard`. The npm package names are unchanged.
- Plugin `.mcp.json` and skill scripts now call `ortk-*` bins on PATH. The previous `${CLAUDE_PLUGIN_ROOT}/../../...` path-traversal escape (which only worked inside the monorepo) is gone.
- `packages/emacs/bin/emacs-eval` and `packages/dashboard-server/bin/dashboard-serve` now resolve `$BASH_SOURCE` through symlinks, so brew's `install_symlink` works.
- README rewritten around the brew install flow.

### Removed
- `packages/dashboard-server/launchd/io.org-roam-toolkit.dashboard.plist.tmpl` and the `make install-launchd` / `make uninstall-launchd` targets — replaced by `brew services`.
