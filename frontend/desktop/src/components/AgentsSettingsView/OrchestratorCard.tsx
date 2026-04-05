import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { AppSettings, OrchestratorSettings, PipelineAgent, ProviderInfo } from "../../types";
import { PipelineAgentRow } from "./PipelineAgentRow";
import { useToast } from "../shared/Toast";

/** Model substrings that indicate a fast/cheap model suitable for orchestration. */
const FAST_MODEL_HINTS = ["haiku", "mini", "flash", "turbo", "glm-5"];

interface OrchestratorCardProps {
  settings: AppSettings;
  providers: ProviderInfo[];
  onSave: (settings: AppSettings) => void;
}

function fastProviders(providers: ProviderInfo[]): ProviderInfo[] {
  return providers
    .map((p) => ({
      ...p,
      models: p.models.filter((m) =>
        FAST_MODEL_HINTS.some((hint) => m.toLowerCase().includes(hint)),
      ),
    }))
    .filter((p) => p.models.length > 0);
}

function defaultFastAgent(providers: ProviderInfo[]): PipelineAgent {
  const fast = fastProviders(providers);
  const first = fast[0] ?? providers[0];
  return { provider: first?.name ?? "", model: first?.models[0] ?? "" };
}

function ensureOrchestrator(
  settings: AppSettings,
  providers: ProviderInfo[],
): OrchestratorSettings {
  if (settings.orchestrator) return settings.orchestrator;
  return { agent: defaultFastAgent(providers), maxIterations: 3 };
}

export function OrchestratorCard({
  settings,
  providers,
  onSave,
}: OrchestratorCardProps): ReactNode {
  const toast = useToast();
  const [openSelect, setOpenSelect] = useState<string | null>(null);
  const orchestrator = ensureOrchestrator(settings, providers);

  const fastOnly = useMemo(() => fastProviders(providers), [providers]);

  // Auto-persist default orchestrator settings when the card renders for the
  // first time and no orchestrator has been configured yet.  Without this the
  // user sees a pre-filled card but `settings.orchestrator` remains null and
  // the pipeline skips the orchestrator stage entirely.
  useEffect(() => {
    if (!settings.orchestrator && providers.length > 0) {
      onSave({ ...settings, orchestrator: { agent: defaultFastAgent(providers), maxIterations: 3 } });
    }
    // Only run on mount / when providers become available.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [providers.length > 0]);

  const save = useCallback(
    (next: OrchestratorSettings) => {
      onSave({ ...settings, orchestrator: next });
      toast.success("Orchestrator updated.");
    },
    [settings, onSave, toast],
  );

  if (providers.length === 0) {
    return null;
  }

  return (
    <div className="rounded-lg border border-edge bg-panel p-5">
      <div className="flex items-center gap-2">
        <span className="inline-flex items-center rounded-lg border border-edge bg-elevated px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-fg">
          Orchestrator
        </span>
      </div>
      <p className="mt-3 text-xs text-fg-muted">
        Enhances prompts and routes to the right pipeline.
        Reads reviewer output to decide if more iterations are needed.
      </p>

      <div className="mt-6 flex items-start gap-14">
        {/* Fast agent */}
        <div>
          <p className="mb-4 text-xs font-semibold uppercase tracking-wider text-fg">
            Fast Agent
          </p>
          <PipelineAgentRow
            agent={orchestrator.agent}
            providers={fastOnly.length > 0 ? fastOnly : providers}
            slotKey="orchestrator"
            openSelect={openSelect}
            onOpenSelectChange={setOpenSelect}
            onChange={(agent) => save({ ...orchestrator, agent })}
          />
        </div>

        {/* Max iterations */}
        <div>
          <p className="mb-4 text-xs font-semibold uppercase tracking-wider text-fg">
            Max Iterations
          </p>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => save({ ...orchestrator, maxIterations: Math.max(1, orchestrator.maxIterations - 1) })}
              disabled={orchestrator.maxIterations <= 1}
              className="flex h-9 w-9 items-center justify-center rounded-lg border border-edge-strong bg-input-bg text-sm text-fg transition-colors hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-40"
            >
              -
            </button>
            <span className="w-8 text-center text-sm font-semibold text-fg">
              {orchestrator.maxIterations}
            </span>
            <button
              type="button"
              onClick={() => save({ ...orchestrator, maxIterations: Math.min(10, orchestrator.maxIterations + 1) })}
              disabled={orchestrator.maxIterations >= 10}
              className="flex h-9 w-9 items-center justify-center rounded-lg border border-edge-strong bg-input-bg text-sm text-fg transition-colors hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-40"
            >
              +
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
