# Agent Install Support Design

## Goal

Make `org-roam-toolkit` installable into both Claude Code and Codex from the same Homebrew package, including skills and the `org-roam` MCP server, without silently modifying user agent configuration during `brew install`.

## Current State

The repository already ships `plugins/org-roam-toolkit/` as a Claude Code plugin. That directory contains slash commands, skills, and `.mcp.json` registering the `org-roam` MCP server through `ortk-mcp`.

The Homebrew formula installs runtime binaries and stages the plugin under `libexec/plugins/org-roam-toolkit`. Current setup instructions ask users to manually symlink that plugin into `~/.claude/plugins/org-roam-toolkit`.

Codex support is missing. There is no `.codex-plugin/plugin.json`, no Codex install command, and no safe flow for adding `[mcp_servers.org-roam]` to `~/.codex/config.toml`.

## Recommended Approach

Add both:

1. A Codex plugin manifest in `plugins/org-roam-toolkit/.codex-plugin/plugin.json`.
2. A new explicit installer binary, `ortk-agent-install`, with subcommands for Claude, Codex, and both agents.

This keeps the repo compatible with Codex plugin discovery while still giving users a practical post-brew command that performs the local configuration steps.

## Command UX

The installer command will support:

```bash
ortk-agent-install claude
ortk-agent-install codex
ortk-agent-install all
ortk-agent-install claude --dry-run
ortk-agent-install codex --dry-run
ortk-agent-install all --dry-run
```

Default behavior writes changes. `--dry-run` prints planned actions without mutating the filesystem.

The command resolves the installed plugin path from the Homebrew layout when available:

```text
<brew-prefix>/opt/org-roam-toolkit/libexec/plugins/org-roam-toolkit
```

For development, the installer will also support an override:

```bash
ortk-agent-install all --plugin-dir ./plugins/org-roam-toolkit
```

The command output must be explicit about every path it reads, writes, links, or skips.

## Claude Install Behavior

Claude install links the full plugin directory:

```text
~/.claude/plugins/org-roam-toolkit -> <plugin-dir>
```

Rules:

- Create `~/.claude/plugins` when missing.
- If target path is missing, create a symlink.
- If target path is already a symlink to the same plugin directory, report it as already installed.
- If target path is a symlink to another location, replace it only with `--force`.
- If target path exists and is not a symlink, refuse to overwrite.

Claude gets commands, skills, and `.mcp.json` from the plugin directory, so no separate MCP config write is required for Claude.

## Codex Install Behavior

Codex install performs two explicit actions.

First, link the plugin directory:

```text
~/.codex/plugins/org-roam-toolkit -> <plugin-dir>
```

The same symlink safety rules as Claude apply.

Second, update `~/.codex/config.toml` to include:

```toml
[mcp_servers.org-roam]
command = "ortk-mcp"
```

Rules:

- Create `~/.codex` when missing.
- Create `~/.codex/config.toml` when missing.
- Preserve all unrelated TOML content.
- If `[mcp_servers.org-roam]` does not exist, append the block.
- If `[mcp_servers.org-roam]` exists with `command = "ortk-mcp"`, leave it unchanged.
- If `[mcp_servers.org-roam]` exists with a different command or args, refuse to overwrite unless `--force` is passed.
- Before mutating an existing config file, write a timestamped backup next to it:

```text
~/.codex/config.toml.bak-YYYYMMDDHHMMSS
```

Codex command support will come from the plugin manifest and skills. The initial scope does not add Codex slash-command equivalents because the existing `commands/` directory is Claude-specific markdown command content.

## Codex Manifest

Add:

```text
plugins/org-roam-toolkit/.codex-plugin/plugin.json
```

The manifest declares:

- plugin metadata matching the Claude plugin metadata
- `skills: "./skills/"`
- `mcpServers: "./.mcp.json"`
- interface metadata for discovery

The plugin's `.mcp.json` already uses:

```json
{
  "mcpServers": {
    "org-roam": {
      "command": "ortk-mcp"
    }
  }
}
```

That file should remain shared between Claude and Codex plugin metadata.

## Implementation Shape

Implement the installer as a new Rust crate:

```text
packages/agent-install/
```

Binary:

```text
ortk-agent-install
```

Responsibilities:

- Parse CLI arguments.
- Resolve `$HOME`.
- Resolve plugin directory.
- Apply symlink safety rules.
- Apply Codex TOML update rules.
- Print dry-run and write-mode summaries.

Rust is preferred over shell because the Codex config update needs safe parsing behavior, conflict detection, backups, and tests.

The implementation can preserve TOML with conservative text editing rather than a full round-trip parser:

- detect exact table headers with line-based parsing
- append a missing table to the end
- detect an existing table's `command` assignment inside its table body
- avoid rewriting the whole file

This keeps comments and formatting intact for unrelated config.

## Homebrew Integration

Update `Formula/org-roam-toolkit.rb` to build and install `ortk-agent-install` alongside `ortk-dashboard` and `ortk-mcp`.

Update caveats to recommend:

```bash
ortk-agent-install all
```

and document narrower choices:

```bash
ortk-agent-install claude
ortk-agent-install codex
```

Homebrew install must not run the installer automatically.

## Makefile And Docs

Update development helpers:

- Add `make install-codex`
- Add `make uninstall-codex`
- Keep `make install-claude` and `make uninstall-claude`
- Add `make install-agents` as a convenience wrapper for both agents

Update documentation:

- README install section
- `plugins/org-roam-toolkit/README.md`
- `docs/developing-skills.md` only if it references Claude-only plugin discovery

## Testing

Add Rust tests for `packages/agent-install`:

- Claude symlink install creates the expected symlink in a temp home.
- Existing matching symlink is idempotent.
- Existing non-symlink target is refused.
- Codex install creates `~/.codex/config.toml` when missing.
- Codex install appends `[mcp_servers.org-roam]` without touching existing content.
- Codex install is idempotent when the correct server already exists.
- Codex install refuses a conflicting `org-roam` server unless forced.
- Dry-run produces no filesystem mutations.

Add integration verification:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
make test-rust
npm test
npm run build
brew audit --formula ibenjamin/tap/org-roam-toolkit
```

When the formula changes, reinstall from the tap or local formula and verify:

```bash
brew reinstall ibenjamin/tap/org-roam-toolkit
ortk-agent-install all --dry-run
```

## Non-Goals

- Do not migrate Claude-specific slash commands into Codex command format in this change.
- Do not automatically install during `brew install`.
- Do not overwrite existing user agent configuration without `--force`.
- Do not remove the existing Claude plugin layout.
- Do not make Playwright/OCR dependencies optional in this change.

## Release Impact

This is a feature release after `v0.1.0`. It should use semantic commits and should update the Homebrew tap formula when ready to publish.
