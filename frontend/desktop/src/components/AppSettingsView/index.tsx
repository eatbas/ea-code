import type { ReactNode } from "react";
import { useState } from "react";
import { useDbStats } from "../../hooks/useDbStats";
import { TableBrowser } from "./TableBrowser";

/** Tables that cannot be truncated (single-row config). */
const PROTECTED_TABLES = new Set(["settings"]);

/** App Settings view — database browser, table management, and app controls. */
export function AppSettingsView(): ReactNode {
  const { stats, loading, fetchRows, truncateTable, restartApp } = useDbStats();
  const [browsingTable, setBrowsingTable] = useState<string | null>(null);
  const [confirmTruncate, setConfirmTruncate] = useState<string | null>(null);
  const [confirmRestart, setConfirmRestart] = useState(false);
  const [truncating, setTruncating] = useState(false);

  async function handleTruncate(tableName: string): Promise<void> {
    setTruncating(true);
    try {
      await truncateTable(tableName);
    } catch (err) {
      console.error("Truncate failed:", err);
    } finally {
      setTruncating(false);
      setConfirmTruncate(null);
    }
  }

  async function handleRestart(): Promise<void> {
    try {
      await restartApp();
    } catch (err) {
      console.error("Restart failed:", err);
    }
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="text-sm text-[#9898b0]">Loading database info…</span>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto px-8 py-8">
      <h1 className="mb-6 text-xl font-semibold text-[#e4e4ed]">App Settings</h1>

      {/* ── Database Section ───────────────────────────────── */}
      <fieldset className="mb-8 rounded-xl border border-[#2e2e48] p-5">
        <legend className="px-2 text-sm font-medium text-[#e4e4ed]">Database</legend>

        {stats && (
          <div className="mb-4 flex flex-wrap items-center gap-4 text-xs text-[#6b6b82]">
            <span title={stats.dbPath}>Path: {stats.dbPath}</span>
            <span>Size: {formatBytes(stats.dbSizeBytes)}</span>
          </div>
        )}

        {stats && (
          <div className="space-y-1">
            {stats.tables.map(({ tableName, rowCount }) => (
              <div
                key={tableName}
                className="flex items-center justify-between rounded-lg px-4 py-2.5 hover:bg-[#1e1e2e] transition-colors"
              >
                <div className="flex items-center gap-3">
                  <span className="text-sm font-medium text-[#e4e4ed]">{tableName}</span>
                  <span className="rounded-full bg-[#24243a] px-2 py-0.5 text-xs text-[#9898b0]">
                    {rowCount} row{rowCount !== 1 ? "s" : ""}
                  </span>
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={() => setBrowsingTable(tableName)}
                    className="rounded px-3 py-1 text-xs text-[#6366f1] hover:bg-[#6366f1]/10 transition-colors"
                  >
                    Browse
                  </button>

                  {!PROTECTED_TABLES.has(tableName) && (
                    <button
                      onClick={() => setConfirmTruncate(tableName)}
                      disabled={rowCount === 0}
                      className="rounded px-3 py-1 text-xs text-red-400 hover:bg-red-400/10 transition-colors disabled:opacity-30 disabled:pointer-events-none"
                    >
                      Clear data
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </fieldset>

      {/* ── Application Section ────────────────────────────── */}
      <fieldset className="rounded-xl border border-[#2e2e48] p-5">
        <legend className="px-2 text-sm font-medium text-[#e4e4ed]">Application</legend>

        <div className="flex items-center justify-between rounded-lg px-4 py-3">
          <div>
            <p className="text-sm text-[#e4e4ed]">Restart application</p>
            <p className="text-xs text-[#6b6b82]">
              Closes and relaunches the app. Unsaved state will be lost.
            </p>
          </div>
          <button
            onClick={() => setConfirmRestart(true)}
            className="rounded-lg bg-[#24243a] px-4 py-2 text-sm text-[#e4e4ed] hover:bg-[#2e2e48] transition-colors"
          >
            Restart
          </button>
        </div>
      </fieldset>

      {/* ── Table Browser Modal ────────────────────────────── */}
      {browsingTable && (
        <TableBrowser
          tableName={browsingTable}
          onFetchRows={fetchRows}
          onClose={() => setBrowsingTable(null)}
        />
      )}

      {/* ── Truncate Confirmation ──────────────────────────── */}
      {confirmTruncate && (
        <ConfirmDialog
          title={`Clear "${confirmTruncate}"?`}
          message={`This will delete all rows from the ${confirmTruncate} table. This action cannot be undone.`}
          confirmLabel={truncating ? "Clearing…" : "Clear data"}
          destructive
          disabled={truncating}
          onConfirm={() => void handleTruncate(confirmTruncate)}
          onCancel={() => setConfirmTruncate(null)}
        />
      )}

      {/* ── Restart Confirmation ───────────────────────────── */}
      {confirmRestart && (
        <ConfirmDialog
          title="Restart application?"
          message="The app will close and relaunch. Any unsaved state will be lost."
          confirmLabel="Restart"
          onConfirm={() => void handleRestart()}
          onCancel={() => setConfirmRestart(false)}
        />
      )}
    </div>
  );
}

// ── Helpers ─────────────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ── Inline confirmation dialog ──────────────────────────────────────

interface ConfirmDialogProps {
  title: string;
  message: string;
  confirmLabel: string;
  destructive?: boolean;
  disabled?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

function ConfirmDialog({
  title,
  message,
  confirmLabel,
  destructive,
  disabled,
  onConfirm,
  onCancel,
}: ConfirmDialogProps): ReactNode {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-full max-w-sm rounded-xl border border-[#2e2e48] bg-[#1a1a24] p-6 shadow-2xl">
        <h3 className="mb-2 text-base font-semibold text-[#e4e4ed]">{title}</h3>
        <p className="mb-5 text-sm text-[#9898b0]">{message}</p>
        <div className="flex justify-end gap-3">
          <button
            onClick={onCancel}
            className="rounded-lg px-4 py-2 text-sm text-[#9898b0] hover:bg-[#24243a] transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            disabled={disabled}
            className={`rounded-lg px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50 ${
              destructive
                ? "bg-red-500/20 text-red-400 hover:bg-red-500/30"
                : "bg-[#6366f1]/20 text-[#6366f1] hover:bg-[#6366f1]/30"
            }`}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
