//! Shared helper for elisp-backed probes.
//!
//! Spawns `ortk-emacs-eval --pkg=PKG "<expr>"`, expects the elisp side to
//! return a JSON-encoded string (via `claude-skill-json-encode`), strips
//! the outer elisp string quoting, and parses the JSON.

use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;
use tokio::time::timeout;

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
    let fut = Command::new(program)
        .arg(format!("--pkg={pkg}"))
        .arg(expr)
        .kill_on_drop(true)
        .output();

    let output = timeout(probe_timeout, fut)
        .await
        .map_err(|_| {
            format!(
                "{program} timed out after {}",
                format_duration(probe_timeout)
            )
        })?
        .map_err(|e| format!("failed to spawn {program}: {e}"))?;

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

    use tokio::time::sleep;

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
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir();
        let script_path = dir.join(format!("ortk-elisp-timeout-{nonce}.sh"));
        let pid_path = dir.join(format!("ortk-elisp-timeout-{nonce}.pid"));

        fs::write(
            &script_path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' $$ > {}\nwhile true; do sleep 1; done\n",
                pid_path.display()
            ),
        )
        .expect("write timeout script");
        Command::new("chmod")
            .arg("+x")
            .arg(&script_path)
            .status()
            .expect("chmod timeout script");

        let result = eval_pkg_json_with_program(
            script_path.to_str().expect("utf8 script path"),
            "org-roam-skill",
            "(never-returns)",
            Duration::from_millis(750),
        )
        .await;

        assert!(result
            .expect_err("probe should time out")
            .contains("timed out"));

        let pid = fs::read_to_string(&pid_path).expect("read child pid");
        sleep(Duration::from_millis(250)).await;
        let still_alive = Command::new("kill")
            .arg("-0")
            .arg(pid.trim())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if still_alive {
            let _ = Command::new("kill").arg("-9").arg(pid.trim()).status();
        }

        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_file(&pid_path);

        assert!(!still_alive, "timed-out ortk-emacs-eval child stayed alive");
    }
}
