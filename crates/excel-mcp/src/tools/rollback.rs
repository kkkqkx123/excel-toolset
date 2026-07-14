// Rollback tool.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "excel_rollback",
        description: "Rollback an Excel file to a previous state using a backup info JSON.",
        input_schema: object_schema(
            vec![
                ("path", string_prop("Path to the current .xlsx file", true)),
                ("backup_info", string_prop("JSON string of BackupInfo", true)),
            ],
            vec!["path", "backup_info"],
        ),
    }]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_rollback".into(), handle_rollback);
}

fn handle_rollback(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let backup_json = get_string(&args, "backup_info").unwrap_or_default();

    let backup_info: excel_core::types::BackupInfo = match serde_json::from_str(&backup_json) {
        Ok(b) => b,
        Err(e) => return format!("Error parsing backup info JSON: {e}"),
    };

    match excel_core::security::rollback(&backup_info, &path) {
        Ok(()) => serde_json::json!({"success": true, "message": "Rollback completed"}).to_string(),
        Err(e) => format!("Error: {e}"),
    }
}
