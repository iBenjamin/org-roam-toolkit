//! 5-second TTL cache per probe name.
//!
//! Not strictly single-flight: if many requests arrive concurrently for a
//! stale key, several may run the probe before the first one stores its
//! result. Acceptable for a 4-probe / 5-second-poll workload; revisit if
//! probe latency becomes a problem.

use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Mutex;

const TTL: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Probe {
    Up {
        data: serde_json::Value,
        #[serde(rename = "probedAt")]
        probed_at: chrono::DateTime<chrono::Utc>,
    },
    Down {
        error: String,
        #[serde(rename = "probedAt")]
        probed_at: chrono::DateTime<chrono::Utc>,
    },
}

impl Probe {
    pub fn up(data: serde_json::Value) -> Self {
        Self::Up {
            data,
            probed_at: chrono::Utc::now(),
        }
    }

    pub fn down(error: impl Into<String>) -> Self {
        Self::Down {
            error: error.into(),
            probed_at: chrono::Utc::now(),
        }
    }

    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up { .. })
    }
}

pub struct ProbeCache {
    inner: Mutex<HashMap<&'static str, (Instant, Probe)>>,
}

impl ProbeCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Return the cached probe value if fresh; otherwise run `f` and cache
    /// the new value.
    pub async fn get_or_run<F, Fut>(&self, key: &'static str, f: F) -> Probe
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Probe>,
    {
        {
            let guard = self.inner.lock().await;
            if let Some((stored_at, value)) = guard.get(key) {
                if stored_at.elapsed() < TTL {
                    return value.clone();
                }
            }
        }

        let value = f().await;
        let mut guard = self.inner.lock().await;
        guard.insert(key, (Instant::now(), value.clone()));
        value
    }
}
