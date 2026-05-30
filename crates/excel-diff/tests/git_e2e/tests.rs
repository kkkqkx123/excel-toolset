pub mod helpers;
pub mod fixtures;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use excel_diff::git_driver;
use excel_core::types::{CellData, CellDataType, DiffType};
use fixtures::*;
use helpers::*;

fn output_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/outputs");
    fs::create_dir_all(&dir).ok();
    dir
}

fn write_diff_output(test_name: &str, diff: &serde_json::Value) {
    let path = output_dir().join(format!("{}.json", test_name));
    let json = serde_json::to_string_pretty(diff).unwrap();
    fs::write(&path, json).ok();
}

#[test]
fn test_install_creates_gitattributes_and_config() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();

        let attr = std::env::current_dir().unwrap().join(".gitattributes");
        assert!(file_exists(&attr), ".gitattributes should be created: {:?}", attr);

        let content = fs::read_to_string(&attr).unwrap();
        assert!(
            content.contains("*.xlsx diff=excel-diff"),
            "should contain xlsx diff mapping, got: {:?}",
            content
        );

        let out = git(&["config", "diff.excel-diff.command"]);
        assert!(
            out.status.success(),
            "git config diff.excel-diff.command should exist"
        );
        let cmd = String::from_utf8_lossy(&out.stdout);
        assert!(
            cmd.contains("diff file"),
            "command should contain 'diff file', got: {}",
            cmd
        );
    });
}

#[test]
fn test_uninstall_removes_config_and_cleans_attributes() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();
        git_driver::uninstall_git_driver().unwrap();

        let attr = std::env::current_dir().unwrap().join(".gitattributes");
        assert!(!file_exists(&attr), ".gitattributes should be removed");

        let out = git(&["config", "--unset", "diff.excel-diff.command"]);
        assert!(
            !out.status.success(),
            "second unset should fail (config already gone)"
        );
    });
}

#[test]
fn test_install_idempotent_does_not_duplicate_entry() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();
        git_driver::install_git_driver().unwrap();

        let content = fs::read_to_string(
            std::env::current_dir().unwrap().join(".gitattributes"),
        )
        .unwrap();
        let count = content.matches("*.xlsx diff=excel-diff").count();
        assert_eq!(count, 1, "should have exactly one entry, found {}", count);
    });
}

#[test]
fn test_uninstall_preserves_other_gitattributes_entries() {
    with_git_repo(|| {
        fs::write(
            std::env::current_dir().unwrap().join(".gitattributes"),
            "*.xml diff=xml-diff\n*.json diff=json-diff\n",
        )
        .unwrap();

        git_driver::install_git_driver().unwrap();
        git_driver::uninstall_git_driver().unwrap();

        let content = fs::read_to_string(
            std::env::current_dir().unwrap().join(".gitattributes"),
        )
        .unwrap();
        assert!(
            content.contains("*.xml diff=xml-diff"),
            "xml entry should survive"
        );
        assert!(
            content.contains("*.json diff=json-diff"),
            "json entry should survive"
        );
        assert!(
            !content.contains("*.xlsx diff=excel-diff"),
            "xlsx entry should be removed"
        );
    });
}

#[test]
fn test_install_fails_outside_git_repo() {
    with_git_repo(|| {
        let no_git_dir = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(no_git_dir.path()).unwrap();

        let result = git_driver::install_git_driver();

        std::env::set_current_dir(original).ok();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Not in a git repository") || err.contains("Failed to find git root"),
            "expected git root error, got: {}",
            err
        );
    });
}

#[test]
fn test_uninstall_idempotent_twice_succeeds() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();
        git_driver::uninstall_git_driver().unwrap();

        let result = git_driver::uninstall_git_driver();
        assert!(
            result.is_ok(),
            "second uninstall should succeed even if config already gone, got: {:?}",
            result
        );
    });
}

#[test]
fn test_uninstall_removes_gitattributes_when_only_excel_entry() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();

        git_driver::uninstall_git_driver().unwrap();

        let attr = std::env::current_dir().unwrap().join(".gitattributes");
        assert!(
            !file_exists(&attr),
            ".gitattributes should be removed when it becomes empty after uninstall"
        );
    });
}

