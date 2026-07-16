use std::time::Duration;

/// Total request timeout. Guards every call site against an upstream that
/// accepts the connection but never finishes responding (the classic
/// "UI hangs forever" case when a relay/GitHub mirror is blackholed).
///
/// NOTE: this is a *total* duration cap and MUST NOT be applied to streaming
/// conversation proxies — an SSE `/v1/messages` reply can legitimately stream
/// for many minutes, and a total cap would sever a healthy connection mid-turn
/// (upstream still emitting tokens, client sees `client_gone`). Streaming paths
/// use `streaming_proxy_client` instead. See the streaming timeout note below.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Connection-phase timeout. Fails fast when the host is unreachable or the
/// DNS/TCP handshake stalls, instead of waiting out the full request timeout.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Idle read timeout for streaming proxies. Unlike a total timeout, this fires
/// only when no bytes arrive for this long, so a long-running SSE turn stays
/// alive as long as the upstream keeps emitting events, while a truly stalled
/// (blackholed) connection is still torn down instead of hanging forever.
const STREAM_READ_TIMEOUT: Duration = Duration::from_secs(300);

pub const ANTHROPIC_VERSION: &str = "2023-06-01";

pub fn proxied_client(user_agent: &str) -> anyhow::Result<reqwest::Client> {
    let ua = normalize_user_agent(user_agent);
    Ok(reqwest::Client::builder()
        .user_agent(ua)
        .timeout(DEFAULT_REQUEST_TIMEOUT)
        .connect_timeout(DEFAULT_CONNECT_TIMEOUT)
        .build()?)
}

/// Client for streaming conversation proxies (`/v1/messages`, Chat Completions
/// relay). Sets no total-duration cap so a long SSE turn is never severed while
/// data is still flowing; a per-read idle timeout still guards against a stalled
/// upstream. Use `proxied_client` for ordinary short requests instead.
pub fn streaming_proxy_client(user_agent: &str) -> anyhow::Result<reqwest::Client> {
    let ua = normalize_user_agent(user_agent);
    Ok(reqwest::Client::builder()
        .user_agent(ua)
        .read_timeout(STREAM_READ_TIMEOUT)
        .connect_timeout(DEFAULT_CONNECT_TIMEOUT)
        .build()?)
}

fn normalize_user_agent(user_agent: &str) -> String {
    if user_agent.trim().is_empty() {
        format!("ClaudeCodexPro/{}", env!("CARGO_PKG_VERSION"))
    } else {
        user_agent.trim().to_string()
    }
}

pub fn apply_api_auth_headers(
    request: reqwest::RequestBuilder,
    api_key: &str,
    anthropic_api_key: bool,
    include_anthropic_version: bool,
) -> reqwest::RequestBuilder {
    if api_key.trim().is_empty() {
        return request;
    }

    let request = if anthropic_api_key {
        request.header("x-api-key", api_key.trim())
    } else {
        request.bearer_auth(api_key.trim())
    };
    if include_anthropic_version {
        request.header("anthropic-version", ANTHROPIC_VERSION)
    } else {
        request
    }
}
