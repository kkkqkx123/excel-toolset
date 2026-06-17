use std::fs;
use std::path::{Path, PathBuf};

use excel_diff::semantic::{self, Verbosity};
use excel_types::{CellDiff, DiffSummary, DiffType, FileDiff, SheetDiff};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn output_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/semantic_e2e/outputs");
    fs::create_dir_all(&dir).ok();
    dir
}

fn write_output(test_name: &str, plain: &str, json: &str) {
    let txt_path = output_dir().join(format!("{}.txt", test_name));
    let json_path = output_dir().join(format!("{}.json", test_name));
    fs::write(&txt_path, plain).ok();
    fs::write(&json_path, json).ok();
}

fn assert_golden(test_name: &str, actual_plain: &str, actual_json: &str) {
    let txt_path = output_dir().join(format!("{}.txt", test_name));
    let json_path = output_dir().join(format!("{}.json", test_name));
    let update = std::env::var("UPDATE_GOLDEN").is_ok();

    match (
        fs::read_to_string(&txt_path),
        fs::read_to_string(&json_path),
    ) {
        (Ok(exp_plain), Ok(exp_json)) => {
            if update {
                fs::write(&txt_path, actual_plain).expect("write golden txt");
                fs::write(&json_path, actual_json).expect("write golden json");
            } else {
                assert_eq!(
                    actual_plain, exp_plain,
                    "Golden file mismatch for {} (.txt). Run with UPDATE_GOLDEN=1 to regenerate.",
                    test_name
                );
                assert_eq!(
                    actual_json, exp_json,
                    "Golden file mismatch for {} (.json). Run with UPDATE_GOLDEN=1 to regenerate.",
                    test_name
                );
            }
        }
        _ => {
            fs::write(&txt_path, actual_plain).expect("write golden txt");
            fs::write(&json_path, actual_json).expect("write golden json");
            if !update {
                eprintln!(
                    "Golden file for '{}' created. Commit it and re-run tests.",
                    test_name
                );
            }
        }
    }
}

fn cell_diff(
    row: u32,
    col: u16,
    cell_ref: &str,
    diff_type: DiffType,
    old_val: Option<&str>,
    new_val: Option<&str>,
    old_formula: Option<&str>,
    new_formula: Option<&str>,
) -> CellDiff {
    CellDiff {
        row,
        col,
        cell_ref: cell_ref.into(),
        diff_type,
        old_value: old_val.map(|s| s.into()),
        new_value: new_val.map(|s| s.into()),
        old_formula: old_formula.map(|s| s.into()),
        new_formula: new_formula.map(|s| s.into()),
    }
}

fn make_file_diff(sheet_diffs: Vec<SheetDiff>) -> FileDiff {
    let mut summary = DiffSummary {
        adds: 0,
        deletes: 0,
        modifies: 0,
        passives: 0,
        total_changes: 0,
    };
    for sd in &sheet_diffs {
        for cd in &sd.cell_diffs {
            match cd.diff_type {
                DiffType::Add => summary.adds += 1,
                DiffType::Delete => summary.deletes += 1,
                DiffType::Modify => summary.modifies += 1,
                DiffType::Passive => summary.passives += 1,
                DiffType::NoChange => continue,
            }
            summary.total_changes += 1;
        }
    }

    FileDiff {
        file_hash_match: false,
        sheet_diffs,
        summary,
    }
}

fn sheet_diff(name: &str, cells: Vec<CellDiff>, row_delta: i32, col_delta: i32) -> SheetDiff {
    SheetDiff {
        sheet_name: name.into(),
        row_count_diff: row_delta,
        col_count_diff: col_delta,
        cell_diffs: cells,
    }
}

// ---------------------------------------------------------------------------
// Test scenarios
// ---------------------------------------------------------------------------

