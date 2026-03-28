import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AgentSelection,
  AppSettings,
  ConversationDetail,
  ConversationSummary,
  PrerequisiteStatus,
  ProjectEntry,
  WorkspaceInfo,
} from "../types";

function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export function selectProjectFolder(): Promise<string | null> {
  return open({ directory: true, multiple: false }).then((selected) => (
    typeof selected === "string" ? selected : null
  ));
}

export function selectWorkspace(path: string): Promise<WorkspaceInfo> {
  return invokeCommand<WorkspaceInfo>("select_workspace", { path });
}

export function listProjects(): Promise<ProjectEntry[]> {
  return invokeCommand<ProjectEntry[]>("list_projects");
}

export function deleteProject(projectPath: string): Promise<void> {
  return invokeCommand<void>("delete_project", { projectPath });
}

export function renameProject(projectPath: string, name: string): Promise<ProjectEntry> {
  return invokeCommand<ProjectEntry>("rename_project", { projectPath, name });
}

export function archiveProject(projectPath: string): Promise<ProjectEntry> {
  return invokeCommand<ProjectEntry>("archive_project", { projectPath });
}

export function openInVsCode(path: string): Promise<void> {
  return invokeCommand<void>("open_in_vscode", { path });
}

export function openProjectFolder(path: string): Promise<void> {
  return invokeCommand<void>("open_project_folder", { path });
}

export function getSettings(): Promise<AppSettings> {
  return invokeCommand<AppSettings>("get_settings");
}

export function saveSettings(newSettings: AppSettings): Promise<void> {
  return invokeCommand<void>("save_settings", { newSettings });
}

export function checkPrerequisites(): Promise<PrerequisiteStatus> {
  return invokeCommand<PrerequisiteStatus>("check_prerequisites");
}

export function invalidateCliCache(): Promise<void> {
  return invokeCommand<void>("invalidate_cli_cache");
}

export function checkCliHealth(settings: AppSettings): Promise<void> {
  return invokeCommand<void>("check_cli_health", { settings });
}

export function checkApiHealth(): Promise<void> {
  return invokeCommand<void>("check_api_health");
}

export function getApiProviders(): Promise<void> {
  return invokeCommand<void>("get_api_providers");
}

export function refreshApiProviders(): Promise<void> {
  return Promise.all([checkApiHealth(), getApiProviders()]).then(() => undefined);
}

export function getApiCliVersions(): Promise<void> {
  return invokeCommand<void>("get_api_cli_versions");
}

export function updateApiCli(provider: string): Promise<void> {
  return invokeCommand<void>("update_api_cli", { provider });
}

export function listWorkspaceConversations(
  workspacePath: string,
  includeArchived = false,
): Promise<ConversationSummary[]> {
  return invokeCommand<ConversationSummary[]>("list_workspace_conversations", {
    workspacePath,
    includeArchived,
  });
}

export function createConversation(
  workspacePath: string,
  agent: AgentSelection,
  initialPrompt?: string,
): Promise<ConversationDetail> {
  return invokeCommand<ConversationDetail>("create_conversation", {
    workspacePath,
    agent,
    initialPrompt,
  });
}

export function getConversation(workspacePath: string, conversationId: string): Promise<ConversationDetail> {
  return invokeCommand<ConversationDetail>("get_conversation", { workspacePath, conversationId });
}

export function sendConversationTurn(
  workspacePath: string,
  conversationId: string,
  prompt: string,
): Promise<ConversationDetail> {
  return invokeCommand<ConversationDetail>("send_conversation_turn", {
    workspacePath,
    conversationId,
    prompt,
  });
}

export function stopConversation(workspacePath: string, conversationId: string): Promise<ConversationSummary> {
  return invokeCommand<ConversationSummary>("stop_conversation", { workspacePath, conversationId });
}

export function deleteConversation(workspacePath: string, conversationId: string): Promise<void> {
  return invokeCommand<void>("delete_conversation", { workspacePath, conversationId });
}

export function renameConversation(
  workspacePath: string,
  conversationId: string,
  title: string,
): Promise<ConversationSummary> {
  return invokeCommand<ConversationSummary>("rename_conversation", {
    workspacePath,
    conversationId,
    title,
  });
}

export function archiveConversation(
  workspacePath: string,
  conversationId: string,
): Promise<ConversationSummary> {
  return invokeCommand<ConversationSummary>("archive_conversation", {
    workspacePath,
    conversationId,
  });
}

export function unarchiveConversation(
  workspacePath: string,
  conversationId: string,
): Promise<ConversationSummary> {
  return invokeCommand<ConversationSummary>("unarchive_conversation", {
    workspacePath,
    conversationId,
  });
}

export function setConversationPinned(
  workspacePath: string,
  conversationId: string,
  pinned: boolean,
): Promise<ConversationSummary> {
  return invokeCommand<ConversationSummary>("set_conversation_pinned", {
    workspacePath,
    conversationId,
    pinned,
  });
}
