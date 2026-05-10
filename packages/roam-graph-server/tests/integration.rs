//! End-to-end exercise of every route via axum's tower test client.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

use ortk_roam_graph::{
    handlers, render::Cache, state::AppState, watch::ReloadEvent,
};

fn fixture_state() -> AppState {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (tx, _) = broadcast::channel::<ReloadEvent>(8);
    AppState {
        db_path: crate_root.join("tests/fixtures/fixture.db"),
        org_root: crate_root.join("tests/fixtures/org"),
        render_cache: Arc::new(Mutex::new(Cache::new(8))),
        reload_tx: tx,
        started_at: chrono::Utc::now(),
    }
}

async fn body_to_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn graph_returns_4_nodes_3_edges() {
    let app = handlers::router(fixture_state());
    let resp = app
        .oneshot(Request::builder().uri("/api/graph").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_to_json(resp).await;
    assert_eq!(v["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(v["edges"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn node_returns_full_with_html() {
    let app = handlers::router(fixture_state());
    let id = "cccccccc-3333-3333-3333-333333333333";
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/node/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_to_json(resp).await;
    assert_eq!(v["title"], "Gamma");
    assert_eq!(v["backlinks"].as_array().unwrap().len(), 2);
    assert!(v["fileHtml"].as_str().unwrap().contains("Gamma is a hub"));
}

#[tokio::test]
async fn search_finds_by_substring() {
    let app = handlers::router(fixture_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/search?q=alph")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_to_json(resp).await;
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Alpha");
}

#[tokio::test]
async fn health_reports_up() {
    let app = handlers::router(fixture_state());
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_to_json(resp).await;
    assert_eq!(v["status"], "up");
    assert_eq!(v["nodes"], 4);
    assert_eq!(v["edges"], 3);
}

#[tokio::test]
async fn file_proxy_blocks_traversal() {
    let app = handlers::router(fixture_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/file/../../../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        resp.status(),
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
    ));
}

#[tokio::test]
async fn root_serves_html_with_sigma_script() {
    let app = handlers::router(fixture_state());
    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("sigma.min.js"));
    assert!(body.contains(r#"id="sigma""#));
}
