use super::*;

fn scenario_dir() -> String {
    test_root()
        .path()
        .join("scenarios")
        .to_string_lossy()
        .to_string()
}

fn file_path_in_scenario(name: &str) -> String {
    let dir = scenario_dir();
    fs::create_dir_all(&dir).ok();
    format!("{}/{}", dir, name)
}

fn cleanup(path: &str) {
    let _ = fs::remove_file(path);
    let comment_sidecar = format!("{}.comments.json", path);
    let _ = fs::remove_file(&comment_sidecar);
}

fn default_params(path: &str) -> SecurityParams {
    SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.to_string(),
    }
}

#[test]
fn scenario_create_populate_verify() {
    let path = file_path_in_scenario("create_populate.xlsx");
    cleanup(&path);

    let result = create_file(&path, "Report");
    assert!(result.is_ok(), "Failed to create file");

    let sheets = list_sheets(&path).expect("Failed to list sheets");
    assert_eq!(sheets, vec!["Report"]);

    let params = default_params(&path);
    let headers = vec![vec![
        CellValue::String("Product".into()),
        CellValue::String("Price".into()),
        CellValue::String("Qty".into()),
    ]];
    let r = write_range(&path, &params, "Report", "A1:C1", &headers);
    assert!(r.is_ok(), "Failed to write headers");

    write_cell(
        &path,
        &params,
        "Report",
        1,
        0,
        &CellValue::String("Widget".into()),
    )
    .expect("write A2");
    write_cell(&path, &params, "Report", 1, 1, &CellValue::Number(9.99)).expect("write B2");
    write_cell(&path, &params, "Report", 1, 2, &CellValue::Number(100.0)).expect("write C2");
    write_cell(
        &path,
        &params,
        "Report",
        2,
        0,
        &CellValue::String("Gadget".into()),
    )
    .expect("write A3");
    write_cell(&path, &params, "Report", 2, 1, &CellValue::Number(24.99)).expect("write B3");
    write_cell(&path, &params, "Report", 2, 2, &CellValue::Number(50.0)).expect("write C3");

    let cell = read_cell(&path, "Report", 0, 0).expect("read A1");
    assert_eq!(cell.value, Some("Product".into()));

    let cell = read_cell(&path, "Report", 2, 2).expect("read C3");
    assert_eq!(cell.value, Some("50".into()));

    cleanup(&path);
}

#[test]
fn scenario_data_row_operations() {
    let path = file_path_in_scenario("data_rows.xlsx");
    cleanup(&path);

    create_file(&path, "Data").expect("create");
    let params = default_params(&path);

    write_range(
        &path,
        &params,
        "Data",
        "A1:B1",
        &[vec![
            CellValue::String("Name".into()),
            CellValue::String("Score".into()),
        ]],
    )
    .expect("write header");

    append_rows(
        &path,
        &params,
        "Data",
        &[vec![
            CellValue::String("Alice".into()),
            CellValue::Number(85.0),
        ]],
    )
    .expect("append Alice");
    append_rows(
        &path,
        &params,
        "Data",
        &[vec![
            CellValue::String("Charlie".into()),
            CellValue::Number(92.0),
        ]],
    )
    .expect("append Charlie");

    insert_rows(
        &path,
        &params,
        "Data",
        1,
        &[vec![
            CellValue::String("Bob".into()),
            CellValue::Number(78.0),
        ]],
    )
    .expect("insert Bob");

    let sheet_data = read_sheet_all(&path, "Data").expect("read all");
    assert_eq!(sheet_data.rows.len(), 4);
    assert_eq!(sheet_data.rows[1][0].value.as_deref(), Some("Bob"));
    assert_eq!(sheet_data.rows[2][0].value.as_deref(), Some("Alice"));
    assert_eq!(sheet_data.rows[3][0].value.as_deref(), Some("Charlie"));

    delete_rows(&path, &params, "Data", 1, 1).expect("delete Bob");

    let sheet_data = read_sheet_all(&path, "Data").expect("read after delete");
    assert_eq!(sheet_data.rows.len(), 3);
    assert_eq!(sheet_data.rows[1][0].value.as_deref(), Some("Alice"));
    assert_eq!(sheet_data.rows[2][0].value.as_deref(), Some("Charlie"));

    cleanup(&path);
}

#[test]
fn scenario_formula_workflow() {
    let path = file_path_in_scenario("formula_workflow.xlsx");
    cleanup(&path);

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Calc").expect("set name");
    ws.write_string(0, 0, "Price").expect("A1");
    ws.write_string(0, 1, "Qty").expect("B1");
    ws.write_string(0, 2, "Total").expect("C1");
    ws.write_number(1, 0, 100.0).expect("A2");
    ws.write_number(1, 1, 5.0).expect("B2");
    ws.write_number(2, 0, 200.0).expect("A3");
    ws.write_number(2, 1, 3.0).expect("B3");
    wb.save(&path).expect("save");

    let params = default_params(&path);

    set_formula(&path, &params, "Calc", "C2", "=A2*B2").expect("set C2 formula");
    set_formula(&path, &params, "Calc", "C3", "=A3*B3").expect("set C3 formula");

    let f1 = read_formula(&path, "Calc", "C2").expect("read C2 formula");
    assert!(f1.is_some());
    assert!(f1.unwrap().contains("A2*B2"));

    let f2 = read_formula(&path, "Calc", "C3").expect("read C3 formula");
    assert!(f2.is_some());
    assert!(f2.unwrap().contains("A3*B3"));

    set_formula(&path, &params, "Calc", "C4", "=SUM(C2:C3)").expect("set SUM formula");
    let f3 = read_formula(&path, "Calc", "C4").expect("read C4 formula");
    assert!(f3.is_some());
    assert!(f3.unwrap().contains("SUM"));

    cleanup(&path);
}

