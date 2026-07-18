// MCP server: tool registry, request dispatch, and lifecycle.
//
// Handles the MCP lifecycle methods:
//   - initialize         → return server capabilities
//   - notifications/initialized → no-op acknowledgement
//   - tools/list         → return all tool definitions
//   - tools/call         → execute a tool and return result
//   - ping               → return empty response

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ToolCallContent, ToolCallResult};
use crate::tools;

/// Defines a single MCP tool with its name, description, and JSON Schema.
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

/// Function signature for tool handlers.
pub type ToolHandler = fn(Value) -> String;

/// Holds the tool registry and dispatches incoming requests.
pub struct Server {
    tools: Vec<ToolDef>,
    handlers: HashMap<String, ToolHandler>,
    initialized: bool,
}

impl Server {
    pub fn new() -> Self {
        let (tools, handlers) = tools::register_all();
        Self {
            tools,
            handlers,
            initialized: false,
        }
    }

    /// Read a single JSON-RPC request from stdin, process it, and write response to stdout.
    pub fn handle_request(&mut self, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        match request.method.as_str() {
            "initialize" => Some(self.handle_initialize(request.id.clone())),
            "notifications/initialized" => {
                self.initialized = true;
                None // Notification: no response
            }
            "tools/list" => {
                if !self.initialized {
                    return Some(JsonRpcResponse::error(
                        request.id.clone(),
                        -32002,
                        "Not initialized".into(),
                    ));
                }
                Some(self.handle_tools_list(request.id.clone()))
            }
            "tools/call" => {
                if !self.initialized {
                    return Some(JsonRpcResponse::error(
                        request.id.clone(),
                        -32002,
                        "Not initialized".into(),
                    ));
                }
                Some(self.handle_tools_call(request.id.clone(), request.params.clone()))
            }
            "ping" => Some(JsonRpcResponse::success(request.id.clone(), json!({}))),
            _ => Some(JsonRpcResponse::method_not_found(request.id.clone())),
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        let result = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "excel-toolset-mcp",
                "version": "0.1.0"
            }
        });
        JsonRpcResponse::success(id, result)
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        let tools: Vec<Value> = self
            .tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })
            })
            .collect();
        JsonRpcResponse::success(id, json!({ "tools": tools }))
    }

    fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => return JsonRpcResponse::invalid_request(id),
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return JsonRpcResponse::invalid_request(id),
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));

        let handler = match self.handlers.get(&tool_name) {
            Some(h) => h,
            None => {
                return JsonRpcResponse::error(id, -32602, format!("Unknown tool: {tool_name}"));
            }
        };

        let text = handler(arguments);

        let result = ToolCallResult {
            content: vec![ToolCallContent {
                content_type: "text".to_string(),
                text,
            }],
        };

        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap_or_default())
    }
}
