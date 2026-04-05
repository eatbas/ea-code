import type { ReactNode } from "react";
import { useRef } from "react";
import type { AppSettings } from "../../types";
import { PopoverSelect } from "../shared/PopoverSelect";
import { ToggleSwitch } from "../shared/ToggleSwitch";
import { useToast } from "../shared/Toast";
import { requestNotificationPermission } from "../../lib/desktopApi";

const COMPLETION_OPTIONS = [
  { value: "always", label: "Always" },
  { value: "never", label: "Never" },
  { value: "when_in_background", label: "When in background" },
];

interface NotificationsSettingsViewProps {
  settings: AppSettings;
  onSave: (settings: AppSettings) => void;
}

export function NotificationsSettingsView({ settings, onSave }: NotificationsSettingsViewProps): ReactNode {
  const toast = useToast();
  const permissionRequested = useRef<boolean>(false);

  /** Request OS permission the first time notifications are enabled. */
  async function ensurePermission(): Promise<void> {
    if (permissionRequested.current) return;
    permissionRequested.current = true;
    try {
      const granted = await requestNotificationPermission();
      if (!granted) {
        toast.info("Notification permission was not granted. You can enable it in System Settings.");
      }
    } catch {
      toast.error("Failed to request notification permission.");
    }
  }

  function handleCompletionChange(value: string): void {
    const next = value as AppSettings["completionNotifications"];
    onSave({ ...settings, completionNotifications: next });
    if (next !== "never") {
      void ensurePermission();
    }
  }

  function handlePermissionToggle(checked: boolean): void {
    onSave({ ...settings, permissionNotifications: checked });
    if (checked) {
      void ensurePermission();
    }
  }

  return (
    <div className="relative flex h-full flex-col bg-surface">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          {/* Header */}
          <div className="mb-2">
            <h1 className="text-xl font-bold text-fg">Notifications</h1>
            <p className="mt-1 text-sm text-fg-muted">
              Control when Maestro sends operating-system notifications.
            </p>
          </div>

          {/* Completion notifications */}
          <div className="rounded-lg border border-edge bg-panel p-5">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-fg">Turn completion notifications</p>
                <p className="mt-0.5 text-xs text-fg-muted">
                  Set when Maestro alerts you that it&apos;s finished.
                </p>
              </div>
              <PopoverSelect
                value={settings.completionNotifications}
                options={COMPLETION_OPTIONS}
                onChange={handleCompletionChange}
              />
            </div>
          </div>

          {/* Permission notifications */}
          <div className="rounded-lg border border-edge bg-panel p-5">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-fg">Enable permission notifications</p>
                <p className="mt-0.5 text-xs text-fg-muted">
                  Show alerts when notification permissions are required.
                </p>
              </div>
              <ToggleSwitch
                checked={settings.permissionNotifications}
                onChange={handlePermissionToggle}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
