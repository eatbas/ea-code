/** Formatting utilities for pipeline metrics display. */

import type { ProjectSummary } from "../types";

/** Extracts the last path segment as a folder display name. */
export function folderName(path: string): string {
  const parts = path.split(/[/\\]+/);
  return parts[parts.length - 1] || path;
}

/** Returns a display name for a project (name or folder fallback). */
export function projectDisplayName(project: ProjectSummary): string {
  return project.name.trim().length > 0 ? project.name : folderName(project.path);
}

/**
 * Parses a timestamp string as UTC.
 *
 * SQLite's CURRENT_TIMESTAMP produces bare strings like "2026-03-10 07:51:00"
 * without timezone info. JavaScript's Date() treats these as *local* time,
 * causing wrong durations when compared with RFC 3339 UTC strings. This helper
 * appends a "Z" suffix when no timezone indicator is present so the value is
 * always interpreted as UTC.
 */
export function parseUtcTimestamp(ts: string): Date {
  const trimmed = ts.trim();
  // Already has timezone info (+00:00, Z, -05:00, etc.)
  if (/[Zz]$/.test(trimmed) || /[+-]\d{2}:\d{2}$/.test(trimmed)) {
    return new Date(trimmed);
  }
  // Replace space separator with 'T' for ISO compliance and add UTC indicator
  return new Date(trimmed.replace(" ", "T") + "Z");
}

/** Formats an ISO timestamp into a readable date/time string. */
export function formatTimestamp(iso: string): string {
  try {
    const d = parseUtcTimestamp(iso);
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) +
      " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  } catch {
    return iso;
  }
}

/** Formats milliseconds into human-readable duration (e.g., "1.3s", "2m 5s", "1h 23m 5s"). */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const totalSecs = Math.floor(ms / 1000);
  if (totalSecs < 60) return `${(ms / 1000).toFixed(1)}s`;
  const hours = Math.floor(totalSecs / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hours > 0) {
    return secs > 0 ? `${hours}h ${mins}m ${secs}s` : `${hours}h ${mins}m`;
  }
  return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
}

/** Formats a token count into short notation (e.g., 13103 → "13.1k"). */
export function formatTokens(count: number): string {
  if (count < 1000) return String(count);
  if (count < 1_000_000) return `${(count / 1000).toFixed(1)}k`;
  return `${(count / 1_000_000).toFixed(1)}M`;
}

/** Formats a cost in USD (e.g., 0.013 → "$0.013"). */
export function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  if (usd < 1) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(2)}`;
}

/** Truncates text to the first N words and appends an ellipsis when truncated. */
export function truncateWords(text: string, maxWords: number): string {
  const words = text.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "";
  if (words.length <= maxWords) return words.join(" ");
  return `${words.slice(0, maxWords).join(" ")}...`;
}

/** Extracts plan-only text from noisy planner/auditor output. */
export function extractPlanOnly(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return "";

  let cleaned = trimmed.replace(/\r\n/g, "\n");
  const lower = cleaned.toLowerCase();

  if (lower.includes("llm not set")) return "";

  const improvedMarkers = ["--- Improved Plan ---", "--- Rewritten Plan ---"];
  for (const marker of improvedMarkers) {
    const idx = cleaned.indexOf(marker);
    if (idx >= 0) {
      const afterMarker = cleaned.slice(idx + marker.length).trim();
      return afterMarker;
    }
  }

  if (cleaned.startsWith("APPROVED\n") || cleaned.startsWith("REJECTED\n")) {
    cleaned = cleaned.split("\n").slice(1).join("\n").trim();
  }

  const contextIdx = cleaned.indexOf("\n--- Context ---");
  if (contextIdx >= 0) {
    cleaned = cleaned.slice(0, contextIdx).trim();
  }

  const workspaceIdx = cleaned.indexOf("\n--- Workspace Context ---");
  if (workspaceIdx >= 0) {
    cleaned = cleaned.slice(0, workspaceIdx).trim();
  }

  const cleanedUpper = cleaned.toUpperCase();
  if (cleanedUpper.startsWith("USER PROMPT (ORIGINAL):") || cleanedUpper.startsWith("ENHANCED EXECUTION PROMPT:")) {
    return "";
  }

  if (cleaned.includes("Planner agent in a multi-agent coding pipeline") || cleaned.includes("Plan Auditor agent in a multi-agent coding pipeline")) {
    return "";
  }

  return cleaned.trim();
}

/** Safely attempts to parse a string as a JSON object. Returns null on failure. */
export function tryParseJson(text: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch { /* not JSON */ }
  return null;
}

/** Parsed structure from Claude CLI JSON output. */
export interface CliResult {
  result?: string;
  totalCostUsd?: number;
  durationMs?: number;
  numTurns?: number;
  usage?: {
    inputTokens?: number;
    outputTokens?: number;
    cacheReadInputTokens?: number;
    cacheCreationInputTokens?: number;
  };
}

/** Tries to parse Claude CLI JSON output into a structured result. */
export function parseCliResult(text: string): CliResult | null {
  const json = tryParseJson(text);
  if (!json || json["type"] !== "result") return null;

  const usage = json["usage"] as Record<string, unknown> | undefined;
  return {
    result: typeof json["result"] === "string" ? json["result"] as string : undefined,
    totalCostUsd: typeof json["total_cost_usd"] === "number" ? json["total_cost_usd"] as number : undefined,
    durationMs: typeof json["duration_ms"] === "number" ? json["duration_ms"] as number : undefined,
    numTurns: typeof json["num_turns"] === "number" ? json["num_turns"] as number : undefined,
    usage: usage ? {
      inputTokens: typeof usage["input_tokens"] === "number" ? usage["input_tokens"] as number : undefined,
      outputTokens: typeof usage["output_tokens"] === "number" ? usage["output_tokens"] as number : undefined,
      cacheReadInputTokens: typeof usage["cache_read_input_tokens"] === "number" ? usage["cache_read_input_tokens"] as number : undefined,
      cacheCreationInputTokens: typeof usage["cache_creation_input_tokens"] === "number" ? usage["cache_creation_input_tokens"] as number : undefined,
    } : undefined,
  };
}
