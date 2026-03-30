use std::time::Duration;

const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(500);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) async fn wait_for_health(base_url: &str) -> Result<(), String> {
    wait_for_health_with_config(base_url, HEALTH_TIMEOUT, HEALTH_POLL_INTERVAL).await
}

pub(crate) async fn wait_for_health_with_config(
    base_url: &str,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<(), String> {
    let url = format!("{base_url}/health");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|error| format!("HTTP client error: {error}"))?;

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                eprintln!("[sidecar] symphony is healthy");
                return Ok(());
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "symphony did not become healthy within {}s",
                timeout.as_secs_f32()
            ));
        }

        tokio::time::sleep(poll_interval).await;
    }
}

pub(crate) async fn is_healthy(base_url: &str) -> bool {
    let url = format!("{base_url}/health");
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    else {
        return false;
    };

    client
        .get(&url)
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::wait_for_health_with_config;

    #[tokio::test]
    async fn wait_for_health_times_out_for_unreachable_server() {
        let result = wait_for_health_with_config(
            "http://127.0.0.1:9",
            Duration::from_millis(50),
            Duration::from_millis(10),
        )
        .await;

        assert!(result.is_err());
        assert!(result
            .expect_err("health check should time out")
            .contains("did not become healthy"));
    }
}
