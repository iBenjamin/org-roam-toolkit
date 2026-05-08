# Conventions

The two-axis layout: `packages/` (capability libraries) vs `mcp-servers/` + `plugins/org-roam-toolkit/skills/` (adapters that expose capabilities to specific consumers). Capabilities don't import from adapters; adapters depend on capabilities, never the other way.

## Naming

| Kind | Example | Notes |
|---|---|---|
| Capability package | `@org-roam-toolkit/emacs`, `@org-roam-toolkit/web` | one per domain (emacs / web / future...) |
| MCP server package | `@org-roam-toolkit/mcp-org-roam` | `mcp-<consumer-name>` prefix |
| Skill directory | `plugins/org-roam-toolkit/skills/org-roam/` | short kebab-case; SKILL.md `name:` may differ from dir name |
| Elisp subpackage | `elisp/org-roam-skill/` | dir name = elisp `(provide ...)` symbol |

## Skills are thin

A skill directory contains:
- `SKILL.md` — frontmatter + prose for the model. Should call the `ortk-*` bins on PATH (installed by Homebrew); these are the public API.
- `scripts/<name>` — optional one-line bash wrappers that pre-apply `--pkg=` or similar to a PATH bin
- `references/*.md` — optional documentation read by the model on demand

A skill **must not** contain business logic, extraction rules, daemon-state management, or schema definitions. If you find yourself writing those in `plugins/org-roam-toolkit/skills/`, lift them into `packages/`.

## Elisp packages

Located under `packages/emacs/elisp/<pkg>/`. Convention:

- `<pkg>/<pkg>.el` is the entry point and must `(provide '<pkg>)`.
- The directory name **must match** the symbol passed to `provide`.
- Submodules are loaded by the entry file via `(require ...)` of fully-qualified symbols (e.g., `(require 'org-roam-skill-core)`).
- Any package may `(require 'claude-skill-base)`; the shared base is always loaded first by `bin/emacs-eval`.

## emacs-eval contract

`ortk-emacs-eval --pkg=NAME "(expr)"` is the **only** sanctioned way for skills and MCP servers to call emacsclient. Direct `emacsclient` invocations from adapters are forbidden — they bypass package auto-loading and daemon health checks.

The Homebrew formula installs the bin as `/opt/homebrew/bin/ortk-emacs-eval` (a symlink into libexec). The script itself resolves `$BASH_SOURCE` through symlinks so it always finds its sibling `lib/` and `elisp/` directories.

A skill's optional partial wrapper looks like:

```bash
#!/bin/bash
exec ortk-emacs-eval --pkg=<pkg-name> "$@"
```

SKILL.md prose should generally call the bare `ortk-emacs-eval --pkg=...` form directly — the `--pkg=` flag is part of the public contract and explicit beats hidden-default.

## Web site handlers

Each site handler implements `SiteHandler` from `packages/web/src/types.ts`:

```ts
{ name; match(url); navOptions?; postNavWait?; extract(page, url) }
```

Register in `packages/web/src/sites/index.ts`. Order matters: the first `match()` returning `true` wins. `genericHandler` is the catch-all and must remain last.

## TypeScript

- All packages use `tsconfig.base.json` and project references.
- ES module output, NodeNext resolution.
- Source in `src/`, output in `dist/` (gitignored).
- DOM types are only enabled in `packages/web` (it runs page.evaluate callbacks). Other packages stay node-only.

## What does NOT belong in this repo

- Hooks / commands / agents / output styles — these are Claude Code Plugin features that may be added later as new top-level adapter directories. They are not part of the current scope.
- Cross-package data formats — if two packages need to exchange data, define the type in the producer and import; do not introduce a shared DTO package without a real reason.
