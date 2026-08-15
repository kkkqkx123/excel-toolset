use super::*;

#[test]
fn test_rollback_restores_file() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["cell", "write", &path, "Sheet1", "A1", "original"]);
    let bk = run_json(&["file", "backup", &path]);
    let bk_path = bk["backup_path"].as_str().unwrap().to_string();
    run_json(&["cell", "write", &path, "Sheet1", "A1", "modified"]);
    assert_eq!(
        run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
            .as_str()
            .unwrap(),
        "modified"
    );
    run_json(&["rollback", &path, &bk_path]);
    assert_eq!(
        run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
            .as_str()
            .unwrap(),
        "original"
    );
}
