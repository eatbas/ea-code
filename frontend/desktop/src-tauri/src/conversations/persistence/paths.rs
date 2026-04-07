use std::path::{Path, PathBuf};

pub(super) const CONVERSATIONS_DIR: &str = ".maestro/conversations";
pub(super) const CONVERSATION_FILE: &str = "conversation.json";
pub(super) const MESSAGES_FILE: &str = "messages.jsonl";
pub(super) const PIPELINE_FILE: &str = "pipeline.json";
pub(super) const PIPELINE_DEBUG_FILE: &str = "pipeline-debug.log";
pub(super) const STALE_RUNNING_ERROR: &str = "maestro closed while this task was running";
pub(super) const RECOVERED_SUMMARY_ERROR: &str =
    "Recovered conversation metadata after an incomplete write";

pub(super) fn conversations_dir(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path).join(CONVERSATIONS_DIR)
}

pub(super) fn conversation_dir(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversations_dir(workspace_path).join(conversation_id)
}

pub(super) fn conversation_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(CONVERSATION_FILE)
}

pub(super) fn conversation_backup_file_path(
    workspace_path: &str,
    conversation_id: &str,
) -> PathBuf {
    let path = conversation_file_path(workspace_path, conversation_id);
    PathBuf::from(format!("{}.bak", path.to_string_lossy()))
}

pub(super) fn messages_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(MESSAGES_FILE)
}

pub(super) fn prompt_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id)
        .join("prompt")
        .join("prompt.md")
}

pub(super) fn plan_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("plan")
}

pub(super) fn pipeline_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(PIPELINE_FILE)
}

pub(super) fn pipeline_debug_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(PIPELINE_DEBUG_FILE)
}

#[allow(dead_code)]
pub(super) fn coder_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("coder")
}

#[allow(dead_code)]
pub(super) fn review_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("review")
}

#[allow(dead_code)]
pub(super) fn review_merged_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("review_merged")
}

#[allow(dead_code)]
pub(super) fn code_fixer_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("code_fixer")
}

pub(super) fn images_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("images")
}

pub(super) fn orchestrator_output_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id)
        .join("prompt_enhanced")
        .join("prompt_enhanced_output.json")
}
