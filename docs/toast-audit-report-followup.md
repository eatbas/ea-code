# Toast Coverage Audit Report (Follow-up)

Date: 2026-03-09
Scope: `frontend/desktop/src`

## Summary

You have implemented the core toast infrastructure and migrated many flows successfully.

### Implemented since the last audit

- Global toast provider and hook are now in place.
  - `frontend/desktop/src/components/shared/Toast.tsx:29-97`
  - `frontend/desktop/src/main.tsx:3-11`
- Key operational failures now use toast (pipeline, history, CLI health, CLI versions, skills CRUD, open VS Code, session archive/load).
  - Examples:
    - `frontend/desktop/src/hooks/usePipeline.ts:199,217,231`
    - `frontend/desktop/src/hooks/useHistory.ts:31,40`
    - `frontend/desktop/src/hooks/useCliHealth.ts:24`
    - `frontend/desktop/src/hooks/useCliVersions.ts:38,57`
    - `frontend/desktop/src/hooks/useSkills.ts:43,46,55,58,67,70`
    - `frontend/desktop/src/components/IdleView.tsx:152-155`
    - `frontend/desktop/src/App.tsx:111,259-260`

## Remaining Gaps

### P0 - Important failures still not surfaced via toast

1. Workspace open/select failures are still stored but not shown to users.
- Error is set in hook:
  - `frontend/desktop/src/hooks/useWorkspace.ts:24,38`
- `App` does not consume workspace `error`:
  - `frontend/desktop/src/App.tsx:37`
- Impact: selecting/opening a project can fail silently.

2. Settings initial load failure still has no toast.
- Error is set during settings load:
  - `frontend/desktop/src/hooks/useSettings.ts:30-33`
- `App` does not consume `useSettings.error`:
  - `frontend/desktop/src/App.tsx:38`
- Impact: startup settings-read problems are not clearly visible.

### P1 - Mixed feedback channels (toast + inline) still present

3. MCP action failures are still shown only in inline error panel.
- Local inline error path in view:
  - `frontend/desktop/src/components/McpView.tsx:48-50,65-67,77-79,90-93`
- Hook emits success toasts, but failure toasts for these actions are not centralised:
  - `frontend/desktop/src/hooks/useMcpServers.ts:48-82`
- Impact: inconsistent UX (success in toast, failures inline).

4. Skills flow still duplicates error feedback (toast + inline panel).
- Hook already emits failure toasts:
  - `frontend/desktop/src/hooks/useSkills.ts:46,58,70`
- View also catches and renders inline error panel:
  - `frontend/desktop/src/components/SkillsView.tsx:113-115,129-131,173-176`
- Impact: duplicated or conflicting error messaging.

5. Skills initial list load failure still only inline.
- Load error set without toast in refresh:
  - `frontend/desktop/src/hooks/useSkills.ts:28-30`
- Rendered via inline panel:
  - `frontend/desktop/src/components/SkillsView.tsx:173-176`
- Impact: inconsistent with other pages that now use toast for load failures.

### P2 - Edge cases and consistency

6. CLI Install button still has no failure toast path.
- Install action opens URL with no catch:
  - `frontend/desktop/src/components/CliSetupView/CliCard.tsx:146`
- Impact: if URL open fails, user gets no feedback.

7. Updater failures remain silent.
- Install failure swallowed:
  - `frontend/desktop/src/hooks/useUpdateCheck.ts:35-39`
- Update-check failure ignored:
  - `frontend/desktop/src/hooks/useUpdateCheck.ts:53-55`
- Impact: no user-facing clue when update checks/installs fail.

8. Potential toast noise from autosave-heavy screens.
- Settings save always shows success toast:
  - `frontend/desktop/src/hooks/useSettings.ts:53`
- `onSave` is triggered on many field changes:
  - `frontend/desktop/src/components/AgentsView/index.tsx:52`
  - `frontend/desktop/src/components/CliSetupView/index.tsx:103`
- Impact: excessive success toasts while configuring forms.

## Recommended Next Pass

1. Surface `useWorkspace.error` and settings-load errors as toast (either in hooks or `App` effect watchers).
2. Standardise MCP and Skills failures to toast-first; keep inline only for validation that must remain near fields.
3. Add failure toast for `openUrl` in CLI Install flow.
4. Decide updater UX policy: either keep silent by design, or add low-frequency informational/error toasts.
5. Reduce toast noise for autosave by throttling/deduping settings success toasts or making save-success optional on bulk edits.
