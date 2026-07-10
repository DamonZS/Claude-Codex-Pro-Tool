use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_AD_LIST_URLS: [&str; 2] = [
    "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main/assets/config/announcement.json",
    "https://cdn.jsdelivr.net/gh/DamonZS/Claude-Codex-Pro-Tool@main/assets/config/announcement.json",
];

pub const OFFICIAL_TOPOREDUCE_AD_ID: &str = "official-toporeduce-api";
pub const OFFICIAL_TOPOREDUCE_AD_URL: &str = "https://api.toporeduce.cn";

pub fn official_toporeduce_ad() -> Value {
    json!({
        "id": OFFICIAL_TOPOREDUCE_AD_ID,
        "type": "normal",
        "badge": "公告",
        "title": "CCP官方中转站",
        "description": "拓扑API是CCP官方中转站，主打稳定接入和划算价格，支持 GPT-5.6、GPT-5.5、Claude Fable 5、Claude Opus 4.8、gpt-image-2-4k等模型与图像能力。",
        "buttonLabel": "拓扑API",
        "url": OFFICIAL_TOPOREDUCE_AD_URL,
        "highlights": ["拓扑API", "稳定接入", "划算价格", "GPT-5.6", "Claude Fable 5", "Claude Opus 4.8", "gpt-image-2-4k"]
    })
}

pub fn normalize_ad_payload(payload: Value) -> Value {
    let version = payload.get("version").and_then(Value::as_u64).unwrap_or(1);
    let mut remote_ads = payload
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
        .collect::<Vec<_>>();

    let remote_official_index = remote_ads.iter().position(|ad| {
        ad.get("id").and_then(Value::as_str) == Some(OFFICIAL_TOPOREDUCE_AD_ID)
            || ad.get("url").and_then(Value::as_str) == Some(OFFICIAL_TOPOREDUCE_AD_URL)
    });
    let mut official = remote_official_index
        .map(|index| remote_ads.remove(index))
        .unwrap_or_else(official_toporeduce_ad);
    if let Some(object) = official.as_object_mut() {
        object.insert(
            "id".to_string(),
            Value::String(OFFICIAL_TOPOREDUCE_AD_ID.to_string()),
        );
    }

    let mut ads = vec![official];
    ads.extend(remote_ads.into_iter().filter(|ad| {
        let id = ad.get("id").and_then(Value::as_str).unwrap_or_default();
        let url = ad.get("url").and_then(Value::as_str).unwrap_or_default();
        id != OFFICIAL_TOPOREDUCE_AD_ID && url != OFFICIAL_TOPOREDUCE_AD_URL
    }));
    json!({ "version": version, "ads": ads })
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
            Ok::<_, anyhow::Error>(normalize_ad_payload(payload))
        }
        .await;
        match result {
            Ok(payload) => return Ok(payload),
            Err(_) => continue,
        }
    }
    Ok(json!({ "version": 1, "ads": [official_toporeduce_ad()] }))
}
