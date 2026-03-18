import { useCallback, useEffect, useState } from "react";
import type {
  ActiveView,
  PipelineRequest,
  PipelineRun,
  RunOptions,
  SessionDetail,
  SessionMeta,
  WorkspaceInfo,
} from "../types";
import { isLiveSessionStatus, isRunInProgress, isRunTerminalState } from "../utils/statusHelpers";

interface UseAppViewStateArgs {
  workspace: WorkspaceInfo | null;
  run: PipelineRun | null;
  sessions: SessionMeta[];
  loadProjects: () => Promise<void>;
  loadSessions: (projectPath: string) => Promise<void>;
  loadSessionDetail: (sessionId: string) => Promise<SessionDetail>;
  loadMoreRuns: (sessionId: string, currentCount: number) => Promise<SessionDetail>;
  openWorkspace: (path: string) => Promise<void>;
  startPipeline: (request: PipelineRequest) => Promise<void>;
  resetRun: () => void;
  notifyError: (message: string) => void;
}

interface UseAppViewStateReturn {
  activeView: ActiveView;
  setActiveView: (view: ActiveView) => void;
  activeSessionId: string | undefined;
  sessionDetail: SessionDetail | null;
  sessionDetailLoading: boolean;
  sessionLoadingMore: boolean;
  handleRun: (options: RunOptions, sessionIdOverride?: string) => Promise<void>;
  handleContinueRun: (options: RunOptions, sessionId?: string) => void;
  handleMissingAgentSetup: () => void;
  handleNewSession: () => void;
  handleBackFromChat: () => void;
  handleSelectSession: (sessionId: string) => Promise<void>;
  handleLoadMoreRuns: () => Promise<void>;
  handleSelectProject: (projectPath: string) => Promise<void>;
  handleArchivedSession: (sessionId: string) => void;
}

