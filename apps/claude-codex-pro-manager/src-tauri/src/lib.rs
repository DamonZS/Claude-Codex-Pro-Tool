pub mod commands;
pub mod install;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);

pub fn run() {
    install_panic_logger();
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
        "manager.start",
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION")
        }),
    );
    let Some(_guard) = acquire_single_instance_guard() else {
        return;
    };
    let show_update = commands::startup_should_show_update();
    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let url = if show_update {
                "index.html?showUpdate=1"
            } else {
                "index.html"
            };
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(url.into()))
                .title("Claude Codex Pro 管理工具")
                .inner_size(1180.0, 820.0)
                .min_inner_size(960.0, 720.0)
                .build()?;
            match setup_tray(app) {
                Ok(()) => TRAY_AVAILABLE.store(true, Ordering::Relaxed),
                Err(error) => {
                    TRAY_AVAILABLE.store(false, Ordering::Relaxed);
                    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                        "manager.tray_failed",
                        serde_json::json!({
                            "error": error.to_string()
                        }),
                    );
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::backend_version,
            commands::startup_options,
            commands::load_overview,
            commands::load_claude_desktop_status,
            commands::load_claude_desktop_integrity,
            commands::focus_claude_desktop,
            commands::verify_claude_desktop,
            commands::open_claude_desktop_devtools,
            commands::open_claude_desktop,
            commands::open_claude_chinese_window,
            commands::load_claude_chinese_window_status,
            commands::load_claude_zh_patch_status,
            commands::install_claude_zh_patch,
            commands::restore_claude_zh_patch,
            commands::open_plugin_hub_window,
            commands::open_prompt_optimizer_window,
            commands::new_claude_desktop_chat,
            commands::paste_claude_desktop_draft,
            commands::submit_claude_desktop_text,
            commands::launch_claude_codex_pro,
            commands::restart_claude_codex_pro,
            commands::load_settings,
            commands::save_settings,
            commands::list_local_sessions,
            commands::list_zed_remote_projects,
            commands::open_zed_remote,
            commands::forget_zed_remote_project,
            commands::delete_local_session,
            commands::load_provider_sync_targets,
            commands::sync_providers_now,
            commands::load_ads,
            commands::refresh_script_market,
            commands::install_market_script,
            commands::refresh_plugin_hub_catalog,
            commands::get_plugin_hub_catalog,
            commands::preview_plugin_hub_install,
            commands::install_plugin_hub_item,
            commands::uninstall_plugin_hub_item,
            commands::set_user_script_enabled,
            commands::delete_user_script,
            commands::open_external_url,
            commands::install_entrypoints,
            commands::uninstall_entrypoints,
            commands::repair_shortcuts,
            commands::repair_backend,
            commands::check_update,
            commands::perform_update,
            commands::load_watcher_state,
            commands::install_watcher,
            commands::uninstall_watcher,
            commands::enable_watcher,
            commands::disable_watcher,
            commands::read_latest_logs,
            commands::copy_diagnostics,
            commands::reset_settings,
            commands::reset_image_overlay_settings,
            commands::relay_status,
            commands::read_relay_files,
            commands::save_relay_file,
            commands::write_diagnostic_event,
            commands::backfill_relay_profile_from_live,
            commands::list_context_entries,
            commands::read_live_context_entries,
            commands::sync_live_context_entries,
            commands::upsert_context_entry,
            commands::delete_context_entry,
            commands::extract_relay_common_config,
            commands::test_relay_profile,
            commands::fetch_relay_profile_models,
            commands::switch_relay_profile,
            commands::apply_relay_injection,
            commands::apply_pure_api_injection,
            commands::clear_relay_injection
        ])
        .run(tauri::generate_context!());
    if let Err(error) = run_result {
        let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
            "manager.run_failed",
            serde_json::json!({
                "error": error.to_string()
            }),
        );
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示管理工具", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Claude Codex Pro 管理工具")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn install_panic_logger() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|message| (*message).to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "非字符串 panic payload".to_string());
        let location = panic_info.location().map(|location| {
            serde_json::json!({
                "file": location.file(),
                "line": location.line(),
                "column": location.column()
            })
        });
        let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
            "manager.panic",
            serde_json::json!({
                "payload": payload,
                "location": location
            }),
        );
    }));
}

fn acquire_single_instance_guard() -> Option<claude_codex_pro_core::ports::LoopbackPortGuard> {
    if std::env::var("CCP_MANAGER_ALLOW_PARALLEL")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
    {
        let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
            "manager.parallel_instance_allowed",
            serde_json::json!({
                "reason": "CCP_MANAGER_ALLOW_PARALLEL"
            }),
        );
        return fallback_single_instance_guard();
    }

    match claude_codex_pro_core::ports::acquire_resilient_loopback_port_guard(
        claude_codex_pro_core::ports::MANAGER_GUARD_PORT,
    ) {
        Ok(guard) => {
            if let Some(fallback_lock_path) = guard.fallback_path() {
                let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                    "manager.guard_fallback",
                    serde_json::json!({
                        "requested_guard_port": claude_codex_pro_core::ports::MANAGER_GUARD_PORT,
                        "fallback_lock_path": fallback_lock_path
                    }),
                );
            }
            Some(guard)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                "manager.guard_conflict_parallel_fallback",
                serde_json::json!({
                    "guard_port": claude_codex_pro_core::ports::MANAGER_GUARD_PORT,
                    "error": error.to_string(),
                    "reason": "existing instance may be hidden or elevated"
                }),
            );
            fallback_single_instance_guard()
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                "manager.guard_conflict_parallel_fallback",
                serde_json::json!({
                    "guard_port": claude_codex_pro_core::ports::MANAGER_GUARD_PORT,
                    "error": error.to_string(),
                    "reason": "existing instance may be hidden or elevated"
                }),
            );
            fallback_single_instance_guard()
        }
        Err(error) => {
            let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                "manager.guard_failed",
                serde_json::json!({
                    "guard_port": claude_codex_pro_core::ports::MANAGER_GUARD_PORT,
                    "error": error.to_string()
                }),
            );
            fallback_single_instance_guard()
        }
    }
}

fn fallback_single_instance_guard() -> Option<claude_codex_pro_core::ports::LoopbackPortGuard> {
    match std::net::TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => Some(claude_codex_pro_core::ports::LoopbackPortGuard::listener(
            listener,
        )),
        Err(fallback_error) => {
            let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                "manager.guard_fallback_failed",
                serde_json::json!({
                    "error": fallback_error.to_string()
                }),
            );
            None
        }
    }
}
