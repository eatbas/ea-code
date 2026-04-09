import type { ReactNode } from "react";
import type { AppSettings } from "../../types";
import { PopoverSelect } from "../shared/PopoverSelect";
import { ToggleSwitch } from "../shared/ToggleSwitch";

const LANGUAGE_OPTIONS = [{ value: "en", label: "English" }];

interface GeneralSettingsViewProps {
  settings: AppSettings;
  onSave: (settings: AppSettings) => void;
}

export function GeneralSettingsView({ settings, onSave }: GeneralSettingsViewProps): ReactNode {
  function handleLanguageChange(value: string): void {
    onSave({ ...settings, language: value });
  }

  function handleKeepAwakeChange(checked: boolean): void {
    onSave({ ...settings, keepAwake: checked });
  }

  return (
    <div className="relative flex h-full flex-col bg-surface">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          {/* Header */}
          <div className="mb-2">
            <h1 className="text-xl font-bold text-fg">General</h1>
            <p className="mt-1 text-sm text-fg-muted">
              Configure language and system behaviour.
            </p>
          </div>

          {/* Language */}
          <div className="rounded-lg border border-edge bg-panel p-5">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-fg">Language</p>
                <p className="mt-0.5 text-xs text-fg-muted">
                  Select the display language for the application.
                </p>
              </div>
              <PopoverSelect
                value={settings.language}
                options={LANGUAGE_OPTIONS}
                onChange={handleLanguageChange}
              />
            </div>
          </div>

          {/* Keep awake */}
          <div className="rounded-lg border border-edge bg-panel p-5">
            <div className="flex items-center justify-between gap-6">
              <div>
                <p className="text-sm font-medium text-fg">Keep awake</p>
                <p className="mt-0.5 text-xs text-fg-muted">
                  Prevent the computer from sleeping whilst Maestro is running.
                </p>
                <p className="mt-2 text-xs text-fg-muted">
                  When this is off, Maestro still keeps the computer awake whilst a task is running.
                </p>
              </div>
              <ToggleSwitch
                checked={settings.keepAwake}
                onChange={handleKeepAwakeChange}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
