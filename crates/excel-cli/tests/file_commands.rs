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
