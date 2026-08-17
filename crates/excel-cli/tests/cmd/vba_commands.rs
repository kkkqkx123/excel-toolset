use super::*;

#[test]
fn test_vba_has_regular_xlsx() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&["vba", "has", &path]);
    assert_ok(&r);
    assert!(!r["has_vba"].as_bool().unwrap_or(true));
}

#[test]
fn test_vba_import_invalid_file() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let junk = tf(id, "junk.bin");
    fs::write(&junk, b"not a vba file").unwrap();
    let r = run_json(&["vba", "import", &path, &junk]);
    assert!(!r["success"].as_bool().unwrap_or(true));
}
