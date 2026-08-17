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
