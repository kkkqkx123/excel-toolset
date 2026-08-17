use super::*;

#[test]
fn test_formula_calc_mode() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    let r = run_json(&["formula", "calc-mode", &path, "--mode", "auto"]);
    assert_ok(&r);
}

#[test]
fn test_formula_trace_dependencies() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
    run_json(&["cell", "write", &path, "Sheet1", "B1", "20"]);
    run_json(&["formula", "set", &path, "Sheet1", "C1", "=A1+B1"]);
    let r = run_json(&["formula", "trace", &path, "Sheet1", "C1"]);
    assert_ok(&r);
    assert!(r["cell"].as_str().is_some());
    assert!(r["direct_precedents"].as_array().is_some());
}

#[test]
fn test_formula_explain() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
    run_json(&["cell", "write", &path, "Sheet1", "B1", "5"]);
    run_json(&["formula", "set", &path, "Sheet1", "C1", "=SUM(A1:B1)"]);
    let r = run_json(&["formula", "explain", &path, "Sheet1", "C1"]);
    assert_ok(&r);
    assert!(r["description"].as_str().is_some());
}

#[test]
fn test_formula_explain_logic() {
    let id = test_id();
    let path = mkfile(id, "f.xlsx");
    run_json(&["cell", "write", &path, "Sheet1", "A1", "10"]);
    run_json(&["cell", "write", &path, "Sheet1", "B1", "5"]);
    run_json(&["formula", "set", &path, "Sheet1", "C1", "=SUM(A1:B1)"]);
    let r = run_json(&["formula", "explain-logic", &path, "Sheet1", "C1"]);
    assert_ok(&r);
    assert!(r["logic_flow"].as_array().is_some());
}
