//! Axum routes.

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::db::{Db, NodeBrief};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/assets/style.css", get(asset_css))
        .route("/assets/app.js", get(asset_app_js))
        .route("/assets/sigma.min.js", get(asset_sigma_js))
        .route("/assets/graphology.umd.min.js", get(asset_graphology_js))
        .route("/assets/fa2.worker.js", get(asset_fa2_worker))
        .route("/api/graph", get(api_graph))
        .route("/api/node/:id", get(api_node))
        .route("/api/search", get(api_search))
        .route("/events", get(sse_events))
        .route("/file/*path", get(file_proxy))
        .route("/health", get(health))
        .with_state(state)
}

fn open_db(state: &AppState) -> Result<Db, (StatusCode, String)> {
    Db::open(&state.db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("open db: {e}"),
        )
    })
}

async fn api_graph(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let db = open_db(&state)?;
    let g = db
        .graph()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("graph: {e}")))?;
    Ok(Json(json!({
        "nodes": g.nodes,
        "edges": g.edges,
    })))
}

#[derive(Serialize)]
struct NodeFullResponse {
    id: String,
    title: String,
    tags: Vec<String>,
    aliases: Vec<String>,
    file: String,
    #[serde(rename = "fileHtml")]
    file_html: String,
    backlinks: Vec<NodeBrief>,
    forward: Vec<NodeBrief>,
}

async fn api_node(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<NodeFullResponse>, (StatusCode, String)> {
    let db = open_db(&state)?;
    let node = db.node(&id).map_err(|e| (StatusCode::NOT_FOUND, format!("node: {e}")))?;

    let mtime = std::fs::metadata(&node.file)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let path = node.file.clone();
    let org_root = state.org_root.clone();
    let path_for_read = path.clone();
    let html = {
        let mut cache = state.render_cache.lock().expect("cache mutex");
        cache.get_or_render(&path, mtime, &org_root, move || {
            std::fs::read_to_string(&path_for_read).unwrap_or_default()
        })
    };

    Ok(Json(NodeFullResponse {
        id: node.brief.id,
        title: node.brief.title,
        tags: node.brief.tags,
        aliases: node.aliases,
        file: node.file.to_string_lossy().into_owned(),
        file_html: html,
        backlinks: node.backlinks,
        forward: node.forward,
    }))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<u32>,
}

async fn api_search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<NodeBrief>>, (StatusCode, String)> {
    let db = open_db(&state)?;
    let needle = q.q.unwrap_or_default();
    let limit = q.limit.unwrap_or(20);
    let hits = db
        .search_title(&needle, limit)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("search: {e}")))?;
    Ok(Json(hits))
}

// --- Task 12: SSE /events ---

async fn sse_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.reload_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(_) => Some(Ok(Event::default().data("reload"))),
        Err(_) => None, // lagged subscriber: drop and let it catch up
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}

// --- Task 13: /file/*path static proxy with traversal protection ---

fn resolve_under_root(
    org_root: &Path,
    requested: &str,
) -> Result<PathBuf, (StatusCode, String)> {
    let root_canon = org_root
        .canonicalize()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("root canon: {e}")))?;
    let target = root_canon.join(requested);
    let target_canon = target
        .canonicalize()
        .map_err(|_| (StatusCode::NOT_FOUND, "not found".to_string()))?;
    if !target_canon.starts_with(&root_canon) {
        return Err((StatusCode::FORBIDDEN, "outside org root".to_string()));
    }
    Ok(target_canon)
}

async fn file_proxy(
    State(state): State<AppState>,
    AxumPath(rel): AxumPath<String>,
) -> Result<Response, (StatusCode, String)> {
    let path = resolve_under_root(&state.org_root, &rel)?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "not found".to_string()))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    Ok((
        [(header::CONTENT_TYPE, mime.essence_str().to_string())],
        Body::from(bytes),
    )
        .into_response())
}

// --- Task 15: SPA shell + asset routes ---

async fn index() -> maud::Markup {
    crate::views::page()
}

async fn asset_css() -> Response {
    static CSS: &str = include_str!("../assets/style.css");
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], CSS).into_response()
}

async fn asset_app_js() -> Response {
    static JS: &str = include_str!("../assets/app.js");
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        JS,
    )
        .into_response()
}

async fn asset_sigma_js() -> Response {
    static JS: &[u8] = include_bytes!("../assets/sigma.min.js");
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        JS,
    )
        .into_response()
}

async fn asset_graphology_js() -> Response {
    static JS: &[u8] = include_bytes!("../assets/graphology.umd.min.js");
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        JS,
    )
        .into_response()
}

async fn asset_fa2_worker() -> Response {
    static JS: &[u8] = include_bytes!("../assets/fa2.worker.js");
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        JS,
    )
        .into_response()
}

// --- Task 14: /health ---

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (status, nodes, edges) = match Db::open(&state.db_path).and_then(|db| db.graph()) {
        Ok(g) => ("up", g.nodes.len(), g.edges.len()),
        Err(_) => ("down", 0, 0),
    };
    Json(json!({
        "status": status,
        "db": state.db_path.to_string_lossy(),
        "nodes": nodes,
        "edges": edges,
        "startedAt": state.started_at.to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_blocks_traversal() {
        let root = TempDir::new().unwrap();
        std::fs::write(root.path().join("ok.txt"), b"ok").unwrap();
        // `..` escape rejected
        let res = resolve_under_root(root.path(), "../etc/passwd");
        assert!(matches!(res, Err((StatusCode::FORBIDDEN, _)) | Err((StatusCode::NOT_FOUND, _))));
        // legit path resolves
        let ok = resolve_under_root(root.path(), "ok.txt").unwrap();
        assert!(ok.ends_with("ok.txt"));
    }
}
