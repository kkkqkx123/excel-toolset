//! Integration tests for excel-cli
//!
//! Tests are organized by command group, each mapping to real business scenarios.
//! Every test corresponds to an actual CLI invocation pattern.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

fn ensure_binary_built() {
    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| {
        let status = Command::new("cargo")
            .args(["build", "-p", "excel-cli"])
            .status()
            .expect("Failed to run cargo build for excel-cli");
        assert!(
            status.success(),
            "cargo build -p excel-cli failed before tests"
        );
    });
}

fn cli() -> PathBuf {
    ensure_binary_built();
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push("excel-cli");
    path
}

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn test_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn test_dir(id: u64) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("test-tmp");
    path.push(format!("t{:04}", id));
    fs::create_dir_all(&path).ok();
    path
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(cli())
        .args(args)
        .output()
        .expect("CLI binary required (run `cargo build -p excel-cli` first)")
}

fn run_json(args: &[&str]) -> serde_json::Value {
    let output = run(args);
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Bad JSON output:\nstdout: {}\nstderr: {}", stdout, stderr)
    })
}

fn tf(id: u64, name: &str) -> String {
    let mut p = test_dir(id);
    p.push(name);
    p.to_string_lossy().to_string()
}

fn assert_ok(json: &serde_json::Value) {
    if let Some(s) = json.get("success").and_then(|v| v.as_bool()) {
        assert!(s, "Command failed: {}", json);
    }
}

fn mkfile(id: u64, name: &str) -> String {
    let path = tf(id, name);
    let _ = fs::remove_file(&path);
    let r = run_json(&["file", "create", &path]);
    assert_ok(&r);
    assert!(fs::metadata(&path).is_ok(), "File not created: {}", path);
    path
}

// ===================================================================
// File Commands
// ===================================================================

mod file_commands {
    use super::*;

    #[test]
    fn test_file_create_default_sheet() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let info = run_json(&["file", "info", &path]);
        let sheets = info["sheets"].as_array().unwrap();
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].as_str().unwrap(), "Sheet1");
    }

    #[test]
    fn test_file_create_custom_sheet() {
        let id = test_id();
        let path = tf(id, "f.xlsx");
        let r = run_json(&["file", "create", &path, "--sheet", "Data"]);
        assert_ok(&r);
        let info = run_json(&["file", "info", &path]);
        let sheets = info["sheets"].as_array().unwrap();
        assert_eq!(sheets[0].as_str().unwrap(), "Data");
    }

    #[test]
    fn test_file_info_returns_metadata() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let info = run_json(&["file", "info", &path]);
        assert!(info["hash"].as_str().is_some());
        assert!(info["size"].as_u64().is_some());
        assert!(info["sheets"].as_array().is_some());
    }

    #[test]
    fn test_file_backup_creates_copy() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["file", "backup", &path]);
        assert_ok(&r);
        let bp = r["backup_path"].as_str().unwrap();
        assert!(fs::metadata(bp).is_ok());
    }

    #[test]
    fn test_file_backup_with_output() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let out = tf(id, "bk.xlsx");
        let r = run_json(&["file", "backup", &path, "--output", &out]);
        assert_ok(&r);
        assert!(fs::metadata(&out).is_ok());
    }

    #[test]
    fn test_file_info_nonexistent_file() {
        let r = run_json(&["file", "info", "/nonexistent/file.xlsx"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }
}

// ===================================================================
// Sheet Commands
// ===================================================================

mod sheet_commands {
    use super::*;

    #[test]
    fn test_sheet_list_empty_file() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["sheet", "list", &path]);
        assert_ok(&r);
        assert_eq!(r["sheets"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_sheet_add_new() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["sheet", "add", &path, "Data"]);
        let r = run_json(&["sheet", "list", &path]);
        let sheets = r["sheets"].as_array().unwrap();
        assert_eq!(sheets.len(), 2);
    }

    #[test]
    fn test_sheet_add_duplicate_fails() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["sheet", "add", &path, "Dup"]);
        let r = run_json(&["sheet", "add", &path, "Dup"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_sheet_delete_existing() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["sheet", "add", &path, "Del"]);
        run_json(&["sheet", "delete", &path, "Del"]);
        let r = run_json(&["sheet", "list", &path]);
        let sheets = r["sheets"].as_array().unwrap();
        assert!(!sheets.iter().any(|s| s.as_str().unwrap() == "Del"));
    }

    #[test]
    fn test_sheet_delete_nonexistent_fails() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["sheet", "delete", &path, "Nope"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_sheet_rename() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["sheet", "add", &path, "Old"]);
        run_json(&["sheet", "rename", &path, "Old", "New"]);
        let r = run_json(&["sheet", "list", &path]);
        let sheets = r["sheets"].as_array().unwrap();
        assert!(sheets.iter().any(|s| s.as_str().unwrap() == "New"));
        assert!(!sheets.iter().any(|s| s.as_str().unwrap() == "Old"));
    }
}

