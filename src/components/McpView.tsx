import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import type { CreateMcpServerPayload, McpServer, UpdateMcpServerPayload } from "../types";
import { useMcpServers } from "../hooks/useMcpServers";

interface McpDraft {
  name: string;
  description: string;
  command: string;
  args: string;
  env: string;
  isEnabled: boolean;
  cliBindings: string[];
}

function toDraft(server: McpServer): McpDraft {
  return {
    name: server.name,
    description: server.description,
    command: server.command,
    args: server.args,
    env: server.env,
    isEnabled: server.isEnabled,
    cliBindings: server.cliBindings,
  };
}

/** MCP server catalogue and per-CLI binding management view. */
export function McpView(): ReactNode {
  const { servers, capableClis, loading, error, createServer, updateServer, deleteServer, setEnabled, setBindings } = useMcpServers();
  const [drafts, setDrafts] = useState<Record<string, McpDraft>>({});
  const [newServer, setNewServer] = useState<CreateMcpServerPayload>({
    name: "",
    description: "",
    command: "npx",
    args: '["-y","@upstash/context7-mcp"]',
    env: "{}",
    isEnabled: false,
    cliBindings: ["claude"],
  });
  const [busy, setBusy] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);

  useEffect(() => {
    const next: Record<string, McpDraft> = {};
    for (const server of servers) {
      next[server.id] = toDraft(server);
    }
    setDrafts(next);
  }, [servers]);

  async function handleCreate(): Promise<void> {
    if (!newServer.name?.trim() || !newServer.command?.trim()) {
      setLocalError("Name and command are required for custom MCP servers.");
      return;
    }
    setBusy("create");
    setLocalError(null);
    try {
      await createServer(newServer);
      setNewServer({
        name: "",
        description: "",
        command: "npx",
        args: '["-y","@upstash/context7-mcp"]',
        env: "{}",
        isEnabled: false,
        cliBindings: ["claude"],
      });
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleSave(serverId: string): Promise<void> {
    const draft = drafts[serverId];
    if (!draft) return;
    const payload: UpdateMcpServerPayload = {
      id: serverId,
      name: draft.name,
      description: draft.description,
      command: draft.command,
      args: draft.args,
      env: draft.env,
      isEnabled: draft.isEnabled,
      cliBindings: draft.cliBindings,
    };
    setBusy(serverId);
    setLocalError(null);
    try {
      await updateServer(payload);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleDelete(serverId: string): Promise<void> {
    setBusy(serverId);
    setLocalError(null);
    try {
      await deleteServer(serverId);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function toggleEnabled(server: McpServer, enabled: boolean): Promise<void> {
    setBusy(server.id);
    setLocalError(null);
    try {
      await setEnabled(server.id, enabled);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function toggleBinding(server: McpServer, cliName: string): Promise<void> {
    const current = new Set(server.cliBindings);
    if (current.has(cliName)) current.delete(cliName); else current.add(cliName);
    setBusy(server.id);
    setLocalError(null);
    try {
      await setBindings(server.id, Array.from(current));
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-4xl flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">MCP Servers</h1>
          <p className="text-sm text-[#9898b0]">Enable servers and bind them per CLI. Built-ins are seeded from the catalogue.</p>
          {(error || localError) && (
            <div className="rounded border border-[#ef4444]/30 bg-[#ef4444]/10 px-3 py-2 text-sm text-[#ef4444]">
              {error || localError}
            </div>
          )}

          <div className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4">
            <h2 className="text-sm font-semibold text-[#e4e4ed]">Add Custom Server</h2>
            <div className="mt-3 grid gap-3 md:grid-cols-2">
              <input value={newServer.name ?? ""} onChange={(e) => setNewServer((p) => ({ ...p, name: e.target.value }))} placeholder="Name" className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]" />
              <input value={newServer.command ?? ""} onChange={(e) => setNewServer((p) => ({ ...p, command: e.target.value }))} placeholder="Command" className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]" />
              <input value={newServer.description ?? ""} onChange={(e) => setNewServer((p) => ({ ...p, description: e.target.value }))} placeholder="Description" className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed] md:col-span-2" />
              <input value={newServer.args ?? "[]"} onChange={(e) => setNewServer((p) => ({ ...p, args: e.target.value }))} placeholder='Args JSON, e.g. ["-y","pkg"]' className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 font-mono text-xs text-[#e4e4ed]" />
              <input value={newServer.env ?? "{}"} onChange={(e) => setNewServer((p) => ({ ...p, env: e.target.value }))} placeholder='Env JSON, e.g. {"KEY":"value"}' className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 font-mono text-xs text-[#e4e4ed]" />
            </div>
            <button onClick={() => void handleCreate()} disabled={busy === "create"} className="mt-3 rounded bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] hover:bg-white disabled:opacity-60">
              {busy === "create" ? "Creating..." : "Create Server"}
            </button>
          </div>

          {loading && <div className="text-sm text-[#9898b0]">Loading MCP servers...</div>}

          <div className="grid gap-4 md:grid-cols-2">
            {servers.map((server) => {
              const draft = drafts[server.id] ?? toDraft(server);
              const isBusy = busy === server.id;
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
                    <input type="checkbox" checked={server.isEnabled} onChange={(e) => void toggleEnabled(server, e.target.checked)} />
                    Enabled
                  </label>

                  <div className="mt-3 flex flex-wrap gap-2">
                    {capableClis.map((cli) => {
                      const bound = server.cliBindings.includes(cli);
                      return (
                        <button key={`${server.id}-${cli}`} onClick={() => void toggleBinding(server, cli)} className={`rounded px-2 py-1 text-xs ${bound ? "bg-[#6366f1]/20 text-[#e4e4ed]" : "bg-[#24243a] text-[#9898b0]"}`}>
                          {cli}
                        </button>
                      );
                    })}
                  </div>

                  {server.isBuiltin ? (
                    <div className="mt-3 rounded bg-[#0f0f14] px-3 py-2 text-[11px] text-[#9898b0]">
                      <code>{server.command} {server.args}</code>
                    </div>
                  ) : (
                    <div className="mt-3 flex flex-col gap-2">
                      <input value={draft.name} onChange={(e) => setDrafts((p) => ({ ...p, [server.id]: { ...draft, name: e.target.value } }))} className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]" />
                      <input value={draft.description} onChange={(e) => setDrafts((p) => ({ ...p, [server.id]: { ...draft, description: e.target.value } }))} className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-xs text-[#e4e4ed]" />
                      <input value={draft.command} onChange={(e) => setDrafts((p) => ({ ...p, [server.id]: { ...draft, command: e.target.value } }))} className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 text-sm text-[#e4e4ed]" />
                      <input value={draft.args} onChange={(e) => setDrafts((p) => ({ ...p, [server.id]: { ...draft, args: e.target.value } }))} className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 font-mono text-xs text-[#e4e4ed]" />
                      <input value={draft.env} onChange={(e) => setDrafts((p) => ({ ...p, [server.id]: { ...draft, env: e.target.value } }))} className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-2 font-mono text-xs text-[#e4e4ed]" />
                      <div className="flex gap-2">
                        <button onClick={() => void handleSave(server.id)} disabled={isBusy} className="rounded bg-[#e4e4ed] px-3 py-1.5 text-xs font-medium text-[#0f0f14] disabled:opacity-60">Save</button>
                        <button onClick={() => void handleDelete(server.id)} disabled={isBusy} className="rounded border border-[#ef4444]/50 bg-[#ef4444]/10 px-3 py-1.5 text-xs font-medium text-[#ef4444] disabled:opacity-60">Delete</button>
                      </div>
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
