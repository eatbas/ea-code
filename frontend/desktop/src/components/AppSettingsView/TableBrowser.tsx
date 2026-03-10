import type { ReactNode } from "react";
import { useCallback, useEffect, useState } from "react";
import type { TableData } from "../../types";
import { PAGE_SIZE } from "../../hooks/useDbStats";

interface TableBrowserProps {
  tableName: string;
  onFetchRows: (tableName: string, page: number) => Promise<TableData>;
  onClose: () => void;
}

/** Paginated data-grid modal for browsing a single table's rows. */
export function TableBrowser({
  tableName,
  onFetchRows,
  onClose,
}: TableBrowserProps): ReactNode {
  const [data, setData] = useState<TableData | null>(null);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadPage = useCallback(
    async (p: number) => {
      setLoading(true);
      setError(null);
      try {
        const result = await onFetchRows(tableName, p);
        setData(result);
        setPage(p);
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
      }
    },
    [tableName, onFetchRows],
  );

  useEffect(() => {
    void loadPage(0);
  }, [loadPage]);

  const totalPages = data ? Math.max(1, Math.ceil(data.totalCount / PAGE_SIZE)) : 1;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="flex max-h-[85vh] w-[90vw] max-w-5xl flex-col rounded-xl border border-[#2e2e48] bg-[#1a1a24] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#2e2e48] px-6 py-4">
          <div>
            <h2 className="text-lg font-semibold text-[#e4e4ed]">{tableName}</h2>
            {data && (
              <p className="text-xs text-[#6b6b82]">
                {data.totalCount} row{data.totalCount !== 1 ? "s" : ""} total
              </p>
            )}
          </div>
          <button
            onClick={onClose}
            className="rounded p-1.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-auto px-6 py-4">
          {loading && (
            <p className="py-8 text-center text-sm text-[#9898b0]">Loading…</p>
          )}

          {error && (
            <p className="py-8 text-center text-sm text-red-400">{error}</p>
          )}

          {!loading && !error && data && data.rows.length === 0 && (
            <p className="py-8 text-center text-sm text-[#6b6b82]">Table is empty.</p>
          )}

          {!loading && !error && data && data.rows.length > 0 && (
            <div className="overflow-x-auto rounded-lg border border-[#2e2e48]">
              <table className="w-full min-w-max text-left text-sm">
                <thead>
                  <tr className="border-b border-[#2e2e48] bg-[#14141e]">
                    {data.columns.map((col) => (
                      <th
                        key={col}
                        className="whitespace-nowrap px-3 py-2 text-xs font-medium text-[#9898b0]"
                      >
                        {col}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {data.rows.map((row, ri) => (
                    <tr
                      key={ri}
                      className="border-b border-[#2e2e48] last:border-b-0 hover:bg-[#1e1e2e]"
                    >
                      {row.map((cell, ci) => (
                        <td
                          key={ci}
                          className="max-w-[300px] truncate whitespace-nowrap px-3 py-2 text-xs text-[#e4e4ed]"
                          title={formatCell(cell)}
                        >
                          {formatCell(cell)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Footer / pagination */}
        <div className="flex items-center justify-between border-t border-[#2e2e48] px-6 py-3">
          <button
            disabled={page === 0 || loading}
            onClick={() => void loadPage(page - 1)}
            className="rounded px-3 py-1.5 text-xs text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors disabled:opacity-30 disabled:pointer-events-none"
          >
            Previous
          </button>
          <span className="text-xs text-[#6b6b82]">
            Page {page + 1} of {totalPages}
          </span>
          <button
            disabled={page >= totalPages - 1 || loading}
            onClick={() => void loadPage(page + 1)}
            className="rounded px-3 py-1.5 text-xs text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors disabled:opacity-30 disabled:pointer-events-none"
          >
            Next
          </button>
        </div>
      </div>
    </div>
  );
}

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return "NULL";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}
