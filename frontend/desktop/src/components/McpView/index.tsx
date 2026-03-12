import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CliHealth } from "../../types";
import { useMcpServers } from "../../hooks/useMcpServers";
import { useMcpRuntime } from "../../hooks/useMcpRuntime";
import { useToast } from "../shared/Toast";
import { parseEnv } from "./helpers";
import { McpServerCard } from "./McpServerCard";

interface McpViewProps {
  cliHealth: CliHealth | null;
}

/** MCP server catalogue view with compact CLI status indicators. */
export function McpView({ cliHealth }: McpViewProps): ReactNode {
  const toast = useToast();
  const { servers, loading, setEnabled, setContext7ApiKey } = useMcpServers();
  const {
    runtimeStatuses,
    runtimeLoading,
    runtimeError,
    fixingKey,
    refreshRuntimeStatuses,
    runFixWithPrompt,
  } = useMcpRuntime();
  const [refreshing, setRefreshing] = useState<boolean>(false);

  /* --- Context7 API key state --- */
  const [savedKey, setSavedKey] = useState<string>("");
  const [keyDraft, setKeyDraft] = useState<string>("");
  const [editingKey, setEditingKey] = useState<boolean>(false);
  const [keyBusy, setKeyBusy] = useState<boolean>(false);

  const builtinOrder = ["context7", "playwright"];
  const builtinServers = useMemo(
    () =>
      servers
        .filter((server) => server.isBuiltin && builtinOrder.includes(server.id))
        .sort((a, b) => builtinOrder.indexOf(a.id) - builtinOrder.indexOf(b.id)),
    [servers],
  );

  const runtimeByCli = useMemo(
    () => new Map(runtimeStatuses.map((row) => [row.cliName, row])),
    [runtimeStatuses],
  );

  /* Sync saved key from server data */
  useEffect(() => {
    const context7 = builtinServers.find((s) => s.id === "context7");
    if (!context7) {
      setSavedKey("");
      return;
    }
    const key = parseEnv(context7.env).CONTEXT7_API_KEY ?? "";
    setSavedKey(key);
    if (!editingKey) setKeyDraft(key);
  }, [builtinServers, editingKey]);

  const handleRefresh = useCallback(async (): Promise<void> => {
    setRefreshing(true);
    try {
      await invoke("invalidate_cli_cache");
      await refreshRuntimeStatuses();
      toast.success("MCP runtime status refreshed.");
    } finally {
      setRefreshing(false);
    }
  }, [refreshRuntimeStatuses, toast]);

  /* --- Context7 key handlers --- */
  async function saveKey(): Promise<void> {
    setKeyBusy(true);
    try {
      await setContext7ApiKey(keyDraft.trim());
      setSavedKey(keyDraft.trim());
      setEditingKey(false);
    } finally {
      setKeyBusy(false);
    }
  }

  async function deleteKey(): Promise<void> {
    setKeyBusy(true);
    try {
      await setContext7ApiKey("");
      setSavedKey("");
      setKeyDraft("");
      setEditingKey(false);
    } finally {
      setKeyBusy(false);
    }
  }

  const actionsDisabled = loading || runtimeLoading || refreshing;

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-4xl flex-col gap-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-bold text-[#e4e4ed]">MCP Servers</h1>
              <p className="mt-1 text-sm text-[#9898b0]">
                Only curated servers are available: Context7 and Playwright.
              </p>
            </div>
            <button
              type="button"
              onClick={() => void handleRefresh()}
              disabled={actionsDisabled}
              className="rounded-md border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] transition-colors hover:bg-[#2e2e48] hover:text-[#e4e4ed] disabled:cursor-not-allowed disabled:opacity-50"
            >
              {refreshing || runtimeLoading ? "Checking..." : "Refresh"}
            </button>
          </div>

          {loading && <div className="text-sm text-[#9898b0]">Loading MCP servers...</div>}
          {runtimeError && <div className="text-sm text-[#ef4444]">{runtimeError}</div>}

          <div className="grid gap-4 md:grid-cols-2">
            {builtinServers.map((server) => (
              <McpServerCard
                key={server.id}
                server={server}
                runtimeByCli={runtimeByCli}
                cliHealth={cliHealth}
                fixingKey={fixingKey}
                actionsDisabled={actionsDisabled}
                onToggleEnabled={async (s, enabled) => { await setEnabled(s.id, enabled); }}
                onRunFix={runFixWithPrompt}
                savedKey={savedKey}
                editingKey={editingKey}
                keyDraft={keyDraft}
                keyBusy={keyBusy}
                onKeyDraftChange={setKeyDraft}
                onSaveKey={saveKey}
                onDeleteKey={deleteKey}
                onStartEditing={() => { setKeyDraft(savedKey); setEditingKey(true); }}
                onCancelEditing={() => { setKeyDraft(savedKey); setEditingKey(false); }}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
