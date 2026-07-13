use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use claude_codex_pro_core::ads::{
    BUNDLED_AD_LIST_JSON, DEFAULT_AD_LIST_URLS, OFFICIAL_TOPOREDUCE_AD_ID, bundled_ad_config,
    bundled_ad_payload, cache_busted_ad_url, fetch_ad_list_from_urls, normalize_ad_payload,
};
use serde_json::json;

#[test]
fn default_ad_urls_match_owned_recommendation_sources() {
    assert_eq!(
        DEFAULT_AD_LIST_URLS,
        [
            "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main/assets/config/announcement.json",
            "https://cdn.jsdelivr.net/gh/DamonZS/Claude-Codex-Pro-Tool@main/assets/config/announcement.json",
        ]
    );
}

#[test]
fn cache_busted_ad_url_appends_version_query_to_plain_url() {
    assert_eq!(
        cache_busted_ad_url("https://example.test/ads.json", 1779035222758),
        "https://example.test/ads.json?v=1779035222758"
    );
}

#[test]
fn cache_busted_ad_url_preserves_existing_query() {
    assert_eq!(
        cache_busted_ad_url("https://example.test/ads.json?source=cdn", 1779035222758),
        "https://example.test/ads.json?source=cdn&v=1779035222758"
    );
}

#[test]
fn normalizes_remote_ads_for_plugin_and_manager_rendering() {
    let payload = normalize_ad_payload(json!({
        "version": 1,
        "enabled": true,
        "ads": [
            {
                "id": "partner",
                "type": "partner",
                "title": "Partner",
                "description": "推荐内容",
                "url": "https://example.test",
                "highlights": ["稳定"]
            },
            {
                "id": "normal",
                "type": "normal",
                "title": "普通推荐",
                "description": "推荐内容",
                "url": "https://example.org"
            },
            {
                "id": "broken",
                "type": "normal",
                "title": "",
                "description": "missing title",
                "url": "https://example.invalid"
            }
        ]
    }));

    assert_eq!(payload["version"], json!(1));
    assert_eq!(payload["enabled"], json!(true));
    assert_eq!(payload["ads"].as_array().unwrap().len(), 1);
    assert_eq!(payload["ads"][0]["type"], json!("normal"));
    assert_eq!(payload["ads"][0]["id"], json!("normal"));
}

#[test]
fn remote_official_announcement_overrides_bundled_fallback_without_duplication() {
    let payload = normalize_ad_payload(json!({
        "version": 2,
        "enabled": true,
        "ads": [
            {
                "id": OFFICIAL_TOPOREDUCE_AD_ID,
                "type": "normal",
                "badge": "最新公告",
                "title": "远程公告标题",
                "description": "远程公告正文",
                "buttonLabel": "查看远程公告",
                "url": "https://api.toporeduce.cn"
            },
            {
                "id": "duplicate-url",
                "type": "normal",
                "title": "远端重复 URL",
                "description": "重复内容",
                "url": "https://api.toporeduce.cn"
            }
        ]
    }));

    assert_eq!(payload["ads"].as_array().unwrap().len(), 1);
    assert_eq!(payload["ads"][0]["id"], json!(OFFICIAL_TOPOREDUCE_AD_ID));
    assert_eq!(payload["ads"][0]["badge"], json!("最新公告"));
    assert_eq!(payload["ads"][0]["title"], json!("远程公告标题"));
    assert_eq!(payload["ads"][0]["description"], json!("远程公告正文"));
    assert_eq!(payload["ads"][0]["buttonLabel"], json!("查看远程公告"));
}

#[test]
fn repository_announcement_config_is_valid_and_remotely_editable() {
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/config/announcement.json");
    let configured: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();
    let announcement = &configured["ads"][0];

    assert_eq!(configured["enabled"], json!(false));
    assert_eq!(announcement["id"], json!(OFFICIAL_TOPOREDUCE_AD_ID));
    assert_eq!(announcement["type"], json!("normal"));
    for field in ["badge", "title", "description", "buttonLabel", "url"] {
        assert!(
            announcement[field]
                .as_str()
                .is_some_and(|value| !value.trim().is_empty()),
            "missing announcement field: {field}"
        );
    }
    assert!(
        announcement["url"]
            .as_str()
            .unwrap()
            .starts_with("https://")
    );
}

#[tokio::test]
async fn fetch_ad_list_tries_backup_url_when_primary_fails() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let thread = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 1024];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            if request.starts_with("GET /primary.json?") {
                stream
                    .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n")
                    .unwrap();
            } else {
                assert!(request.starts_with("GET /backup.json?"), "{request}");
                let body = json!({
                    "version": 1,
                    "enabled": true,
                    "ads": [{
                        "id": "backup-ad",
                        "type": "normal",
                        "title": "Backup",
                        "description": "Loaded from backup",
                        "url": "https://example.test",
                        "highlights": []
                    }]
                })
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        }
    });

    let payload = fetch_ad_list_from_urls(&[
        format!("http://127.0.0.1:{port}/primary.json"),
        format!("http://127.0.0.1:{port}/backup.json"),
    ])
    .await
    .unwrap();
    thread.join().unwrap();

    assert_eq!(payload["enabled"], json!(true));
    assert_eq!(payload["ads"][0]["id"], json!("backup-ad"));
}

#[tokio::test]
async fn fetch_ad_list_falls_back_to_disabled_bundled_announcement_when_all_urls_fail() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 1024];
        let read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..read]);
        assert!(request.starts_with("GET /unavailable.json?"), "{request}");
        stream
            .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });

    let payload = fetch_ad_list_from_urls(&[format!("http://127.0.0.1:{port}/unavailable.json")])
        .await
        .unwrap();
    thread.join().unwrap();

    assert_eq!(payload["enabled"], json!(false));
    assert!(payload["ads"].as_array().unwrap().is_empty());
}

#[test]
fn disabled_or_legacy_payload_does_not_show_bundled_announcement() {
    for payload in [
        json!({ "version": 2, "enabled": false, "ads": [{
            "id": OFFICIAL_TOPOREDUCE_AD_ID,
            "type": "normal",
            "title": "不应显示",
            "description": "远程已关闭",
            "url": "https://api.toporeduce.cn"
        }] }),
        json!({ "version": 1, "ads": [{
            "id": "legacy",
            "type": "normal",
            "title": "旧配置",
            "description": "缺少开关",
            "url": "https://example.test"
        }] }),
    ] {
        let normalized = normalize_ad_payload(payload);
        assert_eq!(normalized["enabled"], json!(false));
        assert!(normalized["ads"].as_array().unwrap().is_empty());
    }
}

#[test]
fn bundled_announcement_is_the_repository_config_and_defaults_to_hidden() {
    let raw: serde_json::Value = serde_json::from_str(BUNDLED_AD_LIST_JSON).unwrap();
    assert_eq!(bundled_ad_config(), raw);
    assert_eq!(raw["enabled"], json!(false));
    assert_eq!(raw["ads"][0]["id"], json!(OFFICIAL_TOPOREDUCE_AD_ID));

    let normalized = bundled_ad_payload();
    assert_eq!(normalized["enabled"], json!(false));
    assert!(normalized["ads"].as_array().unwrap().is_empty());
}
