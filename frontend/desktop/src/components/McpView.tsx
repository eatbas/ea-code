import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CliHealth, McpServer, McpRuntimeStatus } from "../types";
import { useMcpServers } from "../hooks/useMcpServers";
import { useMcpRuntime } from "../hooks/useMcpRuntime";
import { useToast } from "./shared/Toast";

const AI_CLI_ORDER = ["claude", "codex", "gemini", "kimi", "opencode"] as const;

function statusChipClass(status: McpRuntimeStatus): string {
  switch (status) {
    case "enabled":
      return "bg-[#22c55e]/15 text-[#22c55e]";
    case "disabled":
      return "bg-[#f59e0b]/15 text-[#f59e0b]";
    case "unknown":
      return "bg-[#64748b]/15 text-[#94a3b8]";
    case "notInstalled":
      return "bg-[#ef4444]/15 text-[#ef4444]";
    case "error":
      return "bg-[#ef4444]/15 text-[#ef4444]";
    default:
      return "bg-[#64748b]/15 text-[#94a3b8]";
  }
}

function statusLabel(status: McpRuntimeStatus): string {
  switch (status) {
    case "enabled":
      return "Enabled";
    case "disabled":
      return "Disabled";
    case "unknown":
      return "Unknown";
    case "notInstalled":
      return "Not Installed";
    case "error":
      return "Error";
    default:
      return "Unknown";
  }
}

function parseEnv(raw: string): Record<string, string> {
  try {
    return JSON.parse(raw) as Record<string, string>;
  } catch {
    return {};
  }
}

interface McpViewProps {
  cliHealth: CliHealth | null;
}

/** MCP server catalogue and per-CLI binding management view. */
export function McpView({ cliHealth }: McpViewProps): ReactNode {
  const toast = useToast();
  const { servers, loading, setEnabled, setBindings, setContext7ApiKey } = useMcpServers();
  const {
    runtimeStatuses,
    runtimeLoading,
    runtimeError,
    fixingKey,
    lastFixResultByKey,
    refreshRuntimeStatuses,
    runFixWithPrompt,
  } = useMcpRuntime();
  const [busy, setBusy] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState<boolean>(false);
  const [context7ApiKey, setContext7ApiKeyValue] = useState<string>("");

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

  /** Derive CLI availability from the app-level health check (no extra backend call). */
  function isCliInstalled(cliName: string): boolean {
    const runtime = runtimeByCli.get(cliName);
    if (runtime !== undefined) return runtime.cliInstalled;
    if (!cliHealth) return false;
    const entry = cliHealth[cliName as keyof CliHealth];
    return entry?.available ?? false;
  }

  useEffect(() => {
    const context7 = builtinServers.find((server) => server.id === "context7");
    if (!context7) {
      setContext7ApiKeyValue("");
      return;
    }
    const env = parseEnv(context7.env);
    setContext7ApiKeyValue(env.CONTEXT7_API_KEY ?? "");
  }, [builtinServers]);

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

  async function toggleEnabled(server: McpServer, enabled: boolean): Promise<void> {
    setBusy(server.id);
    try {
      await setEnabled(server.id, enabled);
    } finally {
      setBusy(null);
    }
  }

  async function toggleBinding(server: McpServer, cliName: string): Promise<void> {
    const current = new Set(server.cliBindings);
    if (current.has(cliName)) current.delete(cliName); else current.add(cliName);
    setBusy(server.id);
    try {
      await setBindings(server.id, Array.from(current));
    } finally {
      setBusy(null);
    }
  }

  async function saveContext7Key(): Promise<void> {
    setBusy("context7-key");
    try {
      await setContext7ApiKey(context7ApiKey);
    } finally {
      setBusy(null);
    }
  }

  async function runFix(serverId: string, cliName: string): Promise<void> {
    await runFixWithPrompt({ cliName, serverId });
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
            {builtinServers.map((server) => {
              const isContext7 = server.id === "context7";
              return (
                <div key={server.id} className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4">
                  <div className="flex items-center justify-between gap-2">
                    <div>
                      <h3 className="text-sm font-semibold text-[#e4e4ed]">{server.name}</h3>
                      <p className="text-xs text-[#9898b0]">{server.description || "No description"}</p>
                    </div>
                    {server.isBuiltin && <span className="rounded bg-[#24243a] px-2 py-1 text-[10px] text-[#9898b0]">Built-in</span>}
                  </div>

                  <label className="mt-3 inline-flex items-center gap-2 text-xs text-[#9898b0]">
                    <input
                      type="checkbox"
                      checked={server.isEnabled}
                      onChange={(e) => void toggleEnabled(server, e.target.checked)}
                    />
                    Enabled
                  </label>

                  <div className="mt-3 flex flex-col gap-2">
                    {AI_CLI_ORDER.map((cliName) => {
                      const runtime = runtimeByCli.get(cliName);
                      const status =
                        runtime?.serverStatuses.find((row) => row.serverId === server.id)?.status ?? "unknown";
                      const installed = isCliInstalled(cliName);
                      const bound = server.cliBindings.includes(cliName);
                      const actionKey = `${cliName}:${server.id}`;
                      const fixResult = lastFixResultByKey[actionKey];

                      return (
                        <div key={`${server.id}-${cliName}`} className="rounded border border-[#2e2e48] bg-[#0f0f14] p-2">
                          <div className="flex items-center justify-between gap-2">
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-medium text-[#e4e4ed]">{cliName}</span>
                              <span className={`rounded px-2 py-0.5 text-[10px] font-medium ${statusChipClass(status)}`}>
                                {statusLabel(status)}
                              </span>
                              <button
                                onClick={() => void toggleBinding(server, cliName)}
                                className={`rounded px-2 py-0.5 text-[10px] ${bound ? "bg-[#6366f1]/20 text-[#e4e4ed]" : "bg-[#24243a] text-[#9898b0]"}`}
                              >
                                {bound ? "Bound" : "Unbound"}
                              </button>
                            </div>
                            <button
                              type="button"
                              onClick={() => void runFix(server.id, cliName)}
                              disabled={!installed || fixingKey === actionKey}
                              className="rounded bg-[#e4e4ed] px-2 py-1 text-[10px] font-medium text-[#0f0f14] disabled:cursor-not-allowed disabled:opacity-50"
                            >
                              {fixingKey === actionKey ? "Running..." : "Install/Fix"}
                            </button>
                          </div>
                          {fixResult && (
                            <p className="mt-1 truncate text-[10px] text-[#9898b0]" title={fixResult.outputSummary}>
                              {fixResult.outputSummary}
                            </p>
                          )}
                        </div>
                      );
                    })}
                  </div>

                  {isContext7 && (
                    <div className="mt-3 flex flex-col gap-2">
                      <label className="text-xs text-[#9898b0]">Context7 API Key</label>
                      <input
                        type="password"
                        value={context7ApiKey}
                        onChange={(e) => setContext7ApiKeyValue(e.target.value)}
                        placeholder="ctx7sk..."
                        className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]"
                      />
                      <button
                        onClick={() => void saveContext7Key()}
                        disabled={busy === "context7-key"}
                        className="self-start rounded bg-[#e4e4ed] px-3 py-1.5 text-xs font-medium text-[#0f0f14] disabled:opacity-60"
                      >
                        {busy === "context7-key" ? "Saving..." : "Save API Key"}
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
