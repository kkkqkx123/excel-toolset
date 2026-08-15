use super::*;

#[test]
fn test_search_exact_match() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:B2",
        r#"[["Alice","engineer"],["Bob","eng"]]"#,
    ]);
    let r = run_json(&[
        "search",
        "sheet",
        &path,
        "Sheet1",
        "eng",
        "--match-type",
        "exact",
    ]);
    let results = r["matches"]
        .as_array()
        .unwrap_or_else(|| r.as_array().unwrap());
    assert!(!results.is_empty());
    for item in results {
        if let Some(v) = item["value"].as_str() {
            assert_eq!(v, "eng");
        }
    }
}

#[test]
fn test_search_case_sensitive() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:A2",
        r#"[["Hello"],["hello"]]"#,
    ]);
    let r = run_json(&[
        "search",
        "sheet",
        &path,
        "Sheet1",
        "Hello",
        "--case-sensitive",
    ]);
    let results = r["matches"]
        .as_array()
        .unwrap_or_else(|| r.as_array().unwrap());
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_regex() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&[
        "range",
        "write",
        &path,
        "Sheet1",
        "A1:A3",
        r#"[["abc123"],["def456"],["xyz789"]]"#,
    ]);
    let r = run_json(&[
        "search",
        "sheet",
        &path,
        "Sheet1",
        r"\d{3}",
        "--match-type",
        "regex",
    ]);
    let results = r["matches"]
        .as_array()
        .unwrap_or_else(|| r.as_array().unwrap());
    assert_eq!(results.len(), 3);
}
