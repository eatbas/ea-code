-- Phase C + D: MCP catalogue/bindings and startup CLI update policy settings.

ALTER TABLE settings ADD COLUMN update_cli_on_run BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN fail_on_cli_update_error BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN cli_update_timeout_ms INTEGER NOT NULL DEFAULT 600000;

CREATE TABLE mcp_servers (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    command TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '[]',
    env TEXT NOT NULL DEFAULT '{}',
    is_enabled BOOLEAN NOT NULL DEFAULT 0,
    is_builtin BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE cli_mcp_bindings (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    cli_name TEXT NOT NULL,
    mcp_server_id TEXT NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(cli_name, mcp_server_id)
);

CREATE INDEX idx_mcp_servers_enabled ON mcp_servers(is_enabled, is_builtin);
CREATE INDEX idx_cli_mcp_bindings_cli ON cli_mcp_bindings(cli_name, mcp_server_id);

-- Built-in MCP catalogue.
INSERT INTO mcp_servers (id, name, description, command, args, env, is_enabled, is_builtin)
VALUES
  ('ea-code-history', 'EA Code History', 'Session-aware local history tools from ea-code-mcp.', 'ea-code-mcp', '[]', '{}', 1, 1),
  ('context7', 'Context7', 'Library and API documentation lookup.', 'npx', '["-y","@upstash/context7-mcp"]', '{}', 0, 1),
  ('github', 'GitHub', 'GitHub API access for repositories and pull requests.', 'npx', '["-y","@modelcontextprotocol/server-github"]', '{}', 0, 1),
  ('filesystem', 'Filesystem', 'Local file system exploration tools.', 'npx', '["-y","@modelcontextprotocol/server-filesystem"]', '{}', 0, 1),
  ('brave-search', 'Brave Search', 'Web search via Brave Search MCP.', 'npx', '["-y","@anthropic/brave-search-mcp"]', '{}', 0, 1),
  ('slack', 'Slack', 'Slack workspace tools for messages and channels.', 'npx', '["-y","@anthropic/slack-mcp"]', '{}', 0, 1);

-- Default bindings.
INSERT INTO cli_mcp_bindings (cli_name, mcp_server_id) VALUES
  ('claude', 'ea-code-history'),
  ('claude', 'context7'),
  ('codex', 'context7');
