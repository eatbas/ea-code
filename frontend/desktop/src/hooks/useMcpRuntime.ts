import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  refreshRuntimeStatuses: () => Promise<boolean>;
  runFixWithPrompt: (
    request: RunCliMcpFixWithPromptRequest,
  ) => Promise<McpCliFixResult | null>;
}

function toKey(cliName: string, serverId: string): string {
  return `${cliName}:${serverId}`;
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

  const refreshRuntimeStatuses = useCallback(async (): Promise<boolean> => {
    try {
      const rows = await invoke<McpCliRuntimeStatus[]>("get_mcp_cli_runtime_statuses");
      setRuntimeStatuses(rows);
      setRuntimeError(null);
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setRuntimeError(message);
      toast.error("Failed to load MCP runtime status.");
      return false;
    } finally {
      setRuntimeLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    void refreshRuntimeStatuses();
  }, [refreshRuntimeStatuses]);

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
        await refreshRuntimeStatuses();
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