/** Owns App-level view/session state and associated side effects. */
export function useAppViewState({
  workspace,
  run,
  sessions,
  loadProjects,
  loadSessions,
  loadSessionDetail,
  loadMoreRuns,
  openWorkspace,
  startPipeline,
  resetRun,
  notifyError,
}: UseAppViewStateArgs): UseAppViewStateReturn {
  const [activeView, setActiveView] = useState<ActiveView>("home");
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [sessionDetail, setSessionDetail] = useState<SessionDetail | null>(null);
  const [sessionDetailLoading, setSessionDetailLoading] = useState(false);
  const [sessionLoadingMore, setSessionLoadingMore] = useState(false);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    if (!workspace) return;
    void loadProjects();
    void loadSessions(workspace.path);
    setActiveSessionId(undefined);
    setSessionDetail(null);
  }, [workspace, loadProjects, loadSessions]);

  useEffect(() => {
    if (!workspace || (!isRunInProgress(run) && !isRunTerminalState(run))) return;
    void loadSessions(workspace.path);
  }, [workspace, run?.status, loadSessions]);

  const hasRunningSession = sessions.some((session) => isLiveSessionStatus(session.lastStatus));

  useEffect(() => {
    if (!hasRunningSession || !workspace) return;
    const interval = setInterval(async () => {
      try {
        await loadSessions(workspace.path);
        if (activeSessionId) {
          const detail = await loadSessionDetail(activeSessionId);
          setSessionDetail(detail);
        }
      } catch {
        // Silent; polling retries automatically.
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [hasRunningSession, workspace, activeSessionId, loadSessions, loadSessionDetail]);

  const handleRun = useCallback(async (options: RunOptions, sessionIdOverride?: string): Promise<void> => {
    if (!workspace) return;
    const request: PipelineRequest = {
      prompt: options.prompt,
      workspacePath: workspace.path,
      sessionId: sessionIdOverride ?? activeSessionId,
      directTask: options.directTask || undefined,
      directTaskAgent: options.directTaskAgent,
      directTaskModel: options.directTaskModel,
      noPlan: options.noPlan || undefined,
    };
    await startPipeline(request);
  }, [workspace, activeSessionId, startPipeline]);

  const handleContinueRun = useCallback((options: RunOptions, sessionId?: string): void => {
    if (sessionId) {
      setActiveSessionId(sessionId);
    }
    void handleRun(options, sessionId);
  }, [handleRun]);

  const handleMissingAgentSetup = useCallback((): void => {
    setActiveView("agents");
  }, []);

  const handleNewSession = useCallback((): void => {
    resetRun();
    setActiveSessionId(undefined);
    setSessionDetail(null);
    setActiveView("home");
    if (workspace) {
      void loadSessions(workspace.path);
    }
  }, [resetRun, workspace, loadSessions]);

  /** Navigate away from ChatView without destroying the run state.
   *  Sets activeSessionId so SessionDetailView renders (which polls
   *  for updates from disk) while the backend keeps running and
   *  usePipelineEvents keeps receiving events in the background.
   */
  const handleBackFromChat = useCallback((): void => {
    const sessionId = run?.sessionId;
    if (sessionId) {
      setActiveSessionId(sessionId);
      setActiveView("home");
      setSessionDetailLoading(true);
      loadSessionDetail(sessionId)
        .then((detail) => setSessionDetail(detail))
        .catch(() => setSessionDetail(null))
        .finally(() => setSessionDetailLoading(false));
    } else {
      // No session to navigate to — fall back to full reset.
      resetRun();
      setActiveSessionId(undefined);
      setSessionDetail(null);
      setActiveView("home");
    }
    if (workspace) {
      void loadSessions(workspace.path);
    }
  }, [run, workspace, resetRun, loadSessions, loadSessionDetail]);

  const handleSelectSession = useCallback(async (sessionId: string): Promise<void> => {
    const isCurrentRun = run && (isRunInProgress(run) || isRunTerminalState(run)) && run.sessionId === sessionId;
    if (isCurrentRun) {
      setActiveSessionId(undefined);
      setSessionDetail(null);
      setActiveView("home");
      return;
    }

    setActiveSessionId(sessionId);
    setActiveView("home");
    setSessionDetailLoading(true);
    try {
      const detail = await loadSessionDetail(sessionId);
      setSessionDetail(detail);
    } catch {
      notifyError("Failed to load session.");
      setSessionDetail(null);
    } finally {
      setSessionDetailLoading(false);
    }
  }, [run, loadSessionDetail, notifyError]);

  const handleLoadMoreRuns = useCallback(async (): Promise<void> => {
    if (!activeSessionId || !sessionDetail) return;
    setSessionLoadingMore(true);
    try {
      const older = await loadMoreRuns(activeSessionId, sessionDetail.runs.length);
      setSessionDetail((prev) => {
        if (!prev) return prev;
        return { ...prev, runs: [...older.runs, ...prev.runs], totalRuns: older.totalRuns };
      });
    } catch {
      notifyError("Failed to load earlier runs.");
    } finally {
      setSessionLoadingMore(false);
    }
  }, [activeSessionId, sessionDetail, loadMoreRuns, notifyError]);

  const handleSelectProject = useCallback(async (projectPath: string): Promise<void> => {
    await openWorkspace(projectPath);
    setActiveView("home");
  }, [openWorkspace]);

  const handleArchivedSession = useCallback((sessionId: string): void => {
    if (activeSessionId !== sessionId) return;
    setActiveSessionId(undefined);
    setSessionDetail(null);
  }, [activeSessionId]);

  return {
    activeView,
    setActiveView,
    activeSessionId,
    sessionDetail,
    sessionDetailLoading,
    sessionLoadingMore,
    handleRun,
    handleContinueRun,
    handleMissingAgentSetup,
    handleNewSession,
    handleBackFromChat,
    handleSelectSession,
    handleLoadMoreRuns,
    handleSelectProject,
    handleArchivedSession,
  };
}
