// JSON Schema builder helpers for tool input schemas.
// Provides concise functions to build standard JSON Schema objects.

use serde_json::{Value, json};

/// Build a JSON Schema object type with required properties.
pub fn object_schema(
    properties: Vec<(&str, Value)>,
    required: Vec<&str>,
) -> Value {
    let mut props = serde_json::Map::new();
    for (name, schema) in properties {
        props.insert(name.to_string(), schema);
    }

    let mut schema = json!({
        "type": "object",
        "properties": props,
    });

    if !required.is_empty() {
        schema["required"] = json!(required);
    }

    schema
}

/// Build a string property schema.
pub fn string_prop(description: &str, required: bool) -> Value {
    let s = json!({
        "type": "string",
        "description": description
    });
    if required {
        s
    } else {
        let mut s = s;
        // Non-required properties don't need special marking in JSON Schema;
        // they're only required if listed in the "required" array.
        s
    }
}

/// Build a boolean property schema.
pub fn bool_prop(description: &str, default: Option<bool>) -> Value {
    let s = json!({
        "type": "boolean",
        "description": description,
    });
    match default {
        Some(v) => json!({ "type": "boolean", "description": description, "default": v }),
        None => s,
    }
}

/// Build an integer property schema.
pub fn int_prop(description: &str) -> Value {
    json!({
        "type": "integer",
        "description": description
    })
}

/// Build a string enum property schema.
pub fn enum_prop(description: &str, values: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values
    })
}

/// Build an array of strings property schema.
pub fn string_array_prop(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": { "type": "string" }
    })
}

/// Get a string argument from the arguments JSON value.
pub fn get_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Get a boolean argument from the arguments JSON value.
pub fn get_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}

/// Get an integer argument from the arguments JSON value.
pub fn get_u32(args: &Value, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

/// Get a string array argument from the arguments JSON value.
pub fn get_string_array(args: &Value, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    })
}

/// Serialize a result to JSON string, or return error string.
pub fn to_result_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("Serialization error: {e}"))
}

/// Create SecurityParams from file path and dry_run flag.
pub fn security_params(path: &str, dry_run: bool) -> excel_core::types::SecurityParams {
    excel_core::types::SecurityParams {
        dry_run,
        create_backup: true,
        file_path: path.to_string(),
    }
}
