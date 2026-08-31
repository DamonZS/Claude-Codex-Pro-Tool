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
            "version": claude_codex_pro_core::version::VERSION,
            "exePath": commands::current_exe_path_string(),
            "exeLastModifiedMs": commands::current_exe_last_modified_ms()
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
                .title("CCP 管理工具")
                .inner_size(1180.0, 820.0)
                .min_inner_size(960.0, 640.0)
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
            tauri::async_runtime::spawn(async {
                commands::ensure_claude_desktop_proxy_on_startup().await;
            });
            tauri::async_runtime::spawn(async {
                // Restore only user-configured manual sidecars. The core
                // filter excludes the reserved managed connection, so startup
                // never downloads, registers, or supervises a Runtime.
                let result = tauri::async_runtime::spawn_blocking(
                    claude_codex_pro_core::multica::start_auto_start_sidecars,
                )
                .await;
                let (started, failed) = match result {
                    Ok(Ok(outcomes)) => outcomes.into_iter().fold(
                        (0_u64, 0_u64),
                        |(started, failed), (_, status)| {
                            // A spawned process is not a successful start until
                            // the isolated Multica health probe has confirmed it.
                            if status.status == "healthy" {
                                (started + 1, failed)
                            } else {
                                (started, failed + 1)
                            }
                        },
                    ),
                    Ok(Err(_)) | Err(_) => (0, 1),
                };
                let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                    "manager.multica.auto_start",
                    serde_json::json!({
                        "started": started,
                        "failed": failed,
                    }),
                );
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                stop_multica_sidecars_before_exit();
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::backend_version,
            commands::startup_options,
            commands::load_overview,
            commands::run_maintenance_check,
            commands::load_claude_desktop_status,
            commands::load_claude_desktop_status_light,
            commands::load_claude_desktop_integrity,
            commands::focus_claude_desktop,
            commands::verify_claude_desktop,
            commands::open_claude_desktop_devtools,
            commands::open_claude_desktop,
            commands::open_claude_chinese_window,
            commands::load_claude_chinese_window_status,
            commands::load_claude_zh_patch_status,
            commands::install_claude_zh_patch,
            commands::install_claude_zh_patch_at_install_root,
            commands::restore_claude_zh_patch,
            commands::open_plugin_hub_window,
            commands::new_claude_desktop_chat,
            commands::paste_claude_desktop_draft,
            commands::submit_claude_desktop_text,
            commands::launch_claude_codex_pro,
            commands::restart_claude_codex_pro,
            commands::load_settings,
            commands::save_settings,
            commands::list_local_sessions,
            commands::load_codex_session_context,
            commands::list_claude_sessions,
            commands::load_claude_session_context,
            commands::load_memory_assist_status,
            commands::migrate_memory_assist_data_dir,
            commands::load_memory_outcome_dashboard,
            commands::load_memory_new_project_guide,
            commands::query_memory_assist,
            commands::list_memory_assist_items,
            commands::learn_memory_assist_item,
            commands::update_memory_assist_item,
            commands::delete_memory_assist_item,
            commands::archive_memory_assist_item,
            commands::restore_memory_assist_item,
            commands::create_memory_assist_candidate,
            commands::list_memory_assist_candidates,
            commands::approve_memory_assist_candidate,
            commands::reject_memory_assist_candidate,
            commands::load_memory_assist_session,
            commands::run_memory_assist_selfcheck,
            commands::export_memory_assist,
            commands::import_memory_assist,
            commands::register_memory_mcp_server,
            commands::list_zed_remote_projects,
            commands::open_zed_remote,
            commands::forget_zed_remote_project,
            commands::delete_local_session,
            commands::delete_claude_session,
            commands::load_provider_sync_targets,
            commands::sync_providers_now,
            commands::load_ads,
            commands::refresh_script_market,
            commands::install_market_script,
            commands::load_codex_plugin_marketplace_status,
            commands::repair_codex_plugin_marketplace,
            commands::list_codex_custom_marketplaces,
            commands::add_codex_custom_marketplace,
            commands::remove_codex_custom_marketplace,
            commands::export_session_universal,
            commands::migrate_session_to_claude_code,
            commands::refresh_plugin_hub_catalog,
            commands::get_plugin_hub_catalog,
            commands::list_codex_themes,
            commands::load_codex_theme_background,
            commands::list_codex_manager_backgrounds,
            commands::preview_codex_diy_theme_background,
            commands::load_codex_diy_theme_background,
            commands::set_codex_manager_background,
            commands::clear_codex_manager_background,
            commands::apply_codex_manager_background,
            commands::delete_codex_manager_background,
            commands::import_codex_theme,
            commands::save_codex_diy_theme,
            commands::download_codex_theme,
            commands::delete_codex_theme,
            commands::apply_codex_theme,
            commands::restore_codex_default_theme,
            commands::list_system_prompts,
            commands::save_system_prompt,
            commands::import_system_prompt,
            commands::delete_system_prompt,
            commands::enable_system_prompt,
            commands::disable_system_prompt,
            commands::sync_system_prompt_url,
            commands::preview_plugin_hub_install,
            commands::install_plugin_hub_item,
            commands::uninstall_plugin_hub_item,
            commands::preview_ponytail_codex_hooks,
            commands::trust_ponytail_codex_hooks,
            commands::generate_ponytail_mcpb_installer,
            commands::load_claude_desktop_org_plugin_status,
            commands::load_claude_desktop_marketplace_status,
            commands::load_claude_desktop_dev_mode_status,
            commands::configure_claude_desktop_dev_mode,
            commands::refresh_claude_third_party_config,
            commands::repair_claude_desktop_marketplaces,
            commands::open_ponytail_claude_desktop_marketplace_setup,
            commands::open_claude_desktop_org_plugins_dir,
            commands::install_ponytail_claude_desktop_org_plugin,
            commands::install_ponytail_claude_desktop_local_bundle,
            commands::set_user_script_enabled,
            commands::delete_user_script,
            commands::open_external_url,
            commands::install_entrypoints,
            commands::uninstall_entrypoints,
            commands::repair_shortcuts,
            commands::repair_backend,
            commands::repair_frontend_connection,
            commands::repair_backend_service,
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
            commands::diagnose_codex_credential_environment,
            commands::clear_codex_user_credential_environment,
            commands::read_relay_files,
            commands::save_relay_file,
            commands::write_diagnostic_event,
            commands::backfill_relay_profile_from_live,
            commands::list_context_entries,
            commands::read_live_context_entries,
            commands::scan_unified_tool_inventory,
            commands::toggle_unified_tool_asset,
            commands::sync_live_context_entries,
            commands::upsert_context_entry,
            commands::delete_context_entry,
            commands::list_claude_context_entries,
            commands::upsert_claude_context_entry,
            commands::delete_claude_context_entry,
            commands::extract_relay_common_config,
            commands::test_relay_profile,
            commands::fetch_relay_profile_models,
            commands::import_ccswitch_codex_providers,
            commands::switch_relay_profile,
            commands::switch_supplier_profile,
            commands::preview_claude_desktop_provider,
            commands::apply_claude_desktop_provider,
            commands::restore_claude_desktop_provider_official,
            commands::apply_relay_injection,
            commands::apply_pure_api_injection,
            commands::clear_relay_injection,
            commands::get_multica_managed_runtime,
            commands::ensure_multica_runtime,
            commands::cancel_multica_runtime_install,
            commands::rollback_multica_runtime,
            commands::login_multica_managed,
            commands::logout_multica_managed,
            commands::set_multica_managed_enabled,
            commands::save_multica_managed_connection,
            commands::check_multica_managed_runtime,
            commands::start_multica_managed_runtime,
            commands::stop_multica_managed_runtime,
            commands::restart_multica_managed_runtime,
            commands::list_multica_connections,
            commands::save_multica_connection,
            commands::delete_multica_connection,
            commands::check_multica_connection,
            commands::get_multica_snapshot,
            commands::start_multica_sidecar,
            commands::stop_multica_sidecar,
            commands::restart_multica_sidecar,
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
    // Keep a final cleanup boundary for exits that do not pass through a
    // window close event (for example a Tauri runtime error). The operation is
    // idempotent and only sees sidecars tracked by the core adapter.
    stop_multica_sidecars_before_exit();
}

