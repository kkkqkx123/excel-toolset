use super::*;

#[test]
fn test_comment_add_and_get() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["comments", "add", &path, "Sheet1", "A1", "My comment"]);
    let d = run_json(&["comments", "get", &path, "Sheet1", "A1"]);
    assert_eq!(d["text"].as_str().unwrap(), "My comment");
}

#[test]
fn test_comment_update() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["comments", "add", &path, "Sheet1", "A1", "orig"]);
    run_json(&["comments", "update", &path, "Sheet1", "A1", "updated"]);
    let d = run_json(&["comments", "get", &path, "Sheet1", "A1"]);
    assert_eq!(d["text"].as_str().unwrap(), "updated");
}

#[test]
fn test_comment_delete() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["comments", "add", &path, "Sheet1", "A1", "delme"]);
    let r = run_json(&["comments", "delete", &path, "Sheet1", "A1"]);
    assert_ok(&r);
}
