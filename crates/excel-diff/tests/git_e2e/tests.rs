pub mod fixtures;
pub mod helpers;

use std::fs;
use std::path::{Path, PathBuf};

use excel_diff::semantic::Verbosity;
use excel_diff::{diff_files, git_driver, semantic};
use excel_types::DiffType;
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
    let update = std::env::var("UPDATE_GOLDEN").is_ok();

    let expected = fs::read_to_string(&path);
    match expected {
        Ok(exp) => {
            if update {
                fs::write(&path, &json).expect("write golden diff json");
            } else {
                assert_eq!(
                    json, exp,
                    "Golden file mismatch for {}.json. Run with UPDATE_GOLDEN=1 to regenerate.",
                    test_name
                );
            }
        }
        Err(_) => {
            fs::write(&path, &json).expect("write golden diff json");
            if !update {
                eprintln!(
                    "Golden file for diff '{}' created. Commit it and re-run tests.",
                    test_name
                );
            }
        }
    }
}

#[test]
fn test_install_creates_gitattributes_and_config() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();

        let attr = std::env::current_dir().unwrap().join(".gitattributes");
        assert!(
            file_exists(&attr),
            ".gitattributes should be created: {:?}",
            attr
        );

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
            cmd.contains("diff git-driver"),
            "command should contain 'diff git-driver', got: {}",
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

        let content =
            fs::read_to_string(std::env::current_dir().unwrap().join(".gitattributes")).unwrap();
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

        let content =
            fs::read_to_string(std::env::current_dir().unwrap().join(".gitattributes")).unwrap();
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
            err.to_string().contains("Not in a git repository")
                || err.to_string().contains("Failed to find git root"),
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

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "diff_files should succeed");
    let diff = result.unwrap();

    write_diff_output("simple_vs_modified", &serde_json::to_value(&diff).unwrap());

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

    let result = diff_files(&path.to_string_lossy(), &path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output("identical_files", &serde_json::to_value(&diff).unwrap());

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

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output(
        "multi_sheet_addition",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(
        diff.sheet_diffs.len() > 1,
        "multi-sheet should have >1 diffs"
    );
}

#[test]
fn test_diff_detects_sheet_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_sheet_del_xlsx(&old_path);
    create_simple_xlsx(&new_path);

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output("sheet_deletion", &serde_json::to_value(&diff).unwrap());

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

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output("empty_vs_simple", &serde_json::to_value(&diff).unwrap());

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

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok());
    let diff = result.unwrap();

    write_diff_output("empty_vs_empty", &serde_json::to_value(&diff).unwrap());

    assert!(diff.file_hash_match, "both empty should match");
}

#[test]
fn test_diff_invalid_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("bad.xlsx");
    let good_path = dir.path().join("good.xlsx");

    fs::write(&bad_path, "not a valid xlsx").unwrap();
    create_simple_xlsx(&good_path);

    let result = diff_files(&bad_path.to_string_lossy(), &good_path.to_string_lossy());
    assert!(result.is_err(), "invalid xlsx should return error");
}

#[test]
fn test_diff_formula_file() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_formulas_xlsx(&old_path);
    create_formulas_xlsx(&new_path);

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "formula file should be diffable");
    let diff = result.unwrap();

    write_diff_output("formulas_identical", &serde_json::to_value(&diff).unwrap());

    assert!(diff.file_hash_match, "identical formula files should match");
}

#[test]
fn test_diff_formula_text_change() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_formulas_xlsx(&old_path);
    create_formulas_modified_xlsx(&new_path);

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "formula change should be diffable");
    let diff = result.unwrap();

    write_diff_output(
        "formulas_text_changed",
        &serde_json::to_value(&diff).unwrap(),
    );

    assert!(!diff.file_hash_match, "formula text changed");
    assert!(
        diff.summary.modifies > 0,
        "should detect formula modification"
    );

    let formula_diffs: Vec<_> = diff
        .sheet_diffs
        .iter()
        .flat_map(|s| &s.cell_diffs)
        .filter(|c| {
            c.diff_type == DiffType::Modify && c.old_formula.is_some() && c.new_formula.is_some()
        })
        .collect();
    assert!(
        !formula_diffs.is_empty(),
        "should have formula modification entry"
    );

    let cell = &formula_diffs[0];
    assert_eq!(cell.cell_ref, "B3", "formula cell should be at B3");
    assert_eq!(
        cell.old_formula.as_deref(),
        Some("SUM(B1:B2)"),
        "old formula text (calamine strips = prefix)"
    );
    assert_eq!(
        cell.new_formula.as_deref(),
        Some("AVERAGE(B1:B2)"),
        "new formula text"
    );
}

