use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// hive-api health response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHealthStatus {
    pub connected: bool,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drone_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Provider availability info from hive-api.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    pub available: bool,
    pub models: Vec<String>,
}

/// CLI version info from hive-api.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCliVersionInfo {
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub up_to_date: bool,
    pub available: bool,
}

const EVENT_API_HEALTH: &str = "api_health_status";
const EVENT_API_PROVIDER: &str = "api_provider_info";
const EVENT_API_PROVIDERS_DONE: &str = "api_providers_check_complete";
const EVENT_API_CLI_VERSION: &str = "api_cli_version_info";
const EVENT_API_CLI_VERSIONS_DONE: &str = "api_versions_check_complete";

const DEFAULT_HIVE_API_PORT: u16 = 8719;

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
    drone_count: Option<u32>,
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

/// Resolves the hive-api base URL from settings.
fn hive_api_base_url() -> String {
    let port = crate::storage::settings::read_settings()
        .map(|s| if s.hive_api_port == 0 { DEFAULT_HIVE_API_PORT } else { s.hive_api_port })
        .unwrap_or(DEFAULT_HIVE_API_PORT);
    format!("http://127.0.0.1:{port}")
}

/// Checks hive-api connectivity and emits `api_health_status`.
#[tauri::command]
pub async fn check_api_health(app: AppHandle) -> Result<(), String> {
    let base_url = hive_api_base_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{base_url}/health");
    let status = match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: HealthResponse = resp
                .json()
                .await
                .unwrap_or(HealthResponse {
                    status: "ok".into(),
                    drone_count: None,
                });
            ApiHealthStatus {
                connected: true,
                url: base_url,
                status: Some(body.status),
                drone_count: body.drone_count,
                error: None,
            }
        }
        Ok(resp) => ApiHealthStatus {
            connected: false,
            url: base_url,
            status: Some(format!("HTTP {}", resp.status())),
            drone_count: None,
            error: Some(format!("hive-api returned {}", resp.status())),
        },
        Err(e) => ApiHealthStatus {
            connected: false,
            url: base_url,
            status: None,
            drone_count: None,
            error: Some(format!("{e}")),
        },
    };

    let _ = app.emit(EVENT_API_HEALTH, &status);
    Ok(())
}

/// Fetches available providers and models from hive-api.
#[tauri::command]
pub async fn get_api_providers(app: AppHandle) -> Result<(), String> {
    let base_url = hive_api_base_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{base_url}/v1/providers?all=true");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(providers) = resp.json::<Vec<ProviderCapability>>().await {
                for p in &providers {
                    let info = ProviderInfo {
                        name: p.provider.clone(),
                        available: p.available,
                        models: p.models.clone(),
                    };
                    let _ = app.emit(EVENT_API_PROVIDER, &info);
                }
            }
        }
        Ok(resp) => {
            eprintln!("[api_health] providers endpoint returned {}", resp.status());
        }
        Err(e) => {
            eprintln!("[api_health] failed to fetch providers: {e}");
        }
    }

    let _ = app.emit(EVENT_API_PROVIDERS_DONE, ());
    Ok(())
}

/// Fetches CLI version info from hive-api.
#[tauri::command]
pub async fn get_api_cli_versions(app: AppHandle) -> Result<(), String> {
    let base_url = hive_api_base_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{base_url}/v1/cli-versions");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(versions) = resp.json::<Vec<CliVersionResponse>>().await {
                for v in &versions {
                    let info = ApiCliVersionInfo {
                        provider: v.provider.clone(),
                        installed_version: v.current_version.clone(),
                        latest_version: v.latest_version.clone(),
                        up_to_date: !v.needs_update,
                        available: true,
                    };
                    let _ = app.emit(EVENT_API_CLI_VERSION, &info);
                }
            }
        }
        Ok(resp) => {
            eprintln!("[api_health] cli-versions endpoint returned {}", resp.status());
        }
        Err(e) => {
            eprintln!("[api_health] failed to fetch CLI versions: {e}");
        }
    }

    let _ = app.emit(EVENT_API_CLI_VERSIONS_DONE, ());
    Ok(())
}

/// Triggers a CLI update for a single provider via hive-api.
#[tauri::command]
pub async fn update_api_cli(app: AppHandle, provider: String) -> Result<(), String> {
    let base_url = hive_api_base_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{base_url}/v1/cli-versions/{provider}/update");
    match client.post(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(v) = resp.json::<CliVersionResponse>().await {
                let info = ApiCliVersionInfo {
                    provider: v.provider,
                    installed_version: v.current_version,
                    latest_version: v.latest_version,
                    up_to_date: !v.needs_update,
                    available: true,
                };
                let _ = app.emit(EVENT_API_CLI_VERSION, &info);
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Update failed for {provider}: HTTP {status} — {body}"));
        }
        Err(e) => {
            return Err(format!("Update request failed for {provider}: {e}"));
        }
    }
    Ok(())
}
