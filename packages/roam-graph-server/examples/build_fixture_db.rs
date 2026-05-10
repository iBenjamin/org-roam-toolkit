//! Generates `tests/fixtures/fixture.db` — a minimal org-roam-shaped
//! SQLite DB used by integration tests.
//!
//! Run from the crate root:
//!
//!     cargo run --example build_fixture_db
//!
//! Re-run after changing the fixture `.org` files. The generated DB is
//! checked in.

use std::path::PathBuf;

use rusqlite::{params, Connection};

fn quoted(s: &str) -> String {
    serde_json::to_string(s).expect("serialize string")
}

fn main() -> anyhow::Result<()> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = crate_root.join("tests/fixtures");
    let db_path = fixtures.join("fixture.db");
    let _ = std::fs::remove_file(&db_path);

    let org = |name: &str| fixtures.join(format!("org/{name}.org"));
    let conn = Connection::open(&db_path)?;

    // Schema mirrors org-roam v2 exactly. Source of truth:
    // org-roam-db.el `Schemata` definitions in upstream org-roam.
    // `level`/`pos`/`atime`/`mtime` are real integers; everything
    // else is emacsql-encoded (literal JSON-quoted strings). The
    // `pos = 1` and `hash = "0"` values below are dummies — fixture
    // tests don't exercise file-position or hash logic.
    conn.execute_batch(
        r#"
        CREATE TABLE files (
          file UNIQUE PRIMARY KEY, title , hash NOT NULL,
          atime NOT NULL, mtime NOT NULL
        );
        CREATE TABLE nodes (
          id NOT NULL PRIMARY KEY, file NOT NULL, level NOT NULL,
          pos NOT NULL, todo , priority , scheduled text,
          deadline text, title , properties , olp ,
          FOREIGN KEY (file) REFERENCES files (file) ON DELETE CASCADE
        );
        CREATE TABLE aliases (
          node_id NOT NULL, alias ,
          FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
        );
        CREATE TABLE refs (
          node_id NOT NULL, ref NOT NULL, type NOT NULL,
          FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
        );
        CREATE TABLE tags (
          node_id NOT NULL, tag ,
          FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
        );
        CREATE TABLE links (
          pos NOT NULL, source NOT NULL, dest NOT NULL,
          type NOT NULL, properties NOT NULL,
          FOREIGN KEY (source) REFERENCES nodes (id) ON DELETE CASCADE
        );
        CREATE TABLE citations (
          node_id NOT NULL, cite_key NOT NULL, pos NOT NULL,
          properties ,
          FOREIGN KEY (node_id) REFERENCES nodes (id) ON DELETE CASCADE
        );
        CREATE INDEX alias_node_id ON aliases (node_id);
        CREATE INDEX refs_node_id ON refs (node_id);
        CREATE INDEX tags_node_id ON tags (node_id);
        "#,
    )?;

    let nodes = [
        ("aaaaaaaa-1111-1111-1111-111111111111", "Alpha",  "alpha"),
        ("bbbbbbbb-2222-2222-2222-222222222222", "Beta",   "beta"),
        ("cccccccc-3333-3333-3333-333333333333", "Gamma",  "gamma"),
        ("dddddddd-4444-4444-4444-444444444444", "Orphan", "orphan"),
    ];

    for (id, title, file_stem) in nodes {
        let path = org(file_stem);
        let path_str = path.to_string_lossy().to_string();
        conn.execute(
            "INSERT INTO files (file, title, hash, atime, mtime) VALUES (?, ?, ?, ?, ?)",
            params![quoted(&path_str), quoted(title), quoted("0"), 0_i64, 0_i64],
        )?;
        conn.execute(
            "INSERT INTO nodes (id, file, level, pos, title, properties, olp) VALUES (?, ?, 0, 1, ?, ?, ?)",
            params![quoted(id), quoted(&path_str), quoted(title), quoted("nil"), quoted("nil")],
        )?;
    }

    let tags = [
        ("aaaaaaaa-1111-1111-1111-111111111111", "ai"),
        ("aaaaaaaa-1111-1111-1111-111111111111", "math"),
        ("bbbbbbbb-2222-2222-2222-222222222222", "book"),
        ("cccccccc-3333-3333-3333-333333333333", "ai"),
    ];
    for (node_id, tag) in tags {
        conn.execute(
            "INSERT INTO tags (node_id, tag) VALUES (?, ?)",
            params![quoted(node_id), quoted(tag)],
        )?;
    }

    // Real graph edges: only `id` type counts.
    let id_links = [
        ("aaaaaaaa-1111-1111-1111-111111111111", "bbbbbbbb-2222-2222-2222-222222222222"),
        ("aaaaaaaa-1111-1111-1111-111111111111", "cccccccc-3333-3333-3333-333333333333"),
        ("bbbbbbbb-2222-2222-2222-222222222222", "cccccccc-3333-3333-3333-333333333333"),
    ];
    for (src, dst) in id_links {
        conn.execute(
            "INSERT INTO links (pos, source, dest, type, properties) VALUES (1, ?, ?, ?, ?)",
            params![quoted(src), quoted(dst), quoted("id"), quoted("nil")],
        )?;
    }

    // A non-id link that must NOT appear in the graph.
    conn.execute(
        "INSERT INTO links (pos, source, dest, type, properties) VALUES (1, ?, ?, ?, ?)",
        params![
            quoted("bbbbbbbb-2222-2222-2222-222222222222"),
            quoted("https://example.com"),
            quoted("https"),
            quoted("nil"),
        ],
    )?;

    println!("wrote {}", db_path.display());
    Ok(())
}
