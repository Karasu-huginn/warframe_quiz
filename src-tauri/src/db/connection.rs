use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Database {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }
}