// ===================================================================
// Cell Commands
// ===================================================================

mod cell_commands {
    use super::*;

    #[test]
    fn test_cell_write_number_reads_back() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "42"]);
        let d = run_json(&["cell", "read", &path, "Sheet1", "A1"]);
        assert_eq!(d["data_type"].as_str().unwrap(), "Float");
        assert_eq!(d["value"].as_str().unwrap(), "42");
    }

    #[test]
    fn test_cell_write_string_reads_back() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "Hello"]);
        let d = run_json(&["cell", "read", &path, "Sheet1", "A1"]);
        assert_eq!(d["data_type"].as_str().unwrap(), "String");
    }

    #[test]
    fn test_cell_write_bool_reads_back() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "true"]);
        let d = run_json(&["cell", "read", &path, "Sheet1", "A1"]);
        assert_eq!(d["data_type"].as_str().unwrap(), "Bool");
    }

    #[test]
    fn test_cell_write_multiple_cells() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "Name"]);
        run_json(&["cell", "write", &path, "Sheet1", "B1", "Age"]);
        run_json(&["cell", "write", &path, "Sheet1", "A2", "Alice"]);
        run_json(&["cell", "write", &path, "Sheet1", "B2", "30"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Name"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "B2"])["value"]
                .as_str()
                .unwrap(),
            "30"
        );
    }

    #[test]
    fn test_cell_write_dry_run_no_change() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let _ = run_json(&["cell", "write", &path, "Sheet1", "A1", "X", "--dry-run"]);
        let d = run_json(&["cell", "read", &path, "Sheet1", "A1"]);
        assert_eq!(d["data_type"].as_str().unwrap(), "Empty");
    }

    #[test]
    fn test_cell_read_nonexistent_sheet() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["cell", "read", &path, "NoSheet", "A1"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_cell_invalid_cell_ref() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["cell", "write", &path, "Sheet1", "INVALID", "t"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }
}

// ===================================================================
// Range Commands
// ===================================================================

mod range_commands {
    use super::*;

    #[test]
    fn test_range_read_empty_range() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let d = run_json(&["range", "read", &path, "Sheet1", "A1:C3"]);
        assert!(d.as_array().is_some());
    }

    #[test]
    fn test_range_write_json_grid() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let grid = r#"[["Name","Score"],["Alice","95"],["Bob","87"]]"#;
        let r = run_json(&["range", "write", &path, "Sheet1", "A1:B3", grid]);
        assert_ok(&r);
        let d = run_json(&["range", "read", &path, "Sheet1", "A1:B3"]);
        let rows = d.as_array().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0][0]["value"].as_str().unwrap(), "Name");
        assert_eq!(rows[1][1]["value"].as_str().unwrap(), "95");
    }

    #[test]
    fn test_range_clear() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:B2",
            r#"[["A","B"],["C","D"]]"#,
        ]);
        run_json(&["range", "clear", &path, "Sheet1", "A1:B2"]);
        let d = run_json(&["range", "read", &path, "Sheet1", "A1:B2"]);
        let rows = d.as_array().unwrap();
        assert_eq!(rows[0][0]["data_type"].as_str().unwrap(), "Empty");
    }

    #[test]
    fn test_range_write_csv() {
        let id = test_id();
        let csv = tf(id, "data.csv");
        fs::write(&csv, "Name,Age\nAlice,30\nBob,25").unwrap();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["range", "write-csv", &path, "Sheet1", "A1", &csv]);
        assert_ok(&r);
        let d = run_json(&["range", "read", &path, "Sheet1", "A1:B3"]);
        let rows = d.as_array().unwrap();
        assert_eq!(rows[0][0]["value"].as_str().unwrap(), "Name");
        assert_eq!(rows[1][1]["value"].as_str().unwrap(), "30");
    }

    #[test]
    fn test_range_write_dry_run_no_change() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let _ = run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1",
            r#"[["A"]]"#,
            "--dry-run",
        ]);
        let d = run_json(&["range", "read", &path, "Sheet1", "A1"]);
        assert_eq!(
            d.as_array().unwrap()[0][0]["data_type"].as_str().unwrap(),
            "Empty"
        );
    }
}

