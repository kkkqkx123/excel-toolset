use duckdb::Connection;

pub fn create_conn() -> Result<Connection, duckdb::Error> {
    Connection::open_in_memory()
}
