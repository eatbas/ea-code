use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::commands::emitter::{emit_done, emit_items};
use crate::commands::AppState;
use crate::http::api_client;
use crate::models::{ApiCliVersionInfo, ApiHealthStatus, ProviderInfo, SidecarLogEvent};
use crate::sidecar::log_buffer::SidecarLogEntry;

pub const EVENT_API_HEALTH: &str = "api_health_status";
pub const EVENT_API_PROVIDER: &str = "api_provider_info";
pub const EVENT_API_PROVIDERS_DONE: &str = "api_providers_check_complete";
pub const EVENT_API_CLI_VERSION: &str = "api_cli_version_info";
pub const EVENT_API_CLI_VERSIONS_DONE: &str = "api_versions_check_complete";

const DEFAULT_SYMPHONY_PORT: u16 = 8719;

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
    musician_count: Option<u32>,
}

#[derive(Deserialize)]
struct ProviderCapability {
    provider: String,
    available: bool,
    models: Vec<String>,
}

#[derive(Deserialize)]
struct CliVersionResponse {
    provider: String,
    current_version: Option<String>,
    latest_version: Option<String>,
    needs_update: bool,
}

pub fn symphony_base_url() -> String {
    let port = crate::storage::settings::read_settings()
        .map(|settings| {
            if settings.symphony_port == 0 {
                DEFAULT_SYMPHONY_PORT
            } else {
                settings.symphony_port
            }
        })
        .unwrap_or(DEFAULT_SYMPHONY_PORT);
    format!("http://127.0.0.1:{port}")
}

fn map_provider_info(provider: ProviderCapability) -> ProviderInfo {
    ProviderInfo {
        name: provider.provider,
        available: provider.available,
        models: provider.models,
    }
}

fn map_api_cli_version(version: CliVersionResponse) -> ApiCliVersionInfo {
    ApiCliVersionInfo {
        provider: version.provider,
        installed_version: version.current_version,
        latest_version: version.latest_version,
        up_to_date: !version.needs_update,
        available: true,
    }
}

fn map_health_success(base_url: String, body: HealthResponse) -> ApiHealthStatus {
    ApiHealthStatus {
        connected: true,
        url: base_url,
        status: Some(body.status),
        musician_count: body.musician_count,
        error: None,
    }
}

fn map_health_failure(
    base_url: String,
    status: Option<String>,
    error: Option<String>,
) -> ApiHealthStatus {
    ApiHealthStatus {
        connected: false,
        url: base_url,
        status,
        musician_count: None,
        error,
    }
}

/// Returns `true` when the symphony sidecar is reachable right now.
/// Used by the frontend to recover from a missed `sidecar_ready` event.
#[tauri::command]
pub async fn check_sidecar_ready(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.sidecar.is_healthy().await)
}

#[tauri::command]
pub async fn check_api_health(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.sidecar.ensure_running().await;
    let base_url = symphony_base_url();
    let client = api_client();
    let url = format!("{base_url}/health");

    let status = match client
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let body = response.json().await.unwrap_or(HealthResponse {
                status: "ok".to_string(),
                musician_count: None,
            });
            map_health_success(base_url, body)
        }
        Ok(response) => map_health_failure(
            base_url,
            Some(format!("HTTP {}", response.status())),
            Some(format!("symphony returned {}", response.status())),
        ),
        Err(error) => map_health_failure(base_url, None, Some(error.to_string())),
    };

    emit_items(&app, EVENT_API_HEALTH, [status]);
    Ok(())
}

#[tauri::command]
pub async fn get_api_providers(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.sidecar.ensure_running().await;
    let base_url = symphony_base_url();
    let client = api_client();
    let url = format!("{base_url}/v1/providers?all=true");

    match client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            if let Ok(providers) = response.json::<Vec<ProviderCapability>>().await {
                emit_items(
                    &app,
                    EVENT_API_PROVIDER,
                    providers.into_iter().map(map_provider_info),
                );
            }
        }
        Ok(response) => {
            eprintln!(
                "[api_health] providers endpoint returned {}",
                response.status()
            );
        }
        Err(error) => {
            eprintln!("[api_health] failed to fetch providers: {error}");
        }
    }

    emit_done(&app, EVENT_API_PROVIDERS_DONE);
    Ok(())
}

#[tauri::command]
pub async fn get_api_cli_versions(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _ = state.sidecar.ensure_running().await;
    let base_url = symphony_base_url();
    let client = api_client();
    let url = format!("{base_url}/v1/cli-versions");

    match client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            if let Ok(versions) = response.json::<Vec<CliVersionResponse>>().await {
                emit_items(
                    &app,
                    EVENT_API_CLI_VERSION,
                    versions.into_iter().map(map_api_cli_version),
                );
            }
        }
        Ok(response) => {
            eprintln!(
                "[api_health] cli-versions endpoint returned {}",
                response.status()
            );
        }
        Err(error) => {
            eprintln!("[api_health] failed to fetch CLI versions: {error}");
        }
    }

    emit_done(&app, EVENT_API_CLI_VERSIONS_DONE);
    Ok(())
}

#[tauri::command]
pub async fn update_api_cli(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: String,
) -> Result<(), String> {
    state.sidecar.ensure_running().await?;
    let base_url = symphony_base_url();
    let client = api_client();
    let url = format!("{base_url}/v1/cli-versions/{provider}/update");

    match client
        .post(&url)
        .timeout(Duration::from_secs(120))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            if let Ok(version) = response.json::<CliVersionResponse>().await {
                emit_items(&app, EVENT_API_CLI_VERSION, [map_api_cli_version(version)]);
            }
        }
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Update failed for {provider}: HTTP {status} — {body}"
            ));
        }
        Err(error) => {
            return Err(format!("Update request failed for {provider}: {error}"));
        }
    }
    Ok(())
}

/// Return all buffered sidecar log entries for retroactive retrieval.
#[tauri::command]
pub async fn get_sidecar_logs(state: State<'_, AppState>) -> Result<Vec<SidecarLogEvent>, String> {
    let entries: Vec<SidecarLogEntry> = state.sidecar.log_buffer().await.snapshot().await;
    Ok(entries
        .into_iter()
        .map(|e| SidecarLogEvent {
            stream: e.stream,
            line: e.line,
            timestamp: e.timestamp,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{map_api_cli_version, map_provider_info, CliVersionResponse, ProviderCapability};

    #[test]
    fn provider_mapping_preserves_models() {
        let info = map_provider_info(ProviderCapability {
            provider: "copilot".to_string(),
            available: true,
            models: vec!["gpt-5.4".to_string()],
        });

        assert_eq!(info.name, "copilot");
        assert!(info.available);
        assert_eq!(info.models, vec!["gpt-5.4"]);
    }

    #[test]
    fn api_cli_version_mapping_inverts_needs_update() {
        let version = map_api_cli_version(CliVersionResponse {
            provider: "copilot".to_string(),
            current_version: Some("1.0.0".to_string()),
            latest_version: Some("1.0.1".to_string()),
            needs_update: true,
        });

        assert_eq!(version.provider, "copilot");
        assert_eq!(version.installed_version.as_deref(), Some("1.0.0"));
        assert_eq!(version.latest_version.as_deref(), Some("1.0.1"));
        assert!(!version.up_to_date);
        assert!(version.available);
    }
}
