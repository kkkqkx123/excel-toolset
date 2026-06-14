#![expect(dead_code)]
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct SqlConfig {
    /// Batch size for insert operations (currently unused, reserved for future optimizations).
    pub page_size: usize,
}

impl Default for SqlConfig {
    fn default() -> Self {
        Self { page_size: 1000 }
    }
}

static CONFIG: OnceLock<SqlConfig> = OnceLock::new();

pub fn get_config() -> &'static SqlConfig {
    CONFIG.get_or_init(SqlConfig::default)
}

pub fn set_config(config: SqlConfig) -> Result<(), &'static str> {
    CONFIG
        .set(config)
        .map_err(|_| "SqlConfig has already been initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = SqlConfig::default();
        assert_eq!(cfg.page_size, 1000);
    }

    #[test]
    fn test_get_config_returns_default() {
        let cfg = get_config();
        assert_eq!(cfg.page_size, 1000);
    }

    #[test]
    fn test_set_config_once() {
        // This test may be affected by test ordering since CONFIG is OnceLock.
        // If get_config() was called first in another test, set_config will fail.
        let result = set_config(SqlConfig { page_size: 500 });
        // Either succeeds or fails with "already initialized" — both are valid.
        match result {
            Ok(()) => assert_eq!(get_config().page_size, 500),
            Err(msg) => assert_eq!(msg, "SqlConfig has already been initialized"),
        }
    }
}
