import assert from "node:assert/strict";
import test from "node:test";

import {
  createWorkspaceSessionInitialState,
  workspaceSessionReducer,
  type WorkspaceSessionState,
} from "./useWorkspaceSession";

function advance(
  state: WorkspaceSessionState,
  actions: Parameters<typeof workspaceSessionReducer>[1][],
): WorkspaceSessionState {
  return actions.reduce(workspaceSessionReducer, state);
}

test("workspace session reducer records projects and opened workspace", () => {
  const initial = createWorkspaceSessionInitialState();

  const next = advance(initial, [
    {
      type: "set-projects",
      projects: [{
        id: "demo",
        path: "/tmp/demo",
        name: "demo",
        isGitRepo: true,
        branch: "main",
        lastOpened: "2026-03-27T10:00:00Z",
        createdAt: "2026-03-27T09:00:00Z",
      }],
    },
    { type: "open-workspace:start" },
    {
      type: "open-workspace:success",
      workspace: {
        path: "/tmp/demo",
        isGitRepo: true,
        isDirty: false,
        branch: "main",
      },
    },
    { type: "open-workspace:end" },
  ]);

  assert.equal(next.projects.length, 1);
  assert.equal(next.workspace?.path, "/tmp/demo");
  assert.equal(next.openingWorkspace, false);
  assert.equal(next.error, null);
});

test("workspace session reducer keeps the previous workspace on open error", () => {
  const initial = advance(createWorkspaceSessionInitialState(), [
    {
      type: "open-workspace:success",
      workspace: {
        path: "/tmp/demo",
        isGitRepo: true,
        isDirty: false,
        branch: "main",
      },
    },
  ]);

  const next = advance(initial, [
    { type: "open-workspace:start" },
    { type: "open-workspace:error", error: "Permission denied" },
    { type: "open-workspace:end" },
  ]);

  assert.equal(next.workspace?.path, "/tmp/demo");
  assert.equal(next.error, "Permission denied");
  assert.equal(next.openingWorkspace, false);
});
