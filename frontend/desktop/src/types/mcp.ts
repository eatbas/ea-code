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

export type McpVerificationConfidence = "native" | "promptOnly" | "none";

export type McpRuntimeStatus =
  | "enabled"
  | "disabled"
  | "unknown"
  | "notInstalled"
  | "error";

export interface McpCliServerRuntimeStatus {
  serverId: string;
  status: McpRuntimeStatus;
  verificationConfidence: McpVerificationConfidence;
  message?: string;
}

export interface McpCliRuntimeStatus {
  cliName: string;
  cliInstalled: boolean;
  serverStatuses: McpCliServerRuntimeStatus[];
}

export interface RunCliMcpFixWithPromptRequest {
  cliName: string;
  serverId: string;
}

export interface McpCliFixResult {
  cliName: string;
  serverId: string;
  success: boolean;
  verificationStatus: McpRuntimeStatus;
  verificationConfidence: McpVerificationConfidence;
  message?: string;
  outputSummary: string;
}
