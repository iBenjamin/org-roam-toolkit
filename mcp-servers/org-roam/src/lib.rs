use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};

pub const SERVER_NAME: &str = "org-roam";
pub const SERVER_VERSION: &str = "0.1.0";

const ORG_ROAM_PKG: &str = "org-roam-skill";
const EVAL_TIMEOUT: Duration = Duration::from_secs(30);
const DAEMON_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub trait EmacsClient {
    fn is_daemon_running(&self) -> bool;
    fn eval_elisp(&self, pkg: &str, expr: &str, timeout: Duration) -> Result<String, String>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RealEmacsClient;

impl EmacsClient for RealEmacsClient {
    fn is_daemon_running(&self) -> bool {
        run_command("emacsclient", &["--eval", "t"], DAEMON_TIMEOUT)
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn eval_elisp(&self, pkg: &str, expr: &str, timeout: Duration) -> Result<String, String> {
        let pkg_arg = format!("--pkg={pkg}");
        let output = run_command("ortk-emacs-eval", &[pkg_arg.as_str(), expr], timeout)?;
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
        Ok(parse_elisp_result_to_text(&stdout))
    }
}

fn run_command(program: &str, args: &[&str], timeout: Duration) -> Result<Output, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn {program}: {e}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("failed to capture {program} stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("failed to capture {program} stderr"))?;

    let stdout_handle = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let stderr_handle = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_handle
                    .join()
                    .map_err(|_| format!("failed to join {program} stdout reader"))?;
                let stderr = stderr_handle
                    .join()
                    .map_err(|_| format!("failed to join {program} stderr reader"))?;
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                return Err(format!("{program} timed out after {}s", timeout.as_secs()));
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(format!("failed to wait for {program}: {e}")),
        }
    }
}

fn parse_elisp_result_to_text(output: &str) -> String {
    let trimmed = output.trim();

    if trimmed == "nil" {
        return "null".to_string();
    }
    if trimmed == "t" {
        return "true".to_string();
    }
    if let Some(unquoted) = unwrap_elisp_string(trimmed) {
        return unquoted;
    }

    trimmed.to_string()
}

fn unwrap_elisp_string(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }

    Some(out)
}

