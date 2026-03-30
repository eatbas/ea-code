/** Single entry in projects.json — recent project list. */
export interface ProjectEntry {
  /** Unique identifier for the project entry. */
  id: string;
  /** Absolute path to the project directory. */
  path: string;
  /** Display name (directory name). */
  name: string;
  /** Whether the project is a git repository. */
  isGitRepo: boolean;
  /** Current git branch if applicable. */
  branch?: string;
  /** Last opened timestamp (RFC 3339). */
  lastOpened?: string;
  /** When the project entry was first created (RFC 3339). */
  createdAt: string;
  /** When the project entry was archived, if hidden from the active list. */
  archivedAt?: string;
}

/** Workspace validation result. */
export interface WorkspaceInfo {
  path: string;
  isGitRepo: boolean;
  isDirty?: boolean;
  branch?: string;
  maestroIgnored?: boolean;
}
