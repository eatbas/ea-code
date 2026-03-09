import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CreateMcpServerPayload, McpServer, UpdateMcpServerPayload } from "../types";

interface UseMcpServersReturn {
  servers: McpServer[];
  capableClis: string[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  setEnabled: (serverId: string, enabled: boolean) => Promise<void>;
  setBindings: (serverId: string, cliNames: string[]) => Promise<void>;
  createServer: (payload: CreateMcpServerPayload) => Promise<void>;
  updateServer: (payload: UpdateMcpServerPayload) => Promise<void>;
  deleteServer: (serverId: string) => Promise<void>;
  setContext7ApiKey: (apiKey: string) => Promise<void>;
}

/** Hook for MCP catalogue CRUD and per-CLI binding management. */
export function useMcpServers(): UseMcpServersReturn {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [capableClis, setCapableClis] = useState<string[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (): Promise<void> => {
    try {
      const [loadedServers, loadedClis] = await Promise.all([
        invoke<McpServer[]>("list_mcp_servers"),
        invoke<string[]>("list_mcp_capable_clis"),
      ]);
      setServers(loadedServers);
      setCapableClis(loadedClis);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const setEnabled = useCallback(async (serverId: string, enabled: boolean): Promise<void> => {
    await invoke("set_mcp_server_enabled", { serverId, enabled });
    await refresh();
  }, [refresh]);

  const setBindings = useCallback(async (serverId: string, cliNames: string[]): Promise<void> => {
    await invoke("set_mcp_server_bindings", { serverId, cliNames });
    await refresh();
  }, [refresh]);

  const createServer = useCallback(async (payload: CreateMcpServerPayload): Promise<void> => {
    await invoke("create_mcp_server", { payload });
    await refresh();
  }, [refresh]);

  const updateServer = useCallback(async (payload: UpdateMcpServerPayload): Promise<void> => {
    await invoke("update_mcp_server", { payload });
    await refresh();
  }, [refresh]);

  const deleteServer = useCallback(async (serverId: string): Promise<void> => {
    await invoke("delete_mcp_server", { serverId });
    await refresh();
  }, [refresh]);

  const setContext7ApiKey = useCallback(async (apiKey: string): Promise<void> => {
    await invoke("set_context7_api_key", { apiKey });
    await refresh();
  }, [refresh]);

  return {
    servers,
    capableClis,
    loading,
    error,
    refresh,
    setEnabled,
    setBindings,
    createServer,
    updateServer,
    deleteServer,
    setContext7ApiKey,
  };
}
