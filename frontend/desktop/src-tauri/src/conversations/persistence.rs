use std::path::{Path, PathBuf};

use crate::models::{
    AgentSelection, ConversationDetail, ConversationMessage, ConversationMessageRole,
    ConversationStatus, ConversationSummary,
};
use crate::storage::{atomic_write, now_rfc3339, with_conversations_lock};

const CONVERSATIONS_DIR: &str = ".ea-code/conversations";
const CONVERSATION_FILE: &str = "conversation.json";
const MESSAGES_FILE: &str = "messages.jsonl";
const STALE_RUNNING_ERROR: &str = "ea-code closed while this task was running";

fn conversations_dir(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path).join(CONVERSATIONS_DIR)
}

fn conversation_dir(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversations_dir(workspace_path).join(conversation_id)
}

fn conversation_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(CONVERSATION_FILE)
}

fn messages_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(MESSAGES_FILE)
}

fn read_summary_unlocked(workspace_path: &str, conversation_id: &str) -> Result<ConversationSummary, String> {
    let path = conversation_file_path(workspace_path, conversation_id);
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read conversation {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Failed to parse conversation {}: {error}", path.display()))
}

fn write_summary_unlocked(summary: &ConversationSummary) -> Result<(), String> {
    let path = conversation_file_path(&summary.workspace_path, &summary.id);
    let json = serde_json::to_string_pretty(summary)
        .map_err(|error| format!("Failed to serialise conversation {}: {error}", path.display()))?;
    atomic_write(&path, &json)
}

fn read_messages_unlocked(workspace_path: &str, conversation_id: &str) -> Result<Vec<ConversationMessage>, String> {
    let path = messages_file_path(workspace_path, conversation_id);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read messages {}: {error}", path.display()))?;

    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<ConversationMessage>(line)
                .map_err(|error| format!("Failed to parse message entry in {}: {error}", path.display()))
        })
        .collect()
}

fn write_messages_unlocked(
    workspace_path: &str,
    conversation_id: &str,
    messages: &[ConversationMessage],
) -> Result<(), String> {
    let path = messages_file_path(workspace_path, conversation_id);
    let mut contents = String::new();
    for message in messages {
        let line = serde_json::to_string(message)
            .map_err(|error| format!("Failed to serialise message for {}: {error}", path.display()))?;
        contents.push_str(&line);
        contents.push('\n');
    }
    atomic_write(&path, &contents)
}

fn normalise_title(prompt: &str) -> String {
    let trimmed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return "New conversation".to_string();
    }

    let mut title = String::new();
    let mut count = 0usize;
    for ch in trimmed.chars() {
        if count >= 48 {
            break;
        }
        title.push(ch);
        count += 1;
    }
    if trimmed.chars().count() > 48 {
        title.push_str("...");
    }
    title
}

fn reconcile_stale_running_unlocked(summary: &mut ConversationSummary) -> Result<(), String> {
    if summary.status != ConversationStatus::Running {
        return Ok(());
    }

    summary.status = ConversationStatus::Failed;
    summary.active_job_id = None;
    summary.error = Some(STALE_RUNNING_ERROR.to_string());
    summary.updated_at = now_rfc3339();
    write_summary_unlocked(summary)
}

fn build_detail_unlocked(summary: ConversationSummary) -> Result<ConversationDetail, String> {
    let messages = read_messages_unlocked(&summary.workspace_path, &summary.id)?;
    Ok(ConversationDetail { summary, messages })
}

pub fn create_conversation(
    workspace_path: &str,
    agent: AgentSelection,
    initial_prompt: Option<&str>,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let now = now_rfc3339();
        let summary = ConversationSummary {
            id: uuid::Uuid::new_v4().to_string(),
            title: initial_prompt.map(normalise_title).unwrap_or_else(|| "New conversation".to_string()),
            workspace_path: workspace_path.to_string(),
            agent,
            status: ConversationStatus::Idle,
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
            last_provider_session_ref: None,
            active_job_id: None,
            error: None,
        };
        write_summary_unlocked(&summary)?;
        write_messages_unlocked(workspace_path, &summary.id, &[])?;
        build_detail_unlocked(summary)
    })
}

pub fn list_conversations(workspace_path: &str) -> Result<Vec<ConversationSummary>, String> {
    with_conversations_lock(|| {
        let dir = conversations_dir(workspace_path);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries: Vec<ConversationSummary> = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .map_err(|error| format!("Failed to read conversations directory {}: {error}", dir.display()))?
        {
            let entry = entry.map_err(|error| format!("Failed to read conversation entry: {error}"))?;
            if !entry.path().is_dir() {
                continue;
            }

            let conversation_id = match entry.file_name().to_str() {
                Some(value) => value.to_string(),
                None => continue,
            };
            let mut summary = read_summary_unlocked(workspace_path, &conversation_id)?;
            reconcile_stale_running_unlocked(&mut summary)?;
            summaries.push(summary);
        }

        summaries.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(summaries)
    })
}

