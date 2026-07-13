use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub const BUNDLED_AD_LIST_JSON: &str = include_str!("../../../assets/config/announcement.json");

pub const DEFAULT_AD_LIST_URLS: [&str; 2] = [
    "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main/assets/config/announcement.json",
    "https://cdn.jsdelivr.net/gh/DamonZS/Claude-Codex-Pro-Tool@main/assets/config/announcement.json",
];

pub const OFFICIAL_TOPOREDUCE_AD_ID: &str = "official-toporeduce-api";
pub const OFFICIAL_TOPOREDUCE_AD_URL: &str = "https://api.toporeduce.cn";

pub fn bundled_ad_config() -> Value {
    serde_json::from_str(BUNDLED_AD_LIST_JSON)
        .unwrap_or_else(|_| json!({ "version": 1, "enabled": false, "ads": [] }))
}

pub fn bundled_ad_payload() -> Value {
    normalize_ad_payload(bundled_ad_config())
}

pub fn official_toporeduce_ad() -> Value {
    bundled_ad_config()
        .get("ads")
        .and_then(Value::as_array)
        .and_then(|ads| {
            ads.iter().find(|ad| {
                ad.get("id").and_then(Value::as_str) == Some(OFFICIAL_TOPOREDUCE_AD_ID)
                    || ad.get("url").and_then(Value::as_str) == Some(OFFICIAL_TOPOREDUCE_AD_URL)
            })
        })
        .cloned()
        .unwrap_or_else(|| json!({}))
}

pub fn normalize_ad_payload(payload: Value) -> Value {
    let version = payload.get("version").and_then(Value::as_u64).unwrap_or(1);
    let enabled = payload.get("enabled").and_then(Value::as_bool) == Some(true);
    if !enabled {
        return json!({ "version": version, "enabled": false, "ads": [] });
    }

    let mut seen_ids = HashSet::new();
    let mut seen_urls = HashSet::new();
    let ads = payload
        .get("ads")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|ad| {
            let ad_type = ad.get("type").and_then(Value::as_str);
            let title = ad.get("title").and_then(Value::as_str);
            let description = ad.get("description").and_then(Value::as_str);
            let url = ad.get("url").and_then(Value::as_str);
            matches!(ad_type, Some("normal"))
                && title.is_some_and(|value| !value.trim().is_empty())
                && description.is_some_and(|value| !value.trim().is_empty())
                && url.is_some_and(|value| !value.trim().is_empty())
        })
        .cloned()
        .filter(|ad| {
            let id = ad.get("id").and_then(Value::as_str).unwrap_or_default();
            let url = ad.get("url").and_then(Value::as_str).unwrap_or_default();
            (id.is_empty() || seen_ids.insert(id.to_string()))
                && (url.is_empty() || seen_urls.insert(url.to_string()))
        })
        .collect::<Vec<_>>();
    json!({ "version": version, "enabled": true, "ads": ads })
}

pub async fn fetch_ad_list() -> anyhow::Result<Value> {
    fetch_ad_list_from_urls(&DEFAULT_AD_LIST_URLS).await
}

pub fn cache_busted_ad_url(url: &str, version: u128) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}v={version}")
}

pub async fn fetch_ad_list_from_urls<S>(urls: &[S]) -> anyhow::Result<Value>
where
    S: AsRef<str>,
{
    let client = crate::http_client::proxied_client("ClaudeCodexPro")?;
    let cache_bust = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    for url in urls {
        let url = cache_busted_ad_url(url.as_ref(), cache_bust);
        let result = async {
            let response = client.get(url).send().await?.error_for_status()?;
            let payload = response.json::<Value>().await?;
            anyhow::ensure!(payload.is_object(), "公告配置根节点必须是对象");
            Ok::<_, anyhow::Error>(normalize_ad_payload(payload))
        }
        .await;
        match result {
            Ok(payload) => return Ok(payload),
            Err(_) => continue,
        }
    }
    Ok(bundled_ad_payload())
}
