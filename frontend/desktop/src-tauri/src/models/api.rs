use serde::{Deserialize, Serialize};

/// Symphony health response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHealthStatus {
    pub connected: bool,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub musicians_booted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub musician_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Provider option choice exposed by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOptionChoice {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Provider option definition exposed by the API for a specific model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOptionDefinition {
    pub key: String,
    pub label: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub choices: Vec<ProviderOptionChoice>,
}

/// Detailed model information from Symphony `/v1/models`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDetail {
    pub provider: String,
    pub model: String,
    pub ready: bool,
    pub busy: bool,
    pub supports_resume: bool,
    pub provider_options_schema: Vec<ProviderOptionDefinition>,
}

/// Provider availability info from Symphony.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    pub enabled: bool,
    pub available: bool,
    pub models: Vec<String>,
    pub supports_resume: bool,
    pub supports_model_override: bool,
    pub session_reference_format: String,
}

/// CLI version info from Symphony.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCliVersionInfo {
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub up_to_date: bool,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_check_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_skipped_reason: Option<String>,
}

/// Sidecar stdout/stderr log entry sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarLogEvent {
    pub stream: String,
    pub line: String,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::{ApiCliVersionInfo, ApiHealthStatus, ProviderInfo};

    #[test]
    fn api_health_status_serialises_with_camel_case_keys() {
        let value = serde_json::to_value(ApiHealthStatus {
            connected: true,
            url: "http://127.0.0.1:8719".to_string(),
            status: Some("ok".to_string()),
            config_path: None,
            shell_path: None,
            bash_version: None,
            musicians_booted: None,
            musician_count: Some(3),
            details: None,
            error: None,
        })
        .expect("health status should serialise");

        assert_eq!(value["connected"], true);
        assert_eq!(value["url"], "http://127.0.0.1:8719");
        assert_eq!(value["status"], "ok");
        assert_eq!(value["musicianCount"], 3);
        assert!(value.get("error").is_none());
    }

    #[test]
    fn api_cli_version_info_omits_empty_optional_fields() {
        let value = serde_json::to_value(ApiCliVersionInfo {
            provider: "copilot".to_string(),
            executable: None,
            installed_version: None,
            latest_version: None,
            up_to_date: false,
            available: true,
            needs_update: None,
            last_checked: None,
            next_check_at: None,
            auto_update: None,
            last_updated: None,
            update_skipped_reason: None,
        })
        .expect("api cli version info should serialise");

        assert_eq!(value["provider"], "copilot");
        assert_eq!(value["upToDate"], false);
        assert_eq!(value["available"], true);
        assert!(value.get("installedVersion").is_none());
        assert!(value.get("latestVersion").is_none());
    }

    #[test]
    fn provider_info_serialises_models() {
        let value = serde_json::to_value(ProviderInfo {
            name: "copilot".to_string(),
            executable: Some("copilot".to_string()),
            enabled: true,
            available: true,
            models: vec!["gpt-5.4".to_string()],
            supports_resume: false,
            supports_model_override: true,
            session_reference_format: "opaque-string".to_string(),
        })
        .expect("provider info should serialise");

        assert_eq!(value["name"], "copilot");
        assert_eq!(value["available"], true);
        assert_eq!(value["models"][0], "gpt-5.4");
    }
}
