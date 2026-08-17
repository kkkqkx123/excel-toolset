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

#[path = "cmd/file_commands.rs"]
mod file_commands;

// ===================================================================
// Sheet Commands
// ===================================================================

#[path = "cmd/sheet_commands.rs"]
mod sheet_commands;

// ===================================================================
// Cell Commands
// ===================================================================

#[path = "cmd/cell_commands.rs"]
mod cell_commands;

// ===================================================================
// Range Commands
// ===================================================================

#[path = "cmd/range_commands.rs"]
mod range_commands;

// ===================================================================
// Data Commands
// ===================================================================

#[path = "cmd/data_commands.rs"]
mod data_commands;

// ===================================================================
// Formula Commands
// ===================================================================

#[path = "cmd/formula_commands.rs"]
mod formula_commands;

// ===================================================================
// Diff Commands
// ===================================================================

#[path = "cmd/diff_commands.rs"]
mod diff_commands;

// ===================================================================
// Batch Commands - CellValue now uses untagged serde
// ===================================================================

#[path = "cmd/batch_commands.rs"]
mod batch_commands;

// ===================================================================
// Format Commands
// ===================================================================

#[path = "cmd/format_commands.rs"]
mod format_commands;

// ===================================================================
// Chart Commands
// ===================================================================

#[path = "cmd/chart_commands.rs"]
mod chart_commands;

// ===================================================================
// Comment Commands
// ===================================================================

#[path = "cmd/comment_commands.rs"]
mod comment_commands;

// ===================================================================
// Named Range Commands
// ===================================================================

#[path = "cmd/named_range_commands.rs"]
mod named_range_commands;

// ===================================================================
// Search Commands
// ===================================================================

#[path = "cmd/search_commands.rs"]
mod search_commands;

// ===================================================================
// Conditional Format Commands
// ===================================================================

#[path = "cmd/conditional_format_commands.rs"]
mod conditional_format_commands;

// ===================================================================
// Rollback Commands
// ===================================================================

#[path = "cmd/rollback_commands.rs"]
mod rollback_commands;

// ===================================================================
// End-to-End Workflow Scenarios
// ===================================================================

#[path = "cmd/e2e_scenarios.rs"]
mod e2e_scenarios;

// ===================================================================
// VBA Commands
// ===================================================================

#[path = "cmd/vba_commands.rs"]
mod vba_commands;

// ===================================================================
// Data Filter Operator Coverage
// ===================================================================

#[path = "cmd/data_filter_operators.rs"]
mod data_filter_operators;

// ===================================================================
// Data Dedup by Column
// ===================================================================

#[path = "cmd/data_dedup_column.rs"]
mod data_dedup_column;

// ===================================================================
// Formula Advanced Commands
// ===================================================================

#[path = "cmd/formula_advanced.rs"]
mod formula_advanced;

// ===================================================================
// Search Advanced Scenarios
// ===================================================================

#[path = "cmd/search_advanced.rs"]
mod search_advanced;

// ===================================================================
// Sheet Error Cases
// ===================================================================

#[path = "cmd/sheet_error_cases.rs"]
mod sheet_error_cases;

// ===================================================================
// Output Format (--pretty)
// ===================================================================

#[path = "cmd/output_format.rs"]
mod output_format;

// ===================================================================
// Diff Git Driver Commands
// ===================================================================

#[path = "cmd/diff_git_driver.rs"]
mod diff_git_driver;
