//! ortk-roam-graph — local org-roam graph viewer.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::Parser;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use ortk_roam_graph::{
    handlers, render::Cache, state::AppState, watch,
};

#[derive(Debug, Parser)]
#[command(
    name = "ortk-roam-graph",
    about = "Local org-roam graph viewer",
    version
)]
struct Args {
    /// Listening port.
    #[arg(long, default_value_t = 9877)]
    port: u16,

    /// Listening host (loopback by default; do not expose publicly).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Path to org-roam.db. Resolution: --db > $ORTK_ROAM_DB > ortk-emacs-eval probe-config > ~/.emacs.d/org-roam.db
    #[arg(long)]
    db: Option<PathBuf>,

    /// Path to org-roam-directory. Same fallback chain as --db (uses
    /// the parent of the resolved DB if not provided).
    #[arg(long = "org-dir")]
    org_dir: Option<PathBuf>,

    /// Open the URL in the default browser after startup.
    #[arg(long)]
    open: bool,
}

fn resolve_db(arg: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p);
    }
    if let Ok(p) = std::env::var("ORTK_ROAM_DB") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(out) = std::process::Command::new("ortk-emacs-eval")
        .args([
            "--pkg=org-roam-skill",
            "(org-roam-skill-probe-config)",
        ])
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // probe-config returns a JSON-encoded string; pull dbPath out.
            if let Ok(json_str) = serde_json::from_str::<String>(stdout.trim()) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(db) = v.get("dbPath").and_then(|s| s.as_str()) {
                        if !db.is_empty() {
                            return Ok(PathBuf::from(db));
                        }
                    }
                }
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PathBuf::from(format!("{home}/.emacs.d/org-roam.db")))
}

fn resolve_org_dir(arg: Option<PathBuf>, db: &PathBuf) -> PathBuf {
    if let Some(p) = arg {
        return p;
    }
    if let Ok(p) = std::env::var("ORTK_ORG_ROAM_DIR") {
        return PathBuf::from(p);
    }
    db.parent().unwrap_or(std::path::Path::new(".")).to_path_buf()
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
    let db_path = resolve_db(args.db)?;
    if !db_path.exists() {
        eprintln!(
            "ortk-roam-graph: db file does not exist: {}\nset --db, $ORTK_ROAM_DB, or run org-roam-db-sync in Emacs.",
            db_path.display(),
        );
        std::process::exit(1);
    }
    let org_root = resolve_org_dir(args.org_dir, &db_path);

    let reload_tx = watch::spawn(&db_path, Duration::from_millis(250))?;
    let state = AppState {
        db_path,
        org_root,
        render_cache: Arc::new(Mutex::new(Cache::new(32))),
        reload_tx,
        started_at: chrono::Utc::now(),
    };

    let app = handlers::router(state);
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("ortk-roam-graph listening on http://{addr}");

    if args.open {
        let url = format!("http://{addr}");
        let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" })
            .arg(url)
            .spawn();
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
