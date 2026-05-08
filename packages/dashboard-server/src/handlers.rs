//! Axum routes.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use maud::Markup;
use serde_json::json;

use crate::{cache::Probe, probes, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/cards/:slug", get(card_fragment))
        .route("/api/health", get(health_all))
        .route("/api/health/:slug", get(health_one))
        .route("/assets/htmx.min.js", get(asset_htmx))
        .route("/assets/style.css", get(asset_css))
        .with_state(state)
}

async fn index() -> Markup {
    crate::views::page()
}

async fn card_fragment(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Markup, (StatusCode, &'static str)> {
    let (title, probe) = run(&state, &slug)
        .await
        .ok_or((StatusCode::NOT_FOUND, "unknown probe"))?;
    Ok(crate::views::card(&slug, title, &probe))
}

async fn health_one(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Probe>, (StatusCode, &'static str)> {
    let (_, probe) = run(&state, &slug)
        .await
        .ok_or((StatusCode::NOT_FOUND, "unknown probe"))?;
    Ok(Json(probe))
}

async fn health_all(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (daemon, mcp, roam_config, graph_stats) = tokio::join!(
        state.cache.get_or_run("daemon", probes::probe_daemon),
        state.cache.get_or_run("mcp", probes::probe_mcp),
        state.cache.get_or_run("roam-config", probes::probe_roam_config),
        state.cache.get_or_run("graph-stats", probes::probe_graph_stats),
    );
    Json(json!({
        "daemon": daemon,
        "mcp": mcp,
        "roam-config": roam_config,
        "graph-stats": graph_stats,
    }))
}

/// Dispatch a probe by URL slug. Returns the human-readable card title
/// alongside the probe result so the same dispatch table can serve both
/// HTML and JSON endpoints.
async fn run(state: &AppState, slug: &str) -> Option<(&'static str, Probe)> {
    let cache = &state.cache;
    let (title, probe) = match slug {
        "daemon" => ("Emacs Daemon", cache.get_or_run("daemon", probes::probe_daemon).await),
        "mcp" => ("MCP Server", cache.get_or_run("mcp", probes::probe_mcp).await),
        "roam-config" => (
            "org-roam Config",
            cache.get_or_run("roam-config", probes::probe_roam_config).await,
        ),
        "graph-stats" => (
            "Graph Stats",
            cache.get_or_run("graph-stats", probes::probe_graph_stats).await,
        ),
        _ => return None,
    };
    Some((title, probe))
}

async fn asset_htmx() -> Response {
    static JS: &str = include_str!("../assets/htmx.min.js");
    ([(header::CONTENT_TYPE, "application/javascript; charset=utf-8")], JS).into_response()
}

async fn asset_css() -> Response {
    static CSS: &str = include_str!("../assets/style.css");
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], CSS).into_response()
}
