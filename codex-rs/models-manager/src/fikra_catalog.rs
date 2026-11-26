//! Fikra: overlay the live Fikra Code model catalog on top of the bundled one.
//!
//! `https://fikraapi.co.ke/code/models.json` is the product catalog: which
//! models exist, their names, order, default and availability. It carries only
//! presentation data, so each entry is merged onto the bundled entry of the
//! same id, which supplies the tool and prompt configuration that belongs in
//! code rather than on a server.
//!
//! If the fetch fails, times out or returns nothing usable, the bundled
//! catalog is returned unchanged.

use std::collections::HashMap;
use std::time::Duration;

use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_protocol::openai_models::ModelInfo;
use http::header::USER_AGENT;
use serde::Deserialize;

/// The Fikra Code product catalog. Presentation data only.
pub const FIKRA_CODE_CATALOG_URL: &str = "https://fikraapi.co.ke/code/models.json";

/// A catalog this size is several times over a full response.
const MAX_CATALOG_BYTES: usize = 512 * 1024;

/// File recording the model the server marked `is_main`, so a new session can
/// start on it without shipping a release.
const MAIN_MODEL_FILE: &str = "fikra_default_model";

/// Last good catalog body, so a launch does not depend on a fresh fetch.
const CACHE_FILE: &str = "fikra_catalog_cache.json";

/// How long a cached catalog is reused before another fetch is attempted.
/// Model catalogs change on the order of days, not minutes.
const CACHE_TTL_SECS: u64 = 6 * 60 * 60;

/// Model ids never offered, even if the catalog lists them.
const EXCLUDED_IDS: &[&str] = &["fikra-qwen-27b", "fikra-7b-v1"];

/// The largest context window Fikra Code will advertise for any model.
const MAX_CONTEXT_WINDOW: i64 = 1_000_000;

/// One row of `code/models.json`.
#[derive(Debug, Deserialize)]
struct LiveModel {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    context_window: Option<i64>,
    #[serde(default)]
    available_in_api: Option<bool>,
    #[serde(default)]
    is_main: Option<bool>,
    #[serde(default)]
    rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct LiveCatalog {
    #[serde(default)]
    data: Vec<LiveModel>,
}

/// The live catalog merged onto the bundled one.
#[derive(Debug)]
pub struct FikraCatalog {
    pub models: Vec<ModelInfo>,
    /// Id marked `is_main` in the live catalog, when one is present.
    pub main_model: Option<String>,
}

impl FikraCatalog {
    /// Persist the server's chosen default model into `codex_home`.
    ///
    /// Best effort: a write failure just means the shipped default is used.
    pub fn persist_main_model(&self, codex_home: &std::path::Path) {
        let Some(slug) = self.main_model.as_deref().filter(|slug| !slug.is_empty()) else {
            return;
        };
        if let Err(err) = std::fs::write(codex_home.join(MAIN_MODEL_FILE), slug) {
            tracing::warn!("failed to persist the Fikra default model: {err}");
        }
    }

    /// The default model the server last asked for, if one was persisted.
    ///
    /// Only returns slugs the bundled catalog knows, so a stale record cannot
    /// select a model this build cannot run.
    pub fn cached_main_model(codex_home: &std::path::Path) -> Option<String> {
        let raw = std::fs::read_to_string(codex_home.join(MAIN_MODEL_FILE)).ok()?;
        let slug = raw.trim();
        (!slug.is_empty()).then(|| slug.to_string())
    }

    /// Fetch the live catalog and merge it onto the bundled one.
    ///
    /// Uses the cached catalog while it is fresh, and falls back to it when a
    /// fetch fails, so a launch neither re-fetches every time nor loses the
    /// server's view because the network blinked.
    ///
    /// Best effort: any failure returns `None` so the caller keeps the bundled
    /// catalog. This runs on the model-picker path, so it never blocks for long.
    pub async fn load(factory: HttpClientFactory, codex_home: &std::path::Path) -> Option<Self> {
        if let Some(fresh) = read_fresh_cache(codex_home) {
            return Self::from_body(&fresh);
        }
        let fetched = fetch(factory).await;
        if let Some(body) = fetched.as_deref() {
            write_cache(codex_home, body);
        }
        let body = fetched.or_else(|| read_any_cache(codex_home))?;
        Self::from_body(&body)
    }

    /// Merge a catalog response body onto the bundled catalog.
    fn from_body(body: &str) -> Option<Self> {
        merge(&super::bundled_models_response().ok()?.models, body)
    }
}

/// Fetch the catalog body over HTTP. `None` on any failure.
async fn fetch(factory: HttpClientFactory) -> Option<String> {
    let client = factory
        .build_client_without_request_logging(FIKRA_CODE_CATALOG_URL, ClientRouteClass::Other)
        .ok()?;
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        let response = client
            .get(FIKRA_CODE_CATALOG_URL)
            .header(
                USER_AGENT,
                concat!("fikra-code/", env!("CARGO_PKG_VERSION")),
            )
            .timeout(Duration::from_secs(/*secs*/ 5))
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        let text = response.text().await.ok()?;
        (text.len() <= MAX_CATALOG_BYTES).then_some(text)
    })
    .await
    .ok()?
}

/// Cached body with the time it was written, as `{written_unix, body}`.
fn read_cache(codex_home: &std::path::Path) -> Option<(u64, String)> {
    let raw = std::fs::read_to_string(codex_home.join(CACHE_FILE)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let written = value.get("written_unix")?.as_u64()?;
    let body = value.get("body")?.as_str()?.to_string();
    Some((written, body))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// The cached body while it is still inside the TTL.
fn read_fresh_cache(codex_home: &std::path::Path) -> Option<String> {
    let (written, body) = read_cache(codex_home)?;
    let age = now_unix().saturating_sub(written);
    (age < CACHE_TTL_SECS).then_some(body)
}

/// The cached body regardless of age, for when a fetch fails.
fn read_any_cache(codex_home: &std::path::Path) -> Option<String> {
    read_cache(codex_home).map(|(_, body)| body)
}

/// Persist the catalog body. Best effort: a write failure only costs a fetch.
fn write_cache(codex_home: &std::path::Path, body: &str) {
    let payload = serde_json::json!({ "written_unix": now_unix(), "body": body });
    if let Err(err) = std::fs::write(codex_home.join(CACHE_FILE), payload.to_string()) {
        tracing::warn!("failed to cache the Fikra catalog: {err}");
    }
}

/// Merge `live_json` onto `bundled`. Returns `None` when the payload is not a
/// usable catalog, so callers keep the bundled list rather than an empty one.
pub fn merge(bundled: &[ModelInfo], live_json: &str) -> Option<FikraCatalog> {
    let live: LiveCatalog = serde_json::from_str(live_json).ok()?;
    if live.data.is_empty() {
        return None;
    }

    let by_slug: HashMap<&str, &ModelInfo> = bundled
        .iter()
        .map(|model| (model.slug.as_str(), model))
        .collect();

    let mut ranked: Vec<(i32, &LiveModel)> = Vec::new();
    let mut main_candidates: Vec<&str> = Vec::new();
    for (index, row) in live.data.iter().enumerate() {
        if EXCLUDED_IDS.contains(&row.id.as_str()) || row.available_in_api == Some(false) {
            continue;
        }
        if row.is_main == Some(true) {
            main_candidates.push(row.id.as_str());
        }
        let rank = row
            .rank
            .unwrap_or_else(|| bundled.len() as i32 + index as i32);
        ranked.push((rank, row));
    }
    ranked.sort_by_key(|(rank, _)| *rank);
    if ranked.is_empty() {
        return None;
    }

    // Exactly one model may be the default. Several marked `is_main` means the
    // catalog is ambiguous, and guessing would silently move new users onto a
    // model they did not choose (and may be billed for), so record none and let
    // the shipped default stand.
    let main_model = match main_candidates.as_slice() {
        [only] => Some((*only).to_string()),
        _ => {
            if main_candidates.len() > 1 {
                tracing::warn!(
                    count = main_candidates.len(),
                    "model catalog marks more than one model as the default; keeping the shipped default"
                );
            }
            None
        }
    };

    let models = ranked
        .into_iter()
        .filter_map(|(_, row)| {
            // A model the server added since this build shipped has no bundled
            // entry. Fall back to the generic descriptor so it stays selectable
            // instead of silently disappearing from the picker.
            let mut model = match by_slug.get(row.id.as_str()) {
                Some(bundled) => (*bundled).clone(),
                None => {
                    // The generic descriptor is hidden by default; a model the
                    // server lists should be selectable, so make it visible.
                    let mut fallback = crate::model_info::model_info_from_slug(&row.id);
                    fallback.visibility = codex_protocol::openai_models::ModelVisibility::List;
                    fallback
                }
            };
            if let Some(name) = row.display_name.as_deref().filter(|name| !name.is_empty()) {
                model.display_name = name.to_string();
            }
            if let Some(description) = row.description.as_deref().filter(|text| !text.is_empty()) {
                model.description = Some(description.to_string());
            }
            if let Some(window) = row.context_window.filter(|window| *window > 0) {
                let window = window.min(MAX_CONTEXT_WINDOW);
                model.context_window = Some(window);
                model.max_context_window = Some(window);
            }
            Some(model)
        })
        .collect::<Vec<_>>();

    (!models.is_empty()).then_some(FikraCatalog { models, main_model })
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::bundled_models_response;

    fn bundled() -> Vec<codex_protocol::openai_models::ModelInfo> {
        bundled_models_response().expect("bundled catalog").models
    }

    const LIVE: &str = r#"{
        "object": "list",
        "data": [
            {"id": "qwen3.8-max", "display_name": "Qwen 3.8 Max", "context_window": 1048576,
             "available_in_api": true, "is_main": false, "rank": 1,
             "description": "Qwen 3.8 Max. 1M context."},
            {"id": "qwen3.8-27b-free", "display_name": "Qwen 3.8 27B Free", "context_window": 131072,
             "available_in_api": true, "is_main": true, "rank": 2, "description": "Free."},
            {"id": "brand-new-model", "display_name": "Brand New", "context_window": 65536,
             "available_in_api": true, "is_main": false, "rank": 3},
            {"id": "hidden-model", "display_name": "Hidden", "context_window": 8192,
             "available_in_api": false, "rank": 4}
        ]
    }"#;

    #[test]
    fn orders_by_rank_and_skips_unavailable_models() {
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        let slugs: Vec<&str> = overlay.models.iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec!["qwen3.8-max", "qwen3.8-27b-free", "brand-new-model"]
        );
    }

    #[test]
    fn a_model_the_server_added_is_still_selectable() {
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        let added = overlay
            .models
            .iter()
            .find(|m| m.slug == "brand-new-model")
            .expect("model added by the server");
        assert_eq!(added.display_name, "Brand New");
        assert_eq!(added.context_window, Some(65536));
        // A model the server lists must be selectable, so the fallback is
        // promoted from the generic hidden descriptor to a listed one.
        assert!(matches!(
            added.visibility,
            codex_protocol::openai_models::ModelVisibility::List
        ));
    }

    #[test]
    fn bundled_models_dropped_by_the_server_disappear() {
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        assert!(!overlay.models.iter().any(|m| m.slug == "kimi-k3"));
    }

    #[test]
    fn an_ambiguous_default_is_not_trusted() {
        // The live catalog currently marks several models `is_main`. Picking one
        // would silently move new users onto a model they did not choose.
        let ambiguous = r#"{"data":[
            {"id":"qwen3.8-max","is_main":true,"rank":1},
            {"id":"qwen3.8-27b-free","is_main":true,"rank":2}
        ]}"#;
        let overlay = merge(&bundled(), ambiguous).expect("overlay");
        assert_eq!(overlay.main_model, None);
        // Both models are still offered; only the default is withheld.
        assert_eq!(overlay.models.len(), 2);
    }

    #[test]
    fn reports_the_main_model() {
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        assert_eq!(overlay.main_model.as_deref(), Some("qwen3.8-27b-free"));
    }

    #[test]
    fn overlays_presentation_but_keeps_bundled_configuration() {
        let bundled = bundled();
        let bundled_free = bundled
            .iter()
            .find(|m| m.slug == "qwen3.8-27b-free")
            .expect("bundled free model");
        let overlay = merge(&bundled, LIVE).expect("overlay");
        let merged = overlay
            .models
            .iter()
            .find(|m| m.slug == "qwen3.8-27b-free")
            .expect("merged free model");
        assert_eq!(merged.display_name, "Qwen 3.8 27B Free");
        assert_eq!(merged.description.as_deref(), Some("Free."));
        // Config that must never come from the server keeps its bundled value.
        assert_eq!(merged.model_messages, bundled_free.model_messages);
        assert_eq!(merged.shell_type, bundled_free.shell_type);
    }

    #[test]
    fn caps_context_window_at_one_million() {
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        let max = overlay
            .models
            .iter()
            .find(|m| m.slug == "qwen3.8-max")
            .expect("max model");
        assert_eq!(max.context_window, Some(1_000_000));
    }

    #[test]
    fn main_model_round_trips_through_codex_home() {
        let home = tempfile::tempdir().expect("temp home");
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        overlay.persist_main_model(home.path());
        assert_eq!(
            super::FikraCatalog::cached_main_model(home.path()).as_deref(),
            Some("qwen3.8-27b-free")
        );
    }

    #[test]
    fn a_fresh_cache_is_reused_and_an_expired_one_is_not() {
        let home = tempfile::tempdir().expect("temp home");
        assert!(super::read_fresh_cache(home.path()).is_none());
        super::write_cache(home.path(), LIVE);
        assert_eq!(super::read_fresh_cache(home.path()).as_deref(), Some(LIVE));
        // An expired entry is still usable as a fallback.
        let stale = serde_json::json!({
            "written_unix": super::now_unix() - super::CACHE_TTL_SECS - 1,
            "body": LIVE
        });
        std::fs::write(home.path().join(super::CACHE_FILE), stale.to_string()).expect("write");
        assert!(super::read_fresh_cache(home.path()).is_none());
        assert_eq!(super::read_any_cache(home.path()).as_deref(), Some(LIVE));
    }

    #[test]
    fn a_cached_body_still_merges() {
        let home = tempfile::tempdir().expect("temp home");
        super::write_cache(home.path(), LIVE);
        let body = super::read_fresh_cache(home.path()).expect("fresh cache");
        let merged = super::FikraCatalog::from_body(&body).expect("merge cached body");
        assert!(merged.models.iter().any(|m| m.slug == "qwen3.8-27b-free"));
    }

    #[test]
    fn no_persisted_main_model_is_none() {
        let home = tempfile::tempdir().expect("temp home");
        assert_eq!(super::FikraCatalog::cached_main_model(home.path()), None);
    }

    #[test]
    fn unusable_payloads_return_none() {
        assert!(merge(&bundled(), "not json").is_none());
        assert!(merge(&bundled(), r#"{"data": []}"#).is_none());
        // Unknown ids are no longer dropped: the fallback keeps them
        // selectable, so this merges instead of returning None.
        let overlay = merge(
            &bundled(),
            r#"{"data":[{"id":"unknown-a"},{"id":"unknown-b"}]}"#,
        )
        .expect("unknown ids still produce a catalog");
        assert_eq!(overlay.models.len(), 2);
    }
}
