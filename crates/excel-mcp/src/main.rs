// excel-mcp: MCP Server for Excel Toolset
//
// Implements Model Context Protocol (MCP) over stdio (JSON-RPC 2.0).
// Compatible with Claude Code, Cursor, VS Code, and other MCP clients.
//
// Usage:
//   cargo run --bin excel-mcp
//
// The server reads JSON-RPC requests line by line from stdin
// and writes JSON-RPC responses to stdout.

use std::io::{BufRead, BufReader, Write};

mod protocol;
mod server;
mod tools;

fn main() {
    let mut server_state = server::Server::new();
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin);
    let mut stdout = std::io::stdout();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading stdin: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: protocol::JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let resp = protocol::JsonRpcResponse::parse_error();
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap_or_default());
                let _ = stdout.flush();
                eprintln!("Parse error: {e}");
                continue;
            }
        };

        let response = server_state.handle_request(&request);

        if let Some(resp) = response {
            let json = serde_json::to_string(&resp).unwrap_or_default();
            let _ = writeln!(stdout, "{json}");
            let _ = stdout.flush();
        }
    }
}
