//! HTML rendering with maud (compile-time HTML DSL).

use maud::{html, Markup, DOCTYPE};
use serde_json::Value;

use crate::cache::Probe;

/// The full dashboard page. Each card initiates an `hx-get` on load and
/// then refreshes itself every 5 seconds via `hx-trigger="every 5s"`.
pub fn page() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "org-roam-toolkit dashboard" }
                link rel="stylesheet" href="/assets/style.css";
                script src="/assets/htmx.min.js" defer {}
            }
            body {
                header {
                    h1 { "org-roam-toolkit" }
                    p.subtitle { "auto-refresh every 5s" }
                }
                main {
                    (card_shell("daemon", "Emacs Daemon"))
                    (card_shell("mcp", "MCP Server"))
                    (card_shell("roam-config", "org-roam Config"))
                    (card_shell("graph-stats", "Graph Stats"))
                }
            }
        }
    }
}

/// Initial empty card shell. HTMX swaps its outerHTML when the probe
/// endpoint responds.
fn card_shell(slug: &str, title: &str) -> Markup {
    html! {
        section.card.card-loading
            id=(format!("card-{slug}"))
            hx-get=(format!("/cards/{slug}"))
            hx-trigger="load, every 5s"
            hx-swap="outerHTML"
        {
            h2 { (title) }
            p.placeholder { "loading…" }
        }
    }
}

/// Render a card based on a probe result. The returned markup *replaces*
/// the entire card element (`hx-swap="outerHTML"`), so we re-emit the
/// hx-* attributes so the next refresh keeps polling.
pub fn card(slug: &str, title: &str, probe: &Probe) -> Markup {
    let class = if probe.is_up() { "card card-up" } else { "card card-down" };
    html! {
        section class=(class)
            id=(format!("card-{slug}"))
            hx-get=(format!("/cards/{slug}"))
            hx-trigger="every 5s"
            hx-swap="outerHTML"
        {
            h2 { (title) }
            @match probe {
                Probe::Up { data, probed_at } => {
                    (render_data(slug, data))
                    p.timestamp { "probed " (timestamp(*probed_at)) }
                }
                Probe::Down { error, probed_at } => {
                    p.error { (error) }
                    p.timestamp { "probed " (timestamp(*probed_at)) }
                }
            }
        }
    }
}

fn timestamp(t: chrono::DateTime<chrono::Utc>) -> String {
    t.with_timezone(&chrono::Local).format("%H:%M:%S").to_string()
}

fn render_data(slug: &str, data: &Value) -> Markup {
    match slug {
        "daemon" => render_daemon(data),
        "mcp" => render_mcp(data),
        "roam-config" => render_roam_config(data),
        "graph-stats" => render_graph_stats(data),
        _ => render_unknown(data),
    }
}

fn render_daemon(data: &Value) -> Markup {
    let pid = data.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
    let uptime = data
        .get("uptimeSeconds")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let features = data
        .get("loadedFeatures")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    html! {
        dl {
            dt { "PID" }      dd { (pid) }
            dt { "Uptime" }   dd { (format_uptime(uptime)) }
            dt { "Features" } dd { (features) " loaded" }
        }
    }
}

fn render_mcp(data: &Value) -> Markup {
    let binary = data.get("binary").and_then(|v| v.as_str()).unwrap_or("?");
    let tools = data.get("tools").and_then(|v| v.as_u64()).unwrap_or(0);
    let name = data
        .pointer("/serverInfo/name")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let version = data
        .pointer("/serverInfo/version")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    html! {
        dl {
            dt { "Server" } dd { (name) " v" (version) }
            dt { "Tools" }  dd { (tools) }
            dt { "Binary" } dd.mono { (binary) }
        }
    }
}

fn render_roam_config(data: &Value) -> Markup {
    let dir = data.get("directory").and_then(|v| v.as_str()).unwrap_or("?");
    let db = data.get("dbPath").and_then(|v| v.as_str()).unwrap_or("?");
    html! {
        dl {
            dt { "Directory" } dd.mono { (dir) }
            dt { "Database" }  dd.mono { (db) }
        }
        @if let Some(subs) = data.get("subdirectories").and_then(|v| v.as_array()) {
            p.subtitle { (subs.len()) " subdirectories" }
        }
    }
}

fn render_graph_stats(data: &Value) -> Markup {
    let nodes = data.get("nodes").and_then(|v| v.as_u64()).unwrap_or(0);
    let edges = data.get("edges").and_then(|v| v.as_u64()).unwrap_or(0);
    let orphans = data.get("orphans").and_then(|v| v.as_u64()).unwrap_or(0);
    let tags = data.get("tags").and_then(|v| v.as_u64()).unwrap_or(0);
    html! {
        dl {
            dt { "Nodes" }   dd { (nodes) }
            dt { "Edges" }   dd { (edges) }
            dt { "Orphans" } dd { (orphans) }
            dt { "Tags" }    dd { (tags) }
        }
    }
}

fn render_unknown(data: &Value) -> Markup {
    html! {
        pre { (serde_json::to_string_pretty(data).unwrap_or_default()) }
    }
}

fn format_uptime(seconds: f64) -> String {
    let total = seconds as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}
