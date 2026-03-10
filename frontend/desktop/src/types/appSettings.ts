/** Row count info for a single database table. */
export interface TableStats {
  tableName: string;
  rowCount: number;
}

/** Overview of the entire database: per-table counts and file size. */
export interface DbStats {
  tables: TableStats[];
  dbSizeBytes: number;
  dbPath: string;
}

/** Paginated row data returned when browsing a table. */
export interface TableData {
  columns: string[];
  rows: unknown[][];
  totalCount: number;
}
