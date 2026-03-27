import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import type {
  AgentSelection,
  ConversationDetail,
  WorkspaceInfo,
} from "../../types";
import { useApiHealth } from "../../hooks/useApiHealth";
import { ConversationComposer } from "./ConversationComposer";
import { providerDisplayName } from "../shared/constants";
import { WorkspaceFooter } from "../shared/WorkspaceFooter";
import { useToast } from "../shared/Toast";

interface SimpleTaskViewProps {
  workspace: WorkspaceInfo;
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  sending: boolean;
  stopping: boolean;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  onSendPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  onStopConversation: () => Promise<void>;
}

export function SimpleTaskView({
  workspace,
  activeConversation,
  activeDraft,
  sending,
  stopping,
  onOpenProjectFolder,
  onOpenInVsCode,
  onSendPrompt,
  onStopConversation,
}: SimpleTaskViewProps): ReactNode {
  const toast = useToast();
  const { providers, checkHealth } = useApiHealth();
  const [selectedAgent, setSelectedAgent] = useState<AgentSelection | null>(null);

  const availableProviders = useMemo(
    () => providers.filter((provider) => provider.available && provider.models.length > 0),
    [providers],
  );

  useEffect(() => {
    checkHealth();
  }, [checkHealth, workspace.path]);

  useEffect(() => {
    if (activeConversation) {
      setSelectedAgent(activeConversation.summary.agent);
      return;
    }
    if (!selectedAgent && availableProviders[0]) {
      setSelectedAgent({
        provider: availableProviders[0].name,
        model: availableProviders[0].models[0] ?? "",
      });
    }
  }, [activeConversation, availableProviders, selectedAgent]);

  const activeRunning = activeConversation?.summary.status === "running";

  return (
    <div className="flex h-full min-h-0 bg-[#0f0f14]">
      <div className="flex min-h-0 flex-1 flex-col">
        <div className="border-b border-[#2e2e48] bg-[linear-gradient(180deg,#181822_0%,#12121a_100%)] px-5 py-4">
          <p className="text-lg font-semibold text-[#e4e4ed]">
            {activeConversation?.summary.title ?? "New conversation"}
          </p>
          <p className="mt-1 text-sm text-[#9898b0]">
            {activeConversation
              ? `${providerDisplayName(activeConversation.summary.agent.provider)} · ${activeConversation.summary.agent.model}`
              : workspace.path}
          </p>
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
                      ? "ml-auto border border-[#3a3a5a] bg-[#24243a] text-[#f1f0ff]"
                      : "mr-auto border border-[#2e2e48] bg-[#1a1a24] text-[#e4e4ed]"
                  }`}
                >
                  <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-[#8f91a7]">
                    {message.role}
                  </p>
                  <p className="whitespace-pre-wrap">{message.content}</p>
                </div>
              ))}
              {activeDraft && (
                <div className="mr-auto max-w-3xl rounded-2xl border border-dashed border-[#3a3a5a] bg-[#181824] px-4 py-3 text-sm leading-6 text-[#e4e4ed]">
                  <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-[#8f91a7]">
                    Assistant
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
              <div className="rounded-3xl border border-[#2e2e48] bg-[#1a1a24] px-8 py-10 text-center shadow-[0_0_0_1px_rgba(46,46,72,0.24)]">
                <img src="/logo.png" alt="EA Code logo" className="mx-auto mb-4 h-14 w-14 object-contain" />
                <p className="text-xl font-semibold text-[#e4e4ed]">Start a new simple task</p>
                <p className="mt-2 text-sm text-[#9898b0]">
                  Pick an agent in the composer and send the first prompt to create a conversation.
                </p>
              </div>
            </div>
          )}
        </div>

        <ConversationComposer
          providers={availableProviders}
          agent={activeConversation?.summary.agent ?? selectedAgent}
          locked={Boolean(activeConversation)}
          sending={sending}
          stopping={stopping}
          activeRunning={Boolean(activeRunning)}
          onAgentChange={setSelectedAgent}
          onSend={async (prompt) => {
            const agent = activeConversation?.summary.agent ?? selectedAgent;
            if (!agent) {
              return;
            }
            await onSendPrompt(prompt, agent);
          }}
          onStop={onStopConversation}
        />

        <div className="border-t border-[#2e2e48] px-5 py-4">
          <div className="mx-auto flex w-full max-w-5xl">
            <WorkspaceFooter
              path={workspace.path}
              onOpenProjectFolder={onOpenProjectFolder}
              onOpenInVsCode={onOpenInVsCode}
              onError={() => {
                toast.error("Failed to open project action.");
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
