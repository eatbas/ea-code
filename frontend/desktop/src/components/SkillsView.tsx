import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import type { CreateSkillPayload, Skill, UpdateSkillPayload } from "../types";

interface SkillsViewProps {
  skills: Skill[];
  loading: boolean;
  error: string | null;
  onCreate: (payload: CreateSkillPayload) => Promise<void>;
  onUpdate: (payload: UpdateSkillPayload) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
}

interface SkillDraft {
  id: string | null;
  name: string;
  description: string;
  tags: string;
  instructions: string;
  isActive: boolean;
}

const EMPTY_DRAFT: SkillDraft = {
  id: null,
  name: "",
  description: "",
  tags: "",
  instructions: "",
  isActive: true,
};

/** Skills management view with list and editor. */
export function SkillsView({
  skills,
  loading,
  error,
  onCreate,
  onUpdate,
  onDelete,
}: SkillsViewProps): ReactNode {
  const [draft, setDraft] = useState<SkillDraft>(EMPTY_DRAFT);
  const [saving, setSaving] = useState<boolean>(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const selectedSkill = useMemo(
    () => (draft.id ? skills.find((skill) => skill.id === draft.id) ?? null : null),
    [draft.id, skills],
  );

  useEffect(() => {
    if (skills.length === 0) {
      setDraft(EMPTY_DRAFT);
      return;
    }
    if (!draft.id || !skills.some((skill) => skill.id === draft.id)) {
      const first = skills[0];
      setDraft({
        id: first.id,
        name: first.name,
        description: first.description,
        tags: first.tags,
        instructions: first.instructions,
        isActive: first.isActive,
      });
    }
  }, [draft.id, skills]);

  function loadSkill(skill: Skill): void {
    setLocalError(null);
    setDraft({
      id: skill.id,
      name: skill.name,
      description: skill.description,
      tags: skill.tags,
      instructions: skill.instructions,
      isActive: skill.isActive,
    });
  }

  function startNewSkill(): void {
    setLocalError(null);
    setDraft(EMPTY_DRAFT);
  }

  async function handleSave(): Promise<void> {
    const name = draft.name.trim();
    if (!name) {
      setLocalError("Skill name is required.");
      return;
    }

    setSaving(true);
    setLocalError(null);
    try {
      if (draft.id) {
        await onUpdate({
          id: draft.id,
          name,
          description: draft.description.trim(),
          tags: draft.tags.trim(),
          instructions: draft.instructions.trim(),
          isActive: draft.isActive,
        });
      } else {
        await onCreate({
          name,
          description: draft.description.trim(),
          tags: draft.tags.trim(),
          instructions: draft.instructions.trim(),
          isActive: draft.isActive,
        });
      }
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(): Promise<void> {
    if (!draft.id) {
      return;
    }
    setSaving(true);
    setLocalError(null);
    try {
      await onDelete(draft.id);
      setDraft(EMPTY_DRAFT);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex h-full bg-[#0f0f14]">
      <aside className="w-72 shrink-0 border-r border-[#2e2e48] bg-[#1a1a24]">
        <div className="flex items-center justify-between border-b border-[#2e2e48] px-4 py-3">
          <h2 className="text-sm font-semibold text-[#e4e4ed]">Skills</h2>
          <button
            onClick={startNewSkill}
            className="rounded border border-[#2e2e48] bg-[#24243a] px-2 py-1 text-xs text-[#e4e4ed] hover:bg-[#2e2e48]"
          >
            New
          </button>
        </div>
        <div className="h-[calc(100%-52px)] overflow-y-auto px-2 py-2">
          {skills.length === 0 && !loading && (
            <p className="px-2 py-3 text-xs text-[#9898b0]">No skills yet</p>
          )}
          {skills.map((skill) => {
            const active = draft.id === skill.id;
            return (
              <button
                key={skill.id}
                onClick={() => loadSkill(skill)}
                className={`mb-1 w-full rounded px-3 py-2 text-left text-sm transition-colors ${
                  active ? "bg-[#24243a] text-[#e4e4ed]" : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                }`}
              >
                <div className="font-medium">{skill.name}</div>
                <div className="line-clamp-2 text-xs text-[#9898b0]">{skill.description || "No description"}</div>
              </button>
            );
          })}
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-3xl flex-col gap-4">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Skill Editor</h1>
          {(error || localError) && (
            <div className="rounded border border-[#ef4444]/30 bg-[#ef4444]/10 px-3 py-2 text-sm text-[#ef4444]">
              {error || localError}
            </div>
          )}

          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-[#9898b0]">Name</span>
            <input
              value={draft.name}
              onChange={(e) => setDraft((prev) => ({ ...prev, name: e.target.value }))}
              className="rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
            />
          </label>

          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-[#9898b0]">Description</span>
            <input
              value={draft.description}
              onChange={(e) => setDraft((prev) => ({ ...prev, description: e.target.value }))}
              className="rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
            />
          </label>

          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-[#9898b0]">Tags (comma-separated)</span>
            <input
              value={draft.tags}
              onChange={(e) => setDraft((prev) => ({ ...prev, tags: e.target.value }))}
              className="rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
            />
          </label>

          <label className="flex flex-col gap-1">
            <span className="text-xs font-medium text-[#9898b0]">Instructions</span>
            <textarea
              value={draft.instructions}
              onChange={(e) => setDraft((prev) => ({ ...prev, instructions: e.target.value }))}
              rows={12}
              className="rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 font-mono text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
            />
          </label>

          <label className="inline-flex items-center gap-2 text-sm text-[#9898b0]">
            <input
              type="checkbox"
              checked={draft.isActive}
              onChange={(e) => setDraft((prev) => ({ ...prev, isActive: e.target.checked }))}
              className="rounded border-[#2e2e48] accent-[#6366f1]"
            />
            Active
          </label>

          <div className="flex items-center gap-2">
            <button
              onClick={() => void handleSave()}
              disabled={saving || loading}
              className="rounded bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] hover:bg-white disabled:opacity-60"
            >
              {draft.id ? "Save Changes" : "Create Skill"}
            </button>
            {selectedSkill && (
              <button
                onClick={() => void handleDelete()}
                disabled={saving}
                className="rounded border border-[#ef4444]/50 bg-[#ef4444]/10 px-4 py-2 text-sm font-medium text-[#ef4444] hover:bg-[#ef4444]/20 disabled:opacity-60"
              >
                Delete
              </button>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
