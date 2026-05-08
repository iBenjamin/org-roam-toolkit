//! Daemon health: pid, uptime, loaded features.

use crate::cache::Probe;

use super::elisp::eval_pkg_json;

pub async fn probe_daemon() -> Probe {
    match eval_pkg_json("claude-skill-base", "(claude-skill-probe-daemon)").await {
        Ok(data) => Probe::up(data),
        Err(e) => Probe::down(e),
    }
}
