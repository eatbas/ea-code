/**
 * Tauri IPC event names — single source of truth for the frontend.
 * Must stay in sync with the Rust `const EVENT_*` declarations in
 * `commands/api_health.rs` and `commands/cli/health.rs`.
 */

/** hive-api events (emitted by `commands/api_health.rs`). */
export const API_EVENTS = {
  HEALTH_STATUS: "api_health_status",
  PROVIDER_INFO: "api_provider_info",
  PROVIDERS_COMPLETE: "api_providers_check_complete",
  CLI_VERSION_INFO: "api_cli_version_info",
  CLI_VERSIONS_COMPLETE: "api_versions_check_complete",
} as const;

/** CLI health events (emitted by `commands/cli/health.rs`). */
export const CLI_EVENTS = {
  HEALTH_STATUS: "cli_health_status",
  HEALTH_COMPLETE: "cli_health_check_complete",
  VERSION_INFO: "cli_version_info",
  VERSIONS_COMPLETE: "cli_versions_check_complete",
} as const;
