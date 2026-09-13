//! SQLite-backed PDO driver using `rusqlite`.
//!
//! Provides real database access for `sqlite:` DSNs. The PDO class dispatch
//! in `dispatch_handlers.rs` routes `sqlite:` connections to this module.

use rusqlite::Connection;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Global registry of SQLite connections by ID.
static CONNECTIONS: OnceLock<Mutex<HashMap<usize, SqlitePdo>>> = OnceLock::new();

/// Monotonic connection ID counter.
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn registry() -> &'static Mutex<HashMap<usize, SqlitePdo>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a new connection and return its ID.
pub fn register_connection(pdo: SqlitePdo) -> usize {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let mut reg = registry().lock().unwrap();
    reg.insert(id, pdo);
    id
}

/// Take a connection out of the registry (for exclusive use).
pub fn take_connection(id: usize) -> Option<SqlitePdo> {
    let mut reg = registry().lock().unwrap();
    reg.remove(&id)
}

/// Put a connection back into the registry.
pub fn put_connection(id: usize, pdo: SqlitePdo) {
    let mut reg = registry().lock().unwrap();
    reg.insert(id, pdo);
}

/// A thread-safe SQLite connection wrapper.
pub struct SqlitePdo {
    conn: Mutex<Connection>,
    last_insert_id: i64,
    last_error: Option<String>,
}

impl SqlitePdo {
    /// Create a new SQLite connection from a DSN path.
    /// `path` is the part after `sqlite:` (e.g. `:memory:` or `/path/to/db.sqlite`).
    pub fn new(path: &str) -> Result<Self, String> {
        let conn = if path == ":memory:" || path.is_empty() {
            Connection::open_in_memory()
        } else {
            Connection::open(path)
        }.map_err(|e| e.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
            last_insert_id: 0,
            last_error: None,
        })
    }

    /// Execute a query that returns rows.
    pub fn query(&mut self, sql: &str) -> Result<Vec<Vec<(String, String)>>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or_default().to_string())
            .collect();
        let rows = stmt.query_map([], |row| {
            let mut result = Vec::with_capacity(col_count);
            for (i, name) in col_names.iter().enumerate() {
                let val: String = row.get::<_, Option<String>>(i)
                    .unwrap_or(None)
                    .unwrap_or_default();
                result.push((name.clone(), val));
            }
            Ok(result)
        }).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Execute a statement (INSERT/UPDATE/DELETE/CREATE) and return rows affected.
    pub fn exec(&mut self, sql: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let affected = conn.execute(sql, []).map_err(|e| {
            self.last_error = Some(e.to_string());
            e.to_string()
        })? as i64;
        self.last_insert_id = conn.last_insert_rowid();
        Ok(affected)
    }

    /// Prepare and execute a statement with bound parameters.
    pub fn prepare_execute(&mut self, sql: &str, params: &[String]) -> Result<Vec<Vec<(String, String)>>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or_default().to_string())
            .collect();
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params.iter()
            .map(|p| p as &dyn rusqlite::ToSql)
            .collect();
        let rows = stmt.query_map(rusqlite_params.as_slice(), |row| {
            let mut result = Vec::with_capacity(col_count);
            for (i, name) in col_names.iter().enumerate() {
                let val: String = row.get::<_, Option<String>>(i)
                    .unwrap_or(None)
                    .unwrap_or_default();
                result.push((name.clone(), val));
            }
            Ok(result)
        }).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    /// Execute a prepared statement that doesn't return rows (INSERT/UPDATE/DELETE).
    pub fn prepare_exec(&mut self, sql: &str, params: &[String]) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params.iter()
            .map(|p| p as &dyn rusqlite::ToSql)
            .collect();
        let affected = conn.execute(sql, rusqlite_params.as_slice()).map_err(|e| {
            self.last_error = Some(e.to_string());
            e.to_string()
        })? as i64;
        self.last_insert_id = conn.last_insert_rowid();
        Ok(affected)
    }

    pub fn last_insert_id(&self) -> i64 {
        self.last_insert_id
    }

    pub fn last_error(&self) -> Option<&String> {
        self.last_error.as_ref()
    }

    pub fn begin_transaction(&mut self) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
        Ok(true)
    }

    pub fn commit(&mut self) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
        Ok(true)
    }

    pub fn rollback(&mut self) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("ROLLBACK").map_err(|e| e.to_string())?;
        Ok(true)
    }
}
