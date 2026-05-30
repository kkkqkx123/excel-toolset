use std::fs;
use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::file_util::{append_timestamp, copy_file, ensure_parent_dir};
use crate::types::BackupInfo;

pub fn compute_file_hash(path: &str) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

pub fn create_backup(path: &str, operation: &str) -> io::Result<BackupInfo> {
    let backup_path = append_timestamp(path);
    ensure_parent_dir(&backup_path)?;
    copy_file(path, &backup_path)?;
    let hash = compute_file_hash(&backup_path)?;
    Ok(BackupInfo {
        backup_path,
        timestamp: chrono::Utc::now(),
        operation: operation.to_string(),
        file_hash: hash,
    })
}

pub fn rollback(backup_info: &BackupInfo, original_path: &str) -> io::Result<()> {
    if !Path::new(&backup_info.backup_path).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Backup file not found: {}", backup_info.backup_path),
        ));
    }
    copy_file(&backup_info.backup_path, original_path)?;
    Ok(())
}

pub fn with_security<F, T>(
    params: crate::types::SecurityParams,
    f: F,
) -> Result<(T, Option<BackupInfo>), Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let file_path = params.file_path.clone();
    let pre_hash = compute_file_hash(&file_path)?;

    let backup_info = if params.create_backup {
        Some(create_backup(&file_path, &pre_hash)?)
    } else {
        None
    };

    if params.dry_run {
        return Err("Dry run: operation skipped".into());
    }

    let result = match f() {
        Ok(val) => val,
        Err(e) => {
            if let Some(ref backup) = backup_info {
                let _ = rollback(backup, &file_path);
            }
            return Err(e);
        }
    };

    if let Some(ref _backup) = backup_info {
        let _post_hash = compute_file_hash(&file_path)?;
    }

    Ok((result, backup_info))
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

        let _ = fs::remove_file(test_file);
        let _ = fs::remove_file(&backup.backup_path);
    }
}
