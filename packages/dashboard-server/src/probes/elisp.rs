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
    let fut = Command::new("ortk-emacs-eval")
        .arg(format!("--pkg={pkg}"))
        .arg(expr)
        .output();

    let output = timeout(PROBE_TIMEOUT, fut)
        .await
        .map_err(|_| "ortk-emacs-eval timed out after 5s".to_string())?
        .map_err(|e| format!("failed to spawn ortk-emacs-eval: {e}"))?;

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
    use super::unwrap_elisp_string;

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
}
