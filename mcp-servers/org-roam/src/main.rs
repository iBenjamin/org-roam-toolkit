use std::io::{self, BufRead, Write};
use std::time::Instant;

use ortk_mcp::{handle_json_rpc_message_with_emacs, parse_error_response, RealEmacsClient};
use serde_json::Value;

fn main() {
    eprintln!("org-roam MCP server running on stdio");

    let emacs = RealEmacsClient;
    let server_start = Instant::now();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(&line) {
            Ok(message) => handle_json_rpc_message_with_emacs(&message, &emacs, &server_start),
            Err(_) => Some(parse_error_response()),
        };

        if let Some(response) = response {
            if let Ok(serialized) = serde_json::to_string(&response) {
                let _ = writeln!(stdout, "{serialized}");
                let _ = stdout.flush();
            }
        }
    }
}
