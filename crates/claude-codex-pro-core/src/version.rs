pub const VERSION: &str = match option_env!("CLAUDE_CODEX_PRO_RELEASE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(
            VERSION,
            option_env!("CLAUDE_CODEX_PRO_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
        );
    }
}
