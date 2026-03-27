import assert from "node:assert/strict";
import test from "node:test";

import { assignByKey, upsertByKey } from "./useEventResource";

interface VersionInfo {
  provider: string;
  latestVersion: string;
}

test("upsertByKey replaces the matching item and preserves non-matching items", () => {
  const previous: VersionInfo[] = [
    { provider: "copilot", latestVersion: "1.0.0" },
    { provider: "openai", latestVersion: "2.0.0" },
  ];

  const next = upsertByKey(
    previous,
    { provider: "copilot", latestVersion: "1.1.0" },
    (value) => value.provider,
  );

  assert.deepEqual(next, [
    { provider: "openai", latestVersion: "2.0.0" },
    { provider: "copilot", latestVersion: "1.1.0" },
  ]);
});

test("assignByKey overwrites the keyed record entry", () => {
  const previous = {
    claude: { provider: "claude", latestVersion: "1.0.0" },
    codex: { provider: "codex", latestVersion: "2.0.0" },
  };

  const next = assignByKey(previous, { provider: "codex", latestVersion: "2.1.0" }, (value) => value.provider as keyof typeof previous);

  assert.deepEqual(next, {
    claude: { provider: "claude", latestVersion: "1.0.0" },
    codex: { provider: "codex", latestVersion: "2.1.0" },
  });
});
