use super::*;

#[test]
fn test_conditional_format_cell_value() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&[
        "conditional-format",
        "add",
        &path,
        "Sheet1",
        "A1",
        "cell_value",
        ">10",
    ]);
    assert_ok(&r);
}

#[test]
fn test_conditional_format_with_style() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let style = r##"{"bold":true,"font_color":"#FF0000"}"##;
    let r = run_json(&[
        "conditional-format",
        "add",
        &path,
        "Sheet1",
        "A1:A10",
        "cell_value",
        ">100",
        "--style",
        style,
    ]);
    assert_ok(&r);
}

#[test]
fn test_conditional_format_remove() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "conditional-format",
        "add",
        &path,
        "Sheet1",
        "A1",
        "duplicate",
        "",
    ]);
    let r = run_json(&["conditional-format", "remove", &path, "Sheet1", "A1"]);
    assert_ok(&r);
}

#[test]
fn test_conditional_format_invalid_rule_type() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&[
        "conditional-format",
        "add",
        &path,
        "Sheet1",
        "A1",
        "unknown_type",
        "",
    ]);
    assert!(!r["success"].as_bool().unwrap_or(true));
}
