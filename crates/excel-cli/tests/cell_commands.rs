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
