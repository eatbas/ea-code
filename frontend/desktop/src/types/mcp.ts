/** MCP server record managed by the local catalogue. */
export interface McpServer {
  id: string;
  name: string;
  description: string;
  command: string;
  args: string;
  env: string;
  isEnabled: boolean;
  isBuiltin: boolean;
  cliBindings: string[];
  createdAt: string;
  updatedAt: string;
}

/** Payload for creating an MCP server entry. */
export interface CreateMcpServerPayload {
  id?: string;
  name: string;
  description?: string;
  command: string;
  args?: string;
  env?: string;
  isEnabled?: boolean;
  cliBindings?: string[];
}

/** Payload for updating an MCP server entry. */
export interface UpdateMcpServerPayload {
  id: string;
  name: string;
  description?: string;
  command: string;
  args?: string;
  env?: string;
  isEnabled?: boolean;
  cliBindings?: string[];
}
