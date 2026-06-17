use std::fs;
use std::path::Path;

use excel_diff::semantic::{Verbosity, to_natural_text};
use excel_diff::summarize::summarize;
use excel_diff::{diff_files, diff_range, diff_sheet_data, diff_sheets};
use excel_types::{CellData, CellDataType, DiffType, SheetData};

// =======================================================================
// Test helpers
// =======================================================================

fn test_fixtures_dir() -> String {
    let dir = "/tmp/excel_diff_e2e";
    fs::create_dir_all(dir).expect("Failed to create test dir");
    dir.to_string()
}

fn fixture_path(name: &str) -> String {
    format!("{}/{}", test_fixtures_dir(), name)
}

fn cell_data(value: &str) -> CellData {
    CellData {
        value: Some(value.into()),
        data_type: CellDataType::String,
        formula: None,
    }
}

fn formula_cell(value: &str, formula: &str) -> CellData {
    CellData {
        value: Some(value.into()),
        data_type: CellDataType::String,
        formula: Some(formula.into()),
    }
}

fn create_xlsx(path: &str, sheets_data: &[(&str, &[Vec<CellData>])]) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    for (i, (name, rows)) in sheets_data.iter().enumerate() {
        let ws = if i == 0 {
            wb.add_worksheet()
        } else {
            wb.add_worksheet()
        };
        ws.set_name(name.to_string()).expect("set sheet name");
        for (ri, row) in rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                if let Some(ref v) = cell.value {
                    ws.write_string(ri as u32, ci as u16, v.as_str())
                        .expect("write cell");
                }
            }
        }
    }
    wb.save(path).expect("save xlsx");
}

fn cleanup(path: &str) {
    let _ = fs::remove_file(path);
}

// =======================================================================
// Business Scenario 1: Compare two versions of a file
// Real scenario: User modifies a report and wants to see what changed.
// =======================================================================

#[test]
fn scenario_file_diff_modified() {
    let old_path = fixture_path("diff_old.xlsx");
    let new_path = fixture_path("diff_new.xlsx");
    cleanup(&old_path);
    cleanup(&new_path);

    create_xlsx(
        &old_path,
        &[(
            "Report",
            &[
                vec![cell_data("Name"), cell_data("Score")],
                vec![cell_data("Alice"), cell_data("100")],
            ],
        )],
    );

    create_xlsx(
        &new_path,
        &[(
            "Report",
            &[
                vec![cell_data("Name"), cell_data("Score")],
                vec![cell_data("Alice"), cell_data("150")],
            ],
        )],
    );

    let diff = diff_files(&old_path, &new_path).expect("diff files");
    assert!(!diff.file_hash_match);
    assert_eq!(diff.sheet_diffs.len(), 1);
    assert_eq!(diff.sheet_diffs[0].sheet_name, "Report");

    // Score changed from 100 to 150
    let changes = &diff.sheet_diffs[0].cell_diffs;
    assert!(!changes.is_empty());

    let score_change = changes.iter().find(|c| c.cell_ref == "B2");
    assert!(score_change.is_some());
    assert_eq!(score_change.unwrap().diff_type, DiffType::Modify);

    let summary = summarize(&diff.sheet_diffs);
    assert!(summary.total_changes > 0);

    cleanup(&old_path);
    cleanup(&new_path);
}

// =======================================================================
// Business Scenario 2: Identical files
// Real scenario: User verifies two copies of a file are identical.
// =======================================================================

#[test]
fn scenario_file_diff_identical() {
    let path = fixture_path("diff_identical.xlsx");
    cleanup(&path);

    create_xlsx(
        &path,
        &[("Sheet1", &[vec![cell_data("A"), cell_data("B")]])],
    );

    let diff = diff_files(&path, &path).expect("diff identical");
    assert!(diff.file_hash_match);
    assert!(diff.sheet_diffs.is_empty());

    cleanup(&path);
}

// =======================================================================
// Business Scenario 3: Compare specific sheet
// Real scenario: Multi-sheet workbook, user only cares about one sheet.
// =======================================================================

#[test]
fn scenario_sheet_diff_specific_sheet() {
    let old_path = fixture_path("diff_sheet_old.xlsx");
    let new_path = fixture_path("diff_sheet_new.xlsx");
    cleanup(&old_path);
    cleanup(&new_path);

    create_xlsx(
        &old_path,
        &[
            (
                "Sales",
                &[vec![cell_data("Amount")], vec![cell_data("100")]],
            ),
            ("Costs", &[vec![cell_data("Amount")], vec![cell_data("50")]]),
        ],
    );

    create_xlsx(
        &new_path,
        &[
            (
                "Sales",
                &[vec![cell_data("Amount")], vec![cell_data("200")]],
            ),
            ("Costs", &[vec![cell_data("Amount")], vec![cell_data("50")]]),
        ],
    );

    // Diff only Sales sheet
    let diff = diff_sheets(&old_path, &new_path, "Sales").expect("diff Sales");
    assert_eq!(diff.sheet_name, "Sales");
    assert!(!diff.cell_diffs.is_empty());

    // Check A2 changed (Amount: 100 -> 200)
    let a2 = diff.cell_diffs.iter().find(|c| c.cell_ref == "A2");
    assert!(a2.is_some());
    assert_eq!(a2.unwrap().diff_type, DiffType::Modify);

    cleanup(&old_path);
    cleanup(&new_path);
}

