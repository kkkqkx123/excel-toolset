use super::*;

#[test]
fn test_pretty_output_is_multiline_json() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let out = run(&["--pretty", "file", "info", &path]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains('\n'));
    assert!(stdout.contains("  "));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
}
