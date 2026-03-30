use std::path::{Path, PathBuf};

use super::{
    archive_conversation, create_conversation, delete_conversation, finish_turn,
    get_conversation, list_conversations, mark_turn_running, rename_conversation,
    set_conversation_pinned, track_running_conversation, unarchive_conversation,
};
use crate::models::{AgentSelection, ConversationStatus};

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("maestro-test-{}", uuid::Uuid::new_v4()));
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
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Investigate the build failure"),
    )
    .expect("conversation should be created");

    let listed = list_conversations(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        false,
    )
    .expect("conversations should list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, first.summary.id);
    assert_eq!(listed[0].title, "Investigate the build failure");
}

#[test]
fn turn_start_and_finish_persist_messages() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "claude".to_string(),
            model: "sonnet".to_string(),
        },
        None,
    )
    .expect("conversation should be created");

    let running = mark_turn_running(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
        "Explain the app structure",
    )
    .expect("turn should start");
    assert_eq!(running.summary.status, ConversationStatus::Running);
    assert_eq!(running.messages.len(), 1);

    finish_turn(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
        ConversationStatus::Completed,
        Some("The app has a Tauri backend and React frontend.".to_string()),
        Some("session-123".to_string()),
        None,
    )
    .expect("turn should finish");

    let loaded = get_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should load");

    assert_eq!(loaded.summary.status, ConversationStatus::Completed);
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(
        loaded.summary.last_provider_session_ref.as_deref(),
        Some("session-123")
    );
}

#[test]
fn stale_running_conversations_reconcile_on_load() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        None,
    )
    .expect("conversation should be created");

    mark_turn_running(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
        "Continue the last task",
    )
    .expect("turn should start");

    let loaded = get_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should load");

    assert_eq!(loaded.summary.status, ConversationStatus::Failed);
    assert_eq!(
        loaded.summary.error.as_deref(),
        Some("maestro closed while this task was running")
    );
}

#[test]
fn tracked_running_conversations_stay_running_on_load() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        None,
    )
    .expect("conversation should be created");

    mark_turn_running(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
        "Keep running in the background",
    )
    .expect("turn should start");

    let _guard = track_running_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should be tracked");

    let loaded = get_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should load");

    assert_eq!(loaded.summary.status, ConversationStatus::Running);
    assert_eq!(loaded.summary.error, None);
}

#[test]
fn deletes_non_running_conversation() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Delete me"),
    )
    .expect("conversation should be created");

    delete_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should delete");

    let listed = list_conversations(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        false,
    )
    .expect("conversations should list");
    assert!(listed.is_empty());
}

#[test]
fn renames_conversation() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Original title"),
    )
    .expect("conversation should be created");

    let renamed = rename_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
        "Renamed conversation",
    )
    .expect("conversation should rename");

    assert_eq!(renamed.title, "Renamed conversation");
}

#[test]
fn archives_conversation_and_hides_it_from_listing() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Archive me"),
    )
    .expect("conversation should be created");

    let archived = archive_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should archive");

    assert!(archived.archived_at.is_some());

    let listed = list_conversations(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        false,
    )
    .expect("conversations should list");
    assert!(listed.is_empty());
}

#[test]
fn includes_archived_conversations_when_requested() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Archive but keep visible"),
    )
    .expect("conversation should be created");

    let archived = archive_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should archive");

    let listed = list_conversations(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        true,
    )
    .expect("conversations should list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, archived.id);
    assert!(listed[0].archived_at.is_some());
}

#[test]
fn unarchives_conversation_and_returns_it_to_default_listing() {
    let workspace = TestWorkspace::new();
    let conversation = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Bring me back"),
    )
    .expect("conversation should be created");

    archive_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should archive");

    let unarchived = unarchive_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &conversation.summary.id,
    )
    .expect("conversation should unarchive");

    assert!(unarchived.archived_at.is_none());

    let listed = list_conversations(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        false,
    )
    .expect("conversations should list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, conversation.summary.id);
}

#[test]
fn pins_conversation_and_lists_it_first() {
    let workspace = TestWorkspace::new();
    let first = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("First conversation"),
    )
    .expect("first conversation should be created");

    let second = create_conversation(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        AgentSelection {
            provider: "codex".to_string(),
            model: "gpt-5.4".to_string(),
        },
        Some("Second conversation"),
    )
    .expect("second conversation should be created");

    let pinned = set_conversation_pinned(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        &first.summary.id,
        true,
    )
    .expect("conversation should pin");

    assert!(pinned.pinned_at.is_some());

    let listed = list_conversations(
        workspace
            .path()
            .to_str()
            .expect("workspace path should be utf-8"),
        false,
    )
    .expect("conversations should list");

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, first.summary.id);
    assert_eq!(listed[1].id, second.summary.id);
}
