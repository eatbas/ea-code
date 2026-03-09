import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CreateMcpServerPayload, McpServer, UpdateMcpServerPayload } from "../types";
import { useToast } from "../components/shared/Toast";

interface UseMcpServersReturn {
  servers: McpServer[];
  capableClis: string[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<boolean>;
  setEnabled: (serverId: string, enabled: boolean) => Promise<void>;
  setBindings: (serverId: string, cliNames: string[]) => Promise<void>;
  createServer: (payload: CreateMcpServerPayload) => Promise<void>;
  updateServer: (payload: UpdateMcpServerPayload) => Promise<void>;
  deleteServer: (serverId: string) => Promise<void>;
  setContext7ApiKey: (apiKey: string) => Promise<void>;
}

/** Hook for MCP catalogue CRUD and per-CLI binding management. */
export function useMcpServers(): UseMcpServersReturn {
  const toast = useToast();
  const [servers, setServers] = useState<McpServer[]>([]);
  const [capableClis, setCapableClis] = useState<string[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (): Promise<boolean> => {
    try {
      const [loadedServers, loadedClis] = await Promise.all([
        invoke<McpServer[]>("list_mcp_servers"),
        invoke<string[]>("list_mcp_capable_clis"),
      ]);
      setServers(loadedServers);
      setCapableClis(loadedClis);
      setError(null);
      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      toast.error("Failed to load MCP servers.");
      return false;
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const setEnabled = useCallback(async (serverId: string, enabled: boolean): Promise<void> => {
    try {
      await invoke("set_mcp_server_enabled", { serverId, enabled });
      const refreshed = await refresh();
      if (!refreshed) {
        return;
      }
      toast.success(enabled ? "Server enabled." : "Server disabled.");
    } catch {
      toast.error("Failed to update server status.");
    }
  }, [refresh, toast]);

  const setBindings = useCallback(async (serverId: string, cliNames: string[]): Promise<void> => {
    try {
      await invoke("set_mcp_server_bindings", { serverId, cliNames });
      const refreshed = await refresh();
      if (!refreshed) {
        return;
      }
      toast.success("CLI bindings updated.");
    } catch {
      toast.error("Failed to update CLI bindings.");
    }
  }, [refresh, toast]);

  const createServer = useCallback(async (payload: CreateMcpServerPayload): Promise<void> => {
    try {
      await invoke("create_mcp_server", { payload });
      const refreshed = await refresh();
      if (!refreshed) {
        return;
      }
      toast.success("MCP server created.");
    } catch {
      toast.error("Failed to create MCP server.");
    }
  }, [refresh, toast]);

  const updateServer = useCallback(async (payload: UpdateMcpServerPayload): Promise<void> => {
    try {
      await invoke("update_mcp_server", { payload });
      const refreshed = await refresh();
      if (!refreshed) {
        return;
      }
      toast.success("MCP server updated.");
    } catch {
      toast.error("Failed to update MCP server.");
    }
  }, [refresh, toast]);

  const deleteServer = useCallback(async (serverId: string): Promise<void> => {
    try {
      await invoke("delete_mcp_server", { serverId });
      const refreshed = await refresh();
      if (!refreshed) {
        return;
      }
      toast.success("MCP server deleted.");
    } catch {
      toast.error("Failed to delete MCP server.");
    }
  }, [refresh, toast]);

  const setContext7ApiKey = useCallback(async (apiKey: string): Promise<void> => {
    try {
      await invoke("set_context7_api_key", { apiKey });
      const refreshed = await refresh();
      if (!refreshed) {
        return;
      }
      toast.success("API key saved.");
    } catch {
      toast.error("Failed to save API key.");
    }
  }, [refresh, toast]);

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
