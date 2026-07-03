use std::time::Duration;

/// Total request timeout. Guards every call site against an upstream that
/// accepts the connection but never finishes responding (the classic
/// "UI hangs forever" case when a relay/GitHub mirror is blackholed).
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Connection-phase timeout. Fails fast when the host is unreachable or the
/// DNS/TCP handshake stalls, instead of waiting out the full request timeout.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub fn proxied_client(user_agent: &str) -> anyhow::Result<reqwest::Client> {
    let ua = if user_agent.trim().is_empty() {
        format!("ClaudeCodexPro/{}", env!("CARGO_PKG_VERSION"))
    } else {
        user_agent.trim().to_string()
    };
    Ok(reqwest::Client::builder()
        .user_agent(ua)
        .timeout(DEFAULT_REQUEST_TIMEOUT)
        .connect_timeout(DEFAULT_CONNECT_TIMEOUT)
        .build()?)
}
