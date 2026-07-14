use std::fs;
use std::io::{self, BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::types::{BackupInfo, SecurityParams, WorkbookHistoryEntry};
use crate::utils::file_util::{append_timestamp, copy_file, ensure_parent_dir};

pub fn compute_file_hash(path: impl AsRef<Path>) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    Ok(hash.iter().map(|b| format!("{:02x}", b)).collect())
}

pub fn create_backup(path: impl AsRef<Path>, operation: &str) -> io::Result<BackupInfo> {
    let backup_path = append_timestamp(&path);
    ensure_parent_dir(&backup_path)?;
    copy_file(&path, &backup_path)?;
    let hash = compute_file_hash(&backup_path)?;
    Ok(BackupInfo {
        backup_path: backup_path.to_string_lossy().to_string(),
        timestamp: chrono::Utc::now(),
        operation: operation.to_string(),
        file_hash: hash,
    })
}

pub fn rollback(backup_info: &BackupInfo, original_path: impl AsRef<Path>) -> io::Result<()> {
    let backup_path = Path::new(&backup_info.backup_path);
    if !backup_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Backup file not found: {}", backup_info.backup_path),
        ));
    }
    if !backup_info.file_hash.is_empty() {
        let actual_hash = compute_file_hash(backup_path)?;
        if actual_hash != backup_info.file_hash {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Backup file hash mismatch; rollback aborted",
            ));
        }
    }
    if original_path.as_ref().exists() {
        copy_file(&original_path, append_timestamp(&original_path))?;
    }
    copy_file(backup_path, original_path)?;
    Ok(())
}

pub fn create_backup_if_needed(params: &SecurityParams) -> io::Result<Option<BackupInfo>> {
    if !params.create_backup {
        return Ok(None);
    }

    if params.file_path.is_empty() {
        return Ok(None);
    }

    let backup = create_backup(&params.file_path, "write")?;
    Ok(Some(backup))
}

pub fn append_history_entry(
    path: &str,
    entry: &WorkbookHistoryEntry,
) -> io::Result<()> {
    let history_path = history_file_path(path);
    let mut entries: Vec<WorkbookHistoryEntry> = Vec::new();

    if Path::new(&history_path).exists() {
        let content = fs::read_to_string(&history_path)?;
        if let Ok(parsed) = serde_json::from_str::<Vec<WorkbookHistoryEntry>>(&content) {
            entries = parsed;
        }
    }

    entries.push(entry.clone());

    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&history_path, json)?;
    Ok(())
}

pub fn list_history_entries(path: &str) -> io::Result<Vec<WorkbookHistoryEntry>> {
    let history_path = history_file_path(path);
    if !Path::new(&history_path).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&history_path)?;
    let entries = serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(entries)
}

fn history_file_path(original_path: &str) -> String {
    format!("{}.history.json", original_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_compute_file_hash() {
        let hash = compute_file_hash("Cargo.toml").unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_create_backup_and_rollback() {
        let test_dir = std::env::temp_dir().join(format!(
            "excel_sec_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let test_file = test_dir.join("test_file.txt");
        fs::write(&test_file, b"hello security").unwrap();

        let backup = create_backup(&test_file, "test_op").unwrap();
        assert!(Path::new(&backup.backup_path).exists());
        assert_eq!(backup.operation, "test_op");

        fs::write(&test_file, b"modified content").unwrap();

        // Small delay to avoid timestamp collision
        std::thread::sleep(std::time::Duration::from_millis(10));

        rollback(&backup, &test_file).unwrap();
        let content_restored = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content_restored, "hello security");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_history_append_and_list() {
        let test_dir = std::env::temp_dir().join(format!(
            "excel_hist_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let test_file = test_dir.join("test_hist.xlsx");
        fs::write(&test_file, b"dummy content").unwrap();
        let test_path = test_file.to_string_lossy().to_string();
        let history_path = format!("{}.history.json", test_path);

        let _ = fs::remove_file(&history_path);

        let entry1 = WorkbookHistoryEntry {
            timestamp: chrono::Utc::now(),
            operation_type: "write_cell".to_string(),
            target_path: test_path.clone(),
            old_hash: "abc123".to_string(),
            new_hash: "def456".to_string(),
            result: "success".to_string(),
        };

        append_history_entry(&test_path, &entry1).unwrap();
        assert!(Path::new(&history_path).exists());

        let entry2 = WorkbookHistoryEntry {
            timestamp: chrono::Utc::now(),
            operation_type: "set_formula".to_string(),
            target_path: test_path.clone(),
            old_hash: "def456".to_string(),
            new_hash: "ghi789".to_string(),
            result: "success".to_string(),
        };

        append_history_entry(&test_path, &entry2).unwrap();

        let entries = list_history_entries(&test_path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].operation_type, "write_cell");
        assert_eq!(entries[1].operation_type, "set_formula");

        let _ = fs::remove_file(&history_path);
        let _ = fs::remove_dir_all(&test_dir);
    }
}
