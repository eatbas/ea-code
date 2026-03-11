pub mod artifacts;
pub mod cleanup;
pub mod mcp;
pub mod models;
pub mod projects;
pub mod questions;
pub mod run_detail;
pub mod run_status;
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

/// Returns the current UTC timestamp as an RFC 3339 string.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Unwraps a Diesel `optional()` query into a `Result`, converting
/// `None` into a user-friendly "not found" error.
pub fn find_or_not_found<T>(
    result: Result<Option<T>, diesel::result::Error>,
    entity: &str,
) -> Result<T, String> {
    result
        .map_err(|e| format!("Failed to load {entity}: {e}"))?
        .ok_or_else(|| format!("{entity} not found"))
}

/// Maximum number of characters stored in a single TEXT field (stage output,
/// artefact content). Anything longer is truncated before insertion.
pub const MAX_STORED_TEXT: usize = 50_000;

/// Truncates `text` to at most [`MAX_STORED_TEXT`] characters, appending an
/// ellipsis marker when clipped.
pub fn truncate_for_storage(text: &str) -> String {
    if text.len() <= MAX_STORED_TEXT {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(MAX_STORED_TEXT).collect();
    truncated.push_str("\n... [truncated]");
    truncated
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

    // Backfill columns/indexes for users with older databases whose migration
    // history predates the consolidated initial schema.
    ensure_schema_compatibility(&pool)?;

    mcp::sync_builtin_catalog(&pool)?;

    Ok(pool)
}

#[derive(diesel::QueryableByName)]
struct TableInfoRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

fn ensure_schema_compatibility(pool: &DbPool) -> Result<(), String> {
    let mut conn = get_conn(pool)?;

    ensure_column(
        &mut conn,
        "settings",
        "retention_days",
        "ALTER TABLE settings ADD COLUMN retention_days INTEGER NOT NULL DEFAULT 90;",
    )?;
    ensure_column(
        &mut conn,
        "runs",
        "current_stage",
        "ALTER TABLE runs ADD COLUMN current_stage TEXT;",
    )?;
    ensure_column(
        &mut conn,
        "runs",
        "current_iteration",
        "ALTER TABLE runs ADD COLUMN current_iteration INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        &mut conn,
        "runs",
        "current_stage_started_at",
        "ALTER TABLE runs ADD COLUMN current_stage_started_at TEXT;",
    )?;

    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_runs_status_completed ON runs(status, completed_at);")
        .execute(&mut conn)
        .map_err(|e| format!("Failed to ensure idx_runs_status_completed: {e}"))?;

    // Logs are no longer used and can dominate DB size on older installs.
    diesel::sql_query("DROP TABLE IF EXISTS logs;")
        .execute(&mut conn)
        .map_err(|e| format!("Failed to drop legacy logs table: {e}"))?;

    Ok(())
}

fn ensure_column(
    conn: &mut SqliteConnection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), String> {
    let pragma_sql = format!("PRAGMA table_info({table})");
    let columns: Vec<TableInfoRow> = diesel::sql_query(pragma_sql)
        .load(conn)
        .map_err(|e| format!("Failed to inspect {table} columns: {e}"))?;

    if columns.iter().any(|c| c.name == column) {
        return Ok(());
    }

    diesel::sql_query(alter_sql)
        .execute(conn)
        .map_err(|e| format!("Failed to add missing column {table}.{column}: {e}"))?;
    Ok(())
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
