import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  AppSettings,
  AgentSelection,
  ConversationDetail,
  ProviderInfo,
  WorkspaceInfo,
} from "../../types";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useSettings } from "../../hooks/useSettings";
import { ConversationComposer } from "./ConversationComposer";
import { formatAssistantLabel, sortProvidersByDisplayName } from "../shared/constants";
import { WorkspaceFooter } from "../shared/WorkspaceFooter";
import { useToast } from "../shared/Toast";
import { getEnabledModels } from "../../utils/modelSettings";
import { parseAgentSelection } from "../../utils/agentSettings";

interface SimpleTaskViewProps {
  workspace: WorkspaceInfo;
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  activePromptDraft: string;
  sending: boolean;
  stopping: boolean;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  onPromptDraftChange: (prompt: string) => void;
  onSendPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  onStopConversation: () => Promise<void>;
}

export function SimpleTaskView({
  workspace,
  activeConversation,
  activeDraft,
  activePromptDraft,
  sending,
  stopping,
  onOpenProjectFolder,
  onOpenInVsCode,
  onPromptDraftChange,
  onSendPrompt,
  onStopConversation,
}: SimpleTaskViewProps): ReactNode {
  const toast = useToast();
  const { providers, checkHealth } = useApiHealth();
  const { settings } = useSettings();
  const [selectedAgent, setSelectedAgent] = useState<AgentSelection | null>(null);
  const prevConversationIdRef = useRef<string | null>(null);

  const availableProviders = useMemo(
    () => sortProvidersByDisplayName(filterProvidersBySettings(providers, settings)),
    [providers, settings],
  );

  useEffect(() => {
    checkHealth();
  }, [checkHealth, workspace.path]);

  // Reset selection when leaving an existing conversation so the
  // default agent is re-applied for the next new conversation.
  useEffect(() => {
    const currentId = activeConversation?.summary.id ?? null;
    const prevId = prevConversationIdRef.current;
    prevConversationIdRef.current = currentId;

    if (prevId !== null && currentId !== prevId) {
      setSelectedAgent(null);
    }
  }, [activeConversation]);

  useEffect(() => {
    if (activeConversation) {
      setSelectedAgent(activeConversation.summary.agent);
      return;
    }

    // Wait for settings to load before making any selection so we
    // don't fall back to the first provider before the default is known.
    if (!settings) return;

    const selectionIsValid = selectedAgent
      ? availableProviders.some((provider) => (
        provider.name === selectedAgent.provider
        && provider.models.includes(selectedAgent.model)
      ))
      : false;

    if (selectionIsValid) return;

    // Try the configured default agent from settings.
    const defaultAgent = parseAgentSelection(settings.defaultAgent);
    if (defaultAgent) {
      const defaultProvider = availableProviders.find(
        (p) => p.name === defaultAgent.provider,
      );
      if (defaultProvider?.models.includes(defaultAgent.model)) {
        setSelectedAgent(defaultAgent);
        return;
      }
    }

    // Fall back to first available provider/model.
    if (availableProviders[0]) {
      setSelectedAgent({
        provider: availableProviders[0].name,
        model: availableProviders[0].models[0] ?? "",
      });
    }
  }, [activeConversation, availableProviders, selectedAgent, settings]);

  const activeRunning = activeConversation?.summary.status === "running";

  const handleSend = useCallback(async (prompt: string) => {
    const agent = activeConversation?.summary.agent ?? selectedAgent;
    if (!agent) {
      return;
    }
    await onSendPrompt(prompt, agent);
  }, [activeConversation, selectedAgent, onSendPrompt]);

  const handleFooterError = useCallback(() => {
    toast.error("Failed to open project action.");
  }, [toast]);

  const promptHistory = useMemo(
    () => activeConversation?.messages
      .filter((message) => message.role === "user")
      .map((message) => message.content) ?? [],
    [activeConversation],
  );

  return (
    <div className="flex h-full min-h-0 bg-surface">
      <div className="flex min-h-0 flex-1 flex-col">
        <div className="border-b border-edge bg-[linear-gradient(180deg,#1a1a1c_0%,#101011_100%)] px-5 py-4">
          <p className="text-lg font-semibold text-fg">
            {activeConversation?.summary.title ?? "New conversation"}
          </p>
          {!activeConversation && (
            <p className="mt-1 text-sm text-fg-muted">{workspace.path}</p>
          )}
        </div>

        {workspace.isGitRepo && workspace.eaCodeIgnored === false && (
          <div className="border-b border-[#5f6f2c]/30 bg-[#18210f] px-5 py-3 text-sm text-[#d5f18b]">
            `.ea-code/` is not currently ignored in this repository. ea-code will attempt to add it to `.gitignore`.
          </div>
        )}

        <div className="flex-1 overflow-y-auto px-5 py-5">
          {activeConversation ? (
            <div className="mx-auto flex w-full max-w-4xl flex-col gap-4">
              {activeConversation.messages.map((message) => (
                <div
                  key={message.id}
                  className={`max-w-3xl rounded-2xl px-4 py-3 text-sm leading-6 ${
                    message.role === "user"
                      ? "ml-auto border border-edge-strong bg-elevated text-fg"
                      : "mr-auto border border-edge bg-panel text-fg"
                  }`}
                >
                  <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                    {message.role === "assistant" && activeConversation
                      ? `Assistant - ${formatAssistantLabel(
                        activeConversation.summary.agent.provider,
                        activeConversation.summary.agent.model,
                      )}`
                      : message.role}
                  </p>
                  <p className="whitespace-pre-wrap">{message.content}</p>
                </div>
              ))}
              {activeDraft && (
                <div className="mr-auto max-w-3xl rounded-2xl border border-dashed border-edge-strong bg-[#1a1a1c] px-4 py-3 text-sm leading-6 text-fg">
                  <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                    {activeConversation
                      ? `Assistant - ${formatAssistantLabel(
                        activeConversation.summary.agent.provider,
                        activeConversation.summary.agent.model,
                      )}`
                      : "Assistant"}
                  </p>
                  <p className="whitespace-pre-wrap">{activeDraft}</p>
                </div>
              )}
              {activeConversation.summary.error && activeConversation.summary.status === "failed" && (
                <div className="mr-auto max-w-3xl rounded-2xl border border-[#7f1d1d] bg-[#2a1418] px-4 py-3 text-sm text-[#fecaca]">
                  {activeConversation.summary.error}
                </div>
              )}
            </div>
          ) : (
            <div className="mx-auto flex h-full max-w-3xl items-center justify-center">
              <div className="rounded-3xl border border-edge bg-panel px-8 py-10 text-center shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
                <img src="/logo.png" alt="EA Code logo" className="mx-auto mb-4 h-14 w-14 object-contain" />
                <p className="text-xl font-semibold text-fg">Start a new conversation</p>
                <p className="mt-2 text-sm text-fg-muted">
                  Pick an agent in the composer and send the first prompt to start a conversation.
                </p>
              </div>
            </div>
          )}
        </div>

        <ConversationComposer
          providers={availableProviders}
          agent={activeConversation?.summary.agent ?? selectedAgent}
          prompt={activePromptDraft}
          promptHistory={promptHistory}
          locked={Boolean(activeConversation)}
          sending={sending}
          stopping={stopping}
          activeRunning={Boolean(activeRunning)}
          onAgentChange={setSelectedAgent}
          onPromptChange={onPromptDraftChange}
          onSend={handleSend}
          onStop={onStopConversation}
        />

        <div className="px-5 pb-3 pt-0">
          <div className="mx-auto flex w-full max-w-5xl">
            <WorkspaceFooter
              path={workspace.path}
              onOpenProjectFolder={onOpenProjectFolder}
              onOpenInVsCode={onOpenInVsCode}
              onError={handleFooterError}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

function filterProvidersBySettings(
  providers: ProviderInfo[],
  settings: AppSettings | null,
): ProviderInfo[] {
  return providers
    .filter((provider) => provider.available)
    .map((provider) => {
      if (!settings) {
        return provider;
      }

      const enabledModels = getEnabledModels(settings, provider.name);
      const models = enabledModels.size > 0
        ? provider.models.filter((model) => enabledModels.has(model))
        : [];

      return {
        ...provider,
        models,
      };
    })
    .filter((provider) => provider.models.length > 0);
}
