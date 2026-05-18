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

    // Memory-conscious PRAGMAs:
    // - WAL keeps writes append-mostly and friendly to concurrent reads.
    // - cache_size = -2000 caps SQLite's page cache at ~2 MB instead of
    //   the 2 MB / 64 MB platform default; we never need a large cache
    //   for this app's tiny tables.
    // - temp_store = MEMORY keeps small temp results off disk.
    // - synchronous = NORMAL pairs well with WAL and avoids extra fsyncs.
    // - mmap_size = 0 disables file mmap so SQLite does not grow the
    //   process working set with mapped pages we never touch again.
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;\
         PRAGMA foreign_keys=ON;\
         PRAGMA cache_size=-2000;\
         PRAGMA temp_store=MEMORY;\
         PRAGMA synchronous=NORMAL;\
         PRAGMA mmap_size=0;",
    )?;
    schema::initialize_database(&conn)?;

    Ok(conn)
}
