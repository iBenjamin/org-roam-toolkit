//! Probe layer. Each probe spawns a subprocess (`ortk-emacs-eval` or
//! `ortk-mcp` from $PATH) and returns a `Probe` envelope.
//!
//! All four probe entry points are async + cheap to call; expensive work
//! happens inside subprocesses. Caching is the caller's responsibility
//! (see `crate::cache::ProbeCache`).

mod daemon;
mod elisp;
mod graph_stats;
mod mcp;
mod roam_config;

pub use daemon::probe_daemon;
pub use graph_stats::probe_graph_stats;
pub use mcp::probe_mcp;
pub use roam_config::probe_roam_config;
