import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PrerequisiteStatus } from "../types";

interface UsePrerequisitesReturn {
  status: PrerequisiteStatus | null;
  loading: boolean;
  dismissed: boolean;
  dismiss: () => void;
}

/** Checks system prerequisites on mount and surfaces missing dependencies. */
export function usePrerequisites(): UsePrerequisitesReturn {
  const [status, setStatus] = useState<PrerequisiteStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    let mounted = true;
    invoke<PrerequisiteStatus>("check_prerequisites")
      .then((result) => {
        if (mounted) setStatus(result);
      })
      .catch(() => {
        // Silently ignore — prerequisites banner simply won't show.
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });
    return () => {
      mounted = false;
    };
  }, []);

  const dismiss = useCallback(() => setDismissed(true), []);

  return { status, loading, dismissed, dismiss };
}
