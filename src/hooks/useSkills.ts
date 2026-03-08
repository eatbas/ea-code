import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CreateSkillPayload, Skill, UpdateSkillPayload } from "../types";

interface UseSkillsReturn {
  skills: Skill[];
  loading: boolean;
  error: string | null;
  refreshSkills: () => Promise<void>;
  createSkill: (payload: CreateSkillPayload) => Promise<void>;
  updateSkill: (payload: UpdateSkillPayload) => Promise<void>;
  deleteSkill: (id: string) => Promise<void>;
}

/** Hook for skills catalogue CRUD commands. */
export function useSkills(): UseSkillsReturn {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const refreshSkills = useCallback(async (): Promise<void> => {
    try {
      const result = await invoke<Skill[]>("list_skills");
      setSkills(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshSkills();
  }, [refreshSkills]);

  const createSkill = useCallback(async (payload: CreateSkillPayload): Promise<void> => {
    try {
      await invoke<Skill>("create_skill", { payload });
      await refreshSkills();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, [refreshSkills]);

  const updateSkill = useCallback(async (payload: UpdateSkillPayload): Promise<void> => {
    try {
      await invoke<Skill>("update_skill", { payload });
      await refreshSkills();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, [refreshSkills]);

  const deleteSkill = useCallback(async (id: string): Promise<void> => {
    try {
      await invoke("delete_skill", { id });
      await refreshSkills();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, [refreshSkills]);

  return {
    skills,
    loading,
    error,
    refreshSkills,
    createSkill,
    updateSkill,
    deleteSkill,
  };
}
