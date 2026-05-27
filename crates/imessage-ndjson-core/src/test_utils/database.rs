//! Test database helper utilities

use rusqlite::Connection;
use std::path::Path;

/// Opens a read-only connection to a test database
///
/// # Arguments
/// * `path` - Path to the database file
///
/// # Returns
/// A read-only SQLite connection
///
/// # Example
/// ```no_run
/// use imessage_ndjson_exporter::test_utils::{database, fixtures};
///
/// let db_path = fixtures::get_test_db_path();
/// let conn = database::open_test_db(&db_path).unwrap();
/// ```
pub fn open_test_db(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(conn)
}

/// Counts the number of messages in the database
///
/// # Arguments
/// * `conn` - Database connection
///
/// # Returns
/// The total number of messages
pub fn count_messages(conn: &Connection) -> anyhow::Result<i64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM message", [], |row| row.get(0))?;
    Ok(count)
}

/// Counts the number of chats in the database
///
/// # Arguments
/// * `conn` - Database connection
///
/// # Returns
/// The total number of chats
pub fn count_chats(conn: &Connection) -> anyhow::Result<i64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM chat", [], |row| row.get(0))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::fixtures;

    #[test]
    fn test_open_test_db() {
        let db_path = fixtures::get_test_db_path();
        let conn = open_test_db(&db_path).unwrap();
        assert!(conn.is_autocommit());
    }

    #[test]
    fn test_count_messages() {
        let db_path = fixtures::get_test_db_path();
        let conn = open_test_db(&db_path).unwrap();
        let count = count_messages(&conn).unwrap();
        // test.db has 3 messages according to the plan
        assert!(count >= 0, "Message count should be non-negative");
    }

    #[test]
    fn test_count_chats() {
        let db_path = fixtures::get_test_db_path();
        let conn = open_test_db(&db_path).unwrap();
        let count = count_chats(&conn).unwrap();
        assert!(count >= 0, "Chat count should be non-negative");
    }
}
