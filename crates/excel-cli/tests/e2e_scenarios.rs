use super::*;

#[test]
fn test_create_edit_backup_rollback_flow() {
    let id = test_id();
    let path = tf(id, "flow.xlsx");
    let r = run_json(&["file", "create", &path, "--sheet", "Employees"]);
    assert_ok(&r);
    run_json(&[
        "data",
        "append-row",
        &path,
        "Employees",
        "Name",
        "Age",
        "Dept",
    ]);
    run_json(&[
        "data",
        "append-row",
        &path,
        "Employees",
        "Alice",
        "30",
        "Eng",
    ]);
    run_json(&["data", "append-row", &path, "Employees", "Bob", "25", "HR"]);
    let bk = run_json(&["file", "backup", &path]);
    let bk_path = bk["backup_path"].as_str().unwrap().to_string();
    run_json(&["cell", "write", &path, "Employees", "B2", "31"]);
    assert_eq!(
        run_json(&["cell", "read", &path, "Employees", "B2"])["value"]
            .as_str()
            .unwrap(),
        "31"
    );
    let diff = run_json(&["diff", "file", &bk_path, &path]);
    assert!(diff["summary"]["total_changes"].as_u64().unwrap() > 0);
    run_json(&["rollback", &path, &bk_path]);
    assert_eq!(
        run_json(&["cell", "read", &path, "Employees", "B2"])["value"]
            .as_str()
            .unwrap(),
        "30"
    );
}

#[test]
fn test_batch_workflow_complete_sheet() {
    let id = test_id();
    let path = tf(id, "batch_flow.xlsx");
    run_json(&["file", "create", &path, "--sheet", "Report"]);
    let ops = r#"[
        {"op":"write_cell","sheet":"Report","row":0,"col":0,"value":"ID"},
        {"op":"write_cell","sheet":"Report","row":0,"col":1,"value":"Name"},
        {"op":"write_cell","sheet":"Report","row":0,"col":2,"value":"Score"},
        {"op":"append_rows","sheet":"Report","data":[["1","Alice","95"],["2","Bob","87"],["3","Carol","92"]]},
        {"op":"set_formula","sheet":"Report","cell":"D1","formula":"=SUM(C2:C4)"},
        {"op":"add_sheet","name":"Summary"},
        {"op":"write_cell","sheet":"Summary","row":0,"col":0,"value":"Total Average"}
    ]"#;
    let r = run_json(&["batch", "modify", &path, "--operations", ops]);
    assert_ok(&r);
    assert_eq!(r["succeeded_count"].as_u64().unwrap(), 7);
    assert_eq!(
        run_json(&["cell", "read", &path, "Report", "A1"])["value"]
            .as_str()
            .unwrap(),
        "ID"
    );
    assert_eq!(
        run_json(&["cell", "read", &path, "Report", "C2"])["value"]
            .as_str()
            .unwrap(),
        "95"
    );
    assert_eq!(
        run_json(&["formula", "read", &path, "Report", "D1"])["formula"]
            .as_str()
            .unwrap(),
        "=SUM(C2:C4)"
    );
    assert_eq!(
        run_json(&["sheet", "list", &path])["sheets"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn test_format_and_merge_workflow() {
    let id = test_id();
    let path = tf(id, "fmt_flow.xlsx");
    run_json(&["file", "create", &path]);
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:C1",
        r#"[["Title","Value","Note"]]"#,
    ]);
    let style = r##"{"bold":true,"font_size":16,"background_color":"#4472C4"}"##;
    run_json(&["format", "set", &path, "Sheet1", "A1:C1", style]);
    run_json(&[
        "format",
        "merge",
        &path,
        "Sheet1",
        "A1:C1",
        "--value",
        "Report Header",
    ]);
    run_json(&["data", "append-row", &path, "Sheet1", "Sales", "1000", "Q1"]);
    run_json(&[
        "data",
        "append-row",
        &path,
        "Sheet1",
        "Revenue",
        "2000",
        "Q1",
    ]);
    let cf_style = r##"{"font_color":"#006100"}"##;
    run_json(&[
        "conditional-format",
        "add",
        &path,
        "Sheet1",
        "B2:B3",
        "cell_value",
        ">500",
        "--style",
        cf_style,
    ]);
    let info = run_json(&["file", "info", &path]);
    assert_ok(&info);
}
