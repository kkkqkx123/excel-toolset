use excel_core::types::{DiffType, FileDiff, SheetDiff};

use crate::formula_tracker::FormulaTracker;

pub fn to_api_response(diff: &FileDiff, tracker: Option<&FormulaTracker>) -> serde_json::Value {
    let sheets: Vec<serde_json::Value> = diff
        .sheet_diffs
        .iter()
        .map(|sd| sheet_diff_to_json(sd, tracker))
        .collect();

    serde_json::json!({
        "success": true,
        "file_hash_match": diff.file_hash_match,
        "summary": {
            "adds": diff.summary.adds,
            "deletes": diff.summary.deletes,
            "modifies": diff.summary.modifies,
            "passives": diff.summary.passives,
            "total_changes": diff.summary.total_changes
        },
        "sheets": sheets
    })
}

fn sheet_diff_to_json(sd: &SheetDiff, tracker: Option<&FormulaTracker>) -> serde_json::Value {
    let cells: Vec<serde_json::Value> = sd
        .cell_diffs
        .iter()
        .map(|cd| {
            let mut obj = serde_json::json!({
                "cell_ref": cd.cell_ref,
                "row": cd.row,
                "col": cd.col,
                "diff_type": match cd.diff_type {
                    DiffType::Add => "Add",
                    DiffType::Delete => "Delete",
                    DiffType::Modify => "Modify",
                    DiffType::Passive => "Passive",
                    DiffType::NoChange => "NoChange",
                },
            });
            if let Some(v) = &cd.old_value {
                obj["old_value"] = serde_json::Value::String(v.clone());
            }
            if let Some(v) = &cd.new_value {
                obj["new_value"] = serde_json::Value::String(v.clone());
            }
            if let Some(f) = &cd.old_formula {
                obj["old_formula"] = serde_json::Value::String(f.clone());
            }
            if let Some(f) = &cd.new_formula {
                obj["new_formula"] = serde_json::Value::String(f.clone());
            }
            if cd.diff_type == DiffType::Passive
                && let Some(t) = tracker
                && let Some(chain) = t.get_dependency_chain(&cd.cell_ref)
            {
                obj["dependency_chain"] = serde_json::Value::String(chain);
            }
            obj
        })
        .collect();

    serde_json::json!({
        "sheet_name": sd.sheet_name,
        "row_count_diff": sd.row_count_diff,
        "col_count_diff": sd.col_count_diff,
        "cell_diffs": cells
    })
}
