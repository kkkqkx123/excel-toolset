use excel_core::types::{AppError, DiffSummary, FileDiff, Result};

use crate::engine::diff_sheet_maps;
use crate::helpers::read_all_sheets_to_map;

/// Full file comparison: hash quick-check first, then cell-level diff.
pub fn diff_files(old_path: &str, new_path: &str) -> Result<FileDiff> {
    use excel_core::security::compute_file_hash;

    let old_hash = compute_file_hash(old_path).map_err(AppError::Io)?;
    let new_hash = compute_file_hash(new_path).map_err(AppError::Io)?;

    if old_hash == new_hash {
        return Ok(FileDiff {
            file_hash_match: true,
            sheet_diffs: Vec::new(),
            summary: DiffSummary {
                adds: 0,
                deletes: 0,
                modifies: 0,
                passives: 0,
                total_changes: 0,
            },
        });
    }

    let old_sheets = read_all_sheets_to_map(old_path)?;
    let new_sheets = read_all_sheets_to_map(new_path)?;

    let sheet_diffs = diff_sheet_maps(&old_sheets, &new_sheets);
    let summary = crate::summarize::summarize(&sheet_diffs);

    Ok(FileDiff {
        file_hash_match: false,
        sheet_diffs,
        summary,
    })
}
