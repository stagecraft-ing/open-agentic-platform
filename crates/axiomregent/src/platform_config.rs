//! Platform integration configuration — reads from environment variables.
//! All values are optional; when absent, axiomregent operates in local-only mode.

/// Configuration for connecting axiomregent to the platform control plane.
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// Seam B: URL for posting audit records (e.g., `http://localhost:4000/api/audit-records`).
    pub audit_url: Option<String>,
    /// Seam A: URL for fetching policy bundles (e.g., `http://localhost:4000/api/policy-bundle`).
    pub policy_url: Option<String>,
    /// Seams C, D: Base URL for platform API (e.g., `http://localhost:4000/api`).
    pub api_url: Option<String>,
    /// Bearer token for all platform calls (M2M auth).
    pub m2m_token: Option<String>,
}

impl PlatformConfig {
    /// Load from environment variables. Missing vars yield `None` (local-only mode).
    pub fn from_env() -> Self {
        Self {
            audit_url: non_empty_env("PLATFORM_AUDIT_URL"),
            policy_url: non_empty_env("PLATFORM_POLICY_URL"),
            api_url: non_empty_env("PLATFORM_API_URL"),
            m2m_token: non_empty_env("PLATFORM_M2M_TOKEN"),
        }
    }

    /// Returns `true` when at least one platform URL is configured.
    pub fn is_connected(&self) -> bool {
        self.audit_url.is_some() || self.policy_url.is_some() || self.api_url.is_some()
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