#[test]
fn scenario_filter_sort_dedup() {
    let path = file_path_in_scenario("data_analysis.xlsx");
    cleanup(&path);

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sales").expect("set name");
    ws.write_string(0, 0, "Name").expect("A1");
    ws.write_string(0, 1, "Amount").expect("B1");
    ws.write_string(1, 0, "Alice").expect("A2");
    ws.write_number(1, 1, 100.0).expect("B2");
    ws.write_string(2, 0, "Bob").expect("A3");
    ws.write_number(2, 1, 300.0).expect("B3");
    ws.write_string(3, 0, "Charlie").expect("A4");
    ws.write_number(3, 1, 200.0).expect("B4");
    ws.write_string(4, 0, "Alice").expect("A5");
    ws.write_number(4, 1, 100.0).expect("B5");
    wb.save(&path).expect("save");

    let conditions = vec![FilterCondition {
        column: 1,
        operator: FilterOp::Gt,
        value: "100".into(),
    }];
    let filtered = filter_rows(&path, "Sales", &conditions).expect("filter");
    // The header is not counted in the result: rows matching Amount>100 are Bob(300) and Charlie(200).
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0][0].value.as_deref(), Some("Bob"));
    assert_eq!(filtered[1][0].value.as_deref(), Some("Charlie"));

    let params = default_params(&path);
    let sort_cols = vec![SortColumn {
        column: 1,
        descending: false,
    }];
    let r = sort_sheet(&path, &params, "Sales", &sort_cols);
    assert!(r.is_ok(), "sort failed");

    let data = read_sheet_all(&path, "Sales").expect("read after sort");
    assert_eq!(data.rows.len(), 5);
    assert_eq!(data.rows[0][0].value.as_deref(), Some("Name"));
    assert_eq!(data.rows[1][0].value.as_deref(), Some("Alice"));
    assert_eq!(data.rows[2][0].value.as_deref(), Some("Alice"));
    assert_eq!(data.rows[3][0].value.as_deref(), Some("Charlie"));
    assert_eq!(data.rows[4][0].value.as_deref(), Some("Bob"));

    let r = dedup_sheet(&path, &params, "Sales", &[0]);
    assert!(r.is_ok(), "dedup failed");

    let data = read_sheet_all(&path, "Sales").expect("read after dedup");
    assert_eq!(data.rows.len(), 4);

    cleanup(&path);
}

#[test]
fn scenario_backup_and_rollback() {
    let path = file_path_in_scenario("security_test.xlsx");
    cleanup(&path);

    create_file(&path, "Data").expect("create");
    let params = default_params(&path);

    write_cell(
        &path,
        &params,
        "Data",
        0,
        0,
        &CellValue::String("Original".into()),
    )
    .expect("write original");

    let hash = compute_file_hash(&path).expect("compute hash");
    let backup = create_backup(&path, &hash).expect("create backup");

    let params2 = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };
    write_cell(
        &path,
        &params2,
        "Data",
        0,
        0,
        &CellValue::String("Modified".into()),
    )
    .expect("write modified");

    let cell = read_cell(&path, "Data", 0, 0).expect("read after modify");
    assert_eq!(cell.value, Some("Modified".into()));

    rollback(&backup, &path).expect("rollback");

    let cell = read_cell(&path, "Data", 0, 0).expect("read after rollback");
    assert_eq!(cell.value, Some("Original".into()));

    cleanup(&path);
}

#[test]
fn scenario_sheet_management() {
    let path = file_path_in_scenario("sheet_mgmt.xlsx");
    cleanup(&path);

    create_file(&path, "Sheet1").expect("create with Sheet1");
    let params = default_params(&path);

    add_sheet(&path, &params, "Data").expect("add Data");
    add_sheet(&path, &params, "Summary").expect("add Summary");

    let sheets = list_sheets(&path).expect("list");
    assert_eq!(sheets.len(), 3);
    assert!(sheets.contains(&"Sheet1".to_string()));
    assert!(sheets.contains(&"Data".to_string()));
    assert!(sheets.contains(&"Summary".to_string()));

    rename_sheet(&path, &params, "Sheet1", "Config").expect("rename Sheet1");
    let sheets = list_sheets(&path).expect("list after rename");
    assert!(!sheets.contains(&"Sheet1".to_string()));
    assert!(sheets.contains(&"Config".to_string()));

    delete_sheet(&path, &params, "Data").expect("delete Data");
    let sheets = list_sheets(&path).expect("list after delete");
    assert_eq!(sheets.len(), 2);
    assert!(sheets.contains(&"Config".to_string()));
    assert!(sheets.contains(&"Summary".to_string()));

    cleanup(&path);
}

