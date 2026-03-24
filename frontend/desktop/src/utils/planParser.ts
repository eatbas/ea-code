/** Extracts clean plan text from noisy planner/auditor CLI output. */

/** Extracts plan-only text from noisy planner/auditor output. */
export function extractPlanOnly(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return "";

  let cleaned = trimmed.replace(/\r\n/g, "\n");
  const lower = cleaned.toLowerCase();

  if (lower.includes("llm not set")) return "";

  // Extract between structured plan markers if present.
  const beginMarker = "--- BEGIN PLAN ---";
  const endMarker = "--- END PLAN ---";
  const beginIdx = cleaned.indexOf(beginMarker);
  if (beginIdx >= 0) {
    const afterBegin = beginIdx + beginMarker.length;
    const endIdx = cleaned.indexOf(endMarker, afterBegin);
    const planText = endIdx >= 0
      ? cleaned.slice(afterBegin, endIdx).trim()
      : cleaned.slice(afterBegin).trim();
    if (planText) {
      return stripPlanTailNoise(planText);
    }
  }

  const lines = cleaned.split("\n");
  let verdictLineIdx = -1;
  let verdict = "";
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i].trim();
    if (line === "APPROVED" || line === "REJECTED") {
      verdictLineIdx = i;
      verdict = line;
    }
  }

  if (verdictLineIdx >= 0) {
    const afterVerdict = lines.slice(verdictLineIdx + 1).join("\n").trim();
    const auditedFromVerdict = extractAfterLastPlanMarker(afterVerdict);
    if (auditedFromVerdict) return auditedFromVerdict;

    if (verdict === "REJECTED") return "";

    const strippedAfterVerdict = stripPlanTailNoise(afterVerdict);
    if (strippedAfterVerdict && !isLikelyTemplateNoise(strippedAfterVerdict)) {
      return strippedAfterVerdict;
    }
  }

  const markedPlan = extractAfterLastPlanMarker(cleaned);
  if (markedPlan) {
    return markedPlan;
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

  return stripPlanTailNoise(cleaned.trim());
}

/** Removes common CLI transcript tail noise from plan output. */
function stripPlanTailNoise(text: string): string {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const kept: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    const lower = trimmed.toLowerCase();

    const isNoiseBoundary =
      lower.startsWith("tokens used") ||
      lower.startsWith("total tokens") ||
      lower.startsWith("total cost") ||
      lower.startsWith("total duration") ||
      lower === "exec" ||
      lower === "codex" ||
      lower.startsWith("<image>") ||
      /^!\[[^\]]*]\([^)]*\)$/.test(trimmed) ||
      /^".*powershell(\.exe)?"/i.test(trimmed) ||
      /^succeeded in \d+(?:\.\d+)?s:/i.test(trimmed);

    if (isNoiseBoundary) break;
    kept.push(line);
  }

  return kept.join("\n").trim();
}

function extractAfterLastPlanMarker(text: string): string {
  const improvedMarkers = ["--- Improved Plan ---", "--- Rewritten Plan ---"];
  let markerIndex = -1;
  let markerLength = 0;
  for (const marker of improvedMarkers) {
    const idx = text.lastIndexOf(marker);
    if (idx > markerIndex) {
      markerIndex = idx;
      markerLength = marker.length;
    }
  }

  if (markerIndex < 0) return "";

  const afterMarker = stripPlanTailNoise(text.slice(markerIndex + markerLength).trim());
  if (!afterMarker) return "";
  if (isLikelyTemplateNoise(afterMarker)) return "";
  return afterMarker;
}

function isLikelyTemplateNoise(text: string): boolean {
  const preview = text.trim().toLowerCase().slice(0, 400);
  if (!preview) return false;
  if (preview.startsWith("# inputs") && preview.includes("# output constraints")) return true;
  if (preview.startsWith("--- workspace context ---")) return true;
  if (preview.startsWith("workspace snapshot")) return true;
  if (preview.startsWith("worktree snapshot")) return true;
  return false;
}

/** Resolves plan text for display, preferring parsed plan and falling back to raw output. */
export function resolvePlanText(primary?: string, fallback?: string): string {
  const parsedPrimary = extractPlanOnly(primary ?? "");
  if (parsedPrimary) return parsedPrimary;

  const parsedFallback = extractPlanOnly(fallback ?? "");
  if (parsedFallback) return parsedFallback;

  const rawPrimary = (primary ?? "").trim();
  if (rawPrimary) return rawPrimary;

  return (fallback ?? "").trim();
}

/** Resolves audited plan text and never falls back to raw noisy output. */
export function resolveAuditedPlanText(primary?: string, fallback?: string): string {
  const parsedPrimary = extractPlanOnly(primary ?? "");
  if (parsedPrimary) return parsedPrimary;

  const parsedFallback = extractPlanOnly(fallback ?? "");
  if (parsedFallback) return parsedFallback;

  return "";
}
