use std::fs;
use std::path::Path;

use excel_core::excel_read::{
    list_sheets, read_cell, read_file_info, read_formula, read_range, read_sheet_all,
};
use excel_core::excel_write::{
    add_sheet, append_rows, clear_range, create_file, delete_rows, delete_sheet, insert_rows,
    merge_cells, rename_sheet, set_formula, write_cell, write_range,
};
use excel_core::features::comments::{add_comment, delete_comment, get_comment, update_comment};
use excel_core::features::search::{
    MatchType, SearchQuery, SearchType, search_sheet, search_workbook,
};
use excel_core::operations::{dedup_sheet, filter_rows, sort_sheet};
use excel_core::security::{compute_file_hash, create_backup, rollback};
use excel_core::types::{
    BackupInfo, CellValue, FilterCondition, FilterOp, SecurityParams, SortColumn,
};

fn test_root() -> &'static tempfile::TempDir {
    use std::sync::OnceLock;
    static ROOT: OnceLock<tempfile::TempDir> = OnceLock::new();
    ROOT.get_or_init(|| tempfile::tempdir().expect("Failed to create temp dir for tests"))
}

fn setup_test_file(name: &str) -> String {
    let file_path = test_root().path().join(name);
    file_path.to_string_lossy().to_string()
}

fn cleanup_test_file(_path: &str) {
    // TempDir handles cleanup on process exit via the static reference.
}

fn create_simple_test_file(path: &str, sheet_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parent = Path::new(path).parent().ok_or("Invalid path")?;
    fs::create_dir_all(parent)?;

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(sheet_name)?;
    ws.write_string(0, 0, "Name")?;
    ws.write_string(0, 1, "Age")?;
    ws.write_string(0, 2, "City")?;
    ws.write_string(1, 0, "Alice")?;
    ws.write_number(1, 1, 25)?;
    ws.write_string(1, 2, "New York")?;
    ws.write_string(2, 0, "Bob")?;
    ws.write_number(2, 1, 30)?;
    ws.write_string(2, 2, "London")?;
    ws.write_string(3, 0, "Charlie")?;
    ws.write_number(3, 1, 35)?;
    ws.write_string(3, 2, "Paris")?;
    wb.save(path)?;
    Ok(())
}

mod file_read_tests {
    use super::*;

