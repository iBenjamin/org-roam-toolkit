# Developing in this repo

## Quick layout reference

```
org-roam-toolkit/
├── packages/
│   ├── emacs/                        # @org-roam-toolkit/emacs — bin/emacs-eval (→ ortk-emacs-eval), lib/, elisp/, src/
│   ├── web/                          # @org-roam-toolkit/web   — dist/{fetch,ocr}-cli.js (→ ortk-fetch / ortk-ocr), src/sites/
│   └── dashboard-server/             # @org-roam-toolkit/dashboard-server
├── mcp-servers/
│   └── org-roam/                     # @org-roam-toolkit/mcp-org-roam
└── plugins/
    └── org-roam-toolkit/             # Claude Code plugin
        ├── commands/                 # 9 slash commands
        └── skills/
            ├── atomic-notes/
            ├── org/
            ├── org-roam/
            ├── fetch/
            └── dashboard/
```

## Daily commands

```bash
make install       # npm install (workspaces)
make build         # tsc -b (all TS packages)
make test          # vitest + (eldev test if Eldev present)
make lint          # vitest lint hooks + eldev lint
make clean         # tsc -b --clean
```

## Adding a new elisp package

1. `mkdir packages/emacs/elisp/<name>`
2. Create `<name>/<name>.el` that ends in `(provide '<name>)`
3. Add submodule files alongside; have `<name>.el` `(require '<sub>)` them
4. Optionally `(require 'claude-skill-base)` for JSON envelope helpers
5. Add `<name>` to the `dolist` in `packages/emacs/Eldev`

Loading is automatic — `ortk-emacs-eval --pkg=<name>` finds it by directory convention.

## Adding a new skill

1. `mkdir -p plugins/org-roam-toolkit/skills/<name>`
2. Write `plugins/org-roam-toolkit/skills/<name>/SKILL.md` with frontmatter (`name:` + `description:` containing trigger words). Have its prose call `ortk-*` bins on PATH directly (e.g. `ortk-emacs-eval --pkg=<pkg>` or `ortk-fetch <url>`).
3. (Optional) `mkdir scripts/` with one-line bash wrappers that pre-apply common flags. Wrappers should be plain `exec ortk-... "$@"` — no path-traversal tricks.
4. `chmod +x plugins/org-roam-toolkit/skills/<name>/scripts/<short-name>`

A skill should not contain `.ts` or `.el` files. The plugin is auto-discovered by Claude Code once the whole `plugins/org-roam-toolkit/` directory is symlinked into `~/.claude/plugins/` (via `make install-claude` in dev, or the brew formula's caveats step in production).

## Adding a new web site handler

1. Add `packages/web/src/sites/<site>.ts` exporting a `SiteHandler` (see existing `wechat.ts` / `archive.ts`)
2. Register it in `packages/web/src/sites/index.ts` **before** `genericHandler`
3. Add a unit test in `packages/web/src/sites/index.test.ts` for the new `match()` rule
4. `make build && make test`

## Adding a new MCP server

1. `mkdir -p mcp-servers/<name>/src`
2. Create `package.json` with name `@org-roam-toolkit/mcp-<name>`, `bin` entry, and dependency on whichever capability packages it needs (e.g. `"@org-roam-toolkit/emacs": "*"`)
3. Add a `tsconfig.json` extending `../../tsconfig.base.json` with `references` to the capability package
4. Add to root `tsconfig.json` `references`
5. Write `src/index.ts` — import capabilities, define MCP `Tool[]`, dispatch in `CallToolRequestSchema` handler

## Testing without a daemon

`packages/emacs/src/emacs-client.ts` exposes pure helpers (`buildKeywordArgs`, `parseElispResult`) that can be unit-tested without launching emacs. The site-handler registry in `packages/web` is similarly testable without a browser. End-to-end tests that actually launch the daemon or browser are out of scope for the unit suite — verify those manually.

## What NOT to do

- **Don't** spawn `emacsclient` directly from a skill or MCP server. Always go through `ortk-emacs-eval` (or `evalElisp` from `@org-roam-toolkit/emacs`).
- **Don't** put extraction rules, regex, or daemon state into `plugins/org-roam-toolkit/skills/`. They belong in `packages/`.
- **Don't** edit `claude-skills/` (the legacy reference repo at `/Users/ben/myworkspace/claude-skills`). It is read-only material; new code lives here.
- **Don't** add `Co-Authored-By` or AI attribution to commit messages.
