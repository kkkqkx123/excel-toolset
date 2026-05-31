use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct SqlConfig {
    pub page_size: usize,
    pub max_rows: usize,
    pub max_cells: usize,
}

impl Default for SqlConfig {
    fn default() -> Self {
        Self {
            page_size: 1000,
            max_rows: 100_000,
            max_cells: 1_000_000,
        }
    }
}

static CONFIG: OnceLock<SqlConfig> = OnceLock::new();

pub fn get_config() -> &'static SqlConfig {
    CONFIG.get_or_init(SqlConfig::default)
}

pub fn set_config(config: SqlConfig) {
    CONFIG.set(config).ok();
}