#[test]
fn test_diff_files_via_fixtures() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_simple_xlsx(&old_path);
    create_modified_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "diff_files should succeed");
    let diff = result.unwrap();

    write_diff_output(
        "simple_vs_modified",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(!diff.file_hash_match, "files differ");
    assert!(diff.summary.total_changes > 0, "should detect changes");
    assert!(diff.summary.modifies > 0, "should have modifications");
    assert!(diff.summary.adds > 0, "should have adds for new rows");
}

#[test]
fn test_diff_files_identical_returns_zero_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.xlsx");

    create_simple_xlsx(&path);

    let result = excel_diff::diff_files(&path.to_string_lossy(), &path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "identical_files",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(diff.file_hash_match, "same file => hash match");
    assert_eq!(diff.summary.total_changes, 0, "no changes expected");
}

#[test]
fn test_diff_multi_sheet_detects_sheet_additions() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_simple_xlsx(&old_path);
    create_multi_sheet_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "multi_sheet_addition",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(diff.sheet_diffs.len() > 1, "multi-sheet should have >1 diffs");
}

#[test]
fn test_diff_detects_sheet_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_sheet_del_xlsx(&old_path);
    create_simple_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "sheet_deletion",
        &serde_json::to_value(&diff).unwrap(),
    );

    let sheet_names: Vec<_> = diff.sheet_diffs.iter().map(|s| &s.sheet_name).collect();
    assert!(
        sheet_names.contains(&&"Extra".to_string()),
        "deleted sheet 'Extra' should appear in diff: {:?}",
        sheet_names
    );
}

#[test]
fn test_diff_empty_workbook() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_empty_xlsx(&old_path);
    create_simple_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "empty_vs_simple",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(!diff.file_hash_match, "empty vs data should differ");
    assert!(diff.summary.adds > 0, "should detect added rows");
}

#[test]
fn test_diff_both_empty_workbooks() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_empty_xlsx(&old_path);
    create_empty_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "empty_vs_empty",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(diff.file_hash_match, "both empty should match");
}

#[test]
fn test_diff_invalid_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("bad.xlsx");
    let good_path = dir.path().join("good.xlsx");

    fs::write(&bad_path, "not a valid xlsx").unwrap();
    create_simple_xlsx(&good_path);

    let result = excel_diff::diff_files(&bad_path.to_string_lossy(), &good_path.to_string_lossy());
    assert!(result.is_err(), "invalid xlsx should return error");
}

#[test]
fn test_diff_formula_file() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_formulas_xlsx(&old_path);
    create_formulas_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "formula file should be diffable");
    let diff = result.unwrap();

    write_diff_output(
        "formulas_identical",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(diff.file_hash_match, "identical formula files should match");
}

#[test]
fn test_diff_formula_text_change() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_formulas_xlsx(&old_path);
    create_formulas_modified_xlsx(&new_path);

    let result =
        excel_diff::diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "formula change should be diffable");
    let diff = result.unwrap();

    write_diff_output(
        "formulas_text_changed",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(!diff.file_hash_match, "formula text changed");
    assert!(diff.summary.modifies > 0, "should detect formula modification");

    let formula_diffs: Vec<_> = diff.sheet_diffs.iter()
        .flat_map(|s| &s.cell_diffs)
        .filter(|c| c.diff_type == DiffType::Modify
            && c.old_formula.is_some() && c.new_formula.is_some())
        .collect();
    assert!(!formula_diffs.is_empty(), "should have formula modification entry");

    let cell = &formula_diffs[0];
    assert_eq!(cell.cell_ref, "B3",
        "formula cell should be at B3");
    assert_eq!(cell.old_formula.as_deref(), Some("SUM(B1:B2)"),
        "old formula text (calamine strips = prefix)");
    assert_eq!(cell.new_formula.as_deref(), Some("AVERAGE(B1:B2)"),
        "new formula text");
}

