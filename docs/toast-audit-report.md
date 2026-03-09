# Toast Coverage Audit Report

Date: 2026-03-09
Scope: `frontend/desktop/src`
Goal: Identify user-facing information paths that do not use toast notifications.

## Current Toast Usage

Only two local toast implementations exist:

1. `frontend/desktop/src/components/shared/PromptInputBar.tsx`
2. `frontend/desktop/src/components/CliSetupView/index.tsx`

This means most async actions across the app still use inline error panels, static text, or only console logging.

## Findings

### P0 - User can miss critical failures (no visible feedback)

1. Pipeline start failure can be invisible when no run exists yet.
- File: `frontend/desktop/src/hooks/usePipeline.ts:188-197`
- Current behaviour: In `startPipeline` catch block, state update is skipped when `prev` is `null`, so no UI error appears.
- Impact: User clicks run and nothing obvious happens.
- Recommendation: Trigger toast on failure and set an explicit transient error state independent of `run`.

2. Workspace selection/open errors are stored but never shown.
- File: `frontend/desktop/src/hooks/useWorkspace.ts:24,38`
- File: `frontend/desktop/src/App.tsx:35`
- Current behaviour: `useWorkspace` has `error`, but `App` does not read or render it.
- Impact: Project open failures are silent.
- Recommendation: Surface via global toast in `App` when workspace error changes.

3. Settings load/save errors are stored but never shown.
- File: `frontend/desktop/src/hooks/useSettings.ts:30,52`
- File: `frontend/desktop/src/App.tsx:36`
- Current behaviour: `useSettings` has `error`, but `App` ignores it.
- Impact: Save/load problems are not visible.
- Recommendation: Show error toast in `App`; optional success toast on save.

4. Project/session list fetch failures are console-only.
- File: `frontend/desktop/src/hooks/useHistory.ts:29,38`
- Current behaviour: failures only `console.error`.
- Impact: Empty lists can look like "no data" instead of "load failed".
- Recommendation: Return error state from hook and show toast from caller.

### P1 - Feedback exists but not toast-based (inconsistent UX)

5. Skills CRUD feedback uses inline error panel; no success toasts.
- File: `frontend/desktop/src/components/SkillsView.tsx:173-177`
- File: `frontend/desktop/src/hooks/useSkills.ts:42,52,62`
- Current behaviour: inline error only; no success messages for create/update/delete.
- Recommendation: Replace transient inline error feedback with toasts; keep field validation inline (`Skill name is required`).

6. MCP actions use inline error panel; no success toasts.
- File: `frontend/desktop/src/components/McpView.tsx:90-94`
- File: `frontend/desktop/src/components/McpView.tsx:40-82`
- Current behaviour: operation errors shown inline, no success toast for toggle/binding/API key save.
- Recommendation: Toast on action success/failure; keep persistent page-level load failures optionally inline.

7. Session-detail load failure is treated as "not found" and logged.
- File: `frontend/desktop/src/App.tsx:101-114`
- File: `frontend/desktop/src/components/SessionDetailView.tsx:43-46`
- Current behaviour: catch logs error then sets `sessionDetail` null; UI shows "Session not found.".
- Impact: network/backend failure masquerades as not-found.
- Recommendation: toast true error and differentiate from real not-found.

8. Cancel/answer failures in question flow are console-only.
- File: `frontend/desktop/src/hooks/usePipeline.ts:214-216,228-230`
- Current behaviour: logs only.
- Impact: user cannot tell why cancel/submit failed.
- Recommendation: toast on failure.

9. "Open in VS Code" failure is silent.
- File: `frontend/desktop/src/components/IdleView.tsx:152`
- Current behaviour: fire-and-forget invoke, no catch.
- Recommendation: add catch and toast if command fails.

10. CLI install action has no toast and no error handling.
- File: `frontend/desktop/src/components/CliSetupView/CliCard.tsx:146`
- Current behaviour: opens URL without catch or feedback.
- Recommendation: on failure, show toast (e.g. "Could not open install page").

### P2 - Architecture consistency gaps

11. Toast implementation is duplicated and local.
- File: `frontend/desktop/src/components/shared/PromptInputBar.tsx:40-94`
- File: `frontend/desktop/src/components/CliSetupView/index.tsx:67-114`
- Current behaviour: two separate timers and UI styles.
- Recommendation: introduce shared global toast service/provider (`useToast`) for consistent style, duration, and behaviour.

12. `useCliVersions` still tracks `error` state even though CLI Setup now relies on result-based toasts.
- File: `frontend/desktop/src/hooks/useCliVersions.ts:14,24,35,54`
- Current behaviour: dual pattern (`error` state + action result).
- Recommendation: standardise to one pattern (prefer action result + global toasts).

## Recommended Rollout Order

1. Build a shared toast provider/hook and migrate existing two local implementations first.
2. Fix all P0 items (pipeline start, workspace/settings/history errors).
3. Migrate Skills and MCP actions to toasts with success + failure messages.
4. Cover remaining P1 edges (cancel/answer, open VS Code, install URL).
5. Remove redundant per-hook error state where not used by UI.

## Suggested Toast Message Matrix (starter)

- Workspace
  - Success: `Project opened.`
  - Error: `Project open failed: <reason>`
- Settings
  - Success: `Settings saved.`
  - Error: `Settings save failed: <reason>`
- History
  - Error: `Could not load projects.` / `Could not load sessions.`
- Pipeline
  - Error: `Pipeline failed to start: <reason>`
  - Error: `Cancel request failed: <reason>`
  - Error: `Answer submission failed: <reason>`
- Skills
  - Success: `Skill created.` / `Skill updated.` / `Skill deleted.`
  - Error: `<Action> failed: <reason>`
- MCP
  - Success: `Server settings updated.` / `Context7 API key saved.`
  - Error: `<Action> failed: <reason>`
- Utilities
  - Error: `Could not open VS Code.`
  - Error: `Could not open install page.`
