use anyhow::{Result, anyhow};
use axum::http::HeaderMap;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Option<String>,
    pub scope: Option<String>,
    pub exp: Option<u64>,
    pub aud: Option<serde_json::Value>,
}

// Cached JWKS: stores (json_body, issuer, fetched_at)
static JWKS_CACHE: Lazy<Mutex<Option<(String, String, std::time::Instant)>>> =
    Lazy::new(|| Mutex::new(None));

pub async fn verify_jwt(
    headers: &HeaderMap,
    oidc_endpoint: &str,
    audience: &str,
) -> Result<Claims> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow!("Missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| anyhow!("Invalid Authorization header format"))?;

    // Fetch JWKS and Issuer (with 10-minute cache)
    let (jwks_json, issuer) = fetch_jwks_and_issuer(oidc_endpoint).await?;

    // Decode header to find kid
    let header = decode_header(token)?;
    let kid = header
        .kid
        .as_ref()
        .ok_or_else(|| anyhow!("Token missing kid"))?;

    // Parse JWKS and locate the key by kid. `from_jwk` handles both RSA and
    // OKP (Ed25519) parameter shapes, so we don't hand-parse n/e or x/crv.
    let jwks: JwkSet =
        serde_json::from_str(&jwks_json).map_err(|e| anyhow!("Failed to parse JWKS: {e}"))?;

    let jwk = jwks
        .find(kid)
        .ok_or_else(|| anyhow!("No matching key for kid: {kid}"))?;

    let decoding_key = DecodingKey::from_jwk(jwk)
        .map_err(|e| anyhow!("Failed to build decoding key from JWK: {e}"))?;

    // Accept both RS256 and EdDSA — Rauthy signs with Ed25519 by default, but
    // RSA keeps working for any rotated/legacy tokens still in flight. `decode`
    // enforces that the token's header alg is in this allowlist.
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.algorithms = vec![Algorithm::RS256, Algorithm::EdDSA];
    validation.set_audience(&[audience]);
    validation.set_issuer(&[&issuer]);
    validation.validate_exp = true;

    let decoded = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(decoded.claims)
}

pub fn has_scope(claims: &Claims, required: &str) -> bool {
    if required.is_empty() {
        return true;
    }
    claims
        .scope
        .as_ref()
        .map(|s| s.split_whitespace().any(|sc| sc == required))
        .unwrap_or(false)
}

async fn fetch_jwks_and_issuer(oidc_endpoint: &str) -> Result<(String, String)> {
    // Check cache
    {
        let cache = JWKS_CACHE
            .lock()
            .map_err(|e| anyhow!("JWKS cache lock poisoned: {}", e))?;
        if let Some((cached_body, cached_iss, ts)) = cache.as_ref()
            && ts.elapsed() < std::time::Duration::from_secs(600)
        {
            return Ok((cached_body.clone(), cached_iss.clone()));
        }
    }

    let url = format!(
        "{}/auth/v1/.well-known/openid-configuration",
        oidc_endpoint.trim_end_matches('/')
    );
    let resp: serde_json::Value = reqwest::get(&url).await?.json().await?;
    let jwks_uri = resp["jwks_uri"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("No jwks_uri in OIDC discovery"))?;
    let issuer = resp["issuer"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("No issuer in OIDC discovery"))?;

    let body = reqwest::get(&jwks_uri).await?.text().await?;

    {
        let mut cache = JWKS_CACHE
            .lock()
            .map_err(|e| anyhow!("JWKS cache lock poisoned: {}", e))?;
        *cache = Some((body.clone(), issuer.clone(), std::time::Instant::now()));
    }
    Ok((body, issuer))
}
