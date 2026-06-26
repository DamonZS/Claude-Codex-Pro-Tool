#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    if claude_codex_pro_manager_lib::commands::handle_internal_cli() {
        return;
    }
    if std::env::args().any(|arg| arg == "--show-update") {
        unsafe {
            std::env::set_var("CLAUDE_CODEX_PRO_SHOW_UPDATE", "1");
        }
    }
    claude_codex_pro_manager_lib::run();
}
