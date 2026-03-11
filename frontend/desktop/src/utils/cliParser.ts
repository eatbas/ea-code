/** Parsing utilities for CLI JSON output. */

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
