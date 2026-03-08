-- Phase B remainder: settings parity for agent max turns and execution mode.
ALTER TABLE settings ADD COLUMN agent_max_turns INTEGER NOT NULL DEFAULT 25;
ALTER TABLE settings ADD COLUMN mode TEXT NOT NULL DEFAULT 'workspace-write';
