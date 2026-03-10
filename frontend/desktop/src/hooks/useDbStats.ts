import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DbStats, TableData } from "../types";

const PAGE_SIZE = 25;

interface UseDbStatsReturn {
  stats: DbStats | null;
  loading: boolean;
  /** Reload table counts and DB size. */
  refreshStats: () => Promise<void>;
  /** Fetch a page of rows for the given table. */
  fetchRows: (tableName: string, page: number) => Promise<TableData>;
  /** Delete all rows from a table, then refresh stats. */
  truncateTable: (tableName: string) => Promise<void>;
  /** Restart the Tauri application process. */
  restartApp: () => Promise<void>;
}

export function useDbStats(): UseDbStatsReturn {
  const [stats, setStats] = useState<DbStats | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshStats = useCallback(async () => {
    try {
      const result = await invoke<DbStats>("get_db_stats");
      setStats(result);
    } catch (err) {
      console.error("Failed to load DB stats:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshStats();
  }, [refreshStats]);

  const fetchRows = useCallback(
    async (tableName: string, page: number): Promise<TableData> => {
      return invoke<TableData>("get_table_rows", {
        tableName,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      });
    },
    [],
  );

  const truncateTable = useCallback(
    async (tableName: string): Promise<void> => {
      await invoke("truncate_table", { tableName });
      await refreshStats();
    },
    [refreshStats],
  );

  const restartApp = useCallback(async (): Promise<void> => {
    await invoke("restart_app");
  }, []);

  return { stats, loading, refreshStats, fetchRows, truncateTable, restartApp };
}

export { PAGE_SIZE };
