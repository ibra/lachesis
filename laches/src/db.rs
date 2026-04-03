use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;

const SCHEMA_VERSION: i32 = 1;

/// Timestamp format used for all session start/end times in the database.
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// A recorded session of focused window usage.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub process_name: String,
    pub exe_path: Option<String>,
    pub window_title: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub idle: bool,
}

/// Aggregated process usage over a time range.
#[derive(Debug, Clone)]
pub struct ProcessSummary {
    pub process_name: String,
    pub total_seconds: i64,
    pub session_count: i64,
    pub active_days: i64,
}

/// Map a database row to a Session struct.
/// Used by all session-returning queries to avoid duplication.
fn map_session_row(row: &rusqlite::Row) -> SqlResult<Session> {
    Ok(Session {
        id: row.get(0)?,
        process_name: row.get(1)?,
        exe_path: row.get(2)?,
        window_title: row.get(3)?,
        start_time: row.get(4)?,
        end_time: row.get(5)?,
        idle: row.get::<_, i32>(6)? != 0,
    })
}

/// Owns a SQLite connection and provides all data operations.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path.
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    #[cfg(test)]
    pub fn open_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );",
        )?;

        let version: i32 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if version < 1 {
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    process_name TEXT NOT NULL,
                    exe_path TEXT,
                    window_title TEXT,
                    start_time TEXT NOT NULL,
                    end_time TEXT,
                    idle INTEGER NOT NULL DEFAULT 0
                );

                CREATE TABLE IF NOT EXISTS tags (
                    process_name TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    PRIMARY KEY (process_name, tag)
                );

                CREATE INDEX IF NOT EXISTS idx_sessions_start ON sessions(start_time);
                CREATE INDEX IF NOT EXISTS idx_sessions_process ON sessions(process_name);

                INSERT INTO schema_version (version) VALUES (1);",
            )?;
        }

        if version > SCHEMA_VERSION {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "database schema version {} is newer than supported version {}. update lachesis to open this database",
                version, SCHEMA_VERSION
            )));
        }

        Ok(())
    }

    // -- session operations --

    /// Start a new session. Returns the session id.
    pub fn start_session(
        &self,
        process_name: &str,
        exe_path: Option<&str>,
        window_title: Option<&str>,
        idle: bool,
    ) -> SqlResult<i64> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        self.conn.execute(
            "INSERT INTO sessions (process_name, exe_path, window_title, start_time, idle)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![process_name, exe_path, window_title, now, idle as i32],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// End a session by setting its end_time to now.
    pub fn end_session(&self, session_id: i64) -> SqlResult<()> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        self.conn.execute(
            "UPDATE sessions SET end_time = ?1 WHERE id = ?2 AND end_time IS NULL",
            params![now, session_id],
        )?;
        Ok(())
    }

    /// Close all open sessions (used on daemon shutdown).
    pub fn close_all_open_sessions(&self) -> SqlResult<usize> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let count = self.conn.execute(
            "UPDATE sessions SET end_time = ?1 WHERE end_time IS NULL",
            params![now],
        )?;
        Ok(count)
    }

    /// Get the currently open session (if any).
    pub fn get_open_session(&self) -> SqlResult<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, process_name, exe_path, window_title, start_time, end_time, idle
             FROM sessions WHERE end_time IS NULL LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], map_session_row)?;
        match rows.next() {
            Some(Ok(session)) => Ok(Some(session)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Query process summaries for a date range, optionally filtered by tag.
    pub fn query_process_summaries(
        &self,
        start_date: &str,
        end_date: &str,
        tag_filter: Option<&str>,
    ) -> SqlResult<Vec<ProcessSummary>> {
        let query = if tag_filter.is_some() {
            "SELECT s.process_name,
                    SUM(CAST(ROUND((julianday(COALESCE(s.end_time, datetime('now', 'localtime'))) - julianday(s.start_time)) * 86400) AS INTEGER)) as total_seconds,
                    COUNT(*) as session_count,
                    COUNT(DISTINCT date(s.start_time)) as active_days
             FROM sessions s
             JOIN tags t ON s.process_name = t.process_name
             WHERE s.start_time >= ?1 AND s.start_time < ?2
               AND s.idle = 0 AND t.tag = ?3
             GROUP BY s.process_name
             ORDER BY total_seconds DESC"
        } else {
            "SELECT process_name,
                    SUM(CAST(ROUND((julianday(COALESCE(end_time, datetime('now', 'localtime'))) - julianday(start_time)) * 86400) AS INTEGER)) as total_seconds,
                    COUNT(*) as session_count,
                    COUNT(DISTINCT date(start_time)) as active_days
             FROM sessions
             WHERE start_time >= ?1 AND start_time < ?2
               AND idle = 0
             GROUP BY process_name
             ORDER BY total_seconds DESC"
        };

        let mut stmt = self.conn.prepare(query)?;

        let map_row = |row: &rusqlite::Row| -> SqlResult<ProcessSummary> {
            Ok(ProcessSummary {
                process_name: row.get(0)?,
                total_seconds: row.get(1)?,
                session_count: row.get(2)?,
                active_days: row.get(3)?,
            })
        };

        if let Some(tag) = tag_filter {
            stmt.query_map(params![start_date, end_date, tag], map_row)?
                .collect()
        } else {
            stmt.query_map(params![start_date, end_date], map_row)?
                .collect()
        }
    }

    /// Get total active (non-idle) seconds for a date range.
    pub fn query_total_active_seconds(&self, start_date: &str, end_date: &str) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(CAST(ROUND((julianday(COALESCE(end_time, datetime('now', 'localtime'))) - julianday(start_time)) * 86400) AS INTEGER)), 0)
             FROM sessions
             WHERE start_time >= ?1 AND start_time < ?2 AND idle = 0",
            params![start_date, end_date],
            |row| row.get(0),
        )
    }

    /// Get total idle seconds for a date range.
    pub fn query_total_idle_seconds(&self, start_date: &str, end_date: &str) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(CAST(ROUND((julianday(COALESCE(end_time, datetime('now', 'localtime'))) - julianday(start_time)) * 86400) AS INTEGER)), 0)
             FROM sessions
             WHERE start_time >= ?1 AND start_time < ?2 AND idle = 1",
            params![start_date, end_date],
            |row| row.get(0),
        )
    }

    /// Get individual sessions for a date range.
    pub fn query_sessions(&self, start_date: &str, end_date: &str) -> SqlResult<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, process_name, exe_path, window_title, start_time, end_time, idle
             FROM sessions
             WHERE start_time >= ?1 AND start_time < ?2
             ORDER BY start_time DESC",
        )?;

        let rows = stmt.query_map(params![start_date, end_date], map_session_row)?;
        rows.collect()
    }

    /// Get daily active totals for a date range, returned as (date_label, seconds) pairs.
    /// Uses a single aggregated query instead of per-day lookups.
    pub fn query_daily_totals(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> SqlResult<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT date(start_time) as day,
                    COALESCE(SUM(CAST(ROUND((julianday(COALESCE(end_time, datetime('now', 'localtime'))) - julianday(start_time)) * 86400) AS INTEGER)), 0)
             FROM sessions
             WHERE start_time >= ?1 AND start_time < ?2 AND idle = 0
             GROUP BY day
             ORDER BY day",
        )?;

        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        rows.collect()
    }

    /// Delete sessions in a date range.
    pub fn delete_sessions(&self, start_date: &str, end_date: &str) -> SqlResult<usize> {
        self.conn.execute(
            "DELETE FROM sessions WHERE start_time >= ?1 AND start_time < ?2",
            params![start_date, end_date],
        )
    }

    /// Delete all sessions.
    pub fn delete_all_sessions(&self) -> SqlResult<usize> {
        self.conn.execute("DELETE FROM sessions", [])
    }

    /// Reset the entire database (drop and recreate tables).
    pub fn reset(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "DELETE FROM sessions;
             DELETE FROM tags;
             DELETE FROM schema_version;
             INSERT INTO schema_version (version) VALUES (1);",
        )
    }

    // -- tag operations --

    /// Add a tag to a process.
    pub fn add_tag(&self, process_name: &str, tag: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tags (process_name, tag) VALUES (?1, ?2)",
            params![process_name, tag],
        )?;
        Ok(())
    }

    /// Remove a tag from a process.
    pub fn remove_tag(&self, process_name: &str, tag: &str) -> SqlResult<bool> {
        let count = self.conn.execute(
            "DELETE FROM tags WHERE process_name = ?1 AND tag = ?2",
            params![process_name, tag],
        )?;
        Ok(count > 0)
    }

    /// List all tags for a process.
    pub fn get_tags(&self, process_name: &str) -> SqlResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM tags WHERE process_name = ?1 ORDER BY tag")?;
        let rows = stmt.query_map(params![process_name], |row| row.get(0))?;
        rows.collect()
    }

    /// List all unique process names that have sessions.
    pub fn get_tracked_processes(&self) -> SqlResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT process_name FROM sessions ORDER BY process_name")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// Export all sessions as a vector (for JSON export).
    pub fn export_sessions(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> SqlResult<Vec<Session>> {
        if let (Some(start), Some(end)) = (start_date, end_date) {
            self.query_sessions(start, end)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, process_name, exe_path, window_title, start_time, end_time, idle
                 FROM sessions ORDER BY start_time DESC",
            )?;
            let rows = stmt.query_map([], map_session_row)?;
            rows.collect()
        }
    }
}

