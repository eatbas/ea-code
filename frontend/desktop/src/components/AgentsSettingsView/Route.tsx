import type { ReactNode } from "react";
import { useEffect } from "react";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useSettings } from "../../hooks/useSettings";
import { AgentsSettingsView } from ".";

export function AgentsSettingsRoute(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();
  const { providers, checkHealth } = useApiHealth();

  useEffect(() => {
    checkHealth();
  }, [checkHealth]);

  if (loading || !settings) {
    return (
      <div className="flex h-full items-center justify-center bg-surface">
        <span className="text-sm text-fg-muted">Loading...</span>
      </div>
    );
  }

  return (
    <AgentsSettingsView
      settings={settings}
      providers={providers}
      onSave={saveSettings}
    />
  );
}
