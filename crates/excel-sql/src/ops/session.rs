use std::collections::HashSet;

use excel_types::{AppError, FilterCondition, SheetData, SortColumn};

use crate::converter::QueryResult;
use crate::db::query::query as db_query;
use crate::db::{create_conn, load_sheet_to_db};

pub struct QuerySession {
    conn: duckdb::Connection,
    loaded_tables: HashSet<String>,
}

impl QuerySession {
    pub fn new() -> Result<Self, AppError> {
        let conn = create_conn()
            .map_err(|e| AppError::DuckDb(format!("Failed to create session connection: {e}")))?;
        Ok(Self {
            conn,
            loaded_tables: HashSet::new(),
        })
    }

    pub fn load_sheet(
        &mut self,
        name: &str,
        data: &SheetData,
        has_header: bool,
    ) -> Result<(), AppError> {
        if self.loaded_tables.contains(name) {
            return Ok(());
        }
        load_sheet_to_db(&self.conn, name, data, has_header)?;
        self.loaded_tables.insert(name.to_string());
        Ok(())
    }

    pub fn ensure_sheet_loaded(
        &mut self,
        data: &SheetData,
        has_header: bool,
    ) -> Result<(), AppError> {
        self.load_sheet(&data.name, data, has_header)
    }

    pub fn query(&self, sql: &str) -> Result<QueryResult, AppError> {
        db_query(&self.conn, sql)
    }

    pub fn sql_query_on_data(
        &mut self,
        data: &[SheetData],
        sql: &str,
        has_header: bool,
    ) -> Result<QueryResult, AppError> {
        for sheet in data {
            self.load_sheet(&sheet.name, sheet, has_header)?;
        }
        db_query(&self.conn, sql)
    }

    pub fn filter_rows_on_data(
        &mut self,
        data: &SheetData,
        sheet: &str,
        conditions: &[FilterCondition],
        has_header: bool,
    ) -> Result<QueryResult, AppError> {
        self.ensure_sheet_loaded(data, has_header)?;
        crate::ops::query::filter_rows_on_data_impl(&self.conn, sheet, conditions)
    }

    pub fn sort_sheet_on_data(
        &mut self,
        data: &SheetData,
        sort_columns: &[SortColumn],
    ) -> Result<SheetData, AppError> {
        crate::ops::write::sort_sheet_on_data_impl(&mut self.conn, data, sort_columns)
    }

    pub fn dedup_sheet_on_data(
        &mut self,
        data: &SheetData,
        columns: &[u16],
    ) -> Result<SheetData, AppError> {
        crate::ops::write::dedup_sheet_on_data_impl(&mut self.conn, data, columns)
    }

    pub fn clear(&mut self) -> Result<(), AppError> {
        self.loaded_tables.clear();
        crate::db::tables::clear_database(&self.conn)
    }

    pub fn table_exists(&self, name: &str) -> Result<bool, AppError> {
        crate::db::tables::table_exists(&self.conn, name)
    }

    pub fn list_tables(&self) -> Result<Vec<String>, AppError> {
        crate::db::tables::list_tables(&self.conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType::*, FilterOp};

    fn make_cell(value: Option<&str>, dt: excel_types::CellDataType) -> CellData {
        CellData {
            value: value.map(|s| s.to_string()),
            data_type: dt,
            formula: None,
        }
    }

    fn sheet1() -> SheetData {
        SheetData {
            name: "s1".to_string(),
            rows: vec![
                vec![make_cell(Some("x"), String)],
                vec![make_cell(Some("y"), String)],
            ],
        }
    }

    fn sheet2() -> SheetData {
        SheetData {
            name: "s2".to_string(),
            rows: vec![
                vec![make_cell(Some("1"), Int)],
                vec![make_cell(Some("2"), Int)],
            ],
        }
    }

    #[test]
    fn test_session_new() {
        let session = QuerySession::new().expect("Should create session");
        assert!(session.list_tables().unwrap().is_empty());
    }

    #[test]
    fn test_session_load_and_query() {
        let mut session = QuerySession::new().unwrap();
        session.load_sheet("s1", &sheet1(), false).unwrap();
        let result = session.query("SELECT * FROM \"s1\"").unwrap();
        assert_eq!(result.row_count, 2);
    }

    #[test]
    fn test_session_deduplicates_loads() {
        let mut session = QuerySession::new().unwrap();
        session.load_sheet("s1", &sheet1(), false).unwrap();
        session.load_sheet("s1", &sheet1(), false).unwrap(); // should be no-op
        let tables = session.list_tables().unwrap();
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn test_session_sql_query_on_data() {
        let mut session = QuerySession::new().unwrap();
        let result = session
            .sql_query_on_data(
                &[sheet1(), sheet2()],
                "SELECT COUNT(*) AS cnt FROM \"s2\"",
                false,
            )
            .unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn test_session_filter_rows_on_data() {
        let mut session = QuerySession::new().unwrap();
        let cond = FilterCondition {
            column: 0,
            operator: FilterOp::Eq,
            value: "x".into(),
        };
        let result = session
            .filter_rows_on_data(&sheet1(), "s1", &[cond], false)
            .unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn test_session_clear() {
        let mut session = QuerySession::new().unwrap();
        session.load_sheet("s1", &sheet1(), false).unwrap();
        assert!(!session.list_tables().unwrap().is_empty());
        session.clear().unwrap();
        assert!(session.list_tables().unwrap().is_empty());
    }

    #[test]
    fn test_session_ensure_sheet_loaded() {
        let mut session = QuerySession::new().unwrap();
        session.ensure_sheet_loaded(&sheet1(), false).unwrap();
        assert!(session.table_exists("s1").unwrap());
    }

    #[test]
    fn test_session_table_exists() {
        let session = QuerySession::new().unwrap();
        assert!(!session.table_exists("nonexistent").unwrap());
    }
}
