use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;

pub fn path_exists(p: &str) -> bool {
    Path::new(p).exists()
}

pub fn copy_file(src: &str, dst: &str) -> io::Result<()> {
    fs::copy(src, dst)?;
    Ok(())
}

pub fn create_temp_dir() -> io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join("excel_tool_gateway");
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

pub fn append_timestamp(path: &str) -> String {
    let p = Path::new(path);
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let parent = p.parent().unwrap_or(Path::new(""));
    parent
        .join(format!("{}_{}{}", stem, timestamp, ext))
        .to_string_lossy()
        .to_string()
}

pub fn ensure_parent_dir(path: &str) -> io::Result<()> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn file_size(path: &str) -> io::Result<u64> {
    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_path_exists() {
        assert!(path_exists("Cargo.toml"));
        assert!(!path_exists("nonexistent_file_xyz"));
    }

    #[test]
    fn test_copy_file() {
        let src = "Cargo.toml";
        let dst = "_test_copy_output.toml";
        let _ = fs::remove_file(dst);
        assert!(copy_file(src, dst).is_ok());
        assert!(path_exists(dst));
        let _ = fs::remove_file(dst);
    }

    #[test]
    fn test_create_temp_dir() {
        let dir = create_temp_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.to_string_lossy().contains("excel_tool_gateway"));
    }

    #[test]
    fn test_append_timestamp() {
        let result = append_timestamp("test.xlsx");
        assert!(result.starts_with("test_"));
        assert!(result.ends_with(".xlsx"));
        assert!(result.len() > "test_.xlsx".len());
    }

    #[test]
    fn test_ensure_parent_dir() {
        let path = "_test_dir/nested/file.txt";
        let _ = fs::remove_dir_all("_test_dir");
        assert!(ensure_parent_dir(path).is_ok());
        assert!(Path::new("_test_dir/nested").exists());
        let _ = fs::remove_dir_all("_test_dir");
    }

    #[test]
    fn test_file_size() {
        let size = file_size("Cargo.toml").unwrap();
        assert!(size > 0);
    }
}
