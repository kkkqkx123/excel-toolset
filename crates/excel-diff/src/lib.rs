mod api_response;
mod engine;
mod file_diff;
mod formula_tracker;
pub mod git_driver;
mod helpers;
mod range_diff;
pub mod semantic;
mod sheet_diff;
pub mod summarize;

pub use file_diff::diff_files;
pub use git_driver::get_git_diff_file_paths;
pub use range_diff::diff_range;
pub use sheet_diff::diff_sheets;

// Export diff_sheet_data API for direct in-memory comparison
pub use engine::compute_cell_diffs as diff_sheet_data;

use excel_types::{
    CellDiff, FormulaDependencyDiff, SemanticDiff, SemanticDiffEntry, SheetData, SheetDiff,
};

/// Compare two SheetData objects in memory.
/// This is useful for testing and scenarios where you already have the data loaded.
///
/// # Arguments
/// * `old_data` - The old sheet data
/// * `new_data` - The new sheet data
///
/// # Returns
/// A vector of cell differences
///
/// # Examples
/// ```
/// use excel_diff::{diff_sheet_data, compute_diffs};
/// use excel_types::{CellData, CellDataType, SheetData};
///
/// let old = SheetData {
///     name: "Sheet1".into(),
///     rows: vec![vec![CellData {
///         value: Some("old".into()),
///         data_type: CellDataType::String,
///         formula: None,
///     }]],
/// };
/// let new = SheetData {
///     name: "Sheet1".into(),
///     rows: vec![vec![CellData {
///         value: Some("new".into()),
///         data_type: CellDataType::String,
///         formula: None,
///     }]],
/// };
/// let diffs = compute_diffs(&old, &new);
/// assert_eq!(diffs.len(), 1);
/// ```
pub fn compute_diffs(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff> {
    engine::compute_cell_diffs(old_data, new_data)
}

pub fn diff_with_semantic(old_path: &str, new_path: &str) -> excel_types::Result<SemanticDiff> {
    let file_diff = diff_files(old_path, new_path)?;
    let report = semantic::to_semantic_report(&file_diff, None);

    let mut entries = Vec::new();
    for (idx, op) in report.operations.iter().enumerate() {
        let sentence = report
            .detail_sentences
            .get(idx)
            .cloned()
            .unwrap_or_default();
        let (cell, change_type, impact) = match op {
            semantic::grouper::LogicalOperation::CellModified {
                sheet,
                cell_ref,
                ..
            } => (
                format!("{}!{}", sheet, cell_ref),
                "modified".to_string(),
                None,
            ),
            semantic::grouper::LogicalOperation::CellPassive {
                sheet,
                cell_ref,
                ..
            } => (
                format!("{}!{}", sheet, cell_ref),
                "passive".to_string(),
                None,
            ),
            semantic::grouper::LogicalOperation::RowAdded { sheet, row, .. } => (
                format!("{}!row-{}", sheet, row + 1),
                "added".to_string(),
                None,
            ),
            semantic::grouper::LogicalOperation::RowDeleted { sheet, row, .. } => (
                format!("{}!row-{}", sheet, row + 1),
                "deleted".to_string(),
                None,
            ),
            semantic::grouper::LogicalOperation::SheetAdded { sheet, .. } => {
                (sheet.clone(), "added".to_string(), None)
            }
            semantic::grouper::LogicalOperation::SheetDeleted { sheet, .. } => {
                (sheet.clone(), "deleted".to_string(), None)
            }
            semantic::grouper::LogicalOperation::SheetResized { sheet, .. } => {
                (sheet.clone(), "resized".to_string(), None)
            }
        };

        entries.push(SemanticDiffEntry {
            cell,
            change_type,
            description: sentence,
            impact,
        });
    }

    Ok(SemanticDiff {
        summary: report.summary,
        entries,
        statistics: file_diff.summary,
    })
}

/// Compare formula dependency graphs of a specific sheet between two files.
pub fn diff_formula_dependencies(
    old_path: &str,
    new_path: &str,
    sheet: &str,
) -> excel_types::Result<FormulaDependencyDiff> {
    use excel_core::excel_read;
    use excel_types::{DependencyNode, FormulaDependencyDiff};

    let old_data = excel_read::read_sheet_all(old_path, sheet)?;
    let new_data = excel_read::read_sheet_all(new_path, sheet)?;

    let old_tracker = crate::formula_tracker::FormulaTracker::build_from_sheet(&old_data);
    let new_tracker = crate::formula_tracker::FormulaTracker::build_from_sheet(&new_data);

    let mut new_deps = Vec::new();
    let mut removed_deps = Vec::new();
    let mut modified_deps = Vec::new();

    for (cell_ref, targets) in &new_tracker.dependencies {
        match old_tracker.dependencies.get(cell_ref) {
            None => {
                let mut sorted: Vec<String> = targets.iter().cloned().collect();
                sorted.sort();
                new_deps.push(DependencyNode {
                    source: cell_ref.clone(),
                    targets: sorted,
                });
            }
            Some(old_targets) => {
                if targets != old_targets {
                    let mut all_changed: Vec<String> = targets.iter().cloned().collect();
                    all_changed.sort();
                    modified_deps.push(DependencyNode {
                        source: cell_ref.clone(),
                        targets: all_changed,
                    });
                }
            }
        }
    }

    for (cell_ref, targets) in &old_tracker.dependencies {
        if !new_tracker.dependencies.contains_key(cell_ref) {
            let mut sorted: Vec<String> = targets.iter().cloned().collect();
            sorted.sort();
            removed_deps.push(DependencyNode {
                source: cell_ref.clone(),
                targets: sorted,
            });
        }
    }

    let cycles_introduced = new_tracker.detect_cycles();

    Ok(FormulaDependencyDiff {
        new_dependencies: new_deps,
        removed_dependencies: removed_deps,
        modified_dependencies: modified_deps,
        cycles_introduced,
    })
}

/// Diff specified sheets between two files.
pub fn diff_batch(
    old_path: &str,
    new_path: &str,
    sheets: &[String],
) -> excel_types::Result<Vec<SheetDiff>> {
    sheets
        .iter()
        .map(|s| diff_sheets(old_path, new_path, s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType, DiffType};

    #[test]
    fn test_compute_diffs_delegates_to_engine() {
        let old = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("old".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("new".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let diffs = compute_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
    }

    #[test]
    fn test_compute_diffs_identical() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let diffs = compute_diffs(&data, &data);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_sheet_data_works() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("old".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("new".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
        assert_eq!(diffs[0].old_value, Some("old".into()));
        assert_eq!(diffs[0].new_value, Some("new".into()));
    }

    #[test]
    fn test_diff_sheet_data_empty() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_sheet_data_add() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("new".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Add);
    }

    #[test]
    fn test_diff_sheet_data_delete() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("old".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Delete);
    }

    // ===== Tests for diff_batch =====

    #[test]
    fn test_diff_batch_multiple_sheets() {
        let old_path = "/tmp/diff_batch_old.xlsx";
        let new_path = "/tmp/diff_batch_new.xlsx";
        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);

        {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, "old").expect("write");
            let ws2 = wb.add_worksheet();
            ws2.set_name("S2".to_string()).expect("set name");
            ws2.write_string(0, 0, "old").expect("write");
            wb.save(old_path).expect("save");
        }
        {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, "new").expect("write");
            let ws2 = wb.add_worksheet();
            ws2.set_name("S2".to_string()).expect("set name");
            ws2.write_string(0, 0, "old").expect("write");
            wb.save(new_path).expect("save");
        }

        let sheets = vec!["S1".to_string(), "S2".to_string()];
        let results = super::diff_batch(old_path, new_path, &sheets).expect("diff_batch");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].sheet_name, "S1");
        assert_eq!(results[0].cell_diffs.len(), 1);
        assert_eq!(results[1].sheet_name, "S2");
        assert_eq!(results[1].cell_diffs.len(), 0);

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);
    }

    #[test]
    fn test_diff_batch_empty_sheets() {
        let old_path = "/tmp/diff_batch_empty_old.xlsx";
        let new_path = "/tmp/diff_batch_empty_new.xlsx";
        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);

        for (path, val) in &[(old_path, "old"), (new_path, "new")] {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, *val).expect("write");
            wb.save(path).expect("save");
        }

        let results = super::diff_batch(old_path, new_path, &[]).expect("diff_batch_empty");
        assert!(results.is_empty());

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);
    }

    // ===== Tests for diff_with_semantic =====

    #[test]
    fn test_diff_with_semantic_no_changes() {
        let path = "/tmp/diff_sem_nochg.xlsx";
        let _ = std::fs::remove_file(path);
        {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, "hello").expect("write");
            wb.save(path).expect("save");
        }

        let result = super::diff_with_semantic(path, path).expect("diff_with_semantic");
        assert!(result.summary.contains("No changes"));
        assert!(result.entries.is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_diff_with_semantic_with_changes() {
        let old_path = "/tmp/diff_sem_old.xlsx";
        let new_path = "/tmp/diff_sem_new.xlsx";
        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);

        {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, "old").expect("write");
            wb.save(old_path).expect("save");
        }
        {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, "new").expect("write");
            wb.save(new_path).expect("save");
        }

        let result = super::diff_with_semantic(old_path, new_path).expect("diff_with_semantic");
        assert!(result.summary.contains("Total"));
        assert!(!result.summary.contains("No changes"));

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);
    }

    // ===== Tests for diff_formula_dependencies =====

    #[test]
    fn test_diff_formula_deps_no_deps() {
        let old_path = "/tmp/diff_fdeps_nodep_old.xlsx";
        let new_path = "/tmp/diff_fdeps_nodep_new.xlsx";
        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);

        for (path, val) in &[(old_path, "old"), (new_path, "new")] {
            let mut wb = rust_xlsxwriter::Workbook::new();
            let ws = wb.add_worksheet();
            ws.set_name("S1".to_string()).expect("set name");
            ws.write_string(0, 0, *val).expect("write");
            wb.save(path).expect("save");
        }

        let result =
            super::diff_formula_dependencies(old_path, new_path, "S1").expect("diff_fdeps");
        assert!(result.new_dependencies.is_empty());
        assert!(result.removed_dependencies.is_empty());
        assert!(result.modified_dependencies.is_empty());
        assert!(result.cycles_introduced.is_empty());

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);
    }

    #[test]
    fn test_formula_tracker_detect_cycles() {
        use std::collections::HashMap;
        use crate::formula_tracker::FormulaTracker;

        // No cycles
        let mut deps1 = HashMap::new();
        deps1.insert("A1".to_string(), ["B1".to_string()].into());
        deps1.insert("B1".to_string(), ["C1".to_string()].into());
        let tracker1 = FormulaTracker { dependencies: deps1 };
        assert!(tracker1.detect_cycles().is_empty());

        // Self-reference cycle
        let mut deps2 = HashMap::new();
        deps2.insert("A1".to_string(), ["A1".to_string()].into());
        let tracker2 = FormulaTracker { dependencies: deps2 };
        assert_eq!(tracker2.detect_cycles(), vec!["A1".to_string()]);

        // Multi-cell cycle — any of A1, B1, C1 may be reported first
        let mut deps3 = HashMap::new();
        deps3.insert("A1".to_string(), ["B1".to_string()].into());
        deps3.insert("B1".to_string(), ["C1".to_string()].into());
        deps3.insert("C1".to_string(), ["A1".to_string()].into());
        let tracker3 = FormulaTracker { dependencies: deps3 };
        let cycles3 = tracker3.detect_cycles();
        assert_eq!(cycles3.len(), 1);
        assert!(
            cycles3[0] == "A1" || cycles3[0] == "B1" || cycles3[0] == "C1",
            "Expected one of A1, B1, C1, got {}",
            cycles3[0]
        );

        // Empty
        let tracker4 = FormulaTracker::default();
        assert!(tracker4.detect_cycles().is_empty());
    }

    #[test]
    fn test_diff_formula_deps_new_dependency() {
        use crate::formula_tracker::FormulaTracker;
        use excel_types::{CellData, CellDataType};

        // Build two in-memory sheets to verify FormulaTracker comparison logic
        let sheet_old = SheetData {
            name: "S1".into(),
            rows: vec![
                vec![CellData {
                    value: Some("1".into()),
                    data_type: CellDataType::Float,
                    formula: None,
                }],
            ],
        };
        let sheet_new = SheetData {
            name: "S1".into(),
            rows: vec![
                vec![CellData {
                    value: Some("1".into()),
                    data_type: CellDataType::Float,
                    formula: None,
                }],
                vec![CellData {
                    value: Some("2".into()),
                    data_type: CellDataType::Float,
                    formula: Some("=A1+B1".into()),
                }],
            ],
        };

        let old_tracker = FormulaTracker::build_from_sheet(&sheet_old);
        let new_tracker = FormulaTracker::build_from_sheet(&sheet_new);

        // A2 should be in new but not old
        assert!(new_tracker.dependencies.contains_key("A2"));
        assert!(!old_tracker.dependencies.contains_key("A2"));
    }
}
