import type { ReactNode } from "react";
import { useSettings } from "../../hooks/useSettings";
import { NotificationsSettingsView } from ".";

export function NotificationsSettingsRoute(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();

  if (loading || !settings) {
    return (
      <div className="flex h-full items-center justify-center bg-surface">
        <span className="text-sm text-fg-muted">Loading...</span>
      </div>
    );
  }

  return <NotificationsSettingsView settings={settings} onSave={saveSettings} />;
}
