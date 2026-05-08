//! ortk-dashboard — local observability dashboard for the org-roam-toolkit.
//!
//! Single static binary. Serves an HTMX-driven HTML dashboard on
//! 127.0.0.1:9876 (by default) plus JSON `/api/health[/...]` endpoints.

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

mod cache;
mod handlers;
mod probes;
mod views;

use cache::ProbeCache;

#[derive(Debug, Parser)]
#[command(
    name = "ortk-dashboard",
    about = "Local observability dashboard for the org-roam-toolkit",
    version,
)]
struct Args {
    /// Listening port.
    #[arg(long, default_value_t = 9876)]
    port: u16,

    /// Listening host (loopback by default; do not expose publicly).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<ProbeCache>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    let state = AppState {
        cache: Arc::new(ProbeCache::new()),
    };

    let app = handlers::router(state);
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("ortk-dashboard listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown requested, draining…");
}
