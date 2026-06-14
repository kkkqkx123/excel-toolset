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

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::CellData;
    use excel_types::CellDataType::*;

    fn make_cell(value: Option<&str>, dt: excel_types::CellDataType) -> CellData {
        CellData {
            value: value.map(|s| s.to_string()),
            data_type: dt,
            formula: None,
        }
    }

    fn sample_sheet() -> SheetData {
        SheetData {
            name: "data".to_string(),
            rows: vec![
                vec![make_cell(Some("Name"), String), make_cell(Some("Age"), Int)],
                vec![make_cell(Some("Alice"), String), make_cell(Some("30"), Int)],
                vec![make_cell(Some("Bob"), String), make_cell(Some("25"), Int)],
            ],
        }
    }

    #[test]
    fn test_engine_new() {
        let engine = ExcelQueryEngine::new().expect("Should create engine");
        assert!(engine.persistent_path.is_none());
    }

    #[test]
    fn test_engine_load_and_query() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = sample_sheet();
        engine.load_with_header("data", &sheet).unwrap();

        let result = engine.query("SELECT * FROM \"data\" ORDER BY c0").unwrap();
        assert_eq!(result.row_count, 2);
        assert_eq!(result.columns, vec!["Name", "Age"]);
    }

    #[test]
    fn test_engine_load_without_header() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = SheetData {
            name: "raw".to_string(),
            rows: vec![
                vec![make_cell(Some("x"), String)],
                vec![make_cell(Some("y"), String)],
            ],
        };
        engine.load_without_header("raw", &sheet).unwrap();
        let result = engine.query("SELECT * FROM \"raw\"").unwrap();
        assert_eq!(result.row_count, 2);
        assert_eq!(result.columns, vec!["c0"]);
    }

    #[test]
    fn test_engine_table_exists() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = sample_sheet();
        engine.load_with_header("data", &sheet).unwrap();
        assert!(engine.table_exists("data").unwrap());
        assert!(!engine.table_exists("nope").unwrap());
    }

    #[test]
    fn test_engine_list_and_drop() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = sample_sheet();
        engine.load_with_header("data", &sheet).unwrap();

        let tables = engine.list_tables().unwrap();
        assert_eq!(tables, vec!["data"]);

        engine.drop_table("data").unwrap();
        assert!(!engine.table_exists("data").unwrap());
    }

    #[test]
    fn test_engine_clear() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = sample_sheet();
        engine.load_with_header("data", &sheet).unwrap();
        engine.clear().unwrap();
        assert!(engine.list_tables().unwrap().is_empty());
    }

    #[test]
    fn test_engine_query_with_params() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = sample_sheet();
        engine.load_without_header("data", &sheet).unwrap();

        let params = [duckdb::types::Value::Text("Alice".to_string())];
        let result = engine
            .query_with_params("SELECT * FROM \"data\" WHERE c0 = ?1", &params)
            .unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn test_engine_query_to_strings() {
        let mut engine = ExcelQueryEngine::new().unwrap();
        let sheet = sample_sheet();
        engine.load_with_header("data", &sheet).unwrap();

        let (cols, rows) = engine
            .query_to_strings("SELECT Name FROM \"data\" ORDER BY Age")
            .unwrap();
        assert_eq!(cols, vec!["Name"]);
        assert_eq!(rows, vec![vec!["Bob"], vec!["Alice"]]);
    }
}
