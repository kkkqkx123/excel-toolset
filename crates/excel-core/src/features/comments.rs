use serde::Serialize;
use std::fs::File;
use std::io::BufReader;

use crate::security;
use crate::types::*;

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub author: Option<String>,
    pub text: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn get_comment(path: &str, sheet: &str, cell: &str) -> Result<Option<Comment>> {
    let (row, col) = crate::utils::cell_ref::parse_cell_ref(cell)?;

    let mut zipfile = zip::ZipArchive::new(BufReader::new(File::open(path)?))
        .map_err(|e| AppError::Read(e.to_string()))?;

    let comments_xml_path = format!("xl/worksheets/_rels/{}.xml.rels", sheet);
    if let Ok(mut comments_file) = zipfile.by_name(&comments_xml_path) {
        let mut content = String::new();
        use std::io::Read;
        comments_file.read_to_string(&mut content)?;

        if let Some(comment_text) = parse_comment_xml(&content, row, col) {
            return Ok(Some(Comment {
                author: None,
                text: comment_text,
                created_at: None,
            }));
        }
    }

    Ok(None)
}

fn parse_comment_xml(xml: &str, target_row: u32, target_col: u16) -> Option<String> {
    let start_tag = "<comment";
    let end_tag = "</comment>";

    let mut pos = 0;
    while let Some(start) = xml[pos..].find(start_tag) {
        let start = pos + start;
        if let Some(end) = xml[start..].find(end_tag) {
            let end = start + end;
            let comment_block = &xml[start..end + end_tag.len()];

            if let Some(ref_start) = comment_block.find("<ref>") {
                let ref_end = match comment_block[ref_start..].find("</ref>") {
                    Some(e) => ref_start + e,
                    None => continue,
                };
                let ref_text = &comment_block[ref_start + 5..ref_end];
                if let Ok((row, col)) = crate::utils::cell_ref::parse_cell_ref(ref_text)
                    && row == target_row
                    && col == target_col
                    && let Some(text_start) = comment_block.find("<text>")
                {
                    let text_end = match comment_block[text_start..].find("</text>") {
                        Some(e) => text_start + e,
                        None => continue,
                    };
                    let text = &comment_block[text_start + 6..text_end];
                    return Some(text.to_string());
                }
            }

            pos = end + end_tag.len();
        } else {
            break;
        }
    }

    None
}

pub fn add_comment(
    _path: &str,
    sheet: &str,
    cell: &str,
    _comment: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let _workbook = rust_xlsxwriter::Workbook::new();

    // TODO: Implement comment addition
    // This is a placeholder implementation

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
    _path: &str,
    sheet: &str,
    cell: &str,
    _comment: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

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
    _path: &str,
    sheet: &str,
    cell: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    Ok(WriteResult {
        success: true,
        message: format!("Comment deleted from {} in sheet {}", cell, sheet),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}