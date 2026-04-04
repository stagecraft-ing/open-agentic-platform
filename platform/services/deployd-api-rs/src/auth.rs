use anyhow::{anyhow, Result};
use axum::http::HeaderMap;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
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

// Cached JWKS: stores (json_body, fetched_at)
static JWKS_CACHE: Lazy<Mutex<Option<(String, std::time::Instant)>>> =
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

    // Fetch JWKS (with 10-minute cache)
    let jwks_uri = fetch_jwks_uri(oidc_endpoint).await?;
    let jwks_json = fetch_jwks(&jwks_uri).await?;

    // Decode header to find kid
    let header = decode_header(token)?;
    let kid = header.kid.ok_or_else(|| anyhow!("Token missing kid"))?;

    // Find matching key
    let keys: serde_json::Value = serde_json::from_str(&jwks_json)?;
    let key_entry = keys["keys"]
        .as_array()
        .and_then(|arr| arr.iter().find(|k| k["kid"].as_str() == Some(&kid)))
        .ok_or_else(|| anyhow!("No matching key for kid: {}", kid))?;

    let n = key_entry["n"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing n in JWK"))?;
    let e = key_entry["e"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing e in JWK"))?;
    let decoding_key = DecodingKey::from_rsa_components(n, e)?;

    let mut validation = Validation::new(Algorithm::RS256);
    if !audience.is_empty() {
        validation.set_audience(&[audience]);
    } else {
        validation.validate_aud = false;
    }
    // Don't validate issuer — Rauthy's issuer URL varies by deployment
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

async fn fetch_jwks_uri(oidc_endpoint: &str) -> Result<String> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        oidc_endpoint.trim_end_matches('/')
    );
    let resp: serde_json::Value = reqwest::get(&url).await?.json().await?;
    resp["jwks_uri"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("No jwks_uri in OIDC discovery"))
}

async fn fetch_jwks(uri: &str) -> Result<String> {
    // Check cache
    {
        let cache = JWKS_CACHE.lock().unwrap();
        if let Some((cached, ts)) = cache.as_ref()
            && ts.elapsed() < std::time::Duration::from_secs(600)
        {
            return Ok(cached.clone());
        }
    }
    let body = reqwest::get(uri).await?.text().await?;
    {
        let mut cache = JWKS_CACHE.lock().unwrap();
        *cache = Some((body.clone(), std::time::Instant::now()));
    }
    Ok(body)
}
