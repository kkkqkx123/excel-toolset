use super::*;

#[test]
fn test_batch_write_multiple_cells() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let ops = r#"[
        {"op":"write_cell","sheet":"Sheet1","row":0,"col":0,"value":"Header1"},
        {"op":"write_cell","sheet":"Sheet1","row":0,"col":1,"value":"Header2"},
        {"op":"write_cell","sheet":"Sheet1","row":1,"col":0,"value":"Data1"},
        {"op":"write_cell","sheet":"Sheet1","row":1,"col":1,"value":"Data2"}
    ]"#;
    let r = run_json(&["batch", "modify", &path, "--operations", ops]);
    assert_ok(&r);
    assert_eq!(r["succeeded_count"].as_u64().unwrap(), 4);
    assert_eq!(
        run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
            .as_str()
            .unwrap(),
        "Header1"
    );
    assert_eq!(
        run_json(&["cell", "read", &path, "Sheet1", "B2"])["value"]
            .as_str()
            .unwrap(),
        "Data2"
    );
}

#[test]
fn test_batch_add_sheet_and_write() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let ops = r#"[
        {"op":"add_sheet","name":"Data"},
        {"op":"write_cell","sheet":"Data","row":0,"col":0,"value":"test"}
    ]"#;
    let r = run_json(&["batch", "modify", &path, "--operations", ops]);
    assert_ok(&r);
    assert_eq!(r["succeeded_count"].as_u64().unwrap(), 2);
    assert_eq!(
        run_json(&["cell", "read", &path, "Data", "A1"])["value"]
            .as_str()
            .unwrap(),
        "test"
    );
}

#[test]
fn test_batch_dry_run() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let ops = r#"[{"op":"write_cell","sheet":"Sheet1","row":0,"col":0,"value":"nope"}]"#;
    let _ = run_json(&["batch", "modify", &path, "--operations", ops, "--dry-run"]);
    let d = run_json(&["cell", "read", &path, "Sheet1", "A1"]);
    assert_eq!(d["data_type"].as_str().unwrap(), "Empty");
}

#[test]
fn test_batch_text_format() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let ops = r#"[{"op":"write_cell","sheet":"Sheet1","row":0,"col":0,"value":"t"}]"#;
    let out = run(&[
        "batch",
        "modify",
        &path,
        "--operations",
        ops,
        "--format",
        "text",
    ]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.trim().starts_with('{'));
}

#[test]
fn test_batch_append_and_range_write() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let ops = r#"[
        {"op":"append_rows","sheet":"Sheet1","data":[["Alice","30"],["Bob","25"]]},
        {"op":"write_range","sheet":"Sheet1","range":"C1:C2","data":[["Eng"],["Mgr"]]}
    ]"#;
    let r = run_json(&["batch", "modify", &path, "--operations", ops]);
    assert_ok(&r);
    assert_eq!(
        run_json(&["cell", "read", &path, "Sheet1", "A1"])["value"]
            .as_str()
            .unwrap(),
        "Alice"
    );
    assert_eq!(
        run_json(&["cell", "read", &path, "Sheet1", "C2"])["value"]
            .as_str()
            .unwrap(),
        "Mgr"
    );
}
