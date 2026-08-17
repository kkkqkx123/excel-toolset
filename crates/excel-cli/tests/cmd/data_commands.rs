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