pub fn quote_elisp_string(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn keyword_args(items: Vec<(&str, Option<&Value>)>) -> String {
    items
        .into_iter()
        .filter_map(|(keyword, value)| {
            value.map(|value| format!(":{keyword} {}", keyword_value(value)))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn keyword_value(value: &Value) -> String {
    match value {
        Value::String(s) => quote_elisp_string(s),
        Value::Bool(b) => {
            if *b {
                "t".to_string()
            } else {
                "nil".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Array(values) => {
            let items = values
                .iter()
                .map(|v| match v {
                    Value::String(s) => quote_elisp_string(s),
                    other => json_value_to_js_string(other),
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("'({items})")
        }
        Value::Object(values) => {
            let items = values
                .iter()
                .map(|(key, value)| {
                    format!(
                        "({} . {})",
                        quote_elisp_string(key),
                        quote_elisp_string(&json_value_to_js_string(value)),
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("'({items})")
        }
        Value::Null => "nil".to_string(),
    }
}

fn json_value_to_js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| String::new()),
    }
}

fn args_object(args: &Value) -> Result<&Map<String, Value>, String> {
    args.as_object()
        .ok_or_else(|| "tool arguments must be a JSON object".to_string())
}

fn string_arg<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing required string argument: {key}"))
}

fn bool_arg(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn build_tool_expression(name: &str, args: Value) -> Result<String, String> {
    let args = args_object(&args)?;

    match name {
        "roam_create_note" => {
            let title = string_arg(args, "title")?;
            let kw = keyword_args(vec![
                ("tags", args.get("tags")),
                ("content", args.get("content")),
                ("subdirectory", args.get("subdirectory")),
                ("source-url", args.get("sourceUrl")),
                ("open-archive", args.get("openArchive")),
                ("properties", args.get("properties")),
            ]);
            Ok(format!(
                "(org-roam-skill-create-note {}{})",
                quote_elisp_string(title),
                keyword_suffix(&kw),
            ))
        }
        "roam_search_title" => Ok(format!(
            "(org-roam-skill-search-by-title {})",
            quote_elisp_string(string_arg(args, "query")?),
        )),
        "roam_search_tag" => Ok(format!(
            "(org-roam-skill-search-by-tag {})",
            quote_elisp_string(string_arg(args, "tag")?),
        )),
        "roam_search_content" => Ok(format!(
            "(org-roam-skill-search-by-content {})",
            quote_elisp_string(string_arg(args, "query")?),
        )),
        "roam_get_backlinks" => Ok(format!(
            "(org-roam-skill-get-backlinks-by-title {})",
            quote_elisp_string(string_arg(args, "title")?),
        )),
        "roam_create_link" => {
            let function = if bool_arg(args, "bidirectional") {
                "org-roam-skill-create-bidirectional-link"
            } else {
                "org-roam-skill-insert-link-in-note"
            };
            Ok(format!(
                "({function} {} {})",
                quote_elisp_string(string_arg(args, "source")?),
                quote_elisp_string(string_arg(args, "target")?),
            ))
        }
        "roam_add_reading_history" => {
            let title = string_arg(args, "title")?;
            let url = string_arg(args, "url")?;
            let kw = keyword_args(vec![
                ("tags", args.get("tags")),
                ("source", args.get("source")),
                ("author", args.get("author")),
                ("summary", args.get("summary")),
                ("points", args.get("points")),
                ("rating", args.get("rating")),
            ]);
            Ok(format!(
                "(org-roam-skill-add-reading-history {} {}{})",
                quote_elisp_string(title),
                quote_elisp_string(url),
                keyword_suffix(&kw),
            ))
        }
        "roam_add_toolkit" => {
            let title = string_arg(args, "title")?;
            let url = string_arg(args, "url")?;
            let kw = keyword_args(vec![
                ("tags", args.get("tags")),
                ("category", args.get("category")),
                ("description", args.get("description")),
            ]);
            Ok(format!(
                "(org-roam-skill-add-toolkit-resource {} {}{})",
                quote_elisp_string(title),
                quote_elisp_string(url),
                keyword_suffix(&kw),
            ))
        }
        "roam_add_to_read" => {
            let title = string_arg(args, "title")?;
            let url = string_arg(args, "url")?;
            let kw = keyword_args(vec![("summary", args.get("summary"))]);
            Ok(format!(
                "(org-roam-skill-add-to-read {} {}{})",
                quote_elisp_string(title),
                quote_elisp_string(url),
                keyword_suffix(&kw),
            ))
        }
        "roam_list_tags" => Ok("(org-roam-skill-list-all-tags)".to_string()),
        "roam_doctor" => Ok("(org-roam-doctor)".to_string()),
        other => Err(format!("Unknown tool: {other}")),
    }
}

fn keyword_suffix(keyword_args: &str) -> String {
    if keyword_args.is_empty() {
        String::new()
    } else {
        format!(" {keyword_args}")
    }
}

pub fn tool_defs() -> Vec<Value> {
    vec![
        json!({
            "name": "roam_create_note",
            "description": "Create a new org-roam note. Returns the file path of created note.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Note title (required)" },
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "List of tags for the note" },
                    "content": { "type": "string", "description": "Initial content in org-mode format" },
                    "subdirectory": {
                        "type": "string",
                        "enum": ["main", "reference", "projects", "daily"],
                        "description": "Subdirectory within org-roam-directory (default: main)"
                    },
                    "sourceUrl": {
                        "type": "string",
                        "description": "Original URL for reference notes (auto-generates References section)"
                    },
                    "openArchive": {
                        "type": "boolean",
                        "description": "Open archive.today submission in browser (default: true for reference notes)"
                    },
                    "properties": {
                        "type": "object",
                        "additionalProperties": { "type": "string" },
                        "description": "Additional PROPERTIES drawer entries as key-value pairs"
                    }
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "roam_search_title",
            "description": "Search org-roam notes by title (partial match). Returns list of [id, title, file] tuples.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search term to match in note titles" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "roam_search_tag",
            "description": "Search org-roam notes by tag. Returns list of [id, title, file, tags] tuples.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tag": { "type": "string", "description": "Tag to search for" }
                },
                "required": ["tag"]
            }
        }),
        json!({
            "name": "roam_search_content",
            "description": "Search org-roam notes by content (full-text). Returns list of [id, title, file] tuples.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search term to find in note content" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "roam_get_backlinks",
            "description": "Get notes that link TO the specified note. Returns list of [id, title, file] tuples.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Title of the note to find backlinks for" }
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "roam_create_link",
            "description": "Create links between two notes. Can create bidirectional links.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Title of the source note" },
                    "target": { "type": "string", "description": "Title of the target note to link to" },
                    "bidirectional": {
                        "type": "boolean",
                        "description": "Create links in both directions (default: false)"
                    }
                },
                "required": ["source", "target"]
            }
        }),
        json!({
            "name": "roam_add_reading_history",
            "description": "Add an entry to the quarterly reading history log. NOT an org-roam node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Article title" },
                    "url": { "type": "string", "description": "Source URL" },
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Classification tags" },
                    "source": { "type": "string", "description": "Website name (e.g., cnblogs, github)" },
                    "author": { "type": "string", "description": "Author name" },
                    "summary": { "type": "string", "description": "One-line summary" },
                    "points": { "type": "array", "items": { "type": "string" }, "description": "Key points from the article" },
                    "rating": { "type": "number", "minimum": 1, "maximum": 5, "description": "Rating 1-5" }
                },
                "required": ["title", "url"]
            }
        }),
        json!({
            "name": "roam_add_toolkit",
            "description": "Add a resource to the quarterly toolkit collection. NOT an org-roam node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Resource name" },
                    "url": { "type": "string", "description": "Resource URL" },
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Classification tags" },
                    "category": {
                        "type": "string",
                        "enum": ["library", "tool", "service", "api"],
                        "description": "Resource type"
                    },
                    "description": { "type": "string", "description": "One-line description" }
                },
                "required": ["title", "url"]
            }
        }),
        json!({
            "name": "roam_add_to_read",
            "description": "Add a TODO item to read later under * Inbox in the read-later file (controlled by `org-roam-skill-to-read-file`; defaults to todo.org alongside org-roam-directory).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Article title" },
                    "url": { "type": "string", "description": "Link to read later" },
                    "summary": { "type": "string", "description": "Brief description of what it is about" }
                },
                "required": ["title", "url"]
            }
        }),
        json!({
            "name": "roam_list_tags",
            "description": "List all unique tags across all org-roam notes.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "roam_doctor",
            "description": "Run comprehensive diagnostic check of org-roam setup.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ]
}

pub fn resource_defs() -> Vec<Value> {
    vec![
        json!({
            "uri": "health://daemon",
            "name": "Emacs daemon health",
            "description": "Liveness, pid, uptime, loaded features",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "health://mcp",
            "name": "MCP server self-health",
            "description": "This server's own metadata (version, uptime, tool count)",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "config://org-roam",
            "name": "org-roam configuration",
            "description": "org-roam-directory, db path/size, subdirectories",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "stats://graph",
            "name": "org-roam graph stats",
            "description": "Node count, edge count, orphans, tags",
            "mimeType": "application/json"
        }),
    ]
}

pub fn handle_json_rpc_message(message: &Value) -> Option<Value> {
    let emacs = DisabledEmacsClient;
    handle_json_rpc_message_with_emacs(message, &emacs, &Instant::now())
}

pub fn handle_json_rpc_message_with_emacs<C: EmacsClient>(
    message: &Value,
    emacs: &C,
    server_start: &Instant,
) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str);

    let Some(method) = method else {
        return id.map(|id| error_response(id, -32600, "Invalid Request", None));
    };

    let id = id?;
    let result = match method {
        "initialize" => initialize_result(message),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_defs() })),
        "resources/list" => Ok(json!({ "resources": resource_defs() })),
        "resources/read" => read_resource_result(message, emacs, server_start),
        "tools/call" => Ok(call_tool_result(message, emacs)),
        _ => {
            return Some(error_response(
                id,
                -32601,
                &format!("Method not found: {method}"),
                None,
            ))
        }
    };

    Some(match result {
        Ok(result) => success_response(id, result),
        Err((code, message, data)) => error_response(id, code, &message, data),
    })
}