/// Clean up only Multica sidecars owned by this manager process before the
/// application exits. The core helper validates each child executable and
/// never searches for or terminates unrelated provider, proxy, Codex, or
/// Claude processes. Shutdown continues when one record cannot be verified.
fn stop_multica_sidecars_before_exit() {
    // Close the core adapter's sidecar admission gate before taking the
    // cleanup snapshot.  The auto-start worker may still be finishing a
    // blocking spawn; the gate makes that race end in a killed, untracked
    // child instead of an orphan process after the manager exits.
    claude_codex_pro_core::multica::request_shutdown();
    let outcomes = match claude_codex_pro_core::multica::stop_all_sidecars() {
        Ok(outcomes) => outcomes,
        Err(_) => {
            let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
                "manager.multica.sidecars_exit",
                serde_json::json!({
                    "tracked": 0_u64,
                    "stopped": 0_u64,
                    "degraded": 1_u64,
                }),
            );
            return;
        }
    };
    let stopped = outcomes
        .iter()
        .filter(|(_, status)| status.status == "stopped")
        .count();
    let degraded = outcomes.len().saturating_sub(stopped);
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
        "manager.multica.sidecars_exit",
        serde_json::json!({
            "tracked": outcomes.len(),
            "stopped": stopped,
            "degraded": degraded,
        }),
    );
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示管理工具", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("CCP 管理工具")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => {
                stop_multica_sidecars_before_exit();
                app.exit(0);
            }
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
