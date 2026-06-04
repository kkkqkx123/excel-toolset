use duckdb::Connection;

pub fn create_conn() -> Result<Connection, duckdb::Error> {
    Connection::open_in_memory()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_conn_success() {
        let conn = create_conn().expect("Failed to create in-memory DuckDB connection");
        conn.execute_batch("SELECT 1")
            .expect("Should be able to execute a simple query");
    }
}
