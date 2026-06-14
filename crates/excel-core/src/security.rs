use std::fs;
use std::io::{self, BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::types::{BackupInfo, SecurityParams};
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
        let test_file = "_test_security_file.txt";
        fs::write(test_file, b"hello security").unwrap();

        let backup = create_backup(test_file, "test_op").unwrap();
        assert!(Path::new(&backup.backup_path).exists());
        assert_eq!(backup.operation, "test_op");

        fs::write(test_file, b"modified content").unwrap();
        let content_after = fs::read_to_string(test_file).unwrap();
        assert_eq!(content_after, "modified content");

        rollback(&backup, test_file).unwrap();
        let content_restored = fs::read_to_string(test_file).unwrap();
        assert_eq!(content_restored, "hello security");

        // Clean up all test files
        for entry in fs::read_dir(".").unwrap() {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("_test_security_file") {
                    let _ = fs::remove_file(&name);
                }
            }
        }
    }
}