// =======================================================================
// Business Scenario 4: Diff a specific range
// Real scenario: User only cares about changes in a specific data region.
// =======================================================================

#[test]
fn scenario_range_diff() {
    let old_path = fixture_path("diff_range_old.xlsx");
    let new_path = fixture_path("diff_range_new.xlsx");
    cleanup(&old_path);
    cleanup(&new_path);

    create_xlsx(
        &old_path,
        &[(
            "Data",
            &[
                vec![cell_data("H1"), cell_data("H2"), cell_data("H3")],
                vec![cell_data("A"), cell_data("B"), cell_data("C")],
                vec![cell_data("D"), cell_data("E"), cell_data("F")],
            ],
        )],
    );

    create_xlsx(
        &new_path,
        &[(
            "Data",
            &[
                vec![cell_data("H1"), cell_data("H2"), cell_data("H3")],
                vec![cell_data("X"), cell_data("Y"), cell_data("Z")],
                vec![cell_data("D"), cell_data("E"), cell_data("F")],
            ],
        )],
    );

    // Diff only rows 2-3
    let diff = diff_range(&old_path, &new_path, "Data", "A2:C3").expect("diff range");
    assert!(!diff.cell_diffs.is_empty());

    // A2 and B2 should have changed
    let modified_cells: Vec<_> = diff
        .cell_diffs
        .iter()
        .filter(|c| c.diff_type == DiffType::Modify)
        .collect();
    assert!(modified_cells.len() >= 2);

    cleanup(&old_path);
    cleanup(&new_path);
}

// =======================================================================
// Business Scenario 5: In-memory diff for programmatic comparison
// Real scenario: Application has data loaded, compares without file I/O.
// =======================================================================

#[test]
fn scenario_in_memory_diff() {
    let old_sheet = SheetData {
        name: "Sheet1".into(),
        rows: vec![
            vec![cell_data("Name"), cell_data("Age")],
            vec![cell_data("Alice"), cell_data("30")],
        ],
    };
    let new_sheet = SheetData {
        name: "Sheet1".into(),
        rows: vec![
            vec![cell_data("Name"), cell_data("Age")],
            vec![cell_data("Alice"), cell_data("31")],
        ],
    };

    let diffs = diff_sheet_data(&old_sheet, &new_sheet);
    assert_eq!(diffs.len(), 1); // only B2 changed (Age: 30 -> 31)
    assert_eq!(diffs[0].cell_ref, "B2");
    assert_eq!(diffs[0].diff_type, DiffType::Modify);
    assert_eq!(diffs[0].old_value, Some("30".into()));
    assert_eq!(diffs[0].new_value, Some("31".into()));
}

// =======================================================================
// Business Scenario 6: Detecting passive formula recalculations
// Real scenario: User changes an input cell, dependents show "passive" change.
// =======================================================================

#[test]
fn scenario_passive_formula_change() {
    let old_sheet = SheetData {
        name: "Calc".into(),
        rows: vec![
            vec![cell_data("10")],             // A1: input value
            vec![formula_cell("20", "=A1*2")], // A2: formula depends on A1
        ],
    };
    let new_sheet = SheetData {
        name: "Calc".into(),
        rows: vec![
            vec![cell_data("15")],             // A1: value changed
            vec![formula_cell("30", "=A1*2")], // A2: formula same, value recalculated
        ],
    };

    let diffs = diff_sheet_data(&old_sheet, &new_sheet);
    assert_eq!(diffs.len(), 2); // A1 modified, A2 passive

    let a1 = diffs.iter().find(|c| c.cell_ref == "A1");
    assert!(a1.is_some());
    assert_eq!(a1.unwrap().diff_type, DiffType::Modify);

    let a2 = diffs.iter().find(|c| c.cell_ref == "A2");
    assert!(a2.is_some());
    assert_eq!(a2.unwrap().diff_type, DiffType::Passive);
}

// =======================================================================
// Business Scenario 7: Row addition and deletion
// Real scenario: User adds/removes rows, wants to see structural changes.
// =======================================================================

