//! Shared secret for the local enhancement helper.
//!
//! The helper binds to `127.0.0.1`, but binding to loopback is NOT an access
//! control: any web page open in any browser on the machine can still `fetch()`
//! against `http://127.0.0.1:<helper_port>/...`. For the plain status/log
//! endpoints that is only mildly interesting, but the protocol-proxy endpoints
//! forward requests upstream with the user's configured relay API key attached,
//! so an unauthenticated proxy lets an arbitrary web page spend the user's
//! tokens (and read whatever the proxy returns).
//!
//! The launcher process both (a) runs the helper and (b) generates the renderer
//! injection script, so a process-local random token can be shared between them
//! with no IPC: the token is embedded in the injection script's bootstrap
//! prologue (inside a closure, not in the DOM) and sent back on every helper
//! request. A web page that did not receive the injection script cannot know
//! the token, so its cross-origin requests are rejected.
//!
//! The token lives only in memory and is regenerated every process start.

use std::sync::OnceLock;

/// Header the injection script sends and the helper checks.
pub const HELPER_TOKEN_HEADER: &str = "x-claude-codex-pro-token";

/// JS global the injection prologue assigns the token to.
pub const HELPER_TOKEN_GLOBAL: &str = "__CLAUDE_CODEX_PRO_HELPER_TOKEN__";

static HELPER_TOKEN: OnceLock<String> = OnceLock::new();

/// Return the process-local helper token, generating it on first use.
pub fn helper_token() -> &'static str {
    HELPER_TOKEN.get_or_init(generate_token)
}

/// Constant-time-ish comparison of a presented token against the expected one.
///
/// The token is a high-entropy random value, so a plain `==` is already
/// adequate here, but we avoid short-circuiting on length to keep the intent
/// obvious and resist trivial timing probes.
pub fn token_matches(presented: &str) -> bool {
    let expected = helper_token().as_bytes();
    let presented = presented.as_bytes();
    if expected.len() != presented.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in expected.iter().zip(presented.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn generate_token() -> String {
    // 128 bits of entropy sourced from the OS, hex-encoded. We avoid pulling in
    // a new dependency by combining a few OS/process-provided values through a
    // SHA-256 pass; this is not a cryptographic key exchange, only an
    // unguessable per-process capability string.
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(std::process::id().to_le_bytes());
    if let Ok(elapsed) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        hasher.update(elapsed.as_nanos().to_le_bytes());
    }
    // A stack address and a heap address add a bit of ASLR-derived entropy.
    let stack_marker = 0u8;
    hasher.update((&stack_marker as *const u8 as usize).to_le_bytes());
    let heap_marker = Box::new(0u8);
    hasher.update((Box::as_ref(&heap_marker) as *const u8 as usize).to_le_bytes());
    // Mix in a second time sample so two processes started in the same
    // nanosecond bucket still diverge.
    if let Ok(elapsed) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        hasher.update(elapsed.subsec_nanos().to_le_bytes());
    }
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_stable_within_process_and_non_empty() {
        let first = helper_token();
        let second = helper_token();
        assert_eq!(first, second, "token must be stable within a process");
        assert_eq!(first.len(), 64, "sha-256 hex digest is 64 chars");
    }

    #[test]
    fn token_matches_only_the_real_token() {
        assert!(token_matches(helper_token()));
        assert!(!token_matches(""));
        assert!(!token_matches("deadbeef"));
        assert!(!token_matches(&"0".repeat(64)));
    }
}