/// Helper: get the start-of-day and start-of-next-day strings for a date.
pub fn date_range_for_day(date: &str) -> Option<(String, String)> {
    let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let next = parsed.succ_opt()?;
    Some((format!("{}T00:00:00", parsed), format!("{}T00:00:00", next)))
}

/// Helper: get date range for "today".
pub fn today_range() -> (String, String) {
    let today = Local::now().format("%Y-%m-%d").to_string();
    date_range_for_day(&today).unwrap()
}

/// Helper: get date range for the last N days (inclusive of today).
pub fn last_n_days_range(n: i64) -> (String, String) {
    let today = Local::now().date_naive();
    let start = today - chrono::Duration::days(n - 1);
    let end = today + chrono::Duration::days(1);
    (format!("{}T00:00:00", start), format!("{}T00:00:00", end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_open_creates_schema() {
        let db = Database::open_memory().unwrap();
        let version: i32 = db
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_start_and_end_session() {
        let db = Database::open_memory().unwrap();

        let id = db
            .start_session("firefox", Some("/usr/bin/firefox"), Some("GitHub"), false)
            .unwrap();
        assert!(id > 0);

        let open = db.get_open_session().unwrap();
        assert!(open.is_some());
        let session = open.unwrap();
        assert_eq!(session.process_name, "firefox");
        assert_eq!(session.exe_path.as_deref(), Some("/usr/bin/firefox"));
        assert!(session.end_time.is_none());

        thread::sleep(Duration::from_millis(10));
        db.end_session(id).unwrap();

        let closed = db.get_open_session().unwrap();
        assert!(closed.is_none());
    }

    #[test]
    fn test_close_all_open_sessions() {
        let db = Database::open_memory().unwrap();

        db.start_session("firefox", None, None, false).unwrap();
        db.start_session("code", None, None, false).unwrap();

        let count = db.close_all_open_sessions().unwrap();
        assert_eq!(count, 2);

        assert!(db.get_open_session().unwrap().is_none());
    }

    #[test]
    fn test_tags_crud() {
        let db = Database::open_memory().unwrap();

        db.add_tag("firefox", "browser").unwrap();
        db.add_tag("firefox", "work").unwrap();
        db.add_tag("code", "dev").unwrap();

        let tags = db.get_tags("firefox").unwrap();
        assert_eq!(tags, vec!["browser", "work"]);

        // duplicate insert is ignored
        db.add_tag("firefox", "browser").unwrap();
        let tags = db.get_tags("firefox").unwrap();
        assert_eq!(tags.len(), 2);

        let removed = db.remove_tag("firefox", "work").unwrap();
        assert!(removed);

        let tags = db.get_tags("firefox").unwrap();
        assert_eq!(tags, vec!["browser"]);

        let removed = db.remove_tag("firefox", "nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_query_process_summaries() {
        let db = Database::open_memory().unwrap();

        // insert sessions with explicit times
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T10:00:00', '2026-04-01T11:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T14:00:00', '2026-04-01T14:30:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('code', '2026-04-01T10:00:00', '2026-04-01T12:00:00', 0)",
                [],
            )
            .unwrap();

        let summaries = db
            .query_process_summaries("2026-04-01T00:00:00", "2026-04-02T00:00:00", None)
            .unwrap();

        assert_eq!(summaries.len(), 2);
        // code should be first (more total time: 2h vs 1.5h)
        assert_eq!(summaries[0].process_name, "code");
        assert_eq!(summaries[0].total_seconds, 7200);
        assert_eq!(summaries[0].session_count, 1);

        assert_eq!(summaries[1].process_name, "firefox");
        assert_eq!(summaries[1].total_seconds, 5400);
        assert_eq!(summaries[1].session_count, 2);
    }

    #[test]
    fn test_query_excludes_idle_sessions() {
        let db = Database::open_memory().unwrap();

        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T10:00:00', '2026-04-01T11:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('idle', '2026-04-01T11:00:00', '2026-04-01T11:30:00', 1)",
                [],
            )
            .unwrap();

        let summaries = db
            .query_process_summaries("2026-04-01T00:00:00", "2026-04-02T00:00:00", None)
            .unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].process_name, "firefox");
    }

    #[test]
    fn test_query_with_tag_filter() {
        let db = Database::open_memory().unwrap();

        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T10:00:00', '2026-04-01T11:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('code', '2026-04-01T10:00:00', '2026-04-01T12:00:00', 0)",
                [],
            )
            .unwrap();

        db.add_tag("code", "work").unwrap();

        let summaries = db
            .query_process_summaries("2026-04-01T00:00:00", "2026-04-02T00:00:00", Some("work"))
            .unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].process_name, "code");
    }

    #[test]
    fn test_delete_sessions_by_range() {
        let db = Database::open_memory().unwrap();

        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T10:00:00', '2026-04-01T11:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-02T10:00:00', '2026-04-02T11:00:00', 0)",
                [],
            )
            .unwrap();

        let deleted = db
            .delete_sessions("2026-04-01T00:00:00", "2026-04-02T00:00:00")
            .unwrap();
        assert_eq!(deleted, 1);

        let all = db.export_sessions(None, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].start_time, "2026-04-02T10:00:00");
    }

    #[test]
    fn test_reset() {
        let db = Database::open_memory().unwrap();

        db.start_session("firefox", None, None, false).unwrap();
        db.add_tag("firefox", "browser").unwrap();

        db.reset().unwrap();

        let all = db.export_sessions(None, None).unwrap();
        assert!(all.is_empty());

        let tags = db.get_tags("firefox").unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_date_range_helpers() {
        let (start, end) = date_range_for_day("2026-04-01").unwrap();
        assert_eq!(start, "2026-04-01T00:00:00");
        assert_eq!(end, "2026-04-02T00:00:00");

        assert!(date_range_for_day("invalid").is_none());
    }

    #[test]
    fn test_get_tracked_processes() {
        let db = Database::open_memory().unwrap();

        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T10:00:00', '2026-04-01T11:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('code', '2026-04-01T10:00:00', '2026-04-01T12:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-02T10:00:00', '2026-04-02T11:00:00', 0)",
                [],
            )
            .unwrap();

        let procs = db.get_tracked_processes().unwrap();
        assert_eq!(procs, vec!["code", "firefox"]);
    }

    #[test]
    fn test_total_active_and_idle_seconds() {
        let db = Database::open_memory().unwrap();

        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('firefox', '2026-04-01T10:00:00', '2026-04-01T11:00:00', 0)",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sessions (process_name, start_time, end_time, idle)
                 VALUES ('idle', '2026-04-01T11:00:00', '2026-04-01T11:30:00', 1)",
                [],
            )
            .unwrap();

        let active = db
            .query_total_active_seconds("2026-04-01T00:00:00", "2026-04-02T00:00:00")
            .unwrap();
        assert_eq!(active, 3600);

        let idle = db
            .query_total_idle_seconds("2026-04-01T00:00:00", "2026-04-02T00:00:00")
            .unwrap();
        assert_eq!(idle, 1800);
    }

    #[test]
    fn test_open_file_db() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        {
            let db = Database::open(&db_path).unwrap();
            db.start_session("firefox", None, None, false).unwrap();
        }

        // reopen and verify data persisted
        {
            let db = Database::open(&db_path).unwrap();
            let open = db.get_open_session().unwrap();
            assert!(open.is_some());
            assert_eq!(open.unwrap().process_name, "firefox");
        }
    }
}
