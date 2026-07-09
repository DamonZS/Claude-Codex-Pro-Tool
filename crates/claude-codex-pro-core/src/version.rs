pub const DEFAULT_RELEASE_VERSION: &str = concat!("dev-", env!("CARGO_PKG_VERSION"));

pub const VERSION: &str = match option_env!("CLAUDE_CODEX_PRO_RELEASE_VERSION") {
    Some(version) => version,
    None => DEFAULT_RELEASE_VERSION,
};

#[cfg(test)]
mod tests {
    use super::{DEFAULT_RELEASE_VERSION, VERSION};

    #[test]
    fn exposes_project_release_version() {
        assert_eq!(
            DEFAULT_RELEASE_VERSION,
            concat!("dev-", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            VERSION,
            option_env!("CLAUDE_CODEX_PRO_RELEASE_VERSION").unwrap_or(DEFAULT_RELEASE_VERSION)
        );
        assert_ne!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
