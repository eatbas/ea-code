import type { ReactNode } from "react";
import { useCallback, useMemo, useState } from "react";
import type { AgentSelection, AppSettings, ProviderInfo } from "../../types";
import { PopoverSelect } from "../shared/PopoverSelect";
import {
  modelOptionsFromProvider,
  providerDisplayName,
  sortProvidersByDisplayName,
} from "../shared/constants";
import { getEnabledModels } from "../../utils/modelSettings";
import { parseAgentSelection, serialiseAgentSelection } from "../../utils/agentSettings";
import { useToast } from "../shared/Toast";

interface AgentsSettingsViewProps {
  settings: AppSettings;
  providers: ProviderInfo[];
  onSave: (settings: AppSettings) => void;
}

export function AgentsSettingsView({
  settings,
  providers,
  onSave,
}: AgentsSettingsViewProps): ReactNode {
  const toast = useToast();
  const [openSelect, setOpenSelect] = useState<"provider" | "model" | null>(null);

  const availableProviders = useMemo(
    () => sortProvidersByDisplayName(filterProvidersBySettings(providers, settings)),
    [providers, settings],
  );

  const savedAgent = parseAgentSelection(settings.defaultAgent);

  const resolvedAgent = useMemo((): AgentSelection | null => {
    if (savedAgent) {
      const matchingProvider = availableProviders.find(
        (p) => p.name === savedAgent.provider,
      );
      if (matchingProvider?.models.includes(savedAgent.model)) {
        return savedAgent;
      }
    }
    const first = availableProviders[0];
    if (!first) return null;
    return { provider: first.name, model: first.models[0] ?? "" };
  }, [savedAgent, availableProviders]);

  const selectedProvider = availableProviders.find(
    (p) => p.name === resolvedAgent?.provider,
  );
  const modelOptions = modelOptionsFromProvider(selectedProvider);
  const providerOptions = useMemo(
    () => availableProviders.map((p) => ({
      value: p.name,
      label: providerDisplayName(p.name),
    })),
    [availableProviders],
  );

  const handleAgentChange = useCallback(
    (agent: AgentSelection) => {
      onSave({ ...settings, defaultAgent: serialiseAgentSelection(agent) });
      toast.success("Default agent updated.");
    },
    [settings, onSave, toast],
  );

  return (
    <div className="relative flex h-full flex-col bg-surface">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          <div>
            <h1 className="text-xl font-bold text-fg">Agents</h1>
            <p className="mt-1 text-sm text-fg-muted">
              Configure the default agent used for new conversations.
            </p>
          </div>

          {/* Default Agent */}
          <div className="rounded-lg border border-edge bg-panel p-5">
            <h2 className="text-sm font-semibold text-fg">Default Agent</h2>
            <p className="mt-1 text-xs text-fg-muted">
              New conversations will start with this provider and model.
              You can override the selection per conversation in the composer.
            </p>

            {availableProviders.length > 0 ? (
              <div className="mt-4 flex items-center gap-3">
                <PopoverSelect
                  value={resolvedAgent?.provider ?? ""}
                  options={providerOptions}
                  placeholder="Provider"
                  direction="down"
                  align="left"
                  open={openSelect === "provider"}
                  onOpenChange={(open) => setOpenSelect(open ? "provider" : null)}
                  onChange={(nextValue) => {
                    const nextProvider = availableProviders.find((p) => p.name === nextValue);
                    if (!nextProvider) return;
                    handleAgentChange({
                      provider: nextProvider.name,
                      model: nextProvider.models[0] ?? "",
                    });
                  }}
                />
                <span className="text-xs text-fg-faint">/</span>
                <PopoverSelect
                  value={resolvedAgent?.model ?? ""}
                  options={modelOptions}
                  placeholder="Model"
                  direction="down"
                  align="left"
                  open={openSelect === "model"}
                  onOpenChange={(open) => setOpenSelect(open ? "model" : null)}
                  onChange={(nextValue) => {
                    if (!resolvedAgent) return;
                    handleAgentChange({
                      provider: resolvedAgent.provider,
                      model: nextValue,
                    });
                  }}
                />
              </div>
            ) : (
              <p className="mt-4 text-xs text-fg-faint">
                No providers available. Check the CLI Setup page to ensure at
                least one provider is installed and has enabled models.
              </p>
            )}
          </div>

          {/* Simple Task pipeline */}
          <div className="rounded-lg border border-edge bg-panel p-5">
            <div className="flex items-center gap-2">
              <span className="inline-flex h-6 items-center rounded-full border border-edge bg-elevated px-2.5 text-[11px] font-medium text-fg">
                Simple Task
              </span>
              <span className="text-xs text-fg-faint">Default pipeline</span>
            </div>
            <p className="mt-3 text-xs text-fg-muted leading-5">
              The Simple Task pipeline sends prompts to a single agent and
              streams the response. New conversations use the default agent
              above; the agent can be changed per conversation in the composer
              toolbar.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

/** Filter to available providers with enabled models. */
function filterProvidersBySettings(
  providers: ProviderInfo[],
  settings: AppSettings | null,
): ProviderInfo[] {
  return providers
    .filter((p) => p.available)
    .map((p) => {
      if (!settings) return p;
      const enabled = getEnabledModels(settings, p.name);
      const models = enabled.size > 0
        ? p.models.filter((m) => enabled.has(m))
        : [];
      return { ...p, models };
    })
    .filter((p) => p.models.length > 0);
}
