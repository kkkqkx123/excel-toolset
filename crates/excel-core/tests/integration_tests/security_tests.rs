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
