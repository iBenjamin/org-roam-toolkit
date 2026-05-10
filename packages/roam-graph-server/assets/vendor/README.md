# Vendored frontend assets

Versions and provenance — pinned at first vendor.

| File                          | Package                              | Version | Source |
|-------------------------------|--------------------------------------|---------|--------|
| `../graphology.umd.min.js`    | `graphology`                         | 0.25.4  | https://cdn.jsdelivr.net/npm/graphology@0.25.4/dist/graphology.umd.min.js |
| `../sigma.min.js`             | `sigma`                              | 3.0.0   | https://cdn.jsdelivr.net/npm/sigma@3.0.0/dist/sigma.min.js |
| `../fa2.worker.js`            | `graphology-layout-forceatlas2`      | 0.10.1  | Inlined IIFE — index.js + iterate.js + helpers.js + defaults.js from https://cdn.jsdelivr.net/npm/graphology-layout-forceatlas2@0.10.1/ plus graphology-utils@2.5.1 (is-graph, getters.js) hand-bundled into a self-contained IIFE. Registers `window.forceAtlas2` (synchronous `.assign` API). No Worker, no bundler. |

(If you had to substitute a different version, update the table above to match what is actually on disk.)

## Update procedure

1. Bump version in this README.
2. Re-download via the curl commands in `docs/superpowers/plans/2026-05-10-ortk-roam-graph.md` (Task 16).
3. `cargo build --release --manifest-path packages/roam-graph-server/Cargo.toml`
4. Smoke-test in browser at http://localhost:9877.
5. Note the version bump in the next commit.
