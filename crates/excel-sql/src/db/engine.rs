use crate::db::{create_conn, drop_table, list_tables, load_sheet_to_db, query, table_exists};
use excel_types::{AppError, SheetData};
use std::path::PathBuf;

pub struct ExcelQueryEngine {
    pub conn: duckdb::Connection,
    pub persistent_path: Option<PathBuf>,
}

impl ExcelQueryEngine {
    pub fn new() -> Result<Self, AppError> {
        let conn = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;
        Ok(Self {
            conn,
            persistent_path: None,
        })
    }

    pub fn with_cache(cache_path: &str) -> Result<Self, AppError> {
        let path = PathBuf::from(cache_path);
        let conn = if path.exists() {
            duckdb::Connection::open(&path).map_err(|e| AppError::DuckDb(e.to_string()))?
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            duckdb::Connection::open(&path).map_err(|e| AppError::DuckDb(e.to_string()))?
        };
        Ok(Self {
            conn,
            persistent_path: Some(path),
        })
    }

    pub fn load_sheet(
        &mut self,
        name: &str,
        data: &SheetData,
        has_header: bool,
    ) -> Result<(), AppError> {
        load_sheet_to_db(&self.conn, name, data, has_header)
    }

    pub fn load_with_header(&mut self, name: &str, data: &SheetData) -> Result<(), AppError> {
        self.load_sheet(name, data, true)
    }

    pub fn load_without_header(&mut self, name: &str, data: &SheetData) -> Result<(), AppError> {
        self.load_sheet(name, data, false)
    }

    pub fn query(&self, sql: &str) -> Result<crate::converter::QueryResult, AppError> {
        query(&self.conn, sql)
    }

    pub fn query_with_params(
        &self,
        sql: &str,
        params: &[duckdb::types::Value],
    ) -> Result<crate::converter::QueryResult, AppError> {
        crate::db::query::query_with_params(&self.conn, sql, params)
    }

    pub fn query_to_strings(&self, sql: &str) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        crate::db::query::query_to_strings(&self.conn, sql)
    }

    pub fn table_exists(&self, name: &str) -> Result<bool, AppError> {
        table_exists(&self.conn, name)
    }

    pub fn drop_table(&self, name: &str) -> Result<(), AppError> {
        drop_table(&self.conn, name)
    }

    pub fn list_tables(&self) -> Result<Vec<String>, AppError> {
        list_tables(&self.conn)
    }

    pub fn clear(&self) -> Result<(), AppError> {
        crate::db::tables::clear_database(&self.conn)
    }
}

impl Drop for ExcelQueryEngine {
    fn drop(&mut self) {
        if self.persistent_path.is_some() {
            let _ = self.conn.execute_batch("CHECKPOINT");
        }
    }
}
