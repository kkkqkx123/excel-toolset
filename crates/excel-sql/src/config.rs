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
