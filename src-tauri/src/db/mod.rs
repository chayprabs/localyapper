// Database subsystem re-exports
pub mod queries;
pub mod schema;

use rusqlite::Connection;
use std::path::Path;

use crate::error::LocalYapperError;

/// Opens (or creates) the SQLite database and initializes the schema.
pub fn open_database(app_data_dir: &Path) -> Result<Connection, LocalYapperError> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("localyapper.db");
    let conn = Connection::open(db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    schema::initialize_database(&conn)?;

    Ok(conn)
}
