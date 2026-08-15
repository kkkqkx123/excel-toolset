use super::*;

#[test]
fn test_filter_rows() {
    let path = setup_test_file("test_filter.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let conditions = vec![FilterCondition {
        column: 1,
        operator: FilterOp::Gt,
        value: "28".to_string(),
    }];

    let result = filter_rows(&path, "Sheet1", &conditions).unwrap();
    assert!(result.len() >= 2);

    cleanup_test_file(&path);
}

#[test]
fn test_sort_sheet() {
    let path = setup_test_file("test_sort.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let sort_columns = vec![SortColumn {
        column: 1,
        descending: true,
    }];

    let result = sort_sheet(&path, &params, "Sheet1", &sort_columns).unwrap();
    assert!(result.success);

    let sheet = read_sheet_all(&path, "Sheet1").unwrap();
    let first_age = sheet.rows[1][1].value.as_ref().unwrap();
    assert_eq!(first_age, "35");

    cleanup_test_file(&path);
}

#[test]
fn test_dedup_sheet() {
    let path = setup_test_file("test_dedup.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = dedup_sheet(&path, &params, "Sheet1", &[]).unwrap();
    assert!(result.success);

    cleanup_test_file(&path);
}

#[test]
fn test_filter_contains() {
    let path = setup_test_file("test_filter_contains.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let conditions = vec![FilterCondition {
        column: 0,
        operator: FilterOp::Contains,
        value: "a".to_string(),
    }];

    let result = filter_rows(&path, "Sheet1", &conditions).unwrap();
    assert!(result.len() >= 2);

    cleanup_test_file(&path);
}

#[test]
fn test_filter_multiple_conditions() {
    let path = setup_test_file("test_filter_multi.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let conditions = vec![
        FilterCondition {
            column: 1,
            operator: FilterOp::Ge,
            value: "25".to_string(),
        },
        FilterCondition {
            column: 2,
            operator: FilterOp::Contains,
            value: "o".to_string(),
        },
    ];

    let result = filter_rows(&path, "Sheet1", &conditions).unwrap();
    assert!(!result.is_empty());

    cleanup_test_file(&path);
}
