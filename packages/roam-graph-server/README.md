# ortk-roam-graph

Local web viewer for an org-roam graph. Single static Rust binary;
embeds JS / CSS at compile time. Reads `org-roam.db` read-only.

## Run

```bash
cargo run --release -- --port 9877
ortk-roam-graph --port 9877      # after brew install iBenjamin/tap/org-roam-toolkit
```

Visit <http://127.0.0.1:9877>.

See `../../docs/superpowers/specs/2026-05-10-ortk-roam-graph-design.md`
for the design.
