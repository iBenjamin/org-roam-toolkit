use ortk_mcp::{
    build_tool_expression, handle_json_rpc_message, handle_json_rpc_message_with_emacs,
    quote_elisp_string, resource_defs, tool_defs, EmacsClient, SERVER_NAME, SERVER_VERSION,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[test]
fn quotes_elisp_strings_like_the_typescript_server() {
    assert_eq!(quote_elisp_string(r#"a"b"#), r#""a\"b""#);
    assert_eq!(quote_elisp_string(r#"a\b"#), r#""a\\b""#);
}

#[test]
fn exposes_the_existing_tool_catalog() {
    let tools = tool_defs();
    let names: Vec<_> = tools
        .iter()
        .map(|tool| tool.get("name").and_then(Value::as_str).unwrap())
        .collect();

    assert_eq!(
        names,
        vec![
            "roam_create_note",
            "roam_search_title",
            "roam_search_tag",
            "roam_search_content",
            "roam_get_backlinks",
            "roam_create_link",
            "roam_add_reading_history",
            "roam_add_toolkit",
            "roam_add_to_read",
            "roam_list_tags",
            "roam_doctor",
        ],
    );

    let create_note = &tools[0];
    assert_eq!(
        create_note.pointer("/inputSchema/required/0"),
        Some(&json!("title")),
    );
    assert_eq!(
        create_note.pointer("/inputSchema/properties/subdirectory/enum"),
        Some(&json!(["main", "reference", "projects", "daily"])),
    );
}

#[test]
fn exposes_the_existing_resource_catalog() {
    let resources = resource_defs();
    let uris: Vec<_> = resources
        .iter()
        .map(|resource| resource.get("uri").and_then(Value::as_str).unwrap())
        .collect();

    assert_eq!(
        uris,
        vec![
            "health://daemon",
            "health://mcp",
            "config://org-roam",
            "stats://graph",
        ],
    );

    assert!(resources
        .iter()
        .all(|resource| resource.get("mimeType") == Some(&json!("application/json"))));
}

#[test]
fn builds_create_note_elisp_expression_with_keyword_arguments() {
    let expr = build_tool_expression(
        "roam_create_note",
        json!({
            "title": "Rust MCP",
            "tags": ["mcp", "rust"],
            "content": "body",
            "subdirectory": "projects",
            "sourceUrl": "https://example.com/a?b=1",
            "openArchive": false,
            "properties": { "SOURCE": "test" }
        }),
    )
    .unwrap();

    assert_eq!(
        expr,
        r#"(org-roam-skill-create-note "Rust MCP" :tags '("mcp" "rust") :content "body" :subdirectory "projects" :source-url "https://example.com/a?b=1" :open-archive nil :properties '(("SOURCE" . "test")))"#,
    );
}

#[test]
fn builds_tool_specific_elisp_expressions() {
    assert_eq!(
        build_tool_expression("roam_search_title", json!({ "query": "mcp" })).unwrap(),
        r#"(org-roam-skill-search-by-title "mcp")"#,
    );
    assert_eq!(
        build_tool_expression(
            "roam_create_link",
            json!({ "source": "A", "target": "B", "bidirectional": true }),
        )
        .unwrap(),
        r#"(org-roam-skill-create-bidirectional-link "A" "B")"#,
    );
    assert_eq!(
        build_tool_expression("roam_doctor", json!({})).unwrap(),
        "(org-roam-doctor)",
    );
}

#[test]
fn handles_initialize_tools_and_resource_list_json_rpc_messages() {
    let init = handle_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" }
        }
    }))
    .unwrap();

    assert_eq!(init.get("id"), Some(&json!(1)));
    assert_eq!(
        init.pointer("/result/serverInfo"),
        Some(&json!({ "name": SERVER_NAME, "version": SERVER_VERSION })),
    );
    assert_eq!(
        init.pointer("/result/capabilities"),
        Some(&json!({ "tools": {}, "resources": {} })),
    );

    let tools = handle_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": "tools",
        "method": "tools/list"
    }))
    .unwrap();
    assert_eq!(
        tools.pointer("/result/tools/0/name"),
        Some(&json!("roam_create_note"))
    );

    let resources = handle_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": "resources",
        "method": "resources/list"
    }))
    .unwrap();
    assert_eq!(
        resources.pointer("/result/resources/0/uri"),
        Some(&json!("health://daemon")),
    );
}

#[test]
fn handles_tool_calls_through_the_emacs_boundary() {
    let emacs = FakeEmacs {
        daemon_running: true,
        eval_result: Ok("created.org".to_string()),
    };
    let response = handle_json_rpc_message_with_emacs(
        &json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "roam_create_note",
                "arguments": { "title": "Rust" }
            }
        }),
        &emacs,
        &Instant::now(),
    )
    .unwrap();

    assert_eq!(
        response.pointer("/result/content/0/text"),
        Some(&json!("created.org")),
    );
    assert_eq!(response.pointer("/result/isError"), None);
}

#[test]
fn reports_tool_call_error_when_daemon_is_down() {
    let emacs = FakeEmacs {
        daemon_running: false,
        eval_result: Ok("unused".to_string()),
    };
    let response = handle_json_rpc_message_with_emacs(
        &json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "roam_list_tags",
                "arguments": {}
            }
        }),
        &emacs,
        &Instant::now(),
    )
    .unwrap();

    assert_eq!(response.pointer("/result/isError"), Some(&json!(true)));
    assert_eq!(
        response.pointer("/result/content/0/text"),
        Some(&json!(
            "Error: Emacs daemon is not running. Start it with: emacs --daemon"
        )),
    );
}

#[test]
fn reads_mcp_self_health_resource_without_emacs() {
    let emacs = FakeEmacs {
        daemon_running: false,
        eval_result: Ok("unused".to_string()),
    };
    let response = handle_json_rpc_message_with_emacs(
        &json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "resources/read",
            "params": { "uri": "health://mcp" }
        }),
        &emacs,
        &Instant::now(),
    )
    .unwrap();

    let text = response
        .pointer("/result/contents/0/text")
        .and_then(Value::as_str)
        .unwrap();
    let probe: Value = serde_json::from_str(text).unwrap();

    assert_eq!(probe.get("status"), Some(&json!("up")));
    assert_eq!(probe.pointer("/data/name"), Some(&json!(SERVER_NAME)));
    assert_eq!(probe.pointer("/data/tools"), Some(&json!(11)));
}

struct FakeEmacs {
    daemon_running: bool,
    eval_result: Result<String, String>,
}

impl EmacsClient for FakeEmacs {
    fn is_daemon_running(&self) -> bool {
        self.daemon_running
    }

    fn eval_elisp(&self, _pkg: &str, _expr: &str, _timeout: Duration) -> Result<String, String> {
        self.eval_result.clone()
    }
}
