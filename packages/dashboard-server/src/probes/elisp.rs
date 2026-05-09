//! Shared helper for elisp-backed probes.
//!
//! Spawns `ortk-emacs-eval --pkg=PKG "<expr>"`, expects the elisp side to
//! return a JSON-encoded string (via `claude-skill-json-encode`), strips
//! the outer elisp string quoting, and parses the JSON.

use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn eval_pkg_json(pkg: &str, expr: &str) -> Result<Value, String> {
    eval_pkg_json_with_program("ortk-emacs-eval", pkg, expr, PROBE_TIMEOUT).await
}

async fn eval_pkg_json_with_program(
    program: &str,
    pkg: &str,
    expr: &str,
    probe_timeout: Duration,
) -> Result<Value, String> {
    // Spawn into a fresh process group so we can SIGKILL the whole tree on
    // timeout. Without this, killing only the direct child (the wrapper
    // shell) leaves its emacsclient grandchild orphaned to PID 1 — when
    // the daemon's eval queue is hung, every probe cycle leaks 3 of them.
    let child = Command::new(program)
        .arg(format!("--pkg={pkg}"))
        .arg(expr)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("failed to spawn {program}: {e}"))?;

    let pid = child
        .id()
        .ok_or_else(|| format!("{program} has no pid"))? as i32;

    let wait_fut = child.wait_with_output();
    tokio::pin!(wait_fut);

    let output = tokio::select! {
        res = &mut wait_fut => {
            res.map_err(|e| format!("waiting on {program}: {e}"))?
        }
        _ = tokio::time::sleep(probe_timeout) => {
            // SIGKILL -PGID kills the wrapper bash AND its emacsclient
            // grandchild atomically; bash's own SIGTERM trap (if it ran)
            // would also clean up, but we don't depend on it.
            unsafe { libc::kill(-pid, libc::SIGKILL) };
            // Drain the future so the kernel reaps our zombie child.
            let _ = wait_fut.await;
            return Err(format!(
                "{program} timed out after {}",
                format_duration(probe_timeout)
            ));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let trimmed = stderr.trim();
        return Err(if trimmed.is_empty() {
            format!("ortk-emacs-eval exited with status {}", output.status)
        } else {
            trimmed.to_string()
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    if trimmed == "nil" {
        return Err("elisp returned nil".to_string());
    }

    let json_str = unwrap_elisp_string(trimmed)
        .ok_or_else(|| format!("expected elisp-quoted JSON string, got: {trimmed}"))?;

    serde_json::from_str(&json_str).map_err(|e| format!("invalid JSON from elisp: {e}"))
}

fn format_duration(duration: Duration) -> String {
    if duration.subsec_millis() == 0 {
        format!("{}s", duration.as_secs())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

/// Strip outer `"..."` from an elisp-printed string and unescape the
/// common escape sequences (`\\`, `\"`, `\n`).
fn unwrap_elisp_string(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::{Command, Stdio};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use tokio::time::{sleep, Instant};

    use super::{eval_pkg_json_with_program, unwrap_elisp_string};

    #[test]
    fn unwraps_simple_string() {
        assert_eq!(unwrap_elisp_string(r#""hello""#).as_deref(), Some("hello"));
    }

    #[test]
    fn unescapes_quotes() {
        assert_eq!(
            unwrap_elisp_string(r#""{\"k\":1}""#).as_deref(),
            Some(r#"{"k":1}"#),
        );
    }

    #[test]
    fn rejects_unquoted() {
        assert!(unwrap_elisp_string("nil").is_none());
        assert!(unwrap_elisp_string("42").is_none());
    }

    #[tokio::test]
    async fn kills_probe_child_when_timeout_fires() {
        // Regression test for the leak that motivated process_group(0):
        // the wrapper shell spawns a long-running grandchild (emacsclient,
        // simulated here with `sleep`); when the probe times out, the
        // *whole tree* must die — killing only the direct child reparents
        // grandchildren to PID 1, which is the bug we're fixing.
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir();
        let script_path = dir.join(format!("ortk-elisp-timeout-{nonce}.sh"));
        let parent_pid_path = dir.join(format!("ortk-elisp-timeout-{nonce}.parent"));
        let child_pid_path = dir.join(format!("ortk-elisp-timeout-{nonce}.child"));

        fs::write(
            &script_path,
            format!(
                "#!/bin/sh\n\
                 printf '%s\\n' $$ > {parent}\n\
                 sleep 600 &\n\
                 printf '%s\\n' $! > {child}\n\
                 wait\n",
                parent = parent_pid_path.display(),
                child = child_pid_path.display(),
            ),
        )
        .expect("write timeout script");
        Command::new("chmod")
            .arg("+x")
            .arg(&script_path)
            .status()
            .expect("chmod timeout script");

        let program = script_path.to_str().expect("utf8 script path").to_string();
        let probe = tokio::spawn(async move {
            eval_pkg_json_with_program(
                &program,
                "org-roam-skill",
                "(never-returns)",
                Duration::from_millis(750),
            )
            .await
        });

        let readiness_deadline = Instant::now() + Duration::from_secs(2);
        while !(parent_pid_path.exists() && child_pid_path.exists())
            && Instant::now() < readiness_deadline
        {
            sleep(Duration::from_millis(25)).await;
        }

        assert!(
            parent_pid_path.exists() && child_pid_path.exists(),
            "timeout script did not write both pids before the readiness deadline"
        );

        let result = probe.await.expect("probe task panicked");

        assert!(result
            .expect_err("probe should time out")
            .contains("timed out"));

        sleep(Duration::from_millis(250)).await;

        let parent_pid = fs::read_to_string(&parent_pid_path).expect("read parent pid");
        let child_pid = fs::read_to_string(&child_pid_path).expect("read child pid");
        let parent_alive = is_alive(parent_pid.trim());
        let child_alive = is_alive(child_pid.trim());

        // Best-effort cleanup before assertions, so a failed test doesn't
        // leak `sleep 600` processes.
        if parent_alive {
            let _ = Command::new("kill").arg("-9").arg(parent_pid.trim()).status();
        }
        if child_alive {
            let _ = Command::new("kill").arg("-9").arg(child_pid.trim()).status();
        }
        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_file(&parent_pid_path);
        let _ = fs::remove_file(&child_pid_path);

        assert!(!parent_alive, "timed-out wrapper shell stayed alive");
        assert!(
            !child_alive,
            "timed-out wrapper grandchild (emacsclient simulator) stayed alive — process group leak"
        );
    }

    fn is_alive(pid: &str) -> bool {
        Command::new("kill")
            .arg("-0")
            .arg(pid)
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}
