import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AppSettings,
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
