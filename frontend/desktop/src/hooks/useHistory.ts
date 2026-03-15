import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectEntry,
  SessionMeta,
  SessionDetail,
} from "../types";
import { useToast } from "../components/shared/Toast";

interface LoadSessionDetailOptions {
  limit?: number;
  offset?: number;
}

interface UseHistoryReturn {
  projects: ProjectEntry[];
  sessions: SessionMeta[];
  loadProjects: () => Promise<void>;
  loadSessions: (projectPath: string) => Promise<void>;
  loadSessionDetail: (sessionId: string, options?: LoadSessionDetailOptions) => Promise<SessionDetail>;
  loadMoreRuns: (sessionId: string, currentCount: number) => Promise<SessionDetail>;
  createSession: (projectPath: string) => Promise<string>;
  deleteSession: (sessionId: string) => Promise<void>;
}

/** Hook for querying project and session history from file-based storage. */
export function useHistory(): UseHistoryReturn {
  const toast = useToast();
  const [projects, setProjects] = useState<ProjectEntry[]>([]);
  const [sessions, setSessions] = useState<SessionMeta[]>([]);

  const loadProjects = useCallback(async (): Promise<void> => {
    try {
      const result = await invoke<ProjectEntry[]>("list_projects");
      setProjects(result);
    } catch {
      toast.error("Failed to load projects.");
    }
  }, [toast]);

  const loadSessions = useCallback(async (projectPath: string): Promise<void> => {
    try {
      const result = await invoke<SessionMeta[]>("list_sessions", { projectPath });
      setSessions(result);
    } catch {
      toast.error("Failed to load sessions.");
    }
  }, [toast]);

  const loadSessionDetail = useCallback(async (sessionId: string, options?: LoadSessionDetailOptions): Promise<SessionDetail> => {
    return invoke<SessionDetail>("get_session_detail", {
      sessionId,
      limit: options?.limit ?? 20,
      offset: options?.offset ?? 0,
    });
  }, []);

  /** Loads earlier runs and prepends them to the current session detail. */
  const loadMoreRuns = useCallback(async (sessionId: string, currentCount: number): Promise<SessionDetail> => {
    return invoke<SessionDetail>("get_session_detail", {
      sessionId,
      limit: 20,
      offset: currentCount,
    });
  }, []);

  const createSession = useCallback(async (projectPath: string): Promise<string> => {
    return invoke<string>("create_session", { projectPath });
  }, []);

  const deleteSession = useCallback(async (sessionId: string): Promise<void> => {
    await invoke("delete_session", { sessionId });
    setSessions((prev) => prev.filter((s) => s.id !== sessionId));
  }, []);

  return { projects, sessions, loadProjects, loadSessions, loadSessionDetail, loadMoreRuns, createSession, deleteSession };
}
