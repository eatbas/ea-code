DROP TABLE IF EXISTS cli_mcp_bindings;
DROP TABLE IF EXISTS mcp_servers;

-- SQLite does not support DROP COLUMN prior to 3.35.0;
-- these columns are ignored when rolling back.
