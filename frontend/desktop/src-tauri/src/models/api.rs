use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::{ApiCliVersionInfo, ApiHealthStatus, ProviderInfo};

    #[test]
    fn api_health_status_serialises_with_camel_case_keys() {
        let value = serde_json::to_value(ApiHealthStatus {
            connected: true,
            url: "http://127.0.0.1:8719".to_string(),
            status: Some("ok".to_string()),
            drone_count: Some(3),
            error: None,
        })
        .expect("health status should serialise");

        assert_eq!(value["connected"], true);
        assert_eq!(value["url"], "http://127.0.0.1:8719");
        assert_eq!(value["status"], "ok");
        assert_eq!(value["droneCount"], 3);
        assert!(value.get("error").is_none());
    }

    #[test]
    fn api_cli_version_info_omits_empty_optional_fields() {
        let value = serde_json::to_value(ApiCliVersionInfo {
            provider: "copilot".to_string(),
            installed_version: None,
            latest_version: None,
            up_to_date: false,
            available: true,
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
            available: true,
            models: vec!["gpt-5.4".to_string()],
        })
        .expect("provider info should serialise");

        assert_eq!(value["name"], "copilot");
        assert_eq!(value["available"], true);
        assert_eq!(value["models"][0], "gpt-5.4");
    }
}
