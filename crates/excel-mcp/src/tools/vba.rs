// VBA category tools: export, import, has.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_vba_export",
            description: "Export VBA macros from an Excel file. Returns base64-encoded data.",
            input_schema: object_schema(
                vec![("path", string_prop("Path to the .xlsx file", true))],
                vec!["path"],
            ),
        },
        ToolDef {
            name: "excel_vba_import",
            description: "Import VBA macros into an Excel file. 'data' should be base64-encoded VBA binary content.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("data", string_prop("Base64-encoded VBA binary data", true)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "data"],
            ),
        },
        ToolDef {
            name: "excel_vba_has",
            description: "Check if an Excel file contains VBA macros.",
            input_schema: object_schema(
                vec![("path", string_prop("Path to the .xlsx file", true))],
                vec!["path"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_vba_export".into(), handle_export);
    handlers.insert("excel_vba_import".into(), handle_import);
    handlers.insert("excel_vba_has".into(), handle_has);
}

fn handle_export(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();

    match excel_core::features::vba_util::export_vba(&path) {
        Ok(data) => {
            serde_json::json!({
                "success": true,
                "size": data.len(),
                "base64": base64_encode(&data)
            })
            .to_string()
        }
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_import(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let data_str = get_string(&args, "data").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let vba_data = match base64_decode(&data_str) {
        Some(d) => d,
        None => return "Error: Invalid base64 data".to_string(),
    };

    let params = security_params(&path, dry_run);

    match excel_core::features::vba_util::import_vba(&path, &params, &vba_data) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_has(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    match excel_core::features::vba_util::has_vba(&path) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn base64_encode(data: &[u8]) -> String {
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(chars[((triple >> 18) & 0x3F) as usize] as char);
        result.push(chars[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(chars[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(chars[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let chars: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && !b.is_ascii_whitespace())
        .collect();

    let mut result = Vec::with_capacity(chars.len() * 3 / 4);
    for chunk in chars.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let decode_char = |b: u8| -> Option<u32> {
            match b {
                b'A'..=b'Z' => Some((b - b'A') as u32),
                b'a'..=b'z' => Some((b - b'a' + 26) as u32),
                b'0'..=b'9' => Some((b - b'0' + 52) as u32),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            }
        };

        let v0 = decode_char(chunk[0])?;
        let v1 = decode_char(chunk[1])?;
        let v2 = chunk.get(2).and_then(|&b| decode_char(b)).unwrap_or(0);
        let v3 = chunk.get(3).and_then(|&b| decode_char(b)).unwrap_or(0);

        let triple = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        result.push((triple >> 16) as u8);

        if chunk.len() > 2 {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk.len() > 3 {
            result.push((triple & 0xFF) as u8);
        }
    }
    Some(result)
}