// ===================================================================
// Data Commands
// ===================================================================

mod data_commands {
    use super::*;

    #[test]
    fn test_data_append_row() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["data", "append-row", &path, "Sheet1", "Alice", "30", "Eng"]);
        assert_ok(&r);
        let d = run_json(&["range", "read", &path, "Sheet1", "A1:C1"]);
        let rows = d.as_array().unwrap();
        assert_eq!(rows[0][0]["value"].as_str().unwrap(), "Alice");
    }

    #[test]
    fn test_data_append_multiple_rows() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "Alice", "30"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Bob", "25"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Carol", "28"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Alice"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A2"])["value"]
                .as_str()
                .unwrap(),
            "Bob"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A3"])["value"]
                .as_str()
                .unwrap(),
            "Carol"
        );
    }

    #[test]
    fn test_data_insert_row() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "First"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Second"]);
        run_json(&["data", "insert-row", &path, "Sheet1", "2", "Inserted"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "First"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A2"])["value"]
                .as_str()
                .unwrap(),
            "Inserted"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A3"])["value"]
                .as_str()
                .unwrap(),
            "Second"
        );
    }

    #[test]
    fn test_data_delete_row() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "Keep"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Remove"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Keep2"]);
        run_json(&["data", "delete-row", &path, "Sheet1", "2"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Keep"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A2"])["value"]
                .as_str()
                .unwrap(),
            "Keep2"
        );
    }

    #[test]
    fn test_data_filter_eq() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "Alice", "30"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Bob", "30"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Carol", "25"]);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "eq", "30"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_data_sort_ascending() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "Carol", "25"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Alice", "30"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Bob", "20"]);
        run_json(&["data", "sort", &path, "Sheet1", "2"]);
        // Header row (row 0) preserved: "Carol"
        // Sorted body by col 2 asc: Bob(20), Alice(30)
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Carol"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A3"])["value"]
                .as_str()
                .unwrap(),
            "Alice"
        );
    }

    #[test]
    fn test_data_sort_descending() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "A", "10"]);
        run_json(&["data", "append-row", &path, "Sheet1", "B", "30"]);
        run_json(&["data", "append-row", &path, "Sheet1", "C", "20"]);
        run_json(&["data", "sort", &path, "Sheet1", "2", "--desc"]);
        // Header row (row 0) preserved: "A"
        // Sorted body by col 2 desc: B(30), C(20)
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "A"
        );
    }

    #[test]
    fn test_data_dedup_all_columns() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "A", "1"]);
        run_json(&["data", "append-row", &path, "Sheet1", "B", "2"]);
        run_json(&["data", "append-row", &path, "Sheet1", "A", "1"]);
        run_json(&["data", "dedup", &path, "Sheet1"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "A"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A2"])["value"]
                .as_str()
                .unwrap(),
            "B"
        );
    }

    #[test]
    fn test_data_dry_run_no_change() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let before = run_json(&["range", "read", &path, "Sheet1", "A1:A1"]);
        let _ = run_json(&["data", "append-row", &path, "Sheet1", "test", "--dry-run"]);
        let after = run_json(&["range", "read", &path, "Sheet1", "A1:A1"]);
        assert_eq!(before.to_string(), after.to_string());
    }
}

// ===================================================================
// Formula Commands
// ===================================================================

mod formula_commands {
    use super::*;

    #[test]
    fn test_formula_set_and_read() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["formula", "set", &path, "Sheet1", "C1", "=A1+B1"]);
        let d = run_json(&["formula", "read", &path, "Sheet1", "C1"]);
        assert_ok(&d);
        assert_eq!(d["formula"].as_str().unwrap(), "=A1+B1");
    }

    #[test]
    fn test_formula_refresh() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
        run_json(&["cell", "write", &path, "Sheet1", "B1", "20"]);
        run_json(&["formula", "set", &path, "Sheet1", "C1", "=A1+B1"]);
        let r = run_json(&["formula", "refresh", &path, "Sheet1"]);
        assert_ok(&r);
    }

    #[test]
    fn test_formula_dry_run() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let _ = run_json(&["formula", "set", &path, "Sheet1", "A1", "=1+1", "--dry-run"]);
        let d = run_json(&["formula", "read", &path, "Sheet1", "A1"]);
        assert_eq!(d["formula"].as_str(), None);
    }
}