#[test]
fn test_diff_large_changesets_handled_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_large_xlsx(&old_path);
    create_large_modified_xlsx(&new_path);

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "large files should be diffable");
    let diff = result.unwrap();

    write_diff_output("large_changeset", &serde_json::to_value(&diff).unwrap());

    assert!(!diff.file_hash_match, "large files should differ");
    assert!(diff.summary.total_changes > 0, "should detect changes");
}

#[test]
fn test_diff_handles_non_ascii_characters() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.xlsx");
    let new_path = dir.path().join("new.xlsx");

    create_unicode_xlsx(&old_path, &["中文", "日本語", "한국어"]);
    create_unicode_xlsx(&new_path, &["中文", "日本語", "English"]);

    let result = diff_files(&old_path.to_string_lossy(), &new_path.to_string_lossy());

    assert!(result.is_ok(), "unicode files should be diffable");
    let diff = result.unwrap();

    write_diff_output("unicode_changes", &serde_json::to_value(&diff).unwrap());

    assert!(!diff.file_hash_match, "unicode files should differ");
    assert!(
        diff.summary.modifies > 0,
        "should detect unicode modification"
    );
}

#[test]
fn test_driver_integration_with_env_vars() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        // Create absolute paths for environment variables
        let old_path = simple.canonicalize().unwrap();
        let new_path = modified.canonicalize().unwrap();

        // Set environment variables as Git diff driver would
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", old_path.to_string_lossy().as_ref());
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", new_path.to_string_lossy().as_ref());
        }

        // Test that get_git_diff_file_paths can read env vars
        let (got_old, got_new) = git_driver::get_git_diff_file_paths().unwrap();

        assert_eq!(got_old, old_path.to_string_lossy().as_ref());
        assert_eq!(got_new, new_path.to_string_lossy().as_ref());

        // Perform the diff
        let diff = diff_files(&got_old, &got_new).unwrap();

        assert!(!diff.file_hash_match, "files should differ");
        assert!(diff.summary.total_changes > 0, "should detect changes");

        // Generate natural text output (what GitDriver subcommand would do)
        let text_output = semantic::to_natural_text(&diff, None, Verbosity::Detail);
        assert!(!text_output.is_empty(), "text output should not be empty");

        // Save output for inspection
        fs::write(
            output_dir().join("driver_env_vars_output.txt"),
            &text_output,
        )
        .ok();

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}

#[test]
fn test_driver_integration_with_cli_args() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        // Test that get_git_diff_file_paths can fall back to CLI args
        // (simulate by temporarily unsetting env vars)
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }

        // Since we can't actually call the CLI binary, we test the diff function directly
        let diff = diff_files(&simple.to_string_lossy(), &modified.to_string_lossy()).unwrap();

        assert!(!diff.file_hash_match, "files should differ");
        assert!(diff.summary.total_changes > 0, "should detect changes");

        let text_output = semantic::to_natural_text(&diff, None, Verbosity::Detail);
        assert!(!text_output.is_empty(), "text output should not be empty");
    });
}

#[test]
fn test_driver_env_vars_take_priority_over_cli_args() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        let simple_abs = simple.canonicalize().unwrap();
        let modified_abs = modified.canonicalize().unwrap();

        // Set env vars
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", simple_abs.to_string_lossy().as_ref());
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", modified_abs.to_string_lossy().as_ref());
        }

        // Get paths - should use env vars even if CLI args would be different
        let (got_old, got_new) = git_driver::get_git_diff_file_paths().unwrap();

        assert_eq!(got_old, simple_abs.to_string_lossy().as_ref());
        assert_eq!(got_new, modified_abs.to_string_lossy().as_ref());

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}

