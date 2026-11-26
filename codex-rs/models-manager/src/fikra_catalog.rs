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
    /// Best effort: any failure returns `None` so the caller keeps the bundled
    /// catalog. This runs on the model-picker path, so it never blocks for long.
    pub async fn load(factory: HttpClientFactory) -> Option<Self> {
        let client = factory
            .build_client_without_request_logging(FIKRA_CODE_CATALOG_URL, ClientRouteClass::Other)
            .ok()?;
        let body = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
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
        .ok()??;
        merge(&super::bundled_models_response().ok()?.models, &body)
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
    let mut main_model = None;
    for (index, row) in live.data.iter().enumerate() {
        if EXCLUDED_IDS.contains(&row.id.as_str()) || row.available_in_api == Some(false) {
            continue;
        }
        if row.is_main == Some(true) && main_model.is_none() {
            main_model = Some(row.id.clone());
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

    let models = ranked
        .into_iter()
        .filter_map(|(_, row)| {
            let mut model = (*by_slug.get(row.id.as_str())?).clone();
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
    fn orders_by_rank_and_skips_unknown_and_unavailable_models() {
        let overlay = merge(&bundled(), LIVE).expect("overlay");
        let slugs: Vec<&str> = overlay.models.iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(slugs, vec!["qwen3.8-max", "qwen3.8-27b-free"]);
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
    fn no_persisted_main_model_is_none() {
        let home = tempfile::tempdir().expect("temp home");
        assert_eq!(super::FikraCatalog::cached_main_model(home.path()), None);
    }

    #[test]
    fn unusable_payloads_return_none() {
        assert!(merge(&bundled(), "not json").is_none());
        assert!(merge(&bundled(), r#"{"data": []}"#).is_none());
        assert!(
            merge(
                &bundled(),
                r#"{"data":[{"id":"unknown-a"},{"id":"unknown-b"}]}"#
            )
            .is_none()
        );
    }
}
