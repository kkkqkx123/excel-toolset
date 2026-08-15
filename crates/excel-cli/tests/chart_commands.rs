use super::*;

#[test]
fn test_chart_create_default_position() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:B4",
        r#"[["Month","Sales"],["Jan","100"],["Feb","200"],["Mar","150"]]"#,
    ]);
    let r = run_json(&[
        "chart", "create", &path, "Sheet1", "A1:B4", "column", "--title", "Sales",
    ]);
    assert_ok(&r);
}

#[test]
fn test_chart_create_custom_position() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:B3",
        r#"[["X","Y"],["1","10"],["2","20"]]"#,
    ]);
    let r = run_json(&[
        "chart",
        "create",
        &path,
        "Sheet1",
        "A1:B3",
        "line",
        "--position",
        "D1",
    ]);
    assert_ok(&r);
}

#[test]
fn test_chart_invalid_type() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&["chart", "create", &path, "Sheet1", "A1", "bogus"]);
    assert!(!r["success"].as_bool().unwrap_or(true));
}
