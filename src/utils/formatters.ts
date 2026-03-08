/** Formatting utilities for pipeline metrics display. */

/** Formats milliseconds into human-readable duration (e.g., "1.3s", "2m 5s"). */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const secs = ms / 1000;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = Math.floor(secs / 60);
  const remainSecs = Math.round(secs % 60);
  return `${mins}m ${remainSecs}s`;
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
