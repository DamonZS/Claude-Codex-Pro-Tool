use claude_codex_pro_core::install_registration::{
    installation_id_from_baseboard_serial, register_baseboard_serial_at,
};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn baseboard_digest_is_normalized_stable_and_non_reversible() {
    let first = installation_id_from_baseboard_serial("  board-123  ").unwrap();
    let repeated = installation_id_from_baseboard_serial("BOARD-123").unwrap();
    let collapsed = installation_id_from_baseboard_serial("board-123\r\n").unwrap();

    assert_eq!(first, repeated);
    assert_eq!(first, collapsed);
    assert_eq!(first.len(), 64);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    assert!(!first.contains("BOARD-123"));
}

#[test]
fn baseboard_placeholders_are_rejected() {
    for value in [
        "",
        "unknown",
        "Default string",
        "To be filled by O.E.M.",
        "N/A",
        "0000-0000-0000",
        "---",
    ] {
        assert_eq!(
            installation_id_from_baseboard_serial(value),
            None,
            "placeholder should be rejected: {value}"
        );
    }
}

#[tokio::test]
async fn registration_sends_only_digest_version_and_platform() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || read_request_and_respond(listener, 200));

    register_baseboard_serial_at(
        "PRIVATE-BOARD-SERIAL-123",
        "0.12.0",
        &format!("http://{address}/api/tools/claude-codex-pro/register"),
    )
    .await
    .unwrap();

    let request = server.join().unwrap();
    let body = request.split("\r\n\r\n").nth(1).unwrap();
    let payload: Value = serde_json::from_str(body).unwrap();
    let object = payload.as_object().unwrap();

    assert_eq!(object.len(), 3);
    assert_eq!(payload["appVersion"], "0.12.0");
    assert_eq!(payload["platform"], "windows");
    assert_eq!(payload["installationId"].as_str().unwrap().len(), 64);
    assert!(!body.contains("PRIVATE-BOARD-SERIAL-123"));
}

#[tokio::test]
async fn registration_rejects_non_success_status_without_hardware_value() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || read_request_and_respond(listener, 503));

    let error = register_baseboard_serial_at(
        "PRIVATE-BOARD-SERIAL-456",
        "0.12.0",
        &format!("http://{address}/api/tools/claude-codex-pro/register"),
    )
    .await
    .unwrap_err()
    .to_string();
    server.join().unwrap();

    assert!(error.contains("HTTP 503"));
    assert!(!error.contains("PRIVATE-BOARD-SERIAL-456"));
}

fn read_request_and_respond(listener: TcpListener, status: u16) -> String {
    let (mut stream, _) = listener.accept().unwrap();
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected_length = None;

    loop {
        let read = stream.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if expected_length.is_none() {
            if let Some(header_end) = find_bytes(&request, b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length: ")
                            .or_else(|| line.strip_prefix("Content-Length: "))
                    })
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                expected_length = Some(header_end + 4 + content_length);
            }
        }
        if expected_length.is_some_and(|length| request.len() >= length) {
            break;
        }
    }

    let reason = if status == 200 {
        "OK"
    } else {
        "Service Unavailable"
    };
    let response_body = if status == 200 {
        r#"{"ok":true,"registered":true,"count":1}"#
    } else {
        r#"{"ok":false}"#
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
        response_body.len()
    )
    .unwrap();
    String::from_utf8(request).unwrap()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
