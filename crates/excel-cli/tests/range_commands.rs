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
