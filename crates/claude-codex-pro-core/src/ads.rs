use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_AD_LIST_URLS: [&str; 2] = [
    "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool-Ad-List/main/ads.json",
    "https://cdn.jsdelivr.net/gh/DamonZS/Claude-Codex-Pro-Tool-Ad-List@main/ads.json",
];

pub const OFFICIAL_TOPOREDUCE_AD_ID: &str = "official-toporeduce-api";
pub const OFFICIAL_TOPOREDUCE_AD_URL: &str = "https://api.toporeduce.cn";

pub fn official_toporeduce_ad() -> Value {
    json!({
        "id": OFFICIAL_TOPOREDUCE_AD_ID,
        "type": "normal",
        "title": "官方中转站",
        "description": "拓扑熵减API｜ClaudeCodexPro官方中转站，主打稳定接入和划算价格，支持 GPT-5.5、GPT-5.4、Claude Opus 4.8、Claude Opus 4.7、gpt-image-2 等模型与图像能力。",
        "url": OFFICIAL_TOPOREDUCE_AD_URL,
        "highlights": ["拓扑熵减API", "稳定接入", "划算价格", "GPT-5.5", "Claude Opus 4.8", "gpt-image-2"]
    })
}

pub fn normalize_ad_payload(payload: Value) -> Value {
    let version = payload.get("version").and_then(Value::as_u64).unwrap_or(1);
    let mut ads = vec![official_toporeduce_ad()];
    ads.extend(
        payload
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
            .filter(|ad| {
                let id = ad.get("id").and_then(Value::as_str).unwrap_or_default();
                let url = ad.get("url").and_then(Value::as_str).unwrap_or_default();
                id != OFFICIAL_TOPOREDUCE_AD_ID && url != OFFICIAL_TOPOREDUCE_AD_URL
            })
            .cloned(),
    );
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
