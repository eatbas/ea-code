import { useCallback } from "react";
import { useToast } from "../components/shared/Toast";

/** Shared error callback for WorkspaceFooter actions. */
export function useFooterErrorHandler(): () => void {
  const toast = useToast();
  return useCallback(() => {
    toast.error("Failed to open project action.");
  }, [toast]);
}
