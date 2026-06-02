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
