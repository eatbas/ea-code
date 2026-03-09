import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectSummary,
  SessionSummary,
  SessionDetail,
} from "../types";
import { useToast } from "../components/shared/Toast";

interface UseHistoryReturn {
  projects: ProjectSummary[];
  sessions: SessionSummary[];
  loadProjects: () => Promise<void>;
  loadSessions: (projectPath: string) => Promise<void>;
  loadSessionDetail: (sessionId: string) => Promise<SessionDetail>;
  createSession: (projectPath: string) => Promise<string>;
  deleteSession: (sessionId: string) => Promise<void>;
}

/** Hook for querying project and session history from the database. */
export function useHistory(): UseHistoryReturn {
  const toast = useToast();
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);

  const loadProjects = useCallback(async (): Promise<void> => {
    try {
      const result = await invoke<ProjectSummary[]>("list_projects");
      setProjects(result);
    } catch {
      toast.error("Failed to load projects.");
    }
  }, [toast]);

  const loadSessions = useCallback(async (projectPath: string): Promise<void> => {
    try {
      const result = await invoke<SessionSummary[]>("list_sessions", { projectPath });
      setSessions(result);
    } catch {
      toast.error("Failed to load sessions.");
    }
  }, [toast]);

  const loadSessionDetail = useCallback(async (sessionId: string): Promise<SessionDetail> => {
    return invoke<SessionDetail>("get_session_detail", { sessionId });
  }, []);

  const createSession = useCallback(async (projectPath: string): Promise<string> => {
    return invoke<string>("create_session", { projectPath });
  }, []);

  const deleteSession = useCallback(async (sessionId: string): Promise<void> => {
    await invoke("delete_session", { sessionId });
    setSessions((prev) => prev.filter((s) => s.id !== sessionId));
  }, []);

  return { projects, sessions, loadProjects, loadSessions, loadSessionDetail, createSession, deleteSession };
}
