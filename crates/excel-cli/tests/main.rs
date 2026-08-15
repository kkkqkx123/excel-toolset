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

mod file_commands;

// ===================================================================
// Sheet Commands
// ===================================================================

mod sheet_commands;

// ===================================================================
// Cell Commands
// ===================================================================

mod cell_commands;

// ===================================================================
// Range Commands
// ===================================================================

mod range_commands;

// ===================================================================
// Data Commands
// ===================================================================

mod data_commands;

// ===================================================================
// Formula Commands
// ===================================================================

mod formula_commands;

// ===================================================================
// Diff Commands
// ===================================================================

mod diff_commands;

// ===================================================================
// Batch Commands - CellValue now uses untagged serde
// ===================================================================

mod batch_commands;

// ===================================================================
// Format Commands
// ===================================================================

mod format_commands;

// ===================================================================
// Chart Commands
// ===================================================================

mod chart_commands;

// ===================================================================
// Comment Commands
// ===================================================================

mod comment_commands;

// ===================================================================
// Named Range Commands
// ===================================================================

mod named_range_commands;

// ===================================================================
// Search Commands
// ===================================================================

mod search_commands;

// ===================================================================
// Conditional Format Commands
// ===================================================================

mod conditional_format_commands;

// ===================================================================
// Rollback Commands
// ===================================================================

mod rollback_commands;

// ===================================================================
// End-to-End Workflow Scenarios
// ===================================================================

mod e2e_scenarios;

// ===================================================================
// VBA Commands
// ===================================================================

mod vba_commands;

// ===================================================================
// Data Filter Operator Coverage
// ===================================================================

mod data_filter_operators;

// ===================================================================
// Data Dedup by Column
// ===================================================================

mod data_dedup_column;

// ===================================================================
// Formula Advanced Commands
// ===================================================================

mod formula_advanced;

// ===================================================================
// Search Advanced Scenarios
// ===================================================================

mod search_advanced;

// ===================================================================
// Sheet Error Cases
// ===================================================================

mod sheet_error_cases;

// ===================================================================
// Output Format (--pretty)
// ===================================================================

mod output_format;

// ===================================================================
// Diff Git Driver Commands
// ===================================================================

mod diff_git_driver;
