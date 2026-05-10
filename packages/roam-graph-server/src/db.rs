//! rusqlite queries against an org-roam SQLite database.
//!
//! org-roam uses emacsql, which encodes every text value as a literal
//! JSON-quoted string (`"foo"`, not `foo`). We decode via `serde_json`
//! because emacsql's encoding is a subset of JSON strings.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

/// Decode an emacsql-encoded text column. Returns `None` if the value
/// is not surrounded by double quotes.
pub fn unquote(raw: &str) -> Option<String> {
    serde_json::from_str::<String>(raw).ok()
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct NodeBrief {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub orphan: bool,
    pub degree: u32,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Edge {
    pub source: String,
    pub dest: String,
}

#[derive(Debug, Serialize)]
pub struct Graph {
    pub nodes: Vec<NodeBrief>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize)]
pub struct NodeFull {
    pub brief: NodeBrief,
    pub file: PathBuf,
    pub aliases: Vec<String>,
    pub backlinks: Vec<NodeBrief>,
    pub forward: Vec<NodeBrief>,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Self { conn })
    }

    pub fn graph(&self) -> anyhow::Result<Graph> {
        // 1. Pull nodes (id + title)
        let mut stmt = self.conn.prepare(
            "SELECT id, title FROM nodes ORDER BY id",
        )?;
        let mut nodes: Vec<NodeBrief> = stmt
            .query_map([], |row| {
                let raw_id: String = row.get(0)?;
                let raw_title: String = row.get(1)?;
                Ok(NodeBrief {
                    id: unquote(&raw_id).unwrap_or(raw_id),
                    title: unquote(&raw_title).unwrap_or(raw_title),
                    tags: vec![],
                    orphan: false,
                    degree: 0,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        // 2. Tags map
        let mut stmt = self.conn.prepare("SELECT node_id, tag FROM tags")?;
        let tag_rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let tag: String = row.get(1)?;
                Ok((
                    unquote(&id).unwrap_or(id),
                    unquote(&tag).unwrap_or(tag),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        for n in nodes.iter_mut() {
            n.tags = tag_rows
                .iter()
                .filter(|(id, _)| id == &n.id)
                .map(|(_, tag)| tag.clone())
                .collect();
            n.tags.sort();
        }

        // 3. Edges (only type='id')
        let mut stmt = self.conn.prepare(
            "SELECT source, dest FROM links WHERE type = '\"id\"' ORDER BY source, dest",
        )?;
        let edges: Vec<Edge> = stmt
            .query_map([], |row| {
                let s: String = row.get(0)?;
                let d: String = row.get(1)?;
                Ok(Edge {
                    source: unquote(&s).unwrap_or(s),
                    dest: unquote(&d).unwrap_or(d),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        // 4. Degree + orphan
        for n in nodes.iter_mut() {
            let deg = edges
                .iter()
                .filter(|e| e.source == n.id || e.dest == n.id)
                .count() as u32;
            n.degree = deg;
            n.orphan = deg == 0;
        }

        Ok(Graph { nodes, edges })
    }

    pub fn node(&self, id: &str) -> anyhow::Result<NodeFull> {
        let graph = self.graph()?;
        let brief = graph
            .nodes
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("node not found: {id}"))?;

        let file: PathBuf = {
            let mut stmt = self
                .conn
                .prepare("SELECT file FROM nodes WHERE id = ?")?;
            let raw: String = stmt.query_row(
                [&serde_json::to_string(id)?],
                |row| row.get(0),
            )?;
            unquote(&raw).unwrap_or(raw).into()
        };

        let aliases: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT alias FROM aliases WHERE node_id = ?")?;
            let rows = stmt.query_map(
                [&serde_json::to_string(id)?],
                |row| {
                    let raw: String = row.get(0)?;
                    Ok(unquote(&raw).unwrap_or(raw))
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
            rows
        };

        let backlinks: Vec<NodeBrief> = graph
            .edges
            .iter()
            .filter(|e| e.dest == id)
            .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.source).cloned())
            .collect();

        let forward: Vec<NodeBrief> = graph
            .edges
            .iter()
            .filter(|e| e.source == id)
            .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.dest).cloned())
            .collect();

        Ok(NodeFull {
            brief,
            file,
            aliases,
            backlinks,
            forward,
        })
    }

    pub fn search_title(&self, q: &str, limit: u32) -> anyhow::Result<Vec<NodeBrief>> {
        // Fetch the full graph then filter in Rust. For 156 nodes this
        // is faster than crafting a LIKE query that handles the
        // emacsql double-quoting on every text column.
        let graph = self.graph()?;
        let needle = q.to_lowercase();

        // Pull aliases too so they participate in the match.
        let mut alias_stmt = self.conn.prepare(
            "SELECT node_id, alias FROM aliases",
        )?;
        let alias_rows: Vec<(String, String)> = alias_stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let alias: String = row.get(1)?;
                Ok((
                    unquote(&id).unwrap_or(id),
                    unquote(&alias).unwrap_or(alias),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;

        let matched_id_via_alias: std::collections::HashSet<String> = alias_rows
            .iter()
            .filter(|(_, a)| a.to_lowercase().contains(&needle))
            .map(|(id, _)| id.clone())
            .collect();

        let mut out: Vec<NodeBrief> = graph
            .nodes
            .into_iter()
            .filter(|n| {
                n.title.to_lowercase().contains(&needle)
                    || matched_id_via_alias.contains(&n.id)
            })
            .collect();
        out.truncate(limit as usize);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_db_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fixture.db")
    }

    // unquote tests (from Task 2)
    #[test]
    fn unquotes_ascii() {
        assert_eq!(unquote(r#""hello""#), Some("hello".to_string()));
    }
    #[test]
    fn unquotes_escaped_quote() {
        assert_eq!(unquote(r#""a\"b""#), Some(r#"a"b"#.to_string()));
    }
    #[test]
    fn unquotes_chinese() {
        assert_eq!(unquote(r#""概念 X""#), Some("概念 X".to_string()));
    }
    #[test]
    fn unquotes_embedded_newline() {
        assert_eq!(unquote(r#""a\nb""#), Some("a\nb".to_string()));
    }
    #[test]
    fn unquotes_empty_string() {
        assert_eq!(unquote(r#""""#), Some(String::new()));
    }
    #[test]
    fn rejects_unquoted_input() {
        assert_eq!(unquote("hello"), None);
        assert_eq!(unquote("nil"), None);
        assert_eq!(unquote("42"), None);
    }

    #[test]
    fn graph_returns_4_nodes_and_3_id_edges() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        let g = db.graph().expect("graph");
        assert_eq!(g.nodes.len(), 4, "expected 4 nodes, got {:?}", g.nodes);
        assert_eq!(g.edges.len(), 3, "expected 3 id edges, got {:?}", g.edges);
        assert!(
            g.edges.iter().all(|e| e.source.len() == 36 && e.dest.len() == 36),
            "edge endpoints should be unquoted UUIDs"
        );
    }

    #[test]
    fn orphan_flagged_when_no_id_link() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        let g = db.graph().expect("graph");
        let orphan = g
            .nodes
            .iter()
            .find(|n| n.title == "Orphan")
            .expect("Orphan node");
        assert!(orphan.orphan, "Orphan should have orphan=true");
        let alpha = g.nodes.iter().find(|n| n.title == "Alpha").unwrap();
        assert!(!alpha.orphan, "Alpha has 2 outgoing edges");
    }

    #[test]
    fn tags_decoded_per_node() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        let g = db.graph().expect("graph");
        let alpha = g.nodes.iter().find(|n| n.title == "Alpha").unwrap();
        assert_eq!(alpha.tags, vec!["ai".to_string(), "math".to_string()]);
        let orphan = g.nodes.iter().find(|n| n.title == "Orphan").unwrap();
        assert_eq!(orphan.tags, Vec::<String>::new());
    }

    #[test]
    fn https_links_excluded_from_edges() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        let g = db.graph().expect("graph");
        for e in &g.edges {
            assert!(
                !e.dest.starts_with("https"),
                "https links must not appear as graph edges: {:?}",
                e
            );
        }
    }

    #[test]
    fn node_returns_brief_file_aliases_backlinks_forward() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        let gamma = db
            .node("cccccccc-3333-3333-3333-333333333333")
            .expect("node Gamma");
        assert_eq!(gamma.brief.title, "Gamma");
        assert_eq!(gamma.aliases, Vec::<String>::new());
        // Gamma has two backlinks (Alpha, Beta), zero forward edges.
        let bl_titles: Vec<&str> = gamma
            .backlinks
            .iter()
            .map(|n| n.title.as_str())
            .collect();
        assert_eq!(bl_titles.len(), 2);
        assert!(bl_titles.contains(&"Alpha"));
        assert!(bl_titles.contains(&"Beta"));
        assert_eq!(gamma.forward.len(), 0);
        assert!(gamma.file.ends_with("gamma.org"), "file: {:?}", gamma.file);
    }

    #[test]
    fn node_missing_id_returns_none() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        assert!(db.node("does-not-exist").is_err());
    }

    #[test]
    fn search_title_case_insensitive_substring() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        let hits = db.search_title("alph", 10).expect("search");
        let titles: Vec<&str> = hits.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["Alpha"]);
    }

    #[test]
    fn search_title_respects_limit() {
        let db = Db::open(&fixture_db_path()).expect("open fixture");
        // empty pattern matches everything; limit must cap.
        let hits = db.search_title("", 2).expect("search");
        assert_eq!(hits.len(), 2);
    }
}
