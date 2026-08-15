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
