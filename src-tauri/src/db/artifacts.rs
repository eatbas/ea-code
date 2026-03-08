use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::artifacts;

use super::models::{ArtifactRow, NewArtifact};

/// Inserts an artefact record (diff, review, judge output, etc.).
pub fn insert(
    pool: &DbPool,
    run_id: &str,
    iteration: i32,
    kind: &str,
    content: &str,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    diesel::insert_into(artifacts::table)
        .values(&NewArtifact {
            run_id,
            iteration,
            kind,
            content,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert artefact: {e}"))?;

    Ok(())
}

/// Returns all artefacts for a given run, ordered by iteration and creation time.
pub fn get_for_run(pool: &DbPool, run_id: &str) -> Result<Vec<ArtifactRow>, String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    artifacts::table
        .filter(artifacts::run_id.eq(run_id))
        .order((artifacts::iteration.asc(), artifacts::created_at.asc()))
        .load::<ArtifactRow>(&mut conn)
        .map_err(|e| format!("Failed to get artefacts: {e}"))
}
