use super::*;

#[test]
fn test_format_set_style() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&[
        "format",
        "set",
        &path,
        "Sheet1",
        "A1",
        r#"{"bold":true,"font_size":14}"#,
    ]);
    assert_ok(&r);
}

#[test]
fn test_format_merge_cells_default_value() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["range", "write", &path, "Sheet1", "A1", r#"[["Merged"]]"#]);
    let r = run_json(&["format", "merge", &path, "Sheet1", "A1:B2"]);
    assert_ok(&r);
}

#[test]
fn test_format_merge_cells_custom_value() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&[
        "format", "merge", &path, "Sheet1", "A1:C3", "--value", "Summary",
    ]);
    assert_ok(&r);
}
