use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String, // "password" | "key"
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub key_passphrase: Option<String>,
    pub group: String,
    pub color: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostGroup {
    pub name: String,
    pub count: usize,
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create data dir: {}", e))?;
        }
        let conn = Connection::open(&db_path).map_err(|e| format!("DB open failed: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS hosts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                hostname TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 22,
                username TEXT NOT NULL DEFAULT 'root',
                auth_type TEXT NOT NULL DEFAULT 'password',
                password TEXT,
                key_path TEXT,
                key_passphrase TEXT,
                group_name TEXT NOT NULL DEFAULT 'Default',
                color TEXT NOT NULL DEFAULT '#667eea',
                notes TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_hosts_group ON hosts(group_name);"
        ).map_err(|e| format!("DB init failed: {}", e))?;

        Ok(Store { conn: Mutex::new(conn) })
    }

    fn db_path() -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "Cannot find data directory".to_string())?
            .join("sshterm");
        Ok(data_dir.join("hosts.db"))
    }

    pub fn list_hosts(&self) -> Result<Vec<Host>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, name, hostname, port, username, auth_type, password, key_path, key_passphrase, group_name, color, notes, created_at, updated_at FROM hosts ORDER BY group_name, name"
        ).map_err(|e| e.to_string())?;

        let hosts = stmt.query_map([], |row| {
            Ok(Host {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_type: row.get(5)?,
                password: row.get(6)?,
                key_path: row.get(7)?,
                key_passphrase: row.get(8)?,
                group: row.get(9)?,
                color: row.get(10)?,
                notes: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(hosts)
    }

    pub fn get_groups(&self) -> Result<Vec<HostGroup>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT group_name, COUNT(*) as cnt FROM hosts GROUP BY group_name ORDER BY group_name"
        ).map_err(|e| e.to_string())?;

        let groups = stmt.query_map([], |row| {
            Ok(HostGroup {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(groups)
    }

    pub fn add_host(&self, host: &Host) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO hosts (id, name, hostname, port, username, auth_type, password, key_path, key_passphrase, group_name, color, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                host.id, host.name, host.hostname, host.port, host.username,
                host.auth_type, host.password, host.key_path, host.key_passphrase,
                host.group, host.color, host.notes, host.created_at, host.updated_at
            ],
        ).map_err(|e| format!("Failed to add host: {}", e))?;
        Ok(())
    }

    pub fn update_host(&self, host: &Host) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE hosts SET name=?1, hostname=?2, port=?3, username=?4, auth_type=?5, password=?6, key_path=?7, key_passphrase=?8, group_name=?9, color=?10, notes=?11, updated_at=?12 WHERE id=?13",
            params![
                host.name, host.hostname, host.port, host.username,
                host.auth_type, host.password, host.key_path, host.key_passphrase,
                host.group, host.color, host.notes, host.updated_at, host.id
            ],
        ).map_err(|e| format!("Failed to update host: {}", e))?;
        Ok(())
    }

    pub fn delete_host(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM hosts WHERE id=?1", params![id])
            .map_err(|e| format!("Failed to delete host: {}", e))?;
        Ok(())
    }
}
