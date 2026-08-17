use super::*;

#[test]
fn test_formula_set_and_read() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["formula", "set", &path, "Sheet1", "C1", "=A1+B1"]);
    let d = run_json(&["formula", "read", &path, "Sheet1", "C1"]);
    assert_ok(&d);
    assert_eq!(d["formula"].as_str().unwrap(), "=A1+B1");
}

#[test]
fn test_formula_refresh() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
    run_json(&["cell", "write", &path, "Sheet1", "B1", "20"]);
    run_json(&["formula", "set", &path, "Sheet1", "C1", "=A1+B1"]);
    let r = run_json(&["formula", "refresh", &path, "Sheet1"]);
    assert_ok(&r);
}

#[test]
fn test_formula_dry_run() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let _ = run_json(&["formula", "set", &path, "Sheet1", "A1", "=1+1", "--dry-run"]);
    let d = run_json(&["formula", "read", &path, "Sheet1", "A1"]);
    assert_eq!(d["formula"].as_str(), None);
}
