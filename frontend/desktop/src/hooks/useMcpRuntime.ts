import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  McpCliFixResult,
  McpCliRuntimeStatus,
  RunCliMcpFixWithPromptRequest,
} from "../types";
import { useToast } from "../components/shared/Toast";

interface UseMcpRuntimeReturn {
  runtimeStatuses: McpCliRuntimeStatus[];
  runtimeLoading: boolean;
  runtimeError: string | null;
  fixingKey: string | null;
  lastFixResultByKey: Record<string, McpCliFixResult>;
  refreshRuntimeStatuses: () => void;
  runFixWithPrompt: (
    request: RunCliMcpFixWithPromptRequest,
  ) => Promise<McpCliFixResult | null>;
}

function toKey(cliName: string, serverId: string): string {
  return `${cliName}:${serverId}`;
}

/** Merges a single CLI row into the existing list (upsert by cliName). */
function mergeRow(
  prev: McpCliRuntimeStatus[],
  row: McpCliRuntimeStatus,
): McpCliRuntimeStatus[] {
  const idx = prev.findIndex((r) => r.cliName === row.cliName);
  if (idx >= 0) {
    const next = [...prev];
    next[idx] = row;
    return next;
  }
  return [...prev, row];
}

export function useMcpRuntime(): UseMcpRuntimeReturn {
  const toast = useToast();
  const [runtimeStatuses, setRuntimeStatuses] = useState<McpCliRuntimeStatus[]>([]);
  const [runtimeLoading, setRuntimeLoading] = useState<boolean>(true);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [fixingKey, setFixingKey] = useState<string | null>(null);
  const [lastFixResultByKey, setLastFixResultByKey] = useState<
    Record<string, McpCliFixResult>
  >({});

  // ── Event listeners (mount once) ──────────────────────────────────────
  // Per-CLI results stream in as each CLI finishes its `mcp list`.
  // `mcp_runtime_check_complete` fires after the last CLI is done.
  useEffect(() => {
    const unlistenRow = listen<McpCliRuntimeStatus>(
      "mcp_cli_runtime_status",
      (event) => {
        setRuntimeStatuses((prev) => mergeRow(prev, event.payload));
      },
    );
    const unlistenDone = listen<void>(
      "mcp_runtime_check_complete",
      () => {
        setRuntimeLoading(false);
      },
    );
    return () => {
      void unlistenRow.then((fn) => fn());
      void unlistenDone.then((fn) => fn());
    };
  }, []);

  // ── Fire-and-forget refresh ───────────────────────────────────────────
  // The command returns immediately; results arrive via events above.
  const refreshRuntimeStatuses = useCallback((): void => {
    setRuntimeLoading(true);
    setRuntimeError(null);
    invoke("get_mcp_cli_runtime_statuses").catch((err: unknown) => {
      const message = err instanceof Error ? err.message : String(err);
      setRuntimeError(message);
      setRuntimeLoading(false);
      toast.error("Failed to start MCP runtime check.");
    });
  }, [toast]);

  // Trigger first check on mount.
  useEffect(() => {
    refreshRuntimeStatuses();
  }, [refreshRuntimeStatuses]);

  // ── Install / fix (still awaitable — individual CLI, not a batch) ─────
  const runFixWithPrompt = useCallback(
    async (
      request: RunCliMcpFixWithPromptRequest,
    ): Promise<McpCliFixResult | null> => {
      const key = toKey(request.cliName, request.serverId);
      setFixingKey(key);
      try {
        const result = await invoke<McpCliFixResult>(
          "run_cli_mcp_fix_with_prompt",
          { request },
        );
        setLastFixResultByKey((prev) => ({ ...prev, [key]: result }));
        // Re-check all CLI statuses (fire-and-forget, non-blocking).
        refreshRuntimeStatuses();
        if (result.success) {
          toast.success(
            `${request.cliName}: ${request.serverId} install/fix completed.`,
          );
        } else {
          toast.error(
            result.message?.trim() ||
              `${request.cliName}: ${request.serverId} install/fix failed.`,
          );
        }
        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        toast.error(message);
        return null;
      } finally {
        setFixingKey(null);
      }
    },
    [refreshRuntimeStatuses, toast],
  );

  return {
    runtimeStatuses,
    runtimeLoading,
    runtimeError,
    fixingKey,
    lastFixResultByKey,
    refreshRuntimeStatuses,
    runFixWithPrompt,
  };
}
