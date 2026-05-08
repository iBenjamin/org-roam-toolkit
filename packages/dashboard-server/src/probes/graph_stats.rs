//! org-roam graph statistics: nodes, edges, orphans, tags.

use crate::cache::Probe;

use super::elisp::eval_pkg_json;

pub async fn probe_graph_stats() -> Probe {
    match eval_pkg_json("org-roam-skill", "(org-roam-skill-probe-graph-stats)").await {
        Ok(data) => Probe::up(data),
        Err(e) => Probe::down(e),
    }
}
