use serde::Serialize;
use tauri::State;

use crate::commands::AppState;
use crate::db::get_conn;

/// All tables in the database, whitelisted for safety.
const ALLOWED_TABLES: &[&str] = &[
    "artifacts",
    "cli_mcp_bindings",
    "iterations",
    "logs",
    "mcp_servers",
    "projects",
    "questions",
    "runs",
    "sessions",
    "settings",
    "skills",
    "stages",
];

/// Tables that must not be truncated (single-row config, etc.).
const PROTECTED_TABLES: &[&str] = &["settings"];

fn validate_table_name(name: &str) -> Result<(), String> {
    if ALLOWED_TABLES.contains(&name) {
        Ok(())
    } else {
        Err(format!("Unknown table: {name}"))
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TableStats {
    pub table_name: String,
    pub row_count: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    pub tables: Vec<TableStats>,
    pub db_size_bytes: u64,
    pub db_path: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_count: i64,
}

/// Returns row counts for every table and the database file size.
#[tauri::command]
pub async fn get_db_stats(state: State<'_, AppState>) -> Result<DbStats, String> {
    use diesel::RunQueryDsl;

    let mut conn = get_conn(&state.db)?;
    let mut tables: Vec<TableStats> = Vec::new();

    for &table in ALLOWED_TABLES {
        let query = format!("SELECT COUNT(*) AS cnt FROM {table}");
        let count: i64 = diesel::sql_query(&query)
            .load::<CountRow>(&mut conn)
            .map_err(|e| format!("Failed to count {table}: {e}"))?
            .first()
            .map(|r| r.cnt)
            .unwrap_or(0);

        tables.push(TableStats {
            table_name: table.to_string(),
            row_count: count,
        });
    }

    let db_path = db_path_string()?;
    let db_size_bytes = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(DbStats {
        tables,
        db_size_bytes,
        db_path,
    })
}

/// Returns paginated rows from a table.
///
/// Uses SQLite's `json_object()` to serialise each row into a single
/// JSON string, which avoids Diesel's compile-time column requirements.
#[tauri::command]
pub async fn get_table_rows(
    state: State<'_, AppState>,
    table_name: String,
    limit: i64,
    offset: i64,
) -> Result<TableData, String> {
    use diesel::RunQueryDsl;

    validate_table_name(&table_name)?;
    let mut conn = get_conn(&state.db)?;

    // Get column names via PRAGMA
    let pragma_query = format!("PRAGMA table_info({table_name})");
    let col_rows: Vec<PragmaTableInfo> = diesel::sql_query(&pragma_query)
        .load(&mut conn)
        .map_err(|e| format!("Failed to get columns for {table_name}: {e}"))?;

    let columns: Vec<String> = col_rows.iter().map(|c| c.name.clone()).collect();

    if columns.is_empty() {
        return Ok(TableData {
            columns: vec![],
            rows: vec![],
            total_count: 0,
        });
    }

    // Get total count
    let count_query = format!("SELECT COUNT(*) AS cnt FROM {table_name}");
    let total_count: i64 = diesel::sql_query(&count_query)
        .load::<CountRow>(&mut conn)
        .map_err(|e| format!("Failed to count {table_name}: {e}"))?
        .first()
        .map(|r| r.cnt)
        .unwrap_or(0);

    // Build json_object() call: json_object('col1', col1, 'col2', col2, ...)
    let json_args = columns
        .iter()
        .map(|c| format!("'{c}', {c}"))
        .collect::<Vec<_>>()
        .join(", ");

    let select_query = format!(
        "SELECT json_object({json_args}) AS json_row FROM {table_name} LIMIT {limit} OFFSET {offset}"
    );

    let json_rows: Vec<JsonRow> = diesel::sql_query(&select_query)
        .load(&mut conn)
        .map_err(|e| format!("Failed to query {table_name}: {e}"))?;

    // Parse each JSON string into an ordered Vec of values
    let rows: Vec<Vec<serde_json::Value>> = json_rows
        .into_iter()
        .filter_map(|jr| {
            let map: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&jr.json_row).ok()?;
            Some(
                columns
                    .iter()
                    .map(|col| map.get(col).cloned().unwrap_or(serde_json::Value::Null))
                    .collect(),
            )
        })
        .collect();

    Ok(TableData {
        columns,
        rows,
        total_count,
    })
}

/// Deletes all rows from a table (protected tables are rejected).
#[tauri::command]
pub async fn truncate_table(
    state: State<'_, AppState>,
    table_name: String,
) -> Result<(), String> {
    use diesel::RunQueryDsl;

    validate_table_name(&table_name)?;

    if PROTECTED_TABLES.contains(&table_name.as_str()) {
        return Err(format!(
            "Table '{table_name}' is protected and cannot be truncated"
        ));
    }

    let mut conn = get_conn(&state.db)?;

    diesel::sql_query("PRAGMA foreign_keys = OFF;")
        .execute(&mut conn)
        .map_err(|e| format!("Failed to disable FK: {e}"))?;

    let delete_query = format!("DELETE FROM {table_name}");
    diesel::sql_query(&delete_query)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to truncate {table_name}: {e}"))?;

    diesel::sql_query("PRAGMA foreign_keys = ON;")
        .execute(&mut conn)
        .map_err(|e| format!("Failed to re-enable FK: {e}"))?;

    Ok(())
}

/// Restarts the Tauri application process.
#[tauri::command]
pub async fn restart_app(app: tauri::AppHandle) -> Result<(), String> {
    app.restart();
}

// ── Helper types for raw SQL queries ────────────────────────────────

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    cnt: i64,
}

#[derive(diesel::QueryableByName)]
#[allow(dead_code)]
struct PragmaTableInfo {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    cid: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(column_name = "type")]
    col_type: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    notnull: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    dflt_value: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pk: i32,
}

/// Single-column row wrapping the JSON-serialised row string.
#[derive(diesel::QueryableByName)]
struct JsonRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    json_row: String,
}

/// Returns the database file path as a string.
fn db_path_string() -> Result<String, String> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| "Unable to determine config directory".to_string())?;
    let path = config_dir.join("ea-code").join("ea-code.db");
    Ok(path.to_string_lossy().to_string())
}
