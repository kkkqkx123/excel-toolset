use super::*;

#[test]
fn test_dedup_by_specific_column() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["data", "append-row", &path, "Sheet1", "Alice", "Eng"]);
    run_json(&["data", "append-row", &path, "Sheet1", "Bob", "Eng"]);
    run_json(&["data", "append-row", &path, "Sheet1", "Carol", "HR"]);
    run_json(&["data", "dedup", &path, "Sheet1", "--column", "2"]);
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
}