pub fn get_conversation(workspace_path: &str, conversation_id: &str) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        reconcile_stale_running_unlocked(&mut summary)?;
        build_detail_unlocked(summary)
    })
}

pub fn mark_turn_running(workspace_path: &str, conversation_id: &str, prompt: &str) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        reconcile_stale_running_unlocked(&mut summary)?;
        if summary.status == ConversationStatus::Running {
            return Err("This conversation is already running".to_string());
        }

        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;
        let user_message = ConversationMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: ConversationMessageRole::User,
            content: prompt.to_string(),
            created_at: now_rfc3339(),
        };
        messages.push(user_message);
        summary.message_count = messages.len();
        if summary.message_count == 1 {
            summary.title = normalise_title(prompt);
        }
        summary.status = ConversationStatus::Running;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        summary.error = None;

        write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        write_summary_unlocked(&summary)?;

        Ok(ConversationDetail { summary, messages })
    })
}

pub fn set_active_job_id(
    workspace_path: &str,
    conversation_id: &str,
    job_id: Option<String>,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.active_job_id = job_id;
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn set_provider_session_ref(
    workspace_path: &str,
    conversation_id: &str,
    provider_session_ref: String,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.last_provider_session_ref = Some(provider_session_ref);
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn finish_turn(
    workspace_path: &str,
    conversation_id: &str,
    status: ConversationStatus,
    assistant_text: Option<String>,
    provider_session_ref: Option<String>,
    error: Option<String>,
) -> Result<(ConversationSummary, Option<ConversationMessage>), String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;

        let assistant_message = assistant_text
            .filter(|text| !text.trim().is_empty())
            .map(|content| ConversationMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: ConversationMessageRole::Assistant,
                content,
                created_at: now_rfc3339(),
            });

        if let Some(message) = &assistant_message {
            messages.push(message.clone());
            write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        }

        summary.message_count = messages.len();
        summary.status = status;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        if provider_session_ref.is_some() {
            summary.last_provider_session_ref = provider_session_ref;
        }
        summary.error = error;
        write_summary_unlocked(&summary)?;

        Ok((summary, assistant_message))
    })
}

pub fn delete_conversation(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    with_conversations_lock(|| {
        let summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.status == ConversationStatus::Running {
            return Err("Cannot delete a running conversation".to_string());
        }

        let dir = conversation_dir(workspace_path, conversation_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|error| format!("Failed to delete conversation {}: {error}", dir.display()))?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        create_conversation, delete_conversation, finish_turn, get_conversation, list_conversations,
        mark_turn_running,
    };
    use crate::models::{AgentSelection, ConversationStatus};

    struct TestWorkspace {
        path: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("ea-code-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("temporary workspace should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn create_and_list_conversations() {
        let workspace = TestWorkspace::new();
        let first = create_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Investigate the build failure"),
        )
        .expect("conversation should be created");

        let listed = list_conversations(workspace.path().to_str().expect("workspace path should be utf-8"))
            .expect("conversations should list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, first.summary.id);
        assert_eq!(listed[0].title, "Investigate the build failure");
    }

    #[test]
    fn turn_start_and_finish_persist_messages() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "claude".to_string(),
                model: "sonnet".to_string(),
            },
            None,
        )
        .expect("conversation should be created");

        let running = mark_turn_running(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            &conversation.summary.id,
            "Explain the app structure",
        )
        .expect("turn should start");
        assert_eq!(running.summary.status, ConversationStatus::Running);
        assert_eq!(running.messages.len(), 1);

        finish_turn(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            &conversation.summary.id,
            ConversationStatus::Completed,
            Some("The app has a Tauri backend and React frontend.".to_string()),
            Some("session-123".to_string()),
            None,
        )
        .expect("turn should finish");

        let loaded = get_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should load");

        assert_eq!(loaded.summary.status, ConversationStatus::Completed);
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.summary.last_provider_session_ref.as_deref(), Some("session-123"));
    }

    #[test]
    fn stale_running_conversations_reconcile_on_load() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            None,
        )
        .expect("conversation should be created");

        mark_turn_running(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            &conversation.summary.id,
            "Continue the last task",
        )
        .expect("turn should start");

        let loaded = get_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should load");

        assert_eq!(loaded.summary.status, ConversationStatus::Failed);
        assert_eq!(
            loaded.summary.error.as_deref(),
            Some("ea-code closed while this task was running")
        );
    }

    #[test]
    fn deletes_non_running_conversation() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Delete me"),
        )
        .expect("conversation should be created");

        delete_conversation(
            workspace.path().to_str().expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should delete");

        let listed = list_conversations(workspace.path().to_str().expect("workspace path should be utf-8"))
            .expect("conversations should list");
        assert!(listed.is_empty());
    }
}
