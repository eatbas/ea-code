import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import type { McpServer } from "../types";
import { useMcpServers } from "../hooks/useMcpServers";

function parseEnv(raw: string): Record<string, string> {
  try {
    return JSON.parse(raw) as Record<string, string>;
  } catch {
    return {};
  }
}

/** MCP server catalogue and per-CLI binding management view. */
export function McpView(): ReactNode {
  const { servers, capableClis, loading, setEnabled, setBindings, setContext7ApiKey } = useMcpServers();
  const [busy, setBusy] = useState<string | null>(null);
  const [context7ApiKey, setContext7ApiKeyValue] = useState<string>("");

  const builtinOrder = ["context7", "playwright"];
  const builtinServers = useMemo(
    () =>
      servers
        .filter((server) => server.isBuiltin && builtinOrder.includes(server.id))
        .sort((a, b) => builtinOrder.indexOf(a.id) - builtinOrder.indexOf(b.id)),
    [servers],
  );

  useEffect(() => {
    const context7 = builtinServers.find((server) => server.id === "context7");
    if (!context7) {
      setContext7ApiKeyValue("");
      return;
    }
    const env = parseEnv(context7.env);
    setContext7ApiKeyValue(env.CONTEXT7_API_KEY ?? "");
  }, [builtinServers]);

  async function toggleEnabled(server: McpServer, enabled: boolean): Promise<void> {
    setBusy(server.id);
    try {
      await setEnabled(server.id, enabled);
    } finally {
      setBusy(null);
    }
  }

  async function toggleBinding(server: McpServer, cliName: string): Promise<void> {
    const current = new Set(server.cliBindings);
    if (current.has(cliName)) current.delete(cliName); else current.add(cliName);
    setBusy(server.id);
    try {
      await setBindings(server.id, Array.from(current));
    } finally {
      setBusy(null);
    }
  }

  async function saveContext7Key(): Promise<void> {
    setBusy("context7-key");
    try {
      await setContext7ApiKey(context7ApiKey);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-4xl flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">MCP Servers</h1>
          <p className="text-sm text-[#9898b0]">Only curated servers are available: Context7 and Playwright.</p>

          {loading && <div className="text-sm text-[#9898b0]">Loading MCP servers...</div>}

          <div className="grid gap-4 md:grid-cols-2">
            {builtinServers.map((server) => {
              const isContext7 = server.id === "context7";
              return (
                <div key={server.id} className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4">
                  <div className="flex items-center justify-between gap-2">
                    <div>
                      <h3 className="text-sm font-semibold text-[#e4e4ed]">{server.name}</h3>
                      <p className="text-xs text-[#9898b0]">{server.description || "No description"}</p>
                    </div>
                    {server.isBuiltin && <span className="rounded bg-[#24243a] px-2 py-1 text-[10px] text-[#9898b0]">Built-in</span>}
                  </div>

                  <label className="mt-3 inline-flex items-center gap-2 text-xs text-[#9898b0]">
                    <input
                      type="checkbox"
                      checked={server.isEnabled}
                      onChange={(e) => void toggleEnabled(server, e.target.checked)}
                    />
                    Enabled
                  </label>

                  <div className="mt-3 flex flex-wrap gap-2">
                    {capableClis.map((cli) => {
                      const bound = server.cliBindings.includes(cli);
                      return (
                        <button
                          key={`${server.id}-${cli}`}
                          onClick={() => void toggleBinding(server, cli)}
                          className={`rounded px-2 py-1 text-xs ${bound ? "bg-[#6366f1]/20 text-[#e4e4ed]" : "bg-[#24243a] text-[#9898b0]"}`}
                        >
                          {cli}
                        </button>
                      );
                    })}
                  </div>

                  {isContext7 && (
                    <div className="mt-3 flex flex-col gap-2">
                      <label className="text-xs text-[#9898b0]">Context7 API Key</label>
                      <input
                        type="password"
                        value={context7ApiKey}
                        onChange={(e) => setContext7ApiKeyValue(e.target.value)}
                        placeholder="ctx7sk..."
                        className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]"
                      />
                      <button
                        onClick={() => void saveContext7Key()}
                        disabled={busy === "context7-key"}
                        className="self-start rounded bg-[#e4e4ed] px-3 py-1.5 text-xs font-medium text-[#0f0f14] disabled:opacity-60"
                      >
                        {busy === "context7-key" ? "Saving..." : "Save API Key"}
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
