//! MCP server health: spawn `ortk-mcp` and run an `initialize` +
//! `tools/list` JSON-RPC handshake over stdio.
//!
//! Cost: ~1 second per probe (process startup + two roundtrips).
//! Mitigated by the 5-second TTL cache upstream.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use crate::cache::Probe;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn probe_mcp() -> Probe {
    match timeout(PROBE_TIMEOUT, handshake()).await {
        Ok(Ok(v)) => Probe::up(v),
        Ok(Err(e)) => Probe::down(e),
        Err(_) => Probe::down("ortk-mcp probe timed out after 5s"),
    }
}

async fn handshake() -> Result<Value, String> {
    let mut child = Command::new("ortk-mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("failed to spawn ortk-mcp: {e}"))?;

    let mut stdin = child.stdin.take().ok_or("no stdin handle")?;
    let stdout = child.stdout.take().ok_or("no stdout handle")?;
    let mut lines = BufReader::new(stdout).lines();

    // 1) initialize
    write_json(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "ortk-dashboard", "version": env!("CARGO_PKG_VERSION") }
            }
        }),
    )
    .await?;

    let init_resp = read_response_with_id(&mut lines, 1).await?;
    let server_info = init_resp
        .pointer("/result/serverInfo")
        .cloned()
        .ok_or("initialize response missing /result/serverInfo")?;

    // 2) initialized notification
    write_json(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    )
    .await?;

    // 3) tools/list
    write_json(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await?;

    let list_resp = read_response_with_id(&mut lines, 2).await?;
    let tool_count = list_resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .ok_or("tools/list response missing /result/tools array")?;

    Ok(json!({
        "binary": "ortk-mcp",
        "tools": tool_count,
        "serverInfo": server_info,
    }))
}

async fn write_json(stdin: &mut tokio::process::ChildStdin, msg: &Value) -> Result<(), String> {
    let line = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| format!("write to mcp stdin: {e}"))?;
    stdin
        .write_all(b"\n")
        .await
        .map_err(|e| format!("write to mcp stdin: {e}"))?;
    stdin
        .flush()
        .await
        .map_err(|e| format!("flush mcp stdin: {e}"))?;
    Ok(())
}

async fn read_response_with_id<R>(
    lines: &mut tokio::io::Lines<R>,
    expect_id: u64,
) -> Result<Value, String>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("read from mcp stdout: {e}"))?
    {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // ignore non-JSON noise (e.g. log lines on stdout)
        };
        if msg.get("id").and_then(|v| v.as_u64()) == Some(expect_id) {
            if let Some(err) = msg.get("error") {
                return Err(format!("mcp error: {err}"));
            }
            return Ok(msg);
        }
    }
    Err("mcp server closed before sending response".to_string())
}
