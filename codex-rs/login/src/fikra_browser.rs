//! `fikra login` through the browser.
//!
//! The CLI starts a sign-in session on the Fikra API with a PKCE challenge,
//! opens `fikraapi.co.ke/code/plans` where the user buys a plan or continues
//! free, and polls until the site approves the session. Only this process
//! holds the PKCE verifier, so a leaked browser URL cannot be redeemed.

use std::time::Duration;
use std::time::Instant;

use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use serde::Deserialize;

use crate::pkce::generate_pkce;

const FIKRA_SITE: &str = "https://fikraapi.co.ke";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// What the user needs to see while the browser is open.
pub struct FikraBrowserPrompt {
    pub verification_url: String,
    pub user_code: String,
    pub browser_opened: bool,
}

/// The key the site issued and the plan it belongs to (None = free).
pub struct FikraBrowserLogin {
    pub api_key: String,
    pub plan: Option<String>,
}

#[derive(Deserialize)]
struct StartResponse {
    session_id: String,
    user_code: String,
    verification_url: String,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct PollResponse {
    status: String,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}

fn site_base() -> String {
    std::env::var("FIKRA_SITE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| FIKRA_SITE.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Runs the browser sign-in. `on_prompt` is called once the URL is known so
/// the caller can print it (the browser may not open on SSH or headless
/// machines; the user then opens the URL on any device).
pub async fn fikra_browser_login(
    open_browser: bool,
    on_prompt: impl FnOnce(&FikraBrowserPrompt),
) -> Result<FikraBrowserLogin, String> {
    let base = site_base();
    let start_url = format!("{base}/api/code/auth/start");
    let poll_url = format!("{base}/api/code/auth/poll");
    let factory = HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);
    let client = factory
        .build_client(&start_url, ClientRouteClass::Other)
        .map_err(|err| format!("could not build HTTP client: {err}"))?;

    let pkce = generate_pkce();
    let response = client
        .post(&start_url)
        .json(&serde_json::json!({
            "code_challenge": pkce.code_challenge,
            "client": format!("fikra-code/{}", env!("CARGO_PKG_VERSION")),
        }))
        .send()
        .await
        .map_err(|err| format!("could not reach the Fikra API: {err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "the Fikra API refused to start sign-in (HTTP {})",
            response.status().as_u16()
        ));
    }
    let start: StartResponse = response
        .json()
        .await
        .map_err(|err| format!("unexpected sign-in response: {err}"))?;

    let browser_opened = open_browser && webbrowser::open(&start.verification_url).is_ok();
    on_prompt(&FikraBrowserPrompt {
        verification_url: start.verification_url.clone(),
        user_code: start.user_code.clone(),
        browser_opened,
    });

    let interval = Duration::from_secs(start.interval.unwrap_or(3).clamp(2, 10));
    let started = Instant::now();
    let mut transient_errors = 0u32;
    loop {
        if started.elapsed() > LOGIN_TIMEOUT {
            return Err("sign-in timed out. Run `fikra login` again.".to_string());
        }
        tokio::time::sleep(interval).await;
        let sent = client
            .post(&poll_url)
            .json(&serde_json::json!({
                "session_id": start.session_id,
                "code_verifier": pkce.code_verifier,
            }))
            .send()
            .await;
        let response = match sent {
            Ok(response) => response,
            Err(err) => {
                transient_errors += 1;
                if transient_errors > 20 {
                    return Err(format!("lost connection to the Fikra API: {err}"));
                }
                continue;
            }
        };
        let status_code = response.status().as_u16();
        let poll: PollResponse = match response.json().await {
            Ok(poll) => poll,
            Err(_) if status_code >= 500 || status_code == 429 => {
                transient_errors += 1;
                continue;
            }
            Err(err) => return Err(format!("unexpected sign-in response: {err}")),
        };
        transient_errors = 0;
        match poll.status.as_str() {
            "pending" => continue,
            "approved" => {
                let api_key = poll
                    .api_key
                    .filter(|key| !key.is_empty())
                    .ok_or_else(|| "the Fikra API approved sign-in without a key".to_string())?;
                return Ok(FikraBrowserLogin {
                    api_key,
                    plan: poll.plan,
                });
            }
            "redeemed" => {
                return Err("this sign-in was already used. Run `fikra login` again.".to_string());
            }
            _ => {
                return Err(poll
                    .detail
                    .unwrap_or_else(|| "sign-in expired. Run `fikra login` again.".to_string()));
            }
        }
    }
}
