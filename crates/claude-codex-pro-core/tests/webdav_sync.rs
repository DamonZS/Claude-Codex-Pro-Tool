//! WebDAV 配置备份同步的集成测试。
//!
//! 每个用例用 `TcpListener::bind("127.0.0.1:0")` 起一个一次性假 WebDAV
//! 服务，端口随机因此可以并发跑；服务线程只接受一个连接，把收到的请求
//! 原样交回断言，不需要额外依赖。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use base64::Engine as _;
use claude_codex_pro_core::webdav_sync::{
    WebdavConfig, backup_file_name, download_backup, upload_backup,
};

#[tokio::test]
async fn upload_puts_body_with_basic_auth_and_returns_remote_url() {
    let server = spawn_webdav_server(201, b"");
    let config = WebdavConfig {
        // Trailing slash must be normalized away, not produce `//`.
        base_url: format!("{}/dav/", server.base_url),
        username: "alice".to_string(),
        password: "s3cret".to_string(),
    };

    let url = upload_backup(&config, "ccp-backup-20261106-180500.json", b"{\"v\":1}")
        .await
        .expect("201 should be reported as success");

    assert_eq!(
        url,
        format!("{}/dav/ccp-backup-20261106-180500.json", server.base_url)
    );
    let request = server.finish();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/dav/ccp-backup-20261106-180500.json");
    assert_eq!(request.authorization, basic("alice", "s3cret"));
    assert_eq!(request.content_type, "application/octet-stream");
    assert_eq!(request.body, b"{\"v\":1}");
}

#[tokio::test]
async fn download_returns_bytes_from_get() {
    let payload = b"{\"manifest\":\"abc\"}";
    let server = spawn_webdav_server(200, payload);
    let config = test_config(&server.base_url);

    let bytes = download_backup(&config, "ccp-backup-20261106-180500.json")
        .await
        .expect("200 should yield the body");

    assert_eq!(bytes, payload);
    let request = server.finish();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/ccp-backup-20261106-180500.json");
    assert_eq!(request.authorization, basic("alice", "s3cret"));
}

#[tokio::test]
async fn download_404_reports_missing_remote_file() {
    let server = spawn_webdav_server(404, b"not found");
    let config = test_config(&server.base_url);

    let error = download_backup(&config, "ccp-backup-20261106-180500.json")
        .await
        .expect_err("404 must fail");
    let text = format!("{error:#}");

    assert!(text.contains("不存在"), "unexpected message: {text}");
    assert!(text.contains("404"), "unexpected message: {text}");
    server.finish();
}

#[tokio::test]
async fn upload_401_reports_auth_failure_with_status_code() {
    let server = spawn_webdav_server(401, b"unauthorized");
    let config = test_config(&server.base_url);

    let error = upload_backup(&config, "ccp-backup-20261106-180500.json", b"{}")
        .await
        .expect_err("401 must fail");
    let text = format!("{error:#}");

    assert!(text.contains("认证失败"), "unexpected message: {text}");
    assert!(text.contains("401"), "unexpected message: {text}");
    server.finish();
}

#[tokio::test]
async fn unsafe_file_names_are_rejected_before_any_request() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind random port");
    listener
        .set_nonblocking(true)
        .expect("listener should be nonblocking");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let config = test_config(&base_url);

    for file_name in [
        "../evil.json",
        "sub/evil.json",
        "sub\\evil.json",
        "..",
        ".",
        "",
        "备份.json",
    ] {
        let error = upload_backup(&config, file_name, b"{}")
            .await
            .expect_err("unsafe file name must be rejected");
        let text = format!("{error:#}");
        assert!(
            text.contains("文件名不合法"),
            "{file_name:?} produced unexpected message: {text}"
        );

        // No connection may have been opened: the guard runs before reqwest.
        match listener.accept() {
            Err(error) => assert_eq!(
                error.kind(),
                std::io::ErrorKind::WouldBlock,
                "unexpected accept error for {file_name:?}: {error}"
            ),
            Ok(_) => panic!("a request was sent for {file_name:?}"),
        }
    }
}

#[test]
fn backup_file_name_uses_utc_second_precision() {
    let instant = chrono::NaiveDate::from_ymd_opt(2026, 11, 6)
        .expect("valid date")
        .and_hms_opt(18, 5, 0)
        .expect("valid time")
        .and_utc();
    let now_ms = instant.timestamp_millis();

    assert_eq!(backup_file_name(now_ms), "ccp-backup-20261106-180500.json");
    // Millisecond digits are truncated: same second, same name (latest wins).
    assert_eq!(
        backup_file_name(now_ms + 999),
        "ccp-backup-20261106-180500.json"
    );
    // 18:05 UTC is not 18:05 in every zone; a name computed from local time
    // would shift with the test machine's TZ.
    assert_eq!(
        backup_file_name(instant.timestamp_millis() - 1000),
        "ccp-backup-20261106-180459.json"
    );
}

fn test_config(base_url: &str) -> WebdavConfig {
    WebdavConfig {
        base_url: base_url.to_string(),
        username: "alice".to_string(),
        password: "s3cret".to_string(),
    }
}

fn basic(username: &str, password: &str) -> String {
    let token = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    format!("Basic {token}")
}

struct CapturedRequest {
    method: String,
    path: String,
    authorization: String,
    content_type: String,
    body: Vec<u8>,
}

struct FakeWebdav {
    base_url: String,
    outcome: mpsc::Receiver<CapturedRequest>,
    handle: thread::JoinHandle<()>,
}

impl FakeWebdav {
    fn finish(self) -> CapturedRequest {
        let request = self
            .outcome
            .recv_timeout(Duration::from_secs(5))
            .expect("fake WebDAV server should capture a request");
        self.handle.join().expect("fake WebDAV server thread");
        request
    }
}

/// One-shot fake WebDAV: accept a single connection, read the request
/// (headers plus `Content-Length` body), answer `status`, then close.
fn spawn_webdav_server(status: u16, response_body: &'static [u8]) -> FakeWebdav {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind random port");
    let address = listener.local_addr().expect("local addr");
    let base_url = format!("http://{address}");
    let (sender, outcome) = mpsc::channel();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];
        let header_end = loop {
            if let Some(position) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                break position + 4;
            }
            let read = stream.read(&mut chunk).expect("read request headers");
            assert!(read > 0, "connection closed before headers completed");
            buffer.extend_from_slice(&chunk[..read]);
        };

        let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
        let content_length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        while buffer.len() < header_end + content_length {
            let read = stream.read(&mut chunk).expect("read request body");
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
        let body = buffer[header_end..header_end + content_length].to_vec();

        let request_line = head.lines().next().unwrap_or_default();
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let path = parts.next().unwrap_or_default().to_string();
        let header_value = |wanted: &str| {
            head.lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case(wanted)
                        .then(|| value.trim().to_string())
                })
                .unwrap_or_default()
        };

        sender
            .send(CapturedRequest {
                method,
                path,
                authorization: header_value("authorization"),
                content_type: header_value("content-type"),
                body,
            })
            .expect("test receives the captured request");

        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response_body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response head");
        stream
            .write_all(response_body)
            .expect("write response body");
        stream.flush().expect("flush response");
    });

    FakeWebdav {
        base_url,
        outcome,
        handle,
    }
}