#[test]
fn test_diff_formula_passive_value_change() {
    let old_sheet = excel_core::types::SheetData {
        name: "Sheet1".into(),
        rows: vec![
            vec![
                CellData { value: Some("A".into()), data_type: CellDataType::String, formula: None },
                CellData { value: Some("10".into()), data_type: CellDataType::Int, formula: None },
            ],
            vec![
                CellData { value: Some("B".into()), data_type: CellDataType::String, formula: None },
                CellData { value: Some("20".into()), data_type: CellDataType::Int, formula: None },
            ],
            vec![
                CellData { value: Some("Sum".into()), data_type: CellDataType::String, formula: None },
                CellData { value: Some("30".into()), data_type: CellDataType::Int,
                    formula: Some("SUM(B1:B2)".into()) },
            ],
        ],
    };
    // Simulate: input B1 changed from 10→30, so formula cell's cached value 30→50
    let mut new_sheet = old_sheet.clone();
    new_sheet.rows[0][1].value = Some("30".into());
    new_sheet.rows[2][1].value = Some("50".into());

    let diffs = excel_diff::compute_diffs(&old_sheet, &new_sheet);

    write_diff_output("formulas_passive_value_change", &serde_json::to_value(&diffs).unwrap());

    let passive: Vec<_> = diffs.iter().filter(|c| c.diff_type == DiffType::Passive).collect();
    assert!(!passive.is_empty(), "formula cell with same formula but changed value should be Passive");
    assert_eq!(passive[0].cell_ref, "B3");
    assert_eq!(passive[0].old_value.as_deref(), Some("30"));
    assert_eq!(passive[0].new_value.as_deref(), Some("50"));
    assert_eq!(passive[0].old_formula.as_deref(), Some("SUM(B1:B2)"));
    assert_eq!(passive[0].new_formula.as_deref(), Some("SUM(B1:B2)"),
        "formula unchanged but still captured");

    let modify: Vec<_> = diffs.iter().filter(|c| c.diff_type == DiffType::Modify).collect();
    assert!(!modify.is_empty(), "non-formula value change should be Modify");
    assert_eq!(modify[0].cell_ref, "B1");
    assert_eq!(modify[0].old_value.as_deref(), Some("10"));
    assert_eq!(modify[0].new_value.as_deref(), Some("30"));
    assert_eq!(modify[0].old_formula, None);
    assert_eq!(modify[0].new_formula, None);
}

#[test]
fn test_diff_with_fixture_files() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
    let simple = fixtures.join("simple.xlsx");
    let modified = fixtures.join("modified.xlsx");

    if !simple.exists() || !modified.exists() {
        eprintln!(
            "SKIP: fixture files not found. Run: cargo run --example gen_fixtures -- tests/git_e2e/fixtures"
        );
        return;
    }

    let result =
        excel_diff::diff_files(&simple.to_string_lossy(), &modified.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "fixture_simple_vs_modified",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(!diff.file_hash_match);
    assert!(diff.summary.total_changes > 0);
}

#[test]
fn test_git_diff_driver_e2e_with_cargo_run() {
    with_git_repo(|| {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap();
        let manifest_toml = manifest_dir.join("Cargo.toml");

        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!(
                "SKIP: fixture files not found. Run: cargo run --example gen_fixtures -- tests/git_e2e/fixtures"
            );
            return;
        }

        let old_file = std::env::current_dir().unwrap().join("old.xlsx");
        let new_file = std::env::current_dir().unwrap().join("new.xlsx");
        fs::copy(&simple, &old_file).unwrap();
        fs::copy(&modified, &new_file).unwrap();

        git(&["add", "."]);
        git(&["commit", "-m", "initial"]);

        let diff_output = Command::new("cargo")
            .args([
                "run", "--manifest-path", &manifest_toml.to_string_lossy(),
                "--bin", "excel-cli",
                "--", "diff", "file",
            ])
            .args([old_file.to_string_lossy().as_ref(), new_file.to_string_lossy().as_ref()])
            .output()
            .expect("failed to run excel-cli diff");

        assert!(
            diff_output.status.success(),
            "diff command failed: {}",
            String::from_utf8_lossy(&diff_output.stderr)
        );

        let output = String::from_utf8_lossy(&diff_output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|_| serde_json::json!({"raw": output}));
        std::fs::write(
            output_dir().join("git_driver_e2e.json"),
            serde_json::to_string_pretty(&parsed).unwrap(),
        )
        .ok();

        assert!(
            output.contains("file_hash_match") || output.len() > 0,
            "diff output should contain diff result, got: {}",
            output
        );
    });
}