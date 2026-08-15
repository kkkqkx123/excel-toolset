use super::*;

#[test]
fn test_create_file() {
    let path = setup_test_file("test_create.xlsx");
    cleanup_test_file(&path);

    let test_dir = "/tmp/excel_test_files";
    fs::create_dir_all(test_dir).ok();

    let result = create_file(&path, "NewSheet").unwrap();
    assert!(result.success);
    assert!(!result.new_hash.is_empty());
    assert!(Path::new(&path).exists());

    let sheets = list_sheets(&path).unwrap();
    assert_eq!(sheets, vec!["NewSheet"]);

    cleanup_test_file(&path);
}

#[test]
fn test_write_cell() {
    let path = setup_test_file("test_write_cell.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = write_cell(
        &path,
        &params,
        "Sheet1",
        5,
        0,
        &CellValue::String("Test".to_string()),
    )
    .unwrap();
    assert!(result.success);
    assert_ne!(result.old_hash, result.new_hash);

    let cell = read_cell(&path, "Sheet1", 5, 0).unwrap();
    assert_eq!(cell.value, Some("Test".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_write_range() {
    let path = setup_test_file("test_write_range.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let data = vec![
        vec![
            CellValue::String("X".to_string()),
            CellValue::String("Y".to_string()),
        ],
        vec![
            CellValue::String("Z".to_string()),
            CellValue::String("W".to_string()),
        ],
    ];

    let result = write_range(&path, &params, "Sheet1", "A5:B6", &data).unwrap();
    assert!(result.success);

    let cell = read_cell(&path, "Sheet1", 4, 0).unwrap();
    assert_eq!(cell.value, Some("X".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_add_sheet() {
    let path = setup_test_file("test_add_sheet.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = add_sheet(&path, &params, "NewSheet").unwrap();
    assert!(result.success);

    let sheets = list_sheets(&path).unwrap();
    assert!(sheets.contains(&"NewSheet".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_add_sheet_duplicate() {
    let path = setup_test_file("test_add_duplicate.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = add_sheet(&path, &params, "Sheet1");
    assert!(result.is_err());

    cleanup_test_file(&path);
}

#[test]
fn test_delete_sheet() {
    let path = setup_test_file("test_delete_sheet.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    // First add another sheet
    add_sheet(&path, &params, "Sheet2").unwrap();

    // Now delete Sheet1
    let result = delete_sheet(&path, &params, "Sheet1").unwrap();
    assert!(result.success);

    let sheets = list_sheets(&path).unwrap();
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0], "Sheet2");

    // Try to delete the last sheet - should fail
    let result = delete_sheet(&path, &params, "Sheet2");
    assert!(result.is_err());

    cleanup_test_file(&path);
}

#[test]
fn test_rename_sheet() {
    let path = setup_test_file("test_rename_sheet.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "OldName").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = rename_sheet(&path, &params, "OldName", "NewName").unwrap();
    assert!(result.success);

    let sheets = list_sheets(&path).unwrap();
    assert_eq!(sheets, vec!["NewName"]);

    cleanup_test_file(&path);
}

#[test]
fn test_clear_range() {
    let path = setup_test_file("test_clear_range.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = clear_range(&path, &params, "Sheet1", "A1:C1").unwrap();
    assert!(result.success);

    let cell = read_cell(&path, "Sheet1", 0, 0).unwrap();
    assert_eq!(cell.value, None);
    assert_eq!(cell.data_type, excel_core::types::CellDataType::Empty);

    cleanup_test_file(&path);
}

#[test]
fn test_set_formula() {
    let path = setup_test_file("test_formula.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = set_formula(&path, &params, "Sheet1", "A5", "=SUM(A2:A4)").unwrap();
    assert!(result.success);

    let formula = read_formula(&path, "Sheet1", "A5").unwrap();
    assert_eq!(formula, Some("=SUM(A2:A4)".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_merge_cells() {
    let path = setup_test_file("test_merge_cells.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = merge_cells(&path, &params, "Sheet1", "A5:B5", "Merged Cell").unwrap();
    assert!(result.success);

    cleanup_test_file(&path);
}

#[test]
fn test_append_data_via_write_range() {
    let path = setup_test_file("test_append_data.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let sheet = read_sheet_all(&path, "Sheet1").unwrap();
    let new_row = sheet.rows.len() as u32;

    let data = vec![vec![
        CellValue::String("David".to_string()),
        CellValue::Number(45.0),
        CellValue::String("Berlin".to_string()),
    ]];

    let result = write_range(
        &path,
        &params,
        "Sheet1",
        &format!("A{}:C{}", new_row + 1, new_row + 1),
        &data,
    )
    .unwrap();
    assert!(result.success);

    let updated_sheet = read_sheet_all(&path, "Sheet1").unwrap();
    assert!(updated_sheet.rows.len() > sheet.rows.len());

    let last_row = &updated_sheet.rows.last().unwrap();
    assert_eq!(last_row[0].value, Some("David".to_string()));

    cleanup_test_file(&path);
}
