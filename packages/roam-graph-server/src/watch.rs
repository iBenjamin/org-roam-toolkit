//! Watch the org-roam DB file. Coalesce SQLite's multi-event writes
//! (journal create / db rewrite / journal delete) into a single
//! `reload` broadcast.

use std::path::Path;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer_opt, Config, DebouncedEventKind};
use tokio::sync::broadcast;

#[derive(Clone, Copy, Debug)]
pub enum ReloadEvent {
    DbChanged,
}

/// Spawn a watcher for `db_path`'s parent directory. Returns a
/// broadcast sender; subscribers get `ReloadEvent` per debounced
/// change to the DB file.
pub fn spawn(
    db_path: &Path,
    debounce: Duration,
) -> anyhow::Result<broadcast::Sender<ReloadEvent>> {
    let (tx, _rx) = broadcast::channel::<ReloadEvent>(16);
    let tx2 = tx.clone();
    let target = db_path.to_path_buf();

    // Watch the *parent directory* because some editors (and
    // SQLite's journal flow) replace the DB file inode rather than
    // editing it in place.
    let parent = db_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("db path has no parent: {db_path:?}"))?
        .to_path_buf();

    // Use a barrier so `spawn()` only returns AFTER `watcher.watch()`
    // has been called. Without this, callers that write the file
    // immediately after `spawn()` returns would miss the event.
    let ready = Arc::new(Barrier::new(2));
    let ready2 = Arc::clone(&ready);

    std::thread::spawn(move || {
        // Disable batch_mode so each debounce window emits exactly ONE event
        // (no AnyContinuous intermediate events) — only the final `Any` fires
        // after `debounce` ms of silence.
        let config = Config::default()
            .with_timeout(debounce)
            .with_batch_mode(false);

        let mut debouncer = match new_debouncer_opt::<_, notify::RecommendedWatcher>(
            config,
            move |res: notify_debouncer_mini::DebounceEventResult| {
                let Ok(events) = res else { return };
                // Filter to final `Any` events touching our target file.
                // `AnyContinuous` events fire mid-stream during sustained writes
                // and would produce duplicate reloads — skip them.
                let target_name = target.file_name();
                if events.iter().any(|e| {
                    e.kind == DebouncedEventKind::Any
                        && (e.path.file_name() == target_name
                            || e.path == target.as_path())
                }) {
                    let _ = tx2.send(ReloadEvent::DbChanged);
                }
            },
        ) as Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>, _>
        {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(error = %e, "watcher init failed");
                ready2.wait(); // unblock caller even on failure
                return;
            }
        };

        if let Err(e) = debouncer
            .watcher()
            .watch(&parent, RecursiveMode::NonRecursive)
        {
            tracing::error!(error = %e, "watcher.watch failed");
            ready2.wait();
            return;
        }

        // Signal caller that the watcher is registered and ready.
        ready2.wait();

        // Park forever — debouncer drops when this thread ends.
        loop {
            std::thread::park();
        }
    });

    // Block until the background thread has called watch().
    ready.wait();

    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::timeout;

    /// Sanity-check: can raw notify detect a file change at all?
    #[test]
    fn raw_notify_detects_write() {
        use notify::{RecommendedWatcher, RecursiveMode, Watcher};

        let dir = TempDir::new().unwrap();
        let db = dir.path().join("org-roam.db");
        fs::write(&db, b"v0").unwrap();

        let (tx, rx) = mpsc::channel::<Result<notify::Event, notify::Error>>();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )
        .unwrap();
        watcher
            .watch(dir.path(), RecursiveMode::NonRecursive)
            .unwrap();

        // Give the watcher time to initialise before writing.
        std::thread::sleep(Duration::from_millis(100));
        fs::write(&db, b"v1").unwrap();

        let got = rx.recv_timeout(Duration::from_secs(3));
        assert!(got.is_ok(), "notify did not fire within 3s: {got:?}");
    }

    #[tokio::test]
    async fn touching_db_emits_one_reload_after_debounce() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("org-roam.db");
        fs::write(&db, b"v0").unwrap();

        let tx = spawn(&db, Duration::from_millis(100)).unwrap();
        let mut rx = tx.subscribe();

        // Rapid-fire 3 writes — debouncer should collapse to 1 event.
        for i in 1..=3 {
            fs::write(&db, format!("v{i}")).unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let first = timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(first.is_ok(), "expected reload within 2s");

        // Confirm no second event arrives in the next 200ms.
        let second = timeout(Duration::from_millis(200), rx.recv()).await;
        assert!(
            second.is_err(),
            "expected ONE event, got second: {second:?}",
        );
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("org-roam.db");
        fs::write(&db, b"v0").unwrap();

        let tx = spawn(&db, Duration::from_millis(100)).unwrap();
        let mut a = tx.subscribe();
        let mut b = tx.subscribe();

        fs::write(&db, b"v1").unwrap();

        // Each subscriber gets its own independent 2-second window.
        let recv_a = timeout(Duration::from_secs(2), a.recv()).await;
        let recv_b = timeout(Duration::from_secs(2), b.recv()).await;
        assert!(recv_a.is_ok(), "subscriber a did not receive: {recv_a:?}");
        assert!(recv_b.is_ok(), "subscriber b did not receive: {recv_b:?}");
    }
}