// ===================================================================
// Diff Commands
// ===================================================================

mod diff_commands {
    use super::*;

    #[test]
    fn test_diff_file_no_changes() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["diff", "file", &path, &path]);
        assert_eq!(r["summary"]["total_changes"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_diff_file_detects_change() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "old"]);
        let bak = tf(id, "bak.xlsx");
        fs::copy(&path, &bak).unwrap();
        run_json(&["cell", "write", &path, "Sheet1", "A1", "new"]);
        let r = run_json(&["diff", "file", &bak, &path]);
        assert!(r["summary"]["total_changes"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_diff_file_with_sheet_filter() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["sheet", "add", &path, "Data"]);
        run_json(&["cell", "write", &path, "Data", "A1", "test"]);
        let bak = tf(id, "bak.xlsx");
        fs::copy(&path, &bak).unwrap();
        run_json(&["cell", "write", &path, "Data", "A1", "changed"]);
        let r = run_json(&["diff", "file", &bak, &path, "--sheet", "Data"]);
        assert_ok(&r);
    }

    #[test]
    fn test_diff_range_detects_change() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "old"]);
        let bak = tf(id, "bak.xlsx");
        fs::copy(&path, &bak).unwrap();
        run_json(&["cell", "write", &path, "Sheet1", "A1", "new"]);
        let r = run_json(&["diff", "range", &bak, &path, "Sheet1", "A1:A1"]);
        let diffs = r["cell_diffs"].as_array().unwrap();
        assert!(!diffs.is_empty());
    }

    #[test]
    fn test_diff_text_format() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "old"]);
        run_json(&["cell", "write", &path, "Sheet1", "B1", "extra"]);
        let bak = tf(id, "bak.xlsx");
        fs::copy(&path, &bak).unwrap();
        run_json(&["cell", "write", &path, "Sheet1", "A1", "new"]);
        let out = run(&["diff", "file", &bak, &path, "--format", "text"]);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.trim().starts_with('{'),
            "Text format should not be JSON"
        );
    }
}

// ===================================================================
// Batch Commands - CellValue now uses untagged serde
// ===================================================================

mod batch_commands {
    use super::*;

    #[test]
    fn test_batch_write_multiple_cells() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let ops = r#"[
            {"op":"write_cell","sheet":"Sheet1","row":0,"col":0,"value":"Header1"},
            {"op":"write_cell","sheet":"Sheet1","row":0,"col":1,"value":"Header2"},
            {"op":"write_cell","sheet":"Sheet1","row":1,"col":0,"value":"Data1"},
            {"op":"write_cell","sheet":"Sheet1","row":1,"col":1,"value":"Data2"}
        ]"#;
        let r = run_json(&["batch", "modify", &path, "--operations", ops]);
        assert_ok(&r);
        assert_eq!(r["succeeded_count"].as_u64().unwrap(), 4);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Header1"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "B2"])["value"]
                .as_str()
                .unwrap(),
            "Data2"
        );
    }

    #[test]
    fn test_batch_add_sheet_and_write() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let ops = r#"[
            {"op":"add_sheet","name":"Data"},
            {"op":"write_cell","sheet":"Data","row":0,"col":0,"value":"test"}
        ]"#;
        let r = run_json(&["batch", "modify", &path, "--operations", ops]);
        assert_ok(&r);
        assert_eq!(r["succeeded_count"].as_u64().unwrap(), 2);
        assert_eq!(
            run_json(&["cell", "read", &path, "Data", "A1"])["value"]
                .as_str()
                .unwrap(),
            "test"
        );
    }

    #[test]
    fn test_batch_dry_run() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let ops = r#"[{"op":"write_cell","sheet":"Sheet1","row":0,"col":0,"value":"nope"}]"#;
        let _ = run_json(&["batch", "modify", &path, "--operations", ops, "--dry-run"]);
        let d = run_json(&["cell", "read", &path, "Sheet1", "A1"]);
        assert_eq!(d["data_type"].as_str().unwrap(), "Empty");
    }

    #[test]
    fn test_batch_text_format() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let ops = r#"[{"op":"write_cell","sheet":"Sheet1","row":0,"col":0,"value":"t"}]"#;
        let out = run(&[
            "batch",
            "modify",
            &path,
            "--operations",
            ops,
            "--format",
            "text",
        ]);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(!text.trim().starts_with('{'));
    }

    #[test]
    fn test_batch_append_and_range_write() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let ops = r#"[
            {"op":"append_rows","sheet":"Sheet1","data":[["Alice","30"],["Bob","25"]]},
            {"op":"write_range","sheet":"Sheet1","range":"C1:C2","data":[["Eng"],["Mgr"]]}
        ]"#;
        let r = run_json(&["batch", "modify", &path, "--operations", ops]);
        assert_ok(&r);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Alice"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "C2"])["value"]
                .as_str()
                .unwrap(),
            "Mgr"
        );
    }
}

