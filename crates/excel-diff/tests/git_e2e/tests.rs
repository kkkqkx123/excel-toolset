pub mod helpers;
pub mod fixtures;

use std::fs;

use excel_diff::git_driver;
use fixtures::*;
use helpers::*;

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
    assert!(diff.sheet_diffs.len() > 1, "multi-sheet should have >1 diffs");
}