#[test]
fn scenario_row_add_delete_in_memory() {
    let old_sheet = SheetData {
        name: "Data".into(),
        rows: vec![
            vec![cell_data("Header1"), cell_data("Header2")],
            vec![cell_data("Row1"), cell_data("10")],
            vec![cell_data("Row2"), cell_data("20")],
        ],
    };
    let new_sheet = SheetData {
        name: "Data".into(),
        rows: vec![
            vec![cell_data("Header1"), cell_data("Header2")],
            vec![cell_data("Row1"), cell_data("10")],
        ],
    };

    let diffs = diff_sheet_data(&old_sheet, &new_sheet);
    // Row2 was deleted: both A3 and B3 are Delete type
    let deleted: Vec<_> = diffs
        .iter()
        .filter(|c| c.diff_type == DiffType::Delete)
        .collect();
    assert_eq!(deleted.len(), 2);
    assert_eq!(deleted[0].cell_ref, "A3");
    assert_eq!(deleted[1].cell_ref, "B3");
}

// =======================================================================
// Business Scenario 8: Natural language diff report
// Real scenario: User gets a human-readable summary of changes.
// =======================================================================

#[test]
fn scenario_natural_language_diff() {
    let old_sheet = SheetData {
        name: "Report".into(),
        rows: vec![
            vec![cell_data("Name"), cell_data("Value")],
            vec![cell_data("X"), cell_data("10")],
        ],
    };
    let new_sheet = SheetData {
        name: "Report".into(),
        rows: vec![
            vec![cell_data("Name"), cell_data("Value")],
            vec![cell_data("X"), cell_data("99")],
        ],
    };

    let diffs = diff_sheet_data(&old_sheet, &new_sheet);

    // Construct a FileDiff for the semantic API
    let file_diff = excel_types::FileDiff {
        file_hash_match: false,
        sheet_diffs: vec![excel_types::SheetDiff {
            sheet_name: "Report".into(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: diffs,
        }],
        summary: excel_types::DiffSummary {
            adds: 0,
            deletes: 0,
            modifies: 1,
            passives: 0,
            total_changes: 1,
        },
    };

    let text = to_natural_text(&file_diff, None, Verbosity::Detail);
    assert!(text.contains("Report"));
    assert!(text.contains("changed") || text.contains("modif") || text.contains("Modif"));
}

// =======================================================================
// Business Scenario 9: New file vs empty file
// Real scenario: Comparing a newly created file against nothing.
// =======================================================================

#[test]
fn scenario_new_file_diff_vs_empty() {
    let old_path = fixture_path("diff_empty.xlsx");
    let new_path = fixture_path("diff_with_data.xlsx");
    cleanup(&old_path);
    cleanup(&new_path);

    // Create empty file
    let mut wb = rust_xlsxwriter::Workbook::new();
    wb.add_worksheet();
    wb.save(&old_path).expect("save empty");

    // Create file with data
    create_xlsx(
        &new_path,
        &[(
            "Data",
            &[
                vec![cell_data("ID"), cell_data("Name")],
                vec![cell_data("1"), cell_data("Test")],
            ],
        )],
    );

    let diff = diff_files(&old_path, &new_path).expect("diff files");
    assert!(!diff.file_hash_match);

    let summary = summarize(&diff.sheet_diffs);
    assert!(summary.total_changes > 0);

    cleanup(&old_path);
    cleanup(&new_path);
}

// =======================================================================
// Business Scenario 10: Summarize counts by diff type
// Real scenario: User wants a quick count of what changed.
// =======================================================================

#[test]
fn scenario_summarize_counts() {
    let sheet_diffs = vec![excel_types::SheetDiff {
        sheet_name: "S1".into(),
        row_count_diff: 0,
        col_count_diff: 0,
        cell_diffs: vec![
            excel_types::CellDiff {
                row: 0,
                col: 0,
                cell_ref: "A1".into(),
                diff_type: DiffType::Add,
                old_value: None,
                new_value: Some("new".into()),
                old_formula: None,
                new_formula: None,
            },
            excel_types::CellDiff {
                row: 1,
                col: 0,
                cell_ref: "A2".into(),
                diff_type: DiffType::Delete,
                old_value: Some("old".into()),
                new_value: None,
                old_formula: None,
                new_formula: None,
            },
            excel_types::CellDiff {
                row: 2,
                col: 0,
                cell_ref: "A3".into(),
                diff_type: DiffType::Modify,
                old_value: Some("10".into()),
                new_value: Some("20".into()),
                old_formula: None,
                new_formula: None,
            },
        ],
    }];

    let summary = summarize(&sheet_diffs);
    assert_eq!(summary.adds, 1);
    assert_eq!(summary.deletes, 1);
    assert_eq!(summary.modifies, 1);
    assert_eq!(summary.passives, 0);
    assert_eq!(summary.total_changes, 3);
}
