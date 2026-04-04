#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub oidc_endpoint: String,
    pub audience: String,
    pub required_scope: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            oidc_endpoint: std::env::var("OIDC_ENDPOINT")
                .or_else(|_| std::env::var("LOGTO_ENDPOINT"))
                .unwrap_or_default(),
            audience: std::env::var("DEPLOYD_AUDIENCE").unwrap_or_default(),
            required_scope: std::env::var("DEPLOYD_REQUIRED_SCOPE").unwrap_or_default(),
        }
    }
}
