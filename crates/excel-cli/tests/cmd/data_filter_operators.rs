use super::*;

fn setup_score_data(_id: u64, path: &str) {
    run_json(&["data", "append-row", path, "Sheet1", "Alice", "95"]);
    run_json(&["data", "append-row", path, "Sheet1", "Bob", "87"]);
    run_json(&["data", "append-row", path, "Sheet1", "Carol", "92"]);
    run_json(&["data", "append-row", path, "Sheet1", "Dave", "73"]);
}

#[test]
fn test_filter_ne() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "2", "ne", "87"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let names: Vec<_> = rows
        .iter()
        .filter_map(|r| {
            r.as_array()
                .and_then(|a| a.first())
                .and_then(|c| c["value"].as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(!names.contains(&"Bob".to_string()));
}

#[test]
fn test_filter_gt() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "2", "gt", "90"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_filter_lt() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "2", "lt", "80"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_filter_ge() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "2", "ge", "92"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_filter_le() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "2", "le", "73"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_filter_contains() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "1", "contains", "li"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn test_filter_no_match() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    setup_score_data(id, &path);
    let r = run_json(&["data", "filter", &path, "Sheet1", "2", "eq", "999"]);
    assert_ok(&r);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
}
