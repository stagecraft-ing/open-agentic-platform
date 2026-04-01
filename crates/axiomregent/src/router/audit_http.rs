//! Seam B — fire-and-forget HTTP forwarding of audit records to the platform.

use serde_json::Value;

/// Forwards audit payloads to the platform's `POST /api/audit-records` endpoint.
/// Failures are logged to stderr but never block MCP dispatch.
pub struct AuditForwarder {
    client: reqwest::Client,
    url: String,
    token: String,
}

impl AuditForwarder {
    pub fn new(url: String, token: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to build reqwest client");
        Self { client, url, token }
    }

    /// Spawn a fire-and-forget POST. Does not block the caller.
    pub fn forward(&self, payload: Value) {
        let client = self.client.clone();
        let url = self.url.clone();
        let token = self.token.clone();
        tokio::spawn(async move {
            let res = client
                .post(&url)
                .bearer_auth(&token)
                .json(&payload)
                .send()
                .await;
            match res {
                Ok(resp) if !resp.status().is_success() => {
                    eprintln!(
                        "[audit_http] platform returned {} for POST {}",
                        resp.status(),
                        url
                    );
                }
                Err(e) => {
                    eprintln!("[audit_http] failed to POST {}: {}", url, e);
                }
                _ => {}
            }
        });
    }
}