fn initialize_result(message: &Value) -> Result<Value, (i64, String, Option<Value>)> {
    let protocol_version = message
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("2024-11-05");

    Ok(json!({
        "protocolVersion": protocol_version,
        "capabilities": { "tools": {}, "resources": {} },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
    }))
}

fn call_tool_result<C: EmacsClient>(message: &Value, emacs: &C) -> Value {
    let name = message
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let args = message
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    if !emacs.is_daemon_running() {
        return tool_error("Error: Emacs daemon is not running. Start it with: emacs --daemon");
    }

    let expr = match build_tool_expression(name, args) {
        Ok(expr) => expr,
        Err(e) => return tool_error(e),
    };

    match emacs.eval_elisp(ORG_ROAM_PKG, &expr, EVAL_TIMEOUT) {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
        Err(e) => tool_error(format!("Error: {e}")),
    }
}

fn tool_error(message: impl Into<String>) -> Value {
    json!({
        "content": [{ "type": "text", "text": message.into() }],
        "isError": true
    })
}

fn read_resource_result<C: EmacsClient>(
    message: &Value,
    emacs: &C,
    server_start: &Instant,
) -> Result<Value, (i64, String, Option<Value>)> {
    let uri = message
        .pointer("/params/uri")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            (
                -32602,
                "resources/read missing params.uri".to_string(),
                None,
            )
        })?;

    let probe = match uri {
        "health://daemon" => probe_daemon(emacs),
        "health://mcp" => probe_up(json!({
            "name": SERVER_NAME,
            "version": SERVER_VERSION,
            "uptimeSeconds": server_start.elapsed().as_secs_f64(),
            "tools": tool_defs().len()
        })),
        "config://org-roam" => probe_pkg_json(emacs, ORG_ROAM_PKG, "(org-roam-skill-probe-config)"),
        "stats://graph" => {
            probe_pkg_json(emacs, ORG_ROAM_PKG, "(org-roam-skill-probe-graph-stats)")
        }
        _ => {
            return Err((
                -32602,
                format!("Unknown resource URI: {uri}"),
                Some(json!({ "uri": uri })),
            ))
        }
    };

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string(&probe).unwrap_or_else(|_| "{}".to_string())
        }]
    }))
}