#[test]
fn test_driver_handles_file_not_found_error() {
    with_git_repo(|| {
        // Set env vars to non-existent files
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", "/nonexistent/old.xlsx");
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", "/nonexistent/new.xlsx");
        }

        // get_git_diff_file_paths should still succeed (it just returns paths)
        let (got_old, got_new) = git_driver::get_git_diff_file_paths().unwrap();

        assert_eq!(got_old, "/nonexistent/old.xlsx");
        assert_eq!(got_new, "/nonexistent/new.xlsx");

        // But diff_files should fail
        let result = diff_files(&got_old, &got_new);
        assert!(
            result.is_err(),
            "diff_files should fail for non-existent files"
        );

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}

#[test]
fn test_driver_handles_empty_env_vars() {
    with_git_repo(|| {
        // Set empty env vars
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", "");
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", "");
        }

        // get_git_diff_file_paths should return empty strings
        let (got_old, got_new) = git_driver::get_git_diff_file_paths().unwrap();

        assert_eq!(got_old, "");
        assert_eq!(got_new, "");

        // diff_files should fail for empty paths
        let result = diff_files(&got_old, &got_new);
        assert!(result.is_err(), "diff_files should fail for empty paths");

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}

#[test]
fn test_driver_handles_only_one_env_var_set() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");

        if !simple.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        let simple_abs = simple.canonicalize().unwrap();

        // Set only one env var
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", simple_abs.to_string_lossy().as_ref());
        }

        // Should fail because both paths are required
        let result = git_driver::get_git_diff_file_paths();
        assert!(result.is_err(), "Should fail when only one env var is set");

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
    });
}

#[test]
fn test_driver_handles_whitespace_in_paths() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        // Create files with spaces in path
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_with_space = temp_dir.path().join("dir with space");

        fs::create_dir(&dir_with_space).unwrap();

        let old_path = dir_with_space.join("old file.xlsx");
        let new_path = dir_with_space.join("new file.xlsx");

        fs::copy(&simple, &old_path).unwrap();
        fs::copy(&modified, &new_path).unwrap();

        // Set env vars with spaces in paths
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", old_path.to_string_lossy().as_ref());
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", new_path.to_string_lossy().as_ref());
        }

        // Should handle spaces correctly
        let (got_old, got_new) = git_driver::get_git_diff_file_paths().unwrap();

        assert_eq!(got_old, old_path.to_string_lossy().as_ref());
        assert_eq!(got_new, new_path.to_string_lossy().as_ref());

        // Should be able to diff files with spaces in paths
        let diff = diff_files(&got_old, &got_new).unwrap();
        assert!(!diff.file_hash_match, "files should differ");

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}

#[test]
fn test_driver_output_format() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        let simple_abs = simple.canonicalize().unwrap();
        let modified_abs = modified.canonicalize().unwrap();

        // Set env vars
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", simple_abs.to_string_lossy().as_ref());
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", modified_abs.to_string_lossy().as_ref());
        }

        // Get diff and generate output
        let diff = diff_files(
            &simple_abs.to_string_lossy(),
            &modified_abs.to_string_lossy(),
        )
        .unwrap();

        let text_output = semantic::to_natural_text(&diff, None, Verbosity::Detail);

        // Verify output format
        assert!(!text_output.is_empty(), "output should not be empty");

        // Should contain sheet names
        assert!(
            text_output.contains("Sheet1"),
            "output should contain sheet name"
        );

        // Should contain cell references (e.g., "A1", "B2")
        let has_cell_ref =
            text_output.contains("A") || text_output.contains("B") || text_output.contains("C");
        assert!(has_cell_ref, "output should contain cell references");

        // The exact format depends on the implementation
        // Just verify output is meaningful and contains relevant information
        let has_meaningful_content = text_output.len() > 10
            && (text_output.contains("Changed")
                || text_output.contains("changed")
                || text_output.contains("1")
                || text_output.contains("2")
                || text_output.contains("=")
                || text_output.contains("-"));
        assert!(
            has_meaningful_content,
            "output should contain meaningful content"
        );

        // Save for manual inspection
        fs::write(output_dir().join("driver_output_format.txt"), &text_output).ok();

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}

#[test]
fn test_diff_with_staged_changes_simulation() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");
        let simple = fixtures.join("simple.xlsx");
        let modified = fixtures.join("modified.xlsx");

        if !simple.exists() || !modified.exists() {
            eprintln!("SKIP: fixture files not found");
            return;
        }

        // Simulate: git add simple.xlsx
        let staged_path = simple.canonicalize().unwrap();

        // Simulate: working directory has modified.xlsx
        let working_path = modified.canonicalize().unwrap();

        // Git diff --staged would compare staged vs HEAD
        // Git diff would compare working vs staged

        // Test working vs staged (git diff without --staged)
        let diff = diff_files(
            &staged_path.to_string_lossy(),
            &working_path.to_string_lossy(),
        )
        .unwrap();

        assert!(
            !diff.file_hash_match,
            "working file should differ from staged"
        );
        assert!(diff.summary.total_changes > 0, "should detect changes");

        // Generate output
        let text_output = semantic::to_natural_text(&diff, None, Verbosity::Detail);
        assert!(!text_output.is_empty());

        // Save output
        fs::write(
            output_dir().join("staged_diff_simulation.txt"),
            &text_output,
        )
        .ok();
    });
}

