pub mod artifacts;
pub mod logs;
pub mod mcp;
pub mod models;
pub mod project_settings;
pub mod projects;
pub mod questions;
pub mod run_detail;
pub mod runs;
pub mod sessions;
pub mod settings;
pub mod skills;

use std::path::PathBuf;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

/// Thread-safe connection pool for SQLite.
pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

/// Convenience function to get a connection from the pool.
pub fn get_conn(pool: &DbPool) -> Result<r2d2::PooledConnection<ConnectionManager<SqliteConnection>>, String> {
    pool.get().map_err(|e| format!("Pool error: {e}"))
}

/// Embedded migrations compiled into the binary.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Returns the path to the database file:
/// `<config_dir>/ea-code/ea-code.db`
fn db_path() -> Result<PathBuf, String> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| "Unable to determine config directory".to_string())?;
    Ok(config_dir.join("ea-code").join("ea-code.db"))
}

/// Opens (or creates) the database, runs pending migrations, and returns
/// a connection pool.
pub fn init_db() -> Result<DbPool, String> {
    let path = db_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    let db_url = path.to_string_lossy().to_string();
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);

    let pool = r2d2::Pool::builder()
        .max_size(5)
        .build(manager)
        .map_err(|e| format!("Failed to build connection pool: {e}"))?;

    // Run pending migrations on startup
    let mut conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    // Enable WAL mode and foreign keys
    diesel::sql_query("PRAGMA journal_mode = WAL;")
        .execute(&mut conn)
        .map_err(|e| format!("Failed to set WAL mode: {e}"))?;
    diesel::sql_query("PRAGMA foreign_keys = ON;")
        .execute(&mut conn)
        .map_err(|e| format!("Failed to enable foreign keys: {e}"))?;

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| format!("Migration failed: {e}"))?;

    mcp::sync_builtin_catalog(&pool)?;

    Ok(pool)
}

/// Attempts to import settings from the legacy JSON file into the database.
/// Called once on first launch after migration.
pub fn import_legacy_settings(pool: &DbPool) -> Result<(), String> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| "Unable to determine config directory".to_string())?;
    let json_path = config_dir.join("ea-code").join("settings.json");

    if !json_path.exists() {
        return Ok(());
    }

    let contents = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read settings.json: {e}"))?;

    // Re-use the existing AppSettings model for deserialisation
    let legacy: crate::models::AppSettings = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(_) => return Ok(()), // Malformed file — skip import
    };

    settings::update(pool, &legacy)?;

    // Rename the legacy file so we don't re-import next launch
    let backup_path = config_dir.join("ea-code").join("settings.json.bak");
    let _ = std::fs::rename(&json_path, &backup_path);

    Ok(())
}