fn probe_daemon<C: EmacsClient>(emacs: &C) -> Value {
    if !emacs.is_daemon_running() {
        return probe_down("emacs daemon not reachable (emacsclient -e t failed)");
    }
    probe_pkg_json(emacs, "claude-skill-base", "(claude-skill-probe-daemon)")
}

fn probe_pkg_json<C: EmacsClient>(emacs: &C, pkg: &str, expr: &str) -> Value {
    match emacs.eval_elisp(pkg, expr, PROBE_TIMEOUT) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(data) => probe_up(data),
            Err(e) => probe_down(format!("invalid JSON from elisp: {e}")),
        },
        Err(e) => probe_down(e),
    }
}

fn probe_up(data: Value) -> Value {
    json!({
        "status": "up",
        "data": data,
        "probedAt": now_iso()
    })
}

fn probe_down(error: impl Into<String>) -> Value {
    json!({
        "status": "down",
        "error": error.into(),
        "probedAt": now_iso()
    })
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message
    });
    if let Some(data) = data {
        error["data"] = data;
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error
    })
}

pub fn parse_error_response() -> Value {
    error_response(Value::Null, -32700, "Parse error", None)
}

#[derive(Clone, Copy, Debug)]
struct DisabledEmacsClient;

impl EmacsClient for DisabledEmacsClient {
    fn is_daemon_running(&self) -> bool {
        false
    }

    fn eval_elisp(&self, _pkg: &str, _expr: &str, _timeout: Duration) -> Result<String, String> {
        Err("disabled emacs client".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{parse_elisp_result_to_text, run_command, unwrap_elisp_string};

    #[test]
    fn parses_elisp_scalar_results_to_tool_text() {
        assert_eq!(parse_elisp_result_to_text("nil"), "null");
        assert_eq!(parse_elisp_result_to_text("t"), "true");
        assert_eq!(parse_elisp_result_to_text("42"), "42");
        assert_eq!(parse_elisp_result_to_text("(1 2 3)"), "(1 2 3)");
    }

    #[test]
    fn unwraps_elisp_string_escapes() {
        assert_eq!(unwrap_elisp_string(r#""hello""#).as_deref(), Some("hello"));
        assert_eq!(unwrap_elisp_string(r#""a\"b""#).as_deref(), Some(r#"a"b"#));
        assert_eq!(unwrap_elisp_string(r#""a\\b""#).as_deref(), Some(r#"a\b"#));
        assert_eq!(
            unwrap_elisp_string(r#""line1\nline2""#).as_deref(),
            Some("line1\nline2"),
        );
    }

    #[test]
    fn run_command_drains_large_stdout_while_waiting() {
        let script = r#"i=0; while [ "$i" -lt 5000 ]; do printf 'abcdefghijklmnopqrstuvwxyz0123456789\n'; i=$((i + 1)); done"#;
        let output = run_command("/bin/sh", &["-c", script], Duration::from_secs(5)).unwrap();

        assert!(output.status.success());
        assert!(output.stdout.len() > 100_000);
    }
}
