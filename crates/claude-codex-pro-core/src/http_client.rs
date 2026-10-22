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
    proxied_client_for(user_agent, false)
}

/// `local` marks a request aimed at a loopback endpoint (the user's own local
/// relay/gateway). Ambient proxies are bypassed for loopback targets: a local
/// HTTP proxy (Clash/…) intercepts loopback traffic too and then answers on the
/// upstream's behalf, so a connection the upstream dropped without replying
/// comes back as a synthetic `502`. On Windows the system-proxy reader
/// (hyper-util `client-proxy-system`) never honors IP-literal `ProxyOverride`
/// entries such as `127.*` — only domain entries — so an enabled system proxy
/// hijacks loopback no matter what the bypass list says. Remote targets keep
/// the historical behavior (`local = false`).
pub fn proxied_client_for(user_agent: &str, local: bool) -> anyhow::Result<reqwest::Client> {
    let ua = normalize_user_agent(user_agent);
    let builder = reqwest::Client::builder()
        .user_agent(ua)
        .timeout(DEFAULT_REQUEST_TIMEOUT)
        .connect_timeout(DEFAULT_CONNECT_TIMEOUT);
    #[cfg(test)]
    let builder = builder.no_proxy();
    Ok(bypass_ambient_proxy_for_loopback(builder, local).build()?)
}

/// Client for streaming conversation proxies (`/v1/messages`, Chat Completions
/// relay). Sets no total-duration cap so a long SSE turn is never severed while
/// data is still flowing; a per-read idle timeout still guards against a stalled
/// upstream. Use `proxied_client` for ordinary short requests instead.
pub fn streaming_proxy_client(user_agent: &str) -> anyhow::Result<reqwest::Client> {
    streaming_proxy_client_for(user_agent, false)
}

/// Streaming counterpart of `proxied_client_for`; see its `local` contract.
pub fn streaming_proxy_client_for(
    user_agent: &str,
    local: bool,
) -> anyhow::Result<reqwest::Client> {
    let ua = normalize_user_agent(user_agent);
    let builder = reqwest::Client::builder()
        .user_agent(ua)
        .read_timeout(STREAM_READ_TIMEOUT)
        .connect_timeout(DEFAULT_CONNECT_TIMEOUT);
    Ok(bypass_ambient_proxy_for_loopback(builder, local).build()?)
}

fn bypass_ambient_proxy_for_loopback(
    builder: reqwest::ClientBuilder,
    local: bool,
) -> reqwest::ClientBuilder {
    if local { builder.no_proxy() } else { builder }
}

/// Whether an outbound URL targets a loopback endpoint (`localhost`,
/// `127.0.0.0/8`, `[::1]`).
///
/// A loopback host is the user's own local relay/gateway, so its requests must
/// go direct: an ambient HTTP proxy (Clash/…) intercepts loopback too and then
/// answers for the upstream, turning a connection the upstream dropped without
/// replying into a synthetic `502`. Callers pass the result as `local` to the
/// `*_for` builders above; remote hosts keep the historical behavior.
pub(crate) fn url_target_is_loopback(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    let host = url.host_str().unwrap_or("");
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .map(|address| address.is_loopback())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the loopback boundary every outbound call site relies on: only the
    /// user's own local relay host may bypass the ambient proxy. Widening this
    /// predicate would silently route remote relays direct; narrowing it would
    /// let Clash/… hijack loopback again (see `proxied_client_for`).
    #[test]
    fn url_target_is_loopback_covers_loopback_hosts_and_rejects_others() {
        // The user's own local relay: must bypass the ambient proxy.
        let loopback = [
            "http://localhost:57321/v1", // local relay addressed by host name
            "http://127.0.0.1:8080",     // loopback IPv4 literal, no path
            "http://127.9.9.9:1/v1",     // every 127.0.0.0/8 address is loopback
            "http://[::1]:57321/v1",     // bracketed IPv6 loopback literal
        ];
        for base_url in loopback {
            assert!(
                url_target_is_loopback(base_url),
                "expected loopback: {base_url}"
            );
        }

        // Not the user's own machine: proxy handling must stay untouched.
        let not_loopback = [
            "http://10.0.0.5:8080/v1",      // private LAN host, not loopback
            "http://192.168.1.20:8080/v1",  // private LAN host, not loopback
            "https://api.toporeduce.cn/v1", // remote relay host
            "",                             // empty base_url
            "not a url",                    // unparsable base_url
        ];
        for base_url in not_loopback {
            assert!(
                !url_target_is_loopback(base_url),
                "expected non-loopback: {base_url}"
            );
        }
    }
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
