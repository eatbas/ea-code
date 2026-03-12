import type { ReactNode } from "react";
import { useState } from "react";
import type {
  CliHealth,
  McpCliRuntimeStatus,
  McpRuntimeStatus,
  McpServer,
  RunCliMcpFixWithPromptRequest,
} from "../../types";
import { AI_CLI_ORDER, dotColourClass, dotTooltip, isFixable } from "./helpers";

interface McpServerCardProps {
  server: McpServer;
  runtimeByCli: Map<string, McpCliRuntimeStatus>;
  cliHealth: CliHealth | null;
  fixingKey: string | null;
  actionsDisabled: boolean;
  onToggleEnabled: (server: McpServer, enabled: boolean) => Promise<void>;
  onRunFix: (request: RunCliMcpFixWithPromptRequest) => Promise<unknown>;
  /** Context7-only props */
  savedKey: string;
  editingKey: boolean;
  keyDraft: string;
  keyBusy: boolean;
  onKeyDraftChange: (value: string) => void;
  onSaveKey: () => Promise<void>;
  onDeleteKey: () => Promise<void>;
  onStartEditing: () => void;
  onCancelEditing: () => void;
}

export function McpServerCard({
  server,
  runtimeByCli,
  cliHealth,
  fixingKey,
  actionsDisabled,
  onToggleEnabled,
  onRunFix,
  savedKey,
  editingKey,
  keyDraft,
  keyBusy,
  onKeyDraftChange,
  onSaveKey,
  onDeleteKey,
  onStartEditing,
  onCancelEditing,
}: McpServerCardProps): ReactNode {
  const [busy, setBusy] = useState<boolean>(false);
  const [batchFixing, setBatchFixing] = useState<boolean>(false);

  const isContext7 = server.id === "context7";
  const missingKey = isContext7 && !savedKey.trim();
  const hasSavedKey = isContext7 && !!savedKey.trim();

  function isCliInstalled(cliName: string): boolean {
    const runtime = runtimeByCli.get(cliName);
    if (runtime !== undefined) return runtime.cliInstalled;
    if (!cliHealth) return false;
    const entry = cliHealth[cliName as keyof CliHealth];
    return entry?.available ?? false;
  }

  function resolveStatus(cliName: string): McpRuntimeStatus {
    if (!isCliInstalled(cliName)) return "notInstalled";
    const runtime = runtimeByCli.get(cliName);
    return runtime?.serverStatuses.find((r) => r.serverId === server.id)?.status ?? "unknown";
  }

  function countFixable(): number {
    if (missingKey) return 0;
    let count = 0;
    for (const cli of AI_CLI_ORDER) {
      if (isFixable(resolveStatus(cli))) count++;
    }
    return count;
  }

  async function toggleEnabled(enabled: boolean): Promise<void> {
    setBusy(true);
    try {
      await onToggleEnabled(server, enabled);
    } finally {
      setBusy(false);
    }
  }

  async function batchFix(): Promise<void> {
    setBatchFixing(true);
    try {
      for (const cliName of AI_CLI_ORDER) {
        if (!isFixable(resolveStatus(cliName))) continue;
        await onRunFix({ cliName, serverId: server.id });
      }
    } finally {
      setBatchFixing(false);
    }
  }

  const fixable = countFixable();

  return (
    <div className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4">
      {/* Header */}
      <div className="flex items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-[#e4e4ed]">{server.name}</h3>
          <p className="text-xs text-[#9898b0]">{server.description || "No description"}</p>
        </div>
        {server.isBuiltin && (
          <span className="rounded bg-[#24243a] px-2 py-1 text-[10px] text-[#9898b0]">Built-in</span>
        )}
      </div>

      {/* Enable toggle + batch Install/Fix */}
      <div className="mt-3 flex items-center justify-between">
        <label className="inline-flex items-center gap-2 text-xs text-[#9898b0]">
          <input
            type="checkbox"
            checked={server.isEnabled}
            onChange={(e) => void toggleEnabled(e.target.checked)}
            disabled={busy || actionsDisabled}
          />
          Enabled
        </label>
        <button
          type="button"
          onClick={() => void batchFix()}
          disabled={batchFixing || fixable === 0 || missingKey || actionsDisabled}
          title={missingKey ? "Set API key first" : undefined}
          className="rounded bg-[#e4e4ed] px-3 py-1.5 text-xs font-medium text-[#0f0f14] disabled:cursor-not-allowed disabled:opacity-50"
        >
          {batchFixing
            ? "Fixing..."
            : missingKey
              ? "No API Key"
              : fixable > 0
                ? `Install/Fix (${fixable})`
                : "All Good"}
        </button>
      </div>

      {/* CLI status dot grid — 3 columns */}
      <div className={`mt-3 grid grid-cols-3 gap-2${missingKey ? " opacity-40" : ""}`}>
        {AI_CLI_ORDER.map((cliName) => {
          const status = missingKey ? ("notInstalled" as McpRuntimeStatus) : resolveStatus(cliName);
          const runtime = runtimeByCli.get(cliName);
          const serverRow = runtime?.serverStatuses.find((r) => r.serverId === server.id);
          const tooltip = missingKey ? "Set API key first" : dotTooltip(status, serverRow?.message);
          const isCurrentlyFixing = !missingKey && fixingKey === `${cliName}:${server.id}`;

          return (
            <div
              key={cliName}
              className="flex items-center gap-2 rounded border border-[#2e2e48] bg-[#0f0f14] px-2 py-1.5"
              title={tooltip}
            >
              <span
                className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${
                  isCurrentlyFixing ? "animate-pulse bg-[#6366f1]" : dotColourClass(status)
                }`}
              />
              <span className="truncate text-[11px] text-[#9898b0]">{cliName}</span>
            </div>
          );
        })}
      </div>

      {/* Context7 API key — below dots */}
      {isContext7 && (
        <div className="mt-3 flex flex-col gap-2">
          <label className="text-xs text-[#9898b0]">Context7 API Key</label>

          {hasSavedKey && !editingKey ? (
            <>
              <span className="truncate rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#9898b0]">
                {savedKey.slice(0, 8)}{"•".repeat(12)}
              </span>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={onStartEditing}
                  className="rounded border border-[#2e2e48] bg-[#24243a] px-3 py-1.5 text-xs font-medium text-[#9898b0] hover:text-[#e4e4ed]"
                >
                  Edit
                </button>
                <button
                  type="button"
                  onClick={() => void onDeleteKey()}
                  disabled={keyBusy}
                  className="rounded border border-[#ef4444]/30 bg-[#ef4444]/10 px-3 py-1.5 text-xs font-medium text-[#ef4444] hover:bg-[#ef4444]/20 disabled:opacity-50"
                >
                  Delete
                </button>
              </div>
            </>
          ) : (
            <>
              <input
                type="password"
                value={keyDraft}
                onChange={(e) => onKeyDraftChange(e.target.value)}
                placeholder="ctx7sk..."
                className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]"
              />
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => void onSaveKey()}
                  disabled={keyBusy || !keyDraft.trim()}
                  className="rounded bg-[#e4e4ed] px-3 py-1.5 text-xs font-medium text-[#0f0f14] disabled:opacity-50"
                >
                  {keyBusy ? "Saving..." : "Save API Key"}
                </button>
                {editingKey && (
                  <button
                    type="button"
                    onClick={onCancelEditing}
                    className="rounded border border-[#2e2e48] bg-[#24243a] px-3 py-1.5 text-xs font-medium text-[#9898b0] hover:text-[#e4e4ed]"
                  >
                    Cancel
                  </button>
                )}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
