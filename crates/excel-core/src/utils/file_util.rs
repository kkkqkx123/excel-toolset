use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;

pub fn copy_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::copy(src, dst)?;
    Ok(())
}

pub fn append_timestamp(path: impl AsRef<Path>) -> PathBuf {
    let p = path.as_ref();
    let stem = p.file_stem().unwrap_or_default();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut result = p.parent().unwrap_or(Path::new("")).to_path_buf();
    result.push(format!("{}_{}{}", stem.to_string_lossy(), timestamp, ext));
    result
}

pub fn ensure_parent_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_copy_file() {
        let src = "Cargo.toml";
        let dst = "_test_copy_output.toml";
        let _ = fs::remove_file(dst);
        assert!(copy_file(src, dst).is_ok());
        assert!(Path::new(dst).exists());
        let _ = fs::remove_file(dst);
    }

    #[test]
    fn test_append_timestamp() {
        let result = append_timestamp("test.xlsx");
        let s = result.to_string_lossy();
        assert!(s.starts_with("test_"));
        assert!(s.ends_with(".xlsx"));
        assert!(s.len() > "test_.xlsx".len());
    }

    #[test]
    fn test_ensure_parent_dir() {
        let path = "_test_dir/nested/file.txt";
        let _ = fs::remove_dir_all("_test_dir");
        assert!(ensure_parent_dir(path).is_ok());
        assert!(Path::new("_test_dir/nested").exists());
        let _ = fs::remove_dir_all("_test_dir");
    }
}