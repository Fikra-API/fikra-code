//! `fikra usage`: balance and recent usage for the signed-in Fikra API key.

use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use serde_json::Value;

const FIKRA_API: &str = "https://fikraapi.co.ke/v1";

async fn get_json(
    client: &codex_http_client::HttpClient,
    url: &str,
    api_key: &str,
) -> Result<Value, String> {
    let response = client
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|err| format!("could not reach the Fikra API: {err}"))?;
    let status = response.status().as_u16();
    if status == 401 || status == 403 {
        return Err("the Fikra API rejected this key. Run `fikra login` again.".to_string());
    }
    if status >= 400 {
        return Err(format!("the Fikra API returned HTTP {status}"));
    }
    response
        .json::<Value>()
        .await
        .map_err(|err| format!("unexpected response from the Fikra API: {err}"))
}

fn human_tokens(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.2}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1}K", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

/// Fetch balance and the last `days` days of usage and render them as text.
pub async fn fikra_usage_report(api_key: &str, days: u32) -> Result<String, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("no API key. Run `fikra login` or set FIKRA_API_KEY.".to_string());
    }
    let days = days.clamp(1, 90);
    let factory = HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);
    let client = factory
        .build_client(FIKRA_API, ClientRouteClass::Other)
        .map_err(|err| format!("could not build HTTP client: {err}"))?;

    let balance = get_json(&client, &format!("{FIKRA_API}/balance"), api_key).await?;
    let usage = get_json(&client, &format!("{FIKRA_API}/usage?days={days}"), api_key).await?;

    let mut out = String::new();
    let total = balance.get("total").and_then(Value::as_f64).unwrap_or(0.0);
    out.push_str(&format!("Balance       ${total:.2}\n"));
    if let Some(tier) = balance.get("tier").and_then(Value::as_str) {
        out.push_str(&format!("Account tier  {tier}\n"));
    }
    out.push_str(&format!("\nLast {days} days\n"));
    out.push_str("  date         requests      tokens       cost\n");
    let mut sum_req = 0.0;
    let mut sum_tok = 0.0;
    let mut sum_cost = 0.0;
    if let Some(daily) = usage.get("daily").and_then(Value::as_array) {
        for day in daily {
            let req = day.get("requests").and_then(Value::as_f64).unwrap_or(0.0);
            let tok = day
                .get("prompt_tokens")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                + day
                    .get("completion_tokens")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
            let cost = day.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0);
            sum_req += req;
            sum_tok += tok;
            sum_cost += cost;
            let date = day.get("date").and_then(Value::as_str).unwrap_or("?");
            out.push_str(&format!(
                "  {date}  {req:>8.0}  {:>10}  ${cost:>8.4}\n",
                human_tokens(tok)
            ));
        }
    }
    out.push_str(&format!(
        "  total       {sum_req:>8.0}  {:>10}  ${sum_cost:>8.4}\n",
        human_tokens(sum_tok)
    ));
    if total <= 0.0 {
        out.push_str(
            "\nYour balance is $0. Top up from $5 or get Fikra Code for KES 200 a week: https://fikraapi.co.ke/code/pricing\n",
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::human_tokens;

    #[test]
    fn formats_token_counts() {
        assert_eq!(human_tokens(950.0), "950");
        assert_eq!(human_tokens(22_323_497.0), "22.3M");
        assert_eq!(human_tokens(3_824_827_726.0), "3.82B");
    }
}
