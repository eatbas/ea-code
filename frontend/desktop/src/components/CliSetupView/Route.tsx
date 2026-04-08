import type { ReactNode } from "react";
import { useCallback, useEffect, useRef } from "react";
import { useApiCliVersions } from "../../hooks/useApiCliVersions";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useCliHealth } from "../../hooks/useCliHealth";
import { useSidecarLogs } from "../../hooks/useSidecarLogs";
import { useSidecarReady } from "../../hooks/useSidecarReady";
import { useSettings } from "../../hooks/useSettings";
import { useToast } from "../shared/Toast";
import { shouldAutoRefreshOnReady } from "../../utils/symphonyStartup";
import { CliSetupView } from ".";

export function CliSetupRoute(): ReactNode {
  const toast = useToast();
  const { settings, loading, saveSettings } = useSettings();
  const { checkHealth: checkCliHealth } = useCliHealth();
  const {
    health: apiHealth,
    providers,
    checking: apiChecking,
    checkHealth: checkApiHealth,
  } = useApiHealth();
  const { sidecarReady, sidecarError } = useSidecarReady();
  const { logs: sidecarLogs } = useSidecarLogs();
  const {
    versions: apiVersions,
    loading: apiVersionsLoading,
    updating: apiVersionsUpdating,
    fetchVersions: fetchApiVersions,
    updateCli: updateApiCli,
  } = useApiCliVersions();
  const previousReadyRef = useRef<boolean | null | undefined>(undefined);

  const refreshAll = useCallback((showSuccessToast: boolean): void => {
    checkApiHealth();
    fetchApiVersions();
    if (showSuccessToast) {
      toast.success("Symphony checks started.");
    }
  }, [checkApiHealth, fetchApiVersions, toast]);

  useEffect(() => {
    if (!settings) {
      return;
    }

    checkCliHealth(settings);
  }, [settings, checkCliHealth]);

  useEffect(() => {
    const previousReady = previousReadyRef.current;
    previousReadyRef.current = sidecarReady;

    if (shouldAutoRefreshOnReady(previousReady, sidecarReady)) {
      refreshAll(false);
    }
  }, [refreshAll, sidecarReady]);

  if (loading || !settings) {
    return (
      <div className="flex h-full items-center justify-center bg-surface">
        <span className="text-sm text-fg-muted">Loading...</span>
      </div>
    );
  }

  return (
    <CliSetupView
      settings={settings}
      apiHealth={apiHealth}
      apiChecking={apiChecking}
      sidecarReady={sidecarReady}
      sidecarError={sidecarError}
      providers={providers}
      apiVersions={apiVersions}
      versionsLoading={apiVersionsLoading}
      updating={apiVersionsUpdating}
      sidecarLogs={sidecarLogs}
      onRefresh={() => refreshAll(true)}
      onUpdateCli={updateApiCli}
      onSave={saveSettings}
    />
  );
}
