//! 托盘快捷菜单（cc-switch 风格）。
//!
//! 菜单事件回调运行在 Tauri 主线程，因此这里只做动作分派：所有可能耗时数秒的
//! 操作（进程枚举/结束、UAC 提权、网络与浏览器启动）都移到独立后台线程执行。
//! 动作失败只写 `manager.tray.<action>` 诊断日志，不 panic、不弹阻塞对话框。

use std::thread;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

use crate::commands;

/// 「显示管理工具」菜单项 id（沿用旧托盘菜单 id）。
pub const MENU_ID_SHOW: &str = "show";
/// 「启动或重启 Codex」菜单项 id。
pub const MENU_ID_CODEX_RESTART: &str = "codex_restart";
/// 「启动或重启 Claude」菜单项 id。
pub const MENU_ID_CLAUDE_RESTART: &str = "claude_restart";
/// 「Claude 一键汉化」菜单项 id。
pub const MENU_ID_CLAUDE_ZH_PATCH: &str = "claude_zh_patch";
/// 「在 GitHub 点个 Star」菜单项 id。
pub const MENU_ID_GITHUB_STAR: &str = "github_star";
/// 「退出」菜单项 id（沿用旧托盘菜单 id）。
pub const MENU_ID_QUIT: &str = "quit";

/// 仓库地址：托盘「点个 Star」入口使用。
pub const GITHUB_REPOSITORY_URL: &str = "https://github.com/DamonZS/Claude-Codex-Pro-Tool";

const TRAY_ID: &str = "main";
const TRAY_TOOLTIP: &str = "CCP 管理工具";
const TRAY_ACTION_THREAD_PREFIX: &str = "ccp-tray-";

/// 构建托盘图标与快捷菜单。
///
/// 左键单击仍只显示主窗口（`show_menu_on_left_click(false)` 保持），
/// 右键弹出 6 项快捷菜单。
pub fn setup(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_ID_SHOW, "显示管理工具", true, None::<&str>)?;
    let codex = MenuItem::with_id(
        app,
        MENU_ID_CODEX_RESTART,
        "启动或重启 Codex",
        true,
        None::<&str>,
    )?;
    let claude = MenuItem::with_id(
        app,
        MENU_ID_CLAUDE_RESTART,
        "启动或重启 Claude",
        true,
        None::<&str>,
    )?;
    let claude_zh = MenuItem::with_id(
        app,
        MENU_ID_CLAUDE_ZH_PATCH,
        "Claude 一键汉化",
        true,
        None::<&str>,
    )?;
    let github_star = MenuItem::with_id(
        app,
        MENU_ID_GITHUB_STAR,
        "在 GitHub 点个 Star",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_ID_QUIT, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show, &codex, &claude, &claude_zh, &github_star, &quit],
    )?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(TRAY_TOOLTIP)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_ID_SHOW => show_main_window(app),
            MENU_ID_CODEX_RESTART => spawn_codex_restart(),
            MENU_ID_CLAUDE_RESTART => spawn_claude_restart(),
            MENU_ID_CLAUDE_ZH_PATCH => spawn_claude_zh_patch(),
            MENU_ID_GITHUB_STAR => spawn_github_star(),
            MENU_ID_QUIT => {
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

/// 显示、取消最小化并聚焦主窗口；窗口尚未创建时静默忽略。
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 「启动或重启 Codex」：复用 `restart_claude_codex_pro`，由其停止旧进程后再启动。
///
/// 请求参数走 `LaunchRequest` 的 serde 默认值（空 `app_path`、默认端口），
/// 让既有 `normalize_launch_request` 自行解析本机 Codex 路径。
fn spawn_codex_restart() {
    spawn_background("codex_restart", || {
        let request: commands::LaunchRequest = match serde_json::from_value(serde_json::json!({})) {
            Ok(request) => request,
            Err(error) => {
                log_tray_failure(
                    "codex_restart",
                    &format!("构造 Codex 启动参数失败：{error}"),
                );
                return;
            }
        };
        let result = tauri::async_runtime::block_on(commands::restart_claude_codex_pro(request));
        log_tray_result_if_failed("codex_restart", &result.status, &result.message);
    });
}

/// 「启动或重启 Claude」：复用 `open_claude_desktop`，其内部已按运行状态选择 open/restart。
fn spawn_claude_restart() {
    spawn_background("claude_restart", || {
        let result = tauri::async_runtime::block_on(commands::open_claude_desktop());
        log_tray_result_if_failed("claude_restart", &result.status, &result.message);
    });
}

/// 「Claude 一键汉化」：复用既有汉化补丁安装链路（含提权分支与失败回滚）。
fn spawn_claude_zh_patch() {
    spawn_background("claude_zh_patch", || {
        let result = tauri::async_runtime::block_on(commands::install_claude_zh_patch());
        log_tray_result_if_failed("claude_zh_patch", &result.status, &result.message);
    });
}

/// 「在 GitHub 点个 Star」：复用 `open_external_url` 的系统浏览器打开与 URL 校验。
fn spawn_github_star() {
    spawn_background("github_star", || {
        let result = commands::open_external_url(GITHUB_REPOSITORY_URL.to_string());
        log_tray_result_if_failed("github_star", &result.status, &result.message);
    });
}

/// 在独立后台线程执行动作，避免阻塞托盘事件与主线程。
fn spawn_background<F>(action: &'static str, task: F)
where
    F: FnOnce() + Send + 'static,
{
    let thread_name = format!("{TRAY_ACTION_THREAD_PREFIX}{action}");
    if let Err(error) = thread::Builder::new().name(thread_name).spawn(task) {
        log_tray_failure(action, &format!("后台线程启动失败：{error}"));
    }
}

/// 仅当命令返回 `failed` 时写诊断日志；`ok` / `accepted` / `warning` 保持静默。
fn log_tray_result_if_failed(action: &str, status: &str, message: &str) {
    if status == "failed" {
        log_tray_failure(action, message);
    }
}

/// 写一条 `manager.tray.<action>` 诊断日志；写日志本身失败也不影响托盘。
fn log_tray_failure(action: &str, message: &str) {
    let _ = claude_codex_pro_core::diagnostic_log::append_diagnostic_log(
        &format!("manager.tray.{action}"),
        serde_json::json!({
            "status": "failed",
            "message": message
        }),
    );
}
