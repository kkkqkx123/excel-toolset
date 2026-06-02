use excel_types::AppError;

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

fn escape_name(name: &str) -> String {
    name.replace('"', "\"\"")
}

pub fn table_exists(db: &duckdb::Connection, name: &str) -> Result<bool, AppError> {
    let sql = r#"SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?"#;
    let count: i64 = db
        .query_row(sql, [name], |row| row.get(0))
        .map_err(|e| AppError::DuckDb(e.to_string()))?;
    Ok(count > 0)
}

pub fn list_tables(db: &duckdb::Connection) -> Result<Vec<String>, AppError> {
    let sql = r#"SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name"#;

    let mut stmt = db
        .prepare(sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let tables = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| AppError::DuckDb(e.to_string()))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    Ok(tables)
}

pub fn drop_table(db: &duckdb::Connection, name: &str) -> Result<(), AppError> {
    let escaped = escape_name(name);
    let sql = format!(r#"DROP TABLE IF EXISTS "{}""#, escaped);
    db.execute_batch(&sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;
    Ok(())
}

pub fn get_table_schema(db: &duckdb::Connection, name: &str) -> Result<Vec<ColumnInfo>, AppError> {
    let sql = r#"SELECT column_name, data_type, is_nullable 
           FROM information_schema.columns 
           WHERE table_name = ? 
           ORDER BY ordinal_position"#;

    let mut stmt = db
        .prepare(sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let columns = stmt
        .query_map([name], |row| {
            let nullable_str: String = row.get(2)?;
            Ok(ColumnInfo {
                name: row.get(0)?,
                data_type: row.get(1)?,
                nullable: nullable_str == "YES",
            })
        })
        .map_err(|e| AppError::DuckDb(e.to_string()))?
        .collect::<Result<Vec<ColumnInfo>, _>>()
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    Ok(columns)
}

pub fn clear_database(db: &duckdb::Connection) -> Result<(), AppError> {
    let tables = list_tables(db)?;
    for table in tables {
        drop_table(db, &table)?;
    }
    Ok(())
}

pub fn table_row_count(db: &duckdb::Connection, name: &str) -> Result<usize, AppError> {
    let escaped = escape_name(name);
    let sql = format!(r#"SELECT COUNT(*) FROM "{}""#, escaped);
    let count: i64 = db
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|e| AppError::DuckDb(e.to_string()))?;
    Ok(count as usize)
}
