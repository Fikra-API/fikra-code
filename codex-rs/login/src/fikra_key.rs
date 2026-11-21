//! Verifies a Fikra API key against the Fikra API before it is saved.

use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;

const FIKRA_KEY_PROBE_URL: &str = "https://fikraapi.co.ke/v1/chat/completions";

/// Returns `Err` with a user-facing message when the Fikra API rejects the key
/// (HTTP 401) or cannot be reached. A deliberately unknown model is requested
/// so a valid key is authenticated without running inference.
pub async fn validate_fikra_api_key(api_key: &str) -> Result<(), String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("no API key provided".to_string());
    }
    let factory = HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);
    let client = factory
        .build_client(FIKRA_KEY_PROBE_URL, ClientRouteClass::Other)
        .map_err(|err| format!("could not build HTTP client: {err}"))?;
    let body = serde_json::json!({
        "model": "__fikra_key_probe__",
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 1
    });
    let response = client
        .post(FIKRA_KEY_PROBE_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("could not reach the Fikra API: {err}"))?;
    match response.status().as_u16() {
        401 | 403 => Err("the Fikra API rejected this key. Get a key at https://fikraapi.co.ke/dashboard".to_string()),
        _ => Ok(()),
    }
}
