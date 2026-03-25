/** Shared helpers for filtering plan and review artifacts from a flat artifact map. */

/** Extracts plan artifacts: keys matching "plan" or "plan_<number>". */
export function buildPlanArtifactMap(artifacts: Record<string, string>): Record<string, string> {
  const map: Record<string, string> = {};
  for (const [key, value] of Object.entries(artifacts)) {
    if (key === "plan" || /^plan_\d+$/.test(key)) {
      map[key] = value;
    }
  }
  return map;
}

/** Extracts review artifacts: keys matching "review" or "review_<number>". */
export function buildReviewArtifactMap(artifacts: Record<string, string>): Record<string, string> {
  const map: Record<string, string> = {};
  for (const [key, value] of Object.entries(artifacts)) {
    if (key === "review" || /^review_\d+$/.test(key)) {
      map[key] = value;
    }
  }
  return map;
}
