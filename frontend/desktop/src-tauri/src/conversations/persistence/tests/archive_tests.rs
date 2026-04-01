use super::super::{
    archive_conversation, create_conversation, list_conversations, set_conversation_pinned,
    unarchive_conversation,
};
use crate::models::AgentSelection;

use super::helpers::TestWorkspace;

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
