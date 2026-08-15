use super::*;

#[test]
fn test_sheet_rename_nonexistent() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&["sheet", "rename", &path, "Ghost", "Real"]);
    assert!(!r["success"].as_bool().unwrap_or(true));
}
