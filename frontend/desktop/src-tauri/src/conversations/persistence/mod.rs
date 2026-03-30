mod crud;
mod io;
mod paths;
mod pipeline_state;
mod recovery;
mod registries;

#[cfg(test)]
mod tests;

// Re-export the full public API so that external callers
// (e.g. `super::persistence::create_conversation`) remain unchanged.

pub use crud::{
    archive_conversation, cleanup_orphaned_conversations, create_conversation,
    delete_conversation, finish_turn, get_conversation, list_conversations, mark_turn_running,
    rename_conversation, set_active_job_id, set_conversation_pinned, set_provider_session_ref,
    set_status, unarchive_conversation,
};

pub use pipeline_state::{
    load_pipeline_state, mark_running_pipeline_stages_stopped, save_pipeline_state,
    update_pipeline_stage,
};

pub use registries::{
    get_pipeline_job_ids, get_pipeline_stage_texts, register_abort_flag,
    register_pipeline_job_slots, register_pipeline_stage_buffers, remove_abort_flag,
    remove_pipeline_job_slots, remove_pipeline_stage_buffers, track_running_conversation,
    trigger_abort,
};

pub type ConversationCleanupStats = registries::ConversationCleanupStats;
pub type RunningConversationGuard = registries::RunningConversationGuard;
