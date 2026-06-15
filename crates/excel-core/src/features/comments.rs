use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use crate::security;
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub author: Option<String>,
    pub text: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get the sidecar file path for storing comments
fn comments_sidecar_path(xlsx_path: &str) -> String {
    format!("{}.comments.json", xlsx_path)
}

/// Load comments from sidecar file
fn load_comments(xlsx_path: &str) -> HashMap<String, Comment> {
    let path = comments_sidecar_path(xlsx_path);
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

/// Save comments to sidecar file
fn save_comments(xlsx_path: &str, comments: &HashMap<String, Comment>) -> Result<()> {
    let path = comments_sidecar_path(xlsx_path);
    let content = serde_json::to_string_pretty(comments)
        .map_err(|e| AppError::Serialize(e.to_string()))?;
    fs::write(&path, content).map_err(AppError::Io)?;
    Ok(())
}

fn cell_key(sheet: &str, cell: &str) -> String {
    format!("{}!{}", sheet, cell)
}

pub fn get_comment(path: &str, sheet: &str, cell: &str) -> Result<Option<Comment>> {
    let comments = load_comments(path);
    Ok(comments.get(&cell_key(sheet, cell)).cloned())
}

pub fn add_comment(
    path: &str,
    sheet: &str,
    cell: &str,
    comment_text: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let mut comments = load_comments(path);
    comments.insert(
        cell_key(sheet, cell),
        Comment {
            author: None,
            text: comment_text.to_string(),
            created_at: None,
        },
    );
    save_comments(path, &comments)?;

    Ok(WriteResult {
        success: true,
        message: format!("Comment added to {} in sheet {}", cell, sheet),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}

pub fn update_comment(
    path: &str,
    sheet: &str,
    cell: &str,
    comment_text: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let mut comments = load_comments(path);
    if let Some(comment) = comments.get_mut(&cell_key(sheet, cell)) {
        comment.text = comment_text.to_string();
    }
    save_comments(path, &comments)?;

    Ok(WriteResult {
        success: true,
        message: format!("Comment updated in {} in sheet {}", cell, sheet),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}

pub fn delete_comment(
    path: &str,
    sheet: &str,
    cell: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let mut comments = load_comments(path);
    comments.remove(&cell_key(sheet, cell));
    save_comments(path, &comments)?;

    Ok(WriteResult {
        success: true,
        message: format!("Comment deleted from {} in sheet {}", cell, sheet),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}
