#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if claude_codex_pro_launcher::should_handle_args(&args) {
        if let Err(error) = claude_codex_pro_launcher::run(args) {
            eprintln!("Claude Codex Pro launcher failed: {error:#}");
            std::process::exit(1);
        }
        return;
    }
    if claude_codex_pro_manager_lib::commands::handle_internal_cli() {
        return;
    }
    if args.iter().any(|arg| arg == "--show-update") {
        unsafe {
            std::env::set_var("CLAUDE_CODEX_PRO_SHOW_UPDATE", "1");
        }
    }
    claude_codex_pro_manager_lib::run();
}
