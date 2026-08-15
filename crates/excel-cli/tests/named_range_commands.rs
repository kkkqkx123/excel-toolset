use super::*;

#[test]
fn test_named_range_create_and_list() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:B2",
        r#"[["a","b"],["c","d"]]"#,
    ]);
    run_json(&["named-range", "create", &path, "MyRange", "A1:B2"]);
    let r = run_json(&["named-range", "list", &path]);
    assert_ok(&r);
}

#[test]
fn test_named_range_delete() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["named-range", "create", &path, "ToDel", "A1"]);
    let r = run_json(&["named-range", "delete", &path, "ToDel"]);
    assert_ok(&r);
}