// ===================================================================
// Format Commands
// ===================================================================

mod format_commands {
    use super::*;

    #[test]
    fn test_format_set_style() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&[
            "format",
            "set",
            &path,
            "Sheet1",
            "A1",
            r#"{"bold":true,"font_size":14}"#,
        ]);
        assert_ok(&r);
    }

    #[test]
    fn test_format_merge_cells_default_value() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["range", "write", &path, "Sheet1", "A1", r#"[["Merged"]]"#]);
        let r = run_json(&["format", "merge", &path, "Sheet1", "A1:B2"]);
        assert_ok(&r);
    }

    #[test]
    fn test_format_merge_cells_custom_value() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&[
            "format", "merge", &path, "Sheet1", "A1:C3", "--value", "Summary",
        ]);
        assert_ok(&r);
    }
}

// ===================================================================
// Chart Commands
// ===================================================================

mod chart_commands {
    use super::*;

    #[test]
    fn test_chart_create_default_position() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:B4",
            r#"[["Month","Sales"],["Jan","100"],["Feb","200"],["Mar","150"]]"#,
        ]);
        let r = run_json(&[
            "chart", "create", &path, "Sheet1", "A1:B4", "column", "--title", "Sales",
        ]);
        assert_ok(&r);
    }

    #[test]
    fn test_chart_create_custom_position() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:B3",
            r#"[["X","Y"],["1","10"],["2","20"]]"#,
        ]);
        let r = run_json(&[
            "chart",
            "create",
            &path,
            "Sheet1",
            "A1:B3",
            "line",
            "--position",
            "D1",
        ]);
        assert_ok(&r);
    }

    #[test]
    fn test_chart_invalid_type() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["chart", "create", &path, "Sheet1", "A1", "bogus"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }
}

// ===================================================================
// Comment Commands
// ===================================================================

mod comment_commands {
    use super::*;

    #[test]
    fn test_comment_add_and_get() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["comments", "add", &path, "Sheet1", "A1", "My comment"]);
        let d = run_json(&["comments", "get", &path, "Sheet1", "A1"]);
        assert_eq!(d["text"].as_str().unwrap(), "My comment");
    }

    #[test]
    fn test_comment_update() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["comments", "add", &path, "Sheet1", "A1", "orig"]);
        run_json(&["comments", "update", &path, "Sheet1", "A1", "updated"]);
        let d = run_json(&["comments", "get", &path, "Sheet1", "A1"]);
        assert_eq!(d["text"].as_str().unwrap(), "updated");
    }

    #[test]
    fn test_comment_delete() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["comments", "add", &path, "Sheet1", "A1", "delme"]);
        let r = run_json(&["comments", "delete", &path, "Sheet1", "A1"]);
        assert_ok(&r);
    }
}

// ===================================================================
// Named Range Commands
// ===================================================================

mod named_range_commands {
    use super::*;

    #[test]
    fn test_named_range_create_and_list() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:B2",
            r#"[["a","b"],["c","d"]]"#,
        ]);
        run_json(&["named-range", "create", &path, "MyRange", "A1:B2"]);
        let r = run_json(&["named-range", "list", &path]);
        assert_ok(&r);
    }

    #[test]
    fn test_named_range_delete() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["named-range", "create", &path, "ToDel", "A1"]);
        let r = run_json(&["named-range", "delete", &path, "ToDel"]);
        assert_ok(&r);
    }
}

// ===================================================================
// Search Commands
// ===================================================================

mod search_commands {
    use super::*;

