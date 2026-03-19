/** Display formatting utilities for pipeline metrics. */

import type { ProjectSummary } from "../types";

// Re-export plan and CLI parsers so existing imports keep working.
export { extractPlanOnly, resolvePlanText, resolveAuditedPlanText } from "./planParser";
export { tryParseJson, parseCliResult } from "./cliParser";
export type { CliResult } from "./cliParser";

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

/** Formats a timestamp for compact UI labels while preserving the start time/date. */
export function formatCompactTimestamp(iso: string): string {
  try {
    const d = parseUtcTimestamp(iso);
    const now = new Date();
    const sameDay =
      d.getFullYear() === now.getFullYear() &&
      d.getMonth() === now.getMonth() &&
      d.getDate() === now.getDate();

    if (sameDay) {
      return d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
    }

    const sameYear = d.getFullYear() === now.getFullYear();
    return d.toLocaleDateString(undefined, sameYear
      ? { month: "short", day: "numeric" }
      : { year: "2-digit", month: "short", day: "numeric" });
  } catch {
    return "";
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

/** Formats a timestamp into a short relative label (e.g., "2m", "6h", "3d"). */
export function formatRelativeTime(iso: string): string {
  try {
    const d = parseUtcTimestamp(iso);
    const diffMs = Date.now() - d.getTime();
    if (diffMs < 0) return "now";
    const secs = Math.floor(diffMs / 1000);
    if (secs < 60) return "now";
    const mins = Math.floor(secs / 60);
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months}mo`;
    return `${Math.floor(months / 12)}y`;
  } catch {
    return "";
  }
}

/** Truncates text to the first N words and appends an ellipsis when truncated. */
export function truncateWords(text: string, maxWords: number): string {
  const words = text.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "";
  if (words.length <= maxWords) return words.join(" ");
  return `${words.slice(0, maxWords).join(" ")}...`;
}

/** Normalises multiline output by trimming only blank leading/trailing lines. */
export function normaliseDisplayText(text: string): string {
  return text
    .replace(/\r\n/g, "\n")
    .replace(/^[\n\r]+/, "")
    .replace(/[\n\r]+$/, "");
}
