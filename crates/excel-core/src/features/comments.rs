//! Cell comments (Excel "notes").
//!
//! Comments are written **into the xlsx package** as real notes via
//! `rust_xlsxwriter::Note`, so Excel / openpyxl / ClosedXML can see them.
//!
//! A sidecar `*.comments.json` is kept in sync purely as a *read-back* cache:
//! calamine (our reader) does not expose the `xl/comments*.xml` parts, so
//! without it `comments get` would have nothing to return. The xlsx note is
//! the source of truth for other tools; the sidecar is the source of truth for
//! our own reader.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use crate::security;
use crate::types::*;
use crate::utils::cell_ref;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub author: Option<String>,
    pub text: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get the sidecar file path used to read comments back.
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
    let content =
        serde_json::to_string_pretty(comments).map_err(|e| AppError::Serialize(e.to_string()))?;
    fs::write(&path, content).map_err(AppError::Io)?;
    Ok(())
}

fn cell_key(sheet: &str, cell: &str) -> String {
    format!("{}!{}", sheet, cell)
}

/// Splits a `"Sheet1!B2"` key back into its parts.
fn split_key(key: &str) -> Option<(&str, &str)> {
    key.rsplit_once('!')
}

/// Rewrites the workbook so that every comment currently in `comments`
/// exists as a real xlsx note. Called after each mutation so add/update/delete
/// all converge on the same state.
fn sync_notes_to_xlsx(
    path: &str,
    params: &SecurityParams,
    comments: &HashMap<String, Comment>,
) -> Result<()> {
    // Deterministic order so repeated runs produce identical files.
    let mut entries: Vec<(&String, &Comment)> = comments.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    // Backups are handled by the caller; avoid a second copy here.
    let write_params = SecurityParams {
        dry_run: params.dry_run,
        create_backup: false,
        file_path: params.file_path.clone(),
    };

    crate::excel_write::modify_file_with_wb(path, &write_params, |_old, wb| {
        for (key, comment) in entries {
            let Some((sheet, cell)) = split_key(key) else {
                continue;
            };
            let (row, col) = cell_ref::parse_cell_ref(cell)?;
            let ws = wb
                .worksheet_from_name(sheet)
                .map_err(|_| AppError::SheetNotFound(sheet.to_string()))?;
            let mut note = rust_xlsxwriter::Note::new(&comment.text);
            if let Some(author) = comment.author.as_deref() {
                note = note.set_author(author);
            }
            ws.insert_note(row, col, &note).map_err(AppError::Xlsx)?;
        }
        Ok(())
    })?;

    Ok(())
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

    // Fail fast on a bad reference before touching the file.
    cell_ref::parse_cell_ref(cell)?;

    security::create_backup_if_needed(params)?;

    let mut comments = load_comments(path);
    comments.insert(
        cell_key(sheet, cell),
        Comment {
            author: None,
            text: comment_text.to_string(),
            created_at: Some(chrono::Utc::now()),
        },
    );

    sync_notes_to_xlsx(path, params, &comments)?;
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

    cell_ref::parse_cell_ref(cell)?;

    security::create_backup_if_needed(params)?;

    let mut comments = load_comments(path);
    let key = cell_key(sheet, cell);
    match comments.get_mut(&key) {
        Some(comment) => comment.text = comment_text.to_string(),
        None => {
            // Updating a non-existent comment silently succeeded before, which
            // made `update` look like it worked on an empty cell. Create it
            // instead of pretending.
            comments.insert(
                key,
                Comment {
                    author: None,
                    text: comment_text.to_string(),
                    created_at: Some(chrono::Utc::now()),
                },
            );
        }
    }

    sync_notes_to_xlsx(path, params, &comments)?;
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

    // Rewriting the workbook from scratch drops every note, then re-inserts
    // only the remaining ones — that is how the deletion reaches the xlsx.
    sync_notes_to_xlsx(path, params, &comments)?;
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
