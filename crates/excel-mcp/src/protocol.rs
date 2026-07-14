// JSON-RPC 2.0 types for MCP protocol over stdio.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Incoming JSON-RPC request from the MCP client.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// Outgoing JSON-RPC success response.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP tool call result content item.
#[derive(Debug, Serialize)]
pub struct ToolCallContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// MCP tool call result.
#[derive(Debug, Serialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolCallContent>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }

    pub fn parse_error() -> Self {
        Self::error(None, -32700, "Parse error".into())
    }

    pub fn invalid_request(id: Option<Value>) -> Self {
        Self::error(id, -32600, "Invalid Request".into())
    }

    pub fn method_not_found(id: Option<Value>) -> Self {
        Self::error(id, -32601, "Method not found".into())
    }

    pub fn internal_error(id: Option<Value>, msg: String) -> Self {
        Self::error(id, -32603, format!("Internal error: {msg}"))
    }
}