    #[test]
    fn test_search_sheet_contains() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:B2",
            r#"[["hello","world"],["foo","bar"]]"#,
        ]);
        let r = run_json(&["search", "sheet", &path, "Sheet1", "hello"]);
        let results = r["matches"]
            .as_array()
            .unwrap_or_else(|| r.as_array().unwrap());
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_workbook_across_sheets() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["range", "write", &path, "Sheet1", "A1", r#"[["hello"]]"#]);
        run_json(&["sheet", "add", &path, "Data"]);
        run_json(&["range", "write", &path, "Data", "A1", r#"[["world"]]"#]);
        let r = run_json(&["search", "workbook", &path, "world"]);
        let results = r["matches"]
            .as_array()
            .unwrap_or_else(|| r.as_array().unwrap());
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_no_match() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["range", "write", &path, "Sheet1", "A1", r#"[["test"]]"#]);
        let r = run_json(&["search", "sheet", &path, "Sheet1", "notfound"]);
        let results = r["matches"]
            .as_array()
            .unwrap_or_else(|| r.as_array().unwrap());
        assert!(results.is_empty());
    }
}

// ===================================================================
// Conditional Format Commands
// ===================================================================

mod conditional_format_commands {
    use super::*;

    #[test]
    fn test_conditional_format_cell_value() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&[
            "conditional-format",
            "add",
            &path,
            "Sheet1",
            "A1",
            "cell_value",
            ">10",
        ]);
        assert_ok(&r);
    }

    #[test]
    fn test_conditional_format_with_style() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let style = r##"{"bold":true,"font_color":"#FF0000"}"##;
        let r = run_json(&[
            "conditional-format",
            "add",
            &path,
            "Sheet1",
            "A1:A10",
            "cell_value",
            ">100",
            "--style",
            style,
        ]);
        assert_ok(&r);
    }

    #[test]
    fn test_conditional_format_remove() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "conditional-format",
            "add",
            &path,
            "Sheet1",
            "A1",
            "duplicate",
            "",
        ]);
        let r = run_json(&["conditional-format", "remove", &path, "Sheet1", "A1"]);
        assert_ok(&r);
    }

    #[test]
    fn test_conditional_format_invalid_rule_type() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&[
            "conditional-format",
            "add",
            &path,
            "Sheet1",
            "A1",
            "unknown_type",
            "",
        ]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }
}

// ===================================================================
// Rollback Commands
// ===================================================================

mod rollback_commands {
    use super::*;

    #[test]
    fn test_rollback_restores_file() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "original"]);
        let bk = run_json(&["file", "backup", &path]);
        let bk_path = bk["backup_path"].as_str().unwrap().to_string();
        run_json(&["cell", "write", &path, "Sheet1", "A1", "modified"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "modified"
        );
        run_json(&["rollback", &path, &bk_path]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "original"
        );
    }
}

// ===================================================================
// End-to-End Workflow Scenarios
// ===================================================================

mod e2e_scenarios {
    use super::*;