#[test]
fn scenario_range_read_write_clear() {
    let path = file_path_in_scenario("range_ops.xlsx");
    cleanup(&path);

    create_file(&path, "Sheet1").expect("create");
    let params = default_params(&path);

    let data: Vec<Vec<CellValue>> = vec![
        vec![
            CellValue::String("A".into()),
            CellValue::String("B".into()),
            CellValue::String("C".into()),
        ],
        vec![
            CellValue::String("D".into()),
            CellValue::String("E".into()),
            CellValue::String("F".into()),
        ],
        vec![
            CellValue::String("G".into()),
            CellValue::String("H".into()),
            CellValue::String("I".into()),
        ],
    ];
    write_range(&path, &params, "Sheet1", "A1:C3", &data).expect("write range");

    let result = read_range(&path, "Sheet1", "A1:C3").expect("read range");
    assert_eq!(result.len(), 3);
    assert_eq!(result[2][2].value.as_deref(), Some("I"));

    clear_range(&path, &params, "Sheet1", "B2:C2").expect("clear B2:C2");

    let result = read_range(&path, "Sheet1", "A1:C3").expect("read after clear");
    assert_eq!(result[1][1].value, None);
    assert_eq!(result[1][2].value, None);
    assert_eq!(result[1][0].value.as_deref(), Some("D"));

    cleanup(&path);
}

#[test]
fn scenario_comments_crud() {
    let path = file_path_in_scenario("comments_test.xlsx");
    cleanup(&path);

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").expect("set name");
    ws.write_string(0, 0, "Data").expect("A1");
    ws.write_number(1, 0, 100.0).expect("A2");
    wb.save(&path).expect("save");

    let params = default_params(&path);

    add_comment(&path, "Sheet1", "A1", "This is a header", &params).expect("add comment");

    let comment = get_comment(&path, "Sheet1", "A1").expect("get comment");
    assert!(comment.is_some());
    assert_eq!(comment.as_ref().unwrap().text, "This is a header");

    let comment = get_comment(&path, "Sheet1", "A2").expect("get A2 comment");
    assert!(comment.is_none());

    update_comment(&path, "Sheet1", "A1", "Updated header", &params).expect("update comment");
    let comment = get_comment(&path, "Sheet1", "A1").expect("get updated");
    assert_eq!(comment.as_ref().unwrap().text, "Updated header");

    delete_comment(&path, "Sheet1", "A1", &params).expect("delete comment");
    let comment = get_comment(&path, "Sheet1", "A1").expect("get after delete");
    assert!(comment.is_none());

    cleanup(&path);
}

#[test]
fn scenario_search() {
    let path = file_path_in_scenario("search_test.xlsx");
    cleanup(&path);

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Data").expect("set name");
    ws.write_string(0, 0, "ID").expect("A1");
    ws.write_string(0, 1, "Name").expect("B1");
    ws.write_string(1, 0, "001").expect("A2");
    ws.write_string(1, 1, "Alice").expect("B2");
    ws.write_string(2, 0, "002").expect("A3");
    ws.write_string(2, 1, "Bob").expect("B3");
    wb.save(&path).expect("save");

    let query = SearchQuery {
        pattern: "Alice".into(),
        search_type: SearchType::Value,
        match_type: MatchType::Exact,
        case_sensitive: false,
        sheets: None,
    };
    let results = search_workbook(&path, &query).expect("search workbook");
    assert!(results.total_matches >= 1);

    let query = SearchQuery {
        pattern: "0".into(),
        search_type: SearchType::Value,
        match_type: MatchType::Contains,
        case_sensitive: false,
        sheets: None,
    };
    let results = search_workbook(&path, &query).expect("search contains");
    assert!(results.total_matches >= 2);

    let query = SearchQuery {
        pattern: "Bob".into(),
        search_type: SearchType::Value,
        match_type: MatchType::Exact,
        case_sensitive: false,
        sheets: None,
    };
    let results = search_sheet(&path, "Data", &query).expect("search sheet");
    assert!(results.total_matches >= 1);

    cleanup(&path);
}

#[test]
fn scenario_merge_cells() {
    let path = file_path_in_scenario("merge_test.xlsx");
    cleanup(&path);

    create_file(&path, "Report").expect("create");
    let params = default_params(&path);

    write_cell(
        &path,
        &params,
        "Report",
        0,
        0,
        &CellValue::String("Sales Report".into()),
    )
    .expect("write A1");

    let r = merge_cells(&path, &params, "Report", "A1:C1", "Sales Report");
    assert!(r.is_ok(), "merge failed");

    let sheets = list_sheets(&path).expect("list sheets");
    assert_eq!(sheets.len(), 1);

    cleanup(&path);
}