    #[test]
    fn test_read_file_info() {
        let path = setup_test_file("test_read_info.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let info = read_file_info(&path).unwrap();
        assert_eq!(info.path, path);
        assert!(!info.hash.is_empty());
        assert!(info.size > 0);
        assert_eq!(info.sheets, vec!["Sheet1"]);

        cleanup_test_file(&path);
    }

    #[test]
    fn test_list_sheets() {
        let path = setup_test_file("test_list_sheets.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Data").unwrap();

        let sheets = list_sheets(&path).unwrap();
        assert_eq!(sheets, vec!["Data"]);

        cleanup_test_file(&path);
    }

    #[test]
    fn test_read_cell() {
        let path = setup_test_file("test_read_cell.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let cell = read_cell(&path, "Sheet1", 1, 0).unwrap();
        assert_eq!(cell.value, Some("Alice".to_string()));
        assert_eq!(cell.data_type, excel_core::types::CellDataType::String);

        let cell = read_cell(&path, "Sheet1", 1, 1).unwrap();
        assert_eq!(cell.value, Some("25".to_string()));

        cleanup_test_file(&path);
    }

    #[test]
    fn test_read_range() {
        let path = setup_test_file("test_read_range.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let range = read_range(&path, "Sheet1", "A1:C2").unwrap();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0][0].value, Some("Name".to_string()));
        assert_eq!(range[1][0].value, Some("Alice".to_string()));

        cleanup_test_file(&path);
    }

    #[test]
    fn test_read_sheet_all() {
        let path = setup_test_file("test_read_sheet.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Data").unwrap();

        let sheet = read_sheet_all(&path, "Data").unwrap();
        assert_eq!(sheet.name, "Data");
        assert_eq!(sheet.rows.len(), 4);
        assert_eq!(sheet.rows[0][0].value, Some("Name".to_string()));

        cleanup_test_file(&path);
    }

    #[test]
    fn test_read_sheet_not_found() {
        let path = setup_test_file("test_not_found.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let result = read_sheet_all(&path, "NonExistent");
        assert!(result.is_err());

        cleanup_test_file(&path);
    }
}

mod file_write_tests {
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
}

mod data_operations_tests {
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
        assert!(result.len() >= 1);

        cleanup_test_file(&path);
    }
}

mod security_tests {
    use super::*;

    #[test]
    fn test_compute_file_hash() {
        let path = setup_test_file("test_hash.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let hash1 = compute_file_hash(&path).unwrap();
        assert_eq!(hash1.len(), 64);

        let hash2 = compute_file_hash(&path).unwrap();
        assert_eq!(hash1, hash2);

        cleanup_test_file(&path);
    }

    #[test]
    fn test_create_backup() {
        let path = setup_test_file("test_backup.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let backup = create_backup(&path, "test_operation").unwrap();
        assert!(Path::new(&backup.backup_path).exists());
        assert_eq!(backup.operation, "test_operation");
        assert!(!backup.file_hash.is_empty());

        cleanup_test_file(&path);

        let backup_path = Path::new(&backup.backup_path);
        if backup_path.exists() {
            fs::remove_file(backup_path).ok();
        }
    }

    #[test]
    fn test_write_with_backup() {
        let path = setup_test_file("test_write_backup.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let params = SecurityParams {
            dry_run: false,
            create_backup: true,
            file_path: path.clone(),
        };

        let result = write_cell(
            &path,
            &params,
            "Sheet1",
            10,
            0,
            &CellValue::String("Test".to_string()),
        )
        .unwrap();
        assert!(result.success);
        assert!(result.backup_info.is_some());
        assert_ne!(result.old_hash, result.new_hash);

        let backup_info = result.backup_info.unwrap();
        assert!(Path::new(&backup_info.backup_path).exists());

        cleanup_test_file(&path);

        if let Ok(backup_path) = fs::read_dir("/tmp/excel_test_files") {
            for entry in backup_path.flatten() {
                let entry_path = entry.path();
                if entry_path != Path::new(&path) {
                    fs::remove_file(entry_path).ok();
                }
            }
        }
    }

    #[test]
    fn test_rollback() {
        let path = setup_test_file("test_rollback.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let old_hash = compute_file_hash(&path).unwrap();
        let backup = create_backup(&path, "before_modification").unwrap();

        write_cell(
            &path,
            &SecurityParams::default(),
            "Sheet1",
            0,
            0,
            &CellValue::String("Modified".to_string()),
        )
        .unwrap();

        let modified_cell = read_cell(&path, "Sheet1", 0, 0).unwrap();
        assert_eq!(modified_cell.value, Some("Modified".to_string()));

        rollback(&backup, &path).unwrap();

        let restored_cell = read_cell(&path, "Sheet1", 0, 0).unwrap();
        assert_eq!(restored_cell.value, Some("Name".to_string()));

        let restored_hash = compute_file_hash(&path).unwrap();
        assert_eq!(old_hash, restored_hash);

        cleanup_test_file(&path);
    }

    #[test]
    fn test_dry_run_mode() {
        let path = setup_test_file("test_dry_run.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let old_hash = compute_file_hash(&path).unwrap();

        let params = SecurityParams {
            dry_run: true,
            create_backup: false,
            file_path: path.clone(),
        };

        let result = write_cell(
            &path,
            &params,
            "Sheet1",
            10,
            0,
            &CellValue::String("Test".to_string()),
        )
        .unwrap();
        assert!(result.success);
        assert_eq!(result.old_hash, result.new_hash);
        assert_eq!(result.new_hash, old_hash);

        let new_hash = compute_file_hash(&path).unwrap();
        assert_eq!(old_hash, new_hash);

        let cell = read_cell(&path, "Sheet1", 10, 0);
        assert!(cell.is_err() || cell.unwrap().value.is_none());

        cleanup_test_file(&path);
    }

    #[test]
    fn test_rollback_invalid_backup() {
        let path = setup_test_file("test_invalid_rollback.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let backup = BackupInfo {
            backup_path: "/tmp/nonexistent_backup.xlsx".to_string(),
            timestamp: chrono::Utc::now(),
            operation: "test".to_string(),
            file_hash: "invalid".to_string(),
        };

        let result = rollback(&backup, &path);
        assert!(result.is_err());

        cleanup_test_file(&path);
    }
}

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_nonexistent_file() {
        let path = setup_test_file("nonexistent.xlsx");
        cleanup_test_file(&path);

        let result = read_file_info(&path);
        assert!(result.is_err());

        let result = list_sheets(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_sheet_name() {
        let path = setup_test_file("test_invalid_sheet.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let result = read_cell(&path, "NonExistent", 0, 0);
        assert!(result.is_err());

        let result = read_range(&path, "NonExistent", "A1:C1");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_cell_reference() {
        let path = setup_test_file("test_invalid_cell.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        // Reading a cell beyond the data range now returns an empty cell
        let result = read_cell(&path, "Sheet1", 100, 0);
        assert!(result.is_ok());
        let cell = result.unwrap();
        assert_eq!(cell.value, None);
        assert_eq!(cell.data_type, excel_core::types::CellDataType::Empty);

        // Reading a range with invalid reference returns empty data
        let result = read_range(&path, "Sheet1", "Z100:AA101");
        assert!(result.is_ok());
        let range = result.unwrap();
        // The range should be empty or contain only empty cells
        assert!(
            range.is_empty()
                || range
                    .iter()
                    .all(|row| row.iter().all(|cell| cell.value.is_none()))
        );

        cleanup_test_file(&path);
    }

    #[test]
    fn test_write_nonexistent_sheet() {
        let path = setup_test_file("test_write_nonexistent.xlsx");
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
            "NonExistent",
            0,
            0,
            &CellValue::String("Test".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_sheet() {
        let path = setup_test_file("test_delete_nonexistent.xlsx");
        cleanup_test_file(&path);

        create_simple_test_file(&path, "Sheet1").unwrap();

        let params = SecurityParams {
            dry_run: false,
            create_backup: false,
            file_path: path.clone(),
        };

        let result = delete_sheet(&path, &params, "NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_to_existing_sheet() {
        let path = setup_test_file("test_rename_existing.xlsx");
        cleanup_test_file(&path);

        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws1 = wb.add_worksheet();
        ws1.set_name("Sheet1").unwrap();
        let ws2 = wb.add_worksheet();
        ws2.set_name("Sheet2").unwrap();
        wb.save(&path).unwrap();

        let params = SecurityParams {
            dry_run: false,
            create_backup: false,
            file_path: path.clone(),
        };

        let result = rename_sheet(&path, &params, "Sheet1", "Sheet2");
        assert!(result.is_err());

        cleanup_test_file(&path);
    }
}

mod business_scenarios {
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
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[1][0].value.as_deref(), Some("Bob"));
        assert_eq!(filtered[2][0].value.as_deref(), Some("Charlie"));

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
}
