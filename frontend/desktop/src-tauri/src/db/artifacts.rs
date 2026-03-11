use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::artifacts;

use super::models::{ArtifactRow, NewArtifact};

/// Higher cap for artefact kinds that may legitimately be large (executive
/// summaries, selected skills, etc.). Standard artefacts use the global
/// [`super::MAX_STORED_TEXT`] (50 K) cap.
const MAX_LARGE_ARTIFACT_TEXT: usize = 200_000;

/// Artefact kinds allowed to use the higher size cap.
const LARGE_ARTIFACT_KINDS: &[&str] = &["executive_summary"];

/// Inserts an artefact record. Content is capped at 200 K characters for
/// large kinds, or 50 K for all others.
pub fn insert(
    pool: &DbPool,
    run_id: &str,
    iteration: i32,
    kind: &str,
    content: &str,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    let stored = if LARGE_ARTIFACT_KINDS.contains(&kind) {
        truncate_to(content, MAX_LARGE_ARTIFACT_TEXT)
    } else {
        super::truncate_for_storage(content)
    };

    diesel::insert_into(artifacts::table)
        .values(&NewArtifact {
            run_id,
            iteration,
            kind,
            content: &stored,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert artefact: {e}"))?;

    Ok(())
}

/// Truncates `text` to at most `max` characters, appending an ellipsis marker
/// when clipped.
fn truncate_to(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(max).collect();
    truncated.push_str("\n... [truncated]");
    truncated
}

/// Returns all artefacts for a given run, ordered by iteration and creation time.
pub fn get_for_run(pool: &DbPool, run_id: &str) -> Result<Vec<ArtifactRow>, String> {
    let mut conn = super::get_conn(pool)?;

    artifacts::table
        .filter(artifacts::run_id.eq(run_id))
        .order((artifacts::iteration.asc(), artifacts::created_at.asc()))
        .load::<ArtifactRow>(&mut conn)
        .map_err(|e| format!("Failed to get artefacts: {e}"))
}