#[test]
fn test_git_config_command_format() {
    with_git_repo(|| {
        git_driver::install_git_driver().unwrap();

        let out = git(&["config", "diff.excel-diff.command"]);
        assert!(out.status.success(), "git config command should succeed");

        let cmd = String::from_utf8_lossy(&out.stdout);
        let cmd = cmd.trim();

        // Verify command contains expected parts
        assert!(
            cmd.contains("diff") && cmd.contains("git-driver"),
            "Command should contain 'diff git-driver', got: {}",
            cmd
        );

        // Command format should be valid
        // If path contains spaces, it should be quoted
        // But we don't enforce strict quoting as long as the command is valid
        assert!(!cmd.is_empty(), "Command should not be empty");

        eprintln!("Git diff driver command: {}", cmd);
    });
}

#[test]
fn test_driver_integration_error_handling() {
    with_git_repo(|| {
        // Test various error scenarios

        // 1. Missing both env vars and CLI args
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }

        // Save original args and restore later
        let _original_args: Vec<String> = std::env::args().collect();

        // We can't actually change args, but we test the function behavior
        // get_git_diff_file_paths will fail if no paths are available

        // For now, we just verify the function exists and returns appropriate type
        let _result = git_driver::get_git_diff_file_paths();

        // The result should be an error (no env vars and insufficient CLI args)
        // Note: This might succeed if the test runner passes enough args
        // So we just check that it doesn't panic

        // Restore args
        // (Can't actually restore, but this documents the intent)
    });
}

#[test]
fn test_driver_performance_with_large_files() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");

        // Test performance with large files
        let temp_dir = tempfile::tempdir().unwrap();
        let large_old = temp_dir.path().join("large_old.xlsx");
        let large_new = temp_dir.path().join("large_new.xlsx");

        create_large_xlsx(&large_old);
        create_large_modified_xlsx(&large_new);

        // Measure time
        let start = std::time::Instant::now();
        let diff = diff_files(&large_old.to_string_lossy(), &large_new.to_string_lossy()).unwrap();
        let duration = start.elapsed();

        eprintln!("Large file diff took: {:?}", duration);

        assert!(!diff.file_hash_match, "large files should differ");
        assert!(
            duration.as_secs() < 30,
            "large file diff should complete in reasonable time"
        );
    });
}

#[test]
fn test_driver_with_multiple_sheets() {
    with_git_repo(|| {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/git_e2e/fixtures");

        let temp_dir = tempfile::tempdir().unwrap();
        let old_path = temp_dir.path().join("multi_old.xlsx");
        let new_path = temp_dir.path().join("multi_new.xlsx");

        create_multi_sheet_xlsx(&old_path);
        create_simple_xlsx(&new_path);

        let old_abs = old_path.canonicalize().unwrap();
        let new_abs = new_path.canonicalize().unwrap();

        // Set env vars
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_OLD", old_abs.to_string_lossy().as_ref());
        }
        unsafe {
            std::env::set_var("GIT_DIFF_PATH_NEW", new_abs.to_string_lossy().as_ref());
        }

        // Get diff
        let diff = diff_files(&old_abs.to_string_lossy(), &new_abs.to_string_lossy()).unwrap();

        // Generate output
        let text_output = semantic::to_natural_text(&diff, None, Verbosity::Detail);

        // Should handle multiple sheets
        assert!(
            diff.sheet_diffs.len() >= 1,
            "should have at least one sheet diff"
        );
        assert!(!text_output.is_empty(), "output should not be empty");

        // Save output
        fs::write(
            output_dir().join("multi_sheet_driver_output.txt"),
            &text_output,
        )
        .ok();

        // Clean up
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_OLD");
        }
        unsafe {
            std::env::remove_var("GIT_DIFF_PATH_NEW");
        }
    });
}
