//! Shared application state passed to axum handlers.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::render::Cache;
use crate::watch::ReloadEvent;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub org_root: PathBuf,
    pub render_cache: Arc<Mutex<Cache>>,
    pub reload_tx: broadcast::Sender<ReloadEvent>,
    pub started_at: chrono::DateTime<chrono::Utc>,
}