#[test]
fn e2e_empty_diff() {
    let diff = make_file_diff(vec![]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("empty_diff", &plain, &json);
    assert_golden("empty_diff", &plain, &json);
    assert_eq!(plain, "No changes");
    assert_eq!(diff.summary.total_changes, 0);
}

#[test]
fn e2e_single_cell_modified() {
    let diff = make_file_diff(vec![sheet_diff(
        "Sheet1",
        vec![cell_diff(
            0,
            0,
            "A1",
            DiffType::Modify,
            Some("old_value"),
            Some("new_value"),
            None,
            None,
        )],
        0,
        0,
    )]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("single_cell_modified", &plain, &json);
    assert_golden("single_cell_modified", &plain, &json);
    assert!(plain.contains("changed from old_value to new_value"));
    assert_eq!(diff.summary.modifies, 1);
}

#[test]
fn e2e_multiple_cells_multi_sheet() {
    let diff = make_file_diff(vec![
        sheet_diff(
            "Sheet1",
            vec![
                cell_diff(
                    0,
                    0,
                    "A1",
                    DiffType::Modify,
                    Some("10"),
                    Some("20"),
                    None,
                    None,
                ),
                cell_diff(
                    1,
                    1,
                    "B2",
                    DiffType::Modify,
                    Some("old"),
                    Some("new"),
                    None,
                    None,
                ),
            ],
            0,
            0,
        ),
        sheet_diff(
            "Sheet2",
            vec![cell_diff(
                2,
                0,
                "A3",
                DiffType::Modify,
                Some("x"),
                Some("y"),
                None,
                None,
            )],
            0,
            0,
        ),
    ]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("multiple_cells_multi_sheet", &plain, &json);
    assert_golden("multiple_cells_multi_sheet", &plain, &json);
    assert!(plain.contains("Sheet1"));
    assert!(plain.contains("Sheet2"));
}

#[test]
fn e2e_cell_added_deleted() {
    let diff = make_file_diff(vec![sheet_diff(
        "Sheet1",
        vec![
            cell_diff(
                0,
                0,
                "A1",
                DiffType::Add,
                None,
                Some("new_cell"),
                None,
                None,
            ),
            cell_diff(
                2,
                1,
                "B3",
                DiffType::Delete,
                Some("deleted_cell"),
                None,
                None,
                None,
            ),
        ],
        0,
        0,
    )]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("cell_added_deleted", &plain, &json);
    assert_golden("cell_added_deleted", &plain, &json);
    assert_eq!(diff.summary.adds, 1);
    assert_eq!(diff.summary.deletes, 1);
}

#[test]
fn e2e_mixed_operations() {
    let diff = make_file_diff(vec![sheet_diff(
        "Data",
        vec![
            cell_diff(
                0,
                0,
                "A1",
                DiffType::Modify,
                Some("old"),
                Some("new"),
                None,
                None,
            ),
            cell_diff(0, 1, "B1", DiffType::Add, None, Some("added"), None, None),
            cell_diff(2, 0, "A3", DiffType::Delete, Some("gone"), None, None, None),
            cell_diff(
                1,
                1,
                "B2",
                DiffType::Passive,
                Some("5"),
                Some("6"),
                Some("=A1+1"),
                Some("=A1+1"),
            ),
        ],
        0,
        0,
    )]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("mixed_operations", &plain, &json);
    assert_golden("mixed_operations", &plain, &json);
    assert_eq!(diff.summary.modifies, 1);
    assert_eq!(diff.summary.adds, 1);
    assert_eq!(diff.summary.deletes, 1);
    assert_eq!(diff.summary.passives, 1);
}

#[test]
fn e2e_formula_changes() {
    let diff = make_file_diff(vec![sheet_diff(
        "Formulas",
        vec![cell_diff(
            0,
            0,
            "A1",
            DiffType::Modify,
            Some("10"),
            Some("20"),
            Some("=SUM(B1:B3)"),
            Some("=AVERAGE(B1:B3)"),
        )],
        0,
        0,
    )]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("formula_changed", &plain, &json);
    assert_golden("formula_changed", &plain, &json);
    assert!(plain.contains("formula changed from"));
    assert!(plain.contains("=SUM(B1:B3)"));
    assert!(plain.contains("=AVERAGE(B1:B3)"));
}

#[test]
fn e2e_passive_value_change() {
    let diff = make_file_diff(vec![sheet_diff(
        "Passive",
        vec![cell_diff(
            0,
            0,
            "A1",
            DiffType::Passive,
            Some("10"),
            Some("20"),
            Some("=B1+1"),
            Some("=B1+1"),
        )],
        0,
        0,
    )]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("passive_value_change", &plain, &json);
    assert_golden("passive_value_change", &plain, &json);
    assert!(plain.contains("recalculated"));
    assert!(plain.contains("=B1+1"));
}

#[test]
fn e2e_sheet_added() {
    let cells = vec![
        cell_diff(0, 0, "A1", DiffType::Add, None, Some("Name"), None, None),
        cell_diff(0, 1, "B1", DiffType::Add, None, Some("Value"), None, None),
        cell_diff(1, 0, "A2", DiffType::Add, None, Some("Alice"), None, None),
        cell_diff(1, 1, "B2", DiffType::Add, None, Some("100"), None, None),
    ];
    let diff = make_file_diff(vec![sheet_diff("NewSheet", cells, 2, 2)]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("sheet_added", &plain, &json);
    assert_golden("sheet_added", &plain, &json);
    assert!(plain.contains("Sheet \"NewSheet\" added"));
    assert!(plain.contains("2 rows"));
    assert!(plain.contains("4 cells"));
}

#[test]
fn e2e_sheet_deleted() {
    let cells = vec![
        cell_diff(0, 0, "A1", DiffType::Delete, Some("Name"), None, None, None),
        cell_diff(
            0,
            1,
            "B1",
            DiffType::Delete,
            Some("Value"),
            None,
            None,
            None,
        ),
        cell_diff(1, 0, "A2", DiffType::Delete, Some("Bob"), None, None, None),
    ];
    let diff = make_file_diff(vec![sheet_diff("OldSheet", cells, -2, -1)]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("sheet_deleted", &plain, &json);
    assert_golden("sheet_deleted", &plain, &json);
    assert!(plain.contains("Sheet \"OldSheet\" deleted"));
}

#[test]
fn e2e_sheet_resized() {
    let diff = make_file_diff(vec![sheet_diff("Resized", vec![], 5, -2)]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("sheet_resized", &plain, &json);
    assert_golden("sheet_resized", &plain, &json);
    assert!(plain.contains("resized"));
    assert!(plain.contains("5 added rows"));
    assert!(plain.contains("2 removed columns"));
}

#[test]
fn e2e_row_added() {
    let cells = vec![
        cell_diff(3, 0, "A4", DiffType::Add, None, Some("Charlie"), None, None),
        cell_diff(3, 1, "B4", DiffType::Add, None, Some("300"), None, None),
        cell_diff(3, 2, "C4", DiffType::Add, None, Some("42"), None, None),
    ];
    let diff = make_file_diff(vec![sheet_diff("Data", cells, 1, 0)]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("row_added", &plain, &json);
    assert_golden("row_added", &plain, &json);
    assert!(plain.contains("Add row 4"));
    assert!(plain.contains("Charlie"));
    assert!(plain.contains("300"));
}

#[test]
fn e2e_row_deleted() {
    let cells = vec![
        cell_diff(
            1,
            0,
            "A2",
            DiffType::Delete,
            Some("DeletedRow"),
            None,
            None,
            None,
        ),
        cell_diff(1, 1, "B2", DiffType::Delete, Some("99"), None, None, None),
    ];
    let diff = make_file_diff(vec![sheet_diff("Data", cells, -1, 0)]);

    let plain = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("row_deleted", &plain, &json);
    assert_golden("row_deleted", &plain, &json);
    assert!(plain.contains("Delete row 2"));
    assert!(plain.contains("DeletedRow"));
    assert!(plain.contains("99"));
}

#[test]
fn e2e_summary_verbosity() {
    let diff = make_file_diff(vec![sheet_diff(
        "S",
        vec![
            cell_diff(
                0,
                0,
                "A1",
                DiffType::Modify,
                Some("a"),
                Some("b"),
                None,
                None,
            ),
            cell_diff(0, 1, "B1", DiffType::Add, None, Some("new"), None, None),
        ],
        0,
        0,
    )]);

    let summary_text = semantic::to_natural_text(&diff, None, Verbosity::Summary);
    let detail_text = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&diff).unwrap();

    write_output("summary_vs_detail", &summary_text, &json);
    assert_golden("summary_vs_detail", &summary_text, &json);
    let detail_path = output_dir().join("summary_vs_detail_detail.txt");
    fs::write(&detail_path, &detail_text).ok();

    assert!(!summary_text.contains("Cell"));
    assert!(detail_text.contains("Cell"));
    assert!(summary_text.contains("1 modified"));
    assert!(summary_text.contains("1 added"));
}

#[test]
fn e2e_identical_files() {
    let diff = make_file_diff(vec![sheet_diff("S", vec![], 0, 0)]);
    let mut hash_match_diff = diff;
    hash_match_diff.file_hash_match = true;

    let plain = semantic::to_natural_text(&hash_match_diff, None, Verbosity::Detail);
    let json = serde_json::to_string_pretty(&hash_match_diff).unwrap();

    write_output("identical_files", &plain, &json);
    assert_golden("identical_files", &plain, &json);
    assert_eq!(plain, "No changes");
    assert!(hash_match_diff.file_hash_match);
}

#[test]
fn e2e_semantic_report_json() {
    let diff = make_file_diff(vec![sheet_diff(
        "Sheet1",
        vec![
            cell_diff(
                0,
                0,
                "A1",
                DiffType::Modify,
                Some("10"),
                Some("20"),
                None,
                None,
            ),
            cell_diff(1, 1, "B2", DiffType::Add, None, Some("new_val"), None, None),
        ],
        0,
        0,
    )]);

    let report = semantic::to_semantic_report(&diff, None);
    let report_json = serde_json::json!({
        "summary": report.summary,
        "detail_sentences": report.detail_sentences,
        "operations_count": report.operations.len(),
    });
    let json = serde_json::to_string_pretty(&report_json).unwrap();

    write_output("semantic_report", "", &json);
    let golden_json_path = output_dir().join("semantic_report.json");
    let expected_json = fs::read_to_string(&golden_json_path);
    match expected_json {
        Ok(exp) => {
            assert_eq!(
                json, exp,
                "Golden file mismatch for semantic_report.json. Run with UPDATE_GOLDEN=1."
            );
        }
        Err(_) => {
            fs::write(&golden_json_path, &json).expect("write golden json");
            let update = std::env::var("UPDATE_GOLDEN").is_ok();
            if !update {
                eprintln!("Golden file for 'semantic_report' created. Commit it and re-run tests.");
            }
        }
    }
    assert_eq!(report.operations.len(), 2);
    assert_eq!(report.detail_sentences.len(), 2);
}