    #[test]
    fn test_create_edit_backup_rollback_flow() {
        let id = test_id();
        let path = tf(id, "flow.xlsx");
        let r = run_json(&["file", "create", &path, "--sheet", "Employees"]);
        assert_ok(&r);
        run_json(&[
            "data",
            "append-row",
            &path,
            "Employees",
            "Name",
            "Age",
            "Dept",
        ]);
        run_json(&[
            "data",
            "append-row",
            &path,
            "Employees",
            "Alice",
            "30",
            "Eng",
        ]);
        run_json(&["data", "append-row", &path, "Employees", "Bob", "25", "HR"]);
        let bk = run_json(&["file", "backup", &path]);
        let bk_path = bk["backup_path"].as_str().unwrap().to_string();
        run_json(&["cell", "write", &path, "Employees", "B2", "31"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Employees", "B2"])["value"]
                .as_str()
                .unwrap(),
            "31"
        );
        let diff = run_json(&["diff", "file", &bk_path, &path]);
        assert!(diff["summary"]["total_changes"].as_u64().unwrap() > 0);
        run_json(&["rollback", &path, &bk_path]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Employees", "B2"])["value"]
                .as_str()
                .unwrap(),
            "30"
        );
    }

    #[test]
    fn test_batch_workflow_complete_sheet() {
        let id = test_id();
        let path = tf(id, "batch_flow.xlsx");
        run_json(&["file", "create", &path, "--sheet", "Report"]);
        let ops = r#"[
            {"op":"write_cell","sheet":"Report","row":0,"col":0,"value":"ID"},
            {"op":"write_cell","sheet":"Report","row":0,"col":1,"value":"Name"},
            {"op":"write_cell","sheet":"Report","row":0,"col":2,"value":"Score"},
            {"op":"append_rows","sheet":"Report","data":[["1","Alice","95"],["2","Bob","87"],["3","Carol","92"]]},
            {"op":"set_formula","sheet":"Report","cell":"D1","formula":"=SUM(C2:C4)"},
            {"op":"add_sheet","name":"Summary"},
            {"op":"write_cell","sheet":"Summary","row":0,"col":0,"value":"Total Average"}
        ]"#;
        let r = run_json(&["batch", "modify", &path, "--operations", ops]);
        assert_ok(&r);
        assert_eq!(r["succeeded_count"].as_u64().unwrap(), 7);
        assert_eq!(
            run_json(&["cell", "read", &path, "Report", "A1"])["value"]
                .as_str()
                .unwrap(),
            "ID"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Report", "C2"])["value"]
                .as_str()
                .unwrap(),
            "95"
        );
        assert_eq!(
            run_json(&["formula", "read", &path, "Report", "D1"])["formula"]
                .as_str()
                .unwrap(),
            "=SUM(C2:C4)"
        );
        assert_eq!(
            run_json(&["sheet", "list", &path])["sheets"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_format_and_merge_workflow() {
        let id = test_id();
        let path = tf(id, "fmt_flow.xlsx");
        run_json(&["file", "create", &path]);
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:C1",
            r#"[["Title","Value","Note"]]"#,
        ]);
        let style = r##"{"bold":true,"font_size":16,"background_color":"#4472C4"}"##;
        run_json(&["format", "set", &path, "Sheet1", "A1:C1", style]);
        run_json(&[
            "format",
            "merge",
            &path,
            "Sheet1",
            "A1:C1",
            "--value",
            "Report Header",
        ]);
        run_json(&["data", "append-row", &path, "Sheet1", "Sales", "1000", "Q1"]);
        run_json(&[
            "data",
            "append-row",
            &path,
            "Sheet1",
            "Revenue",
            "2000",
            "Q1",
        ]);
        let cf_style = r##"{"font_color":"#006100"}"##;
        run_json(&[
            "conditional-format",
            "add",
            &path,
            "Sheet1",
            "B2:B3",
            "cell_value",
            ">500",
            "--style",
            cf_style,
        ]);
        let info = run_json(&["file", "info", &path]);
        assert_ok(&info);
    }
}

// ===================================================================
// VBA Commands
// ===================================================================

mod vba_commands {
    use super::*;

    #[test]
    fn test_vba_has_regular_xlsx() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["vba", "has", &path]);
        assert_ok(&r);
        assert!(!r["has_vba"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_vba_import_invalid_file() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let junk = tf(id, "junk.bin");
        fs::write(&junk, b"not a vba file").unwrap();
        let r = run_json(&["vba", "import", &path, &junk]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }
}

// ===================================================================
// Data Filter Operator Coverage
// ===================================================================

mod data_filter_operators {
    use super::*;

    fn setup_score_data(_id: u64, path: &str) {
        run_json(&["data", "append-row", path, "Sheet1", "Alice", "95"]);
        run_json(&["data", "append-row", path, "Sheet1", "Bob", "87"]);
        run_json(&["data", "append-row", path, "Sheet1", "Carol", "92"]);
        run_json(&["data", "append-row", path, "Sheet1", "Dave", "73"]);
    }

    #[test]
    fn test_filter_ne() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "ne", "87"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        let names: Vec<_> = rows
            .iter()
            .filter_map(|r| {
                r.as_array()
                    .and_then(|a| a.first())
                    .and_then(|c| c["value"].as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        assert!(!names.contains(&"Bob".to_string()));
    }

    #[test]
    fn test_filter_gt() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "gt", "90"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_filter_lt() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "lt", "80"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_filter_ge() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "ge", "92"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_filter_le() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "le", "73"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_filter_contains() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "1", "contains", "li"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_filter_no_match() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        setup_score_data(id, &path);
        let r = run_json(&["data", "filter", &path, "Sheet1", "2", "eq", "999"]);
        assert_ok(&r);
        let rows = r["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
    }
}

// ===================================================================
// Data Dedup by Column
// ===================================================================

mod data_dedup_column {
    use super::*;

    #[test]
    fn test_dedup_by_specific_column() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["data", "append-row", &path, "Sheet1", "Alice", "Eng"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Bob", "Eng"]);
        run_json(&["data", "append-row", &path, "Sheet1", "Carol", "HR"]);
        run_json(&["data", "dedup", &path, "Sheet1", "--column", "2"]);
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
                .as_str()
                .unwrap(),
            "Alice"
        );
        assert_eq!(
            run_json(&["cell", "read", &path, "Sheet1", "A2"])["value"]
                .as_str()
                .unwrap(),
            "Bob"
        );
    }
}

// ===================================================================
// Formula Advanced Commands
// ===================================================================

mod formula_advanced {
    use super::*;

    #[test]
    fn test_formula_calc_mode() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["formula", "calc-mode", &path, "--mode", "auto"]);
        assert_ok(&r);
    }

    #[test]
    fn test_formula_trace_dependencies() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
        run_json(&["cell", "write", &path, "Sheet1", "B1", "20"]);
        run_json(&["formula", "set", &path, "Sheet1", "C1", "=A1+B1"]);
        let r = run_json(&["formula", "trace", &path, "Sheet1", "C1"]);
        assert_ok(&r);
        assert!(r["cell"].as_str().is_some());
        assert!(r["direct_precedents"].as_array().is_some());
    }

