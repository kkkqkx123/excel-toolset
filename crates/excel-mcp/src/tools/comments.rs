// Comments category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_comments_get",
            description: "Get the comment from a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference like A1", true)),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
        ToolDef {
            name: "excel_comments_add",
            description: "Add a comment to a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference like A1", true)),
                    ("text", string_prop("Comment text", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "cell", "text"],
            ),
        },
        ToolDef {
            name: "excel_comments_update",
            description: "Update an existing comment on a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference like A1", true)),
                    ("text", string_prop("New comment text", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "cell", "text"],
            ),
        },
        ToolDef {
            name: "excel_comments_delete",
            description: "Delete a comment from a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference like A1", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_comments_get".into(), handle_get);
    handlers.insert("excel_comments_add".into(), handle_add);
    handlers.insert("excel_comments_update".into(), handle_update);
    handlers.insert("excel_comments_delete".into(), handle_delete);
}

fn handle_get(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();

    match excel_core::features::comments::get_comment(&path, &sheet, &cell) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_add(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let text = get_string(&args, "text").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::comments::add_comment(
        &path,
        &sheet,
        &cell,
        &text,
        &security_params(&path, dry_run),
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_update(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let text = get_string(&args, "text").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::comments::update_comment(
        &path,
        &sheet,
        &cell,
        &text,
        &security_params(&path, dry_run),
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_delete(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::comments::delete_comment(
        &path,
        &sheet,
        &cell,
        &security_params(&path, dry_run),
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
