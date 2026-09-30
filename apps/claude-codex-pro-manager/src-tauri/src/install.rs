pub use claude_codex_pro_core::install::{EntryPointState, ShortcutState, inspect_entrypoints};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_entrypoints_reports_two_entrypoints() {
        let state = inspect_entrypoints();

        assert!(matches!(state.silent_shortcut.installed, true | false));
        assert!(matches!(state.management_shortcut.installed, true | false));
    }
}
