import type { ReactNode } from "react";
import { useEffect } from "react";
import { useApiCliVersions } from "../../hooks/useApiCliVersions";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useCliHealth } from "../../hooks/useCliHealth";
import { useSettings } from "../../hooks/useSettings";
import { CliSetupView } from ".";

export function CliSetupRoute(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();
  const { checkHealth: checkCliHealth } = useCliHealth();
  const { health: apiHealth, providers, checkHealth: checkApiHealth } = useApiHealth();
  const {
    versions: apiVersions,
    loading: apiVersionsLoading,
    updating: apiVersionsUpdating,
    fetchVersions: fetchApiVersions,
    updateCli: updateApiCli,
  } = useApiCliVersions();

  useEffect(() => {
    if (!settings) {
      return;
    }

    checkCliHealth(settings);
    checkApiHealth();
  }, [settings, checkApiHealth, checkCliHealth]);

  if (loading || !settings) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0b0b0c]">
        <span className="text-sm text-[#8b8b93]">Loading...</span>
      </div>
    );
  }

  return (
    <CliSetupView
      settings={settings}
      apiHealth={apiHealth}
      providers={providers}
      apiVersions={apiVersions}
      versionsLoading={apiVersionsLoading}
      updating={apiVersionsUpdating}
      onFetchVersions={fetchApiVersions}
      onRefreshProviders={checkApiHealth}
      onUpdateCli={updateApiCli}
      onSave={saveSettings}
    />
  );
}