    #[test]
    fn test_formula_explain() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
        run_json(&["cell", "write", &path, "Sheet1", "B1", "5"]);
        run_json(&["formula", "set", &path, "Sheet1", "C1", "=SUM(A1:B1)"]);
        let r = run_json(&["formula", "explain", &path, "Sheet1", "C1"]);
        assert_ok(&r);
        assert!(r["description"].as_str().is_some());
    }

    #[test]
    fn test_formula_explain_logic() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
        run_json(&["cell", "write", &path, "Sheet1", "B1", "5"]);
        run_json(&["formula", "set", &path, "Sheet1", "C1", "=SUM(A1:B1)"]);
        let r = run_json(&["formula", "explain-logic", &path, "Sheet1", "C1"]);
        assert_ok(&r);
        assert!(r["logic_flow"].as_array().is_some());
    }
}

// ===================================================================
// Search Advanced Scenarios
// ===================================================================

mod search_advanced {
    use super::*;

    #[test]
    fn test_search_exact_match() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:B2",
            r#"[["Alice","engineer"],["Bob","eng"]]"#,
        ]);
        let r = run_json(&[
            "search",
            "sheet",
            &path,
            "Sheet1",
            "eng",
            "--match-type",
            "exact",
        ]);
        let results = r["matches"]
            .as_array()
            .unwrap_or_else(|| r.as_array().unwrap());
        assert!(!results.is_empty());
        for item in results {
            if let Some(v) = item["value"].as_str() {
                assert_eq!(v, "eng");
            }
        }
    }

    #[test]
    fn test_search_case_sensitive() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:A2",
            r#"[["Hello"],["hello"]]"#,
        ]);
        let r = run_json(&[
            "search",
            "sheet",
            &path,
            "Sheet1",
            "Hello",
            "--case-sensitive",
        ]);
        let results = r["matches"]
            .as_array()
            .unwrap_or_else(|| r.as_array().unwrap());
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_regex() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        run_json(&[
            "range",
            "write",
            &path,
            "Sheet1",
            "A1:A3",
            r#"[["abc123"],["def456"],["xyz789"]]"#,
        ]);
        let r = run_json(&[
            "search",
            "sheet",
            &path,
            "Sheet1",
            r"\d{3}",
            "--match-type",
            "regex",
        ]);
        let results = r["matches"]
            .as_array()
            .unwrap_or_else(|| r.as_array().unwrap());
        assert_eq!(results.len(), 3);
    }
}

// ===================================================================
// Sheet Error Cases
// ===================================================================

mod sheet_error_cases {
    use super::*;

    #[test]
    fn test_sheet_rename_nonexistent() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let r = run_json(&["sheet", "rename", &path, "Ghost", "Real"]);
        assert!(!r["success"].as_bool().unwrap_or(true));
    }
}

// ===================================================================
// Output Format (--pretty)
// ===================================================================

mod output_format {
    use super::*;

    #[test]
    fn test_pretty_output_is_multiline_json() {
        let id = test_id();
        let path = mkfile(id, "f.xlsx");
        let out = run(&["--pretty", "file", "info", &path]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains('\n'));
        assert!(stdout.contains("  "));
        assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
    }
}

// ===================================================================
// Diff Git Driver Commands
// ===================================================================

mod diff_git_driver {
    use super::*;

    #[test]
    fn test_diff_install_git_driver() {
        let r = run_json(&["diff", "install-git-driver"]);
        assert_ok(&r);
    }

    #[test]
    fn test_diff_uninstall_git_driver() {
        let r = run_json(&["diff", "uninstall-git-driver"]);
        assert_ok(&r);
    }
}
