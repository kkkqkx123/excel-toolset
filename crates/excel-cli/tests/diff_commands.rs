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
