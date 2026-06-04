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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::conn::create_conn;

    fn setup_table(db: &duckdb::Connection) {
        db.execute_batch(r#"CREATE TABLE "test_tbl" (c0 INTEGER, c1 VARCHAR)"#)
            .unwrap();
        db.execute_batch(r#"INSERT INTO "test_tbl" VALUES (1, 'a'), (2, 'b')"#)
            .unwrap();
    }

    #[test]
    fn test_table_exists_true() {
        let conn = create_conn().unwrap();
        setup_table(&conn);
        assert!(table_exists(&conn, "test_tbl").unwrap());
    }

    #[test]
    fn test_table_exists_false() {
        let conn = create_conn().unwrap();
        assert!(!table_exists(&conn, "nonexistent").unwrap());
    }

    #[test]
    fn test_list_tables() {
        let conn = create_conn().unwrap();
        assert!(list_tables(&conn).unwrap().is_empty());

        setup_table(&conn);
        let tables = list_tables(&conn).unwrap();
        assert_eq!(tables, vec!["test_tbl"]);
    }

    #[test]
    fn test_drop_table() {
        let conn = create_conn().unwrap();
        setup_table(&conn);
        assert!(table_exists(&conn, "test_tbl").unwrap());
        drop_table(&conn, "test_tbl").unwrap();
        assert!(!table_exists(&conn, "test_tbl").unwrap());
    }

    #[test]
    fn test_drop_table_nonexistent() {
        let conn = create_conn().unwrap();
        drop_table(&conn, "nonexistent").unwrap(); // should not panic
    }

    #[test]
    fn test_get_table_schema() {
        let conn = create_conn().unwrap();
        setup_table(&conn);
        let schema = get_table_schema(&conn, "test_tbl").unwrap();
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].name, "c0");
        assert_eq!(schema[1].name, "c1");
    }

    #[test]
    fn test_table_row_count() {
        let conn = create_conn().unwrap();
        setup_table(&conn);
        assert_eq!(table_row_count(&conn, "test_tbl").unwrap(), 2);
    }

    #[test]
    fn test_clear_database() {
        let conn = create_conn().unwrap();
        setup_table(&conn);
        clear_database(&conn).unwrap();
        assert!(list_tables(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_escape_name() {
        let conn = create_conn().unwrap();
        // Table name with quotes
        let sql = r#"CREATE TABLE "quo""te" (c0 INTEGER)"#;
        conn.execute_batch(sql).unwrap();
        assert!(table_exists(&conn, "quo\"te").unwrap());
        drop_table(&conn, "quo\"te").unwrap();
    }
}
