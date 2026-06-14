use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub sheets: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub backup_path: String,
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub file_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityParams {
    pub dry_run: bool,
    pub create_backup: bool,
    pub file_path: String,
}

impl Default for SecurityParams {
    fn default() -> Self {
        Self {
            dry_run: false,
            create_backup: true,
            file_path: String::new(),
        }
    }
}
