//! org-roam configuration: directory, db path, subdirectories.

use crate::cache::Probe;

use super::elisp::eval_pkg_json;

pub async fn probe_roam_config() -> Probe {
    match eval_pkg_json("org-roam-skill", "(org-roam-skill-probe-config)").await {
        Ok(data) => Probe::up(data),
        Err(e) => Probe::down(e),
    }
}
