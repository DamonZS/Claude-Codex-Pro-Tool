// 活体证据：把本仓当前的注入脚本打进 Codex 页面，然后真的去点那枚 CCP 徽标。
// 这条同时覆盖「页面里已经有上一代脚本」的情况：新脚本会把徽标换成新节点再重绑，
// 并且把上一代留下的浮条收掉，所以点完只会看到一条当前结论。
// 前提：本机 Codex 桌面端在跑（调试端口 9230）。
// 运行：cargo test -p claude-codex-pro-core --test badge_click_live -- --include-ignored --nocapture
use claude_codex_pro_core::{bridge, cdp};

const DEBUG_PORT: u16 = 9230;
const SCRIPT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/inject/renderer-inject.js"
);

fn text_of(value: &serde_json::Value) -> String {
    value.to_string()
}

#[tokio::test]
#[ignore = "需要本机 Codex 正在运行"]
async fn clicking_the_ccp_badge_reports_the_update_state_instead_of_failing() {
    let script = std::fs::read_to_string(SCRIPT).expect("read the injection script");
    let targets = cdp::list_targets(DEBUG_PORT)
        .await
        .expect("list cdp targets");
    let target = cdp::pick_injectable_codex_page_target(&targets).expect("pick codex page target");
    let ws = target
        .web_socket_debugger_url
        .clone()
        .expect("target websocket url");

    bridge::evaluate_script_with_await_promise(&ws, &script, false)
        .await
        .expect("inject the current script");
    tokio::time::sleep(std::time::Duration::from_millis(700)).await;

    let badge = bridge::evaluate_script_with_await_promise(
        &ws,
        r#"(() => {
            const badge = document.querySelector(".claude-codex-pro-trigger");
            const count = document.querySelectorAll(".claude-codex-pro-trigger").length;
            if (!badge) return JSON.stringify({ clicked: false, count });
            badge.click();
            return JSON.stringify({
                clicked: true,
                count,
                label: (badge.textContent || "").trim(),
                token: badge.dataset.claudeCodexProTriggerInstalled || "",
            });
        })()"#,
        true,
    )
    .await
    .expect("click the injected badge");
    println!("badge: {badge}");
    let badge_text = text_of(&badge);
    assert!(
        badge_text.contains("clicked\\\":true") || badge_text.contains("\"clicked\":true"),
        "the injected badge must be clickable, got {badge_text}"
    );

    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    let toasts = bridge::evaluate_script_with_await_promise(
        &ws,
        r#"(() => JSON.stringify(Array.from(document.querySelectorAll(".claude-codex-pro-update-toast")).map((node) => (node.textContent || "").trim())))()"#,
        true,
    )
    .await
    .expect("read the update toast");
    println!("toasts: {toasts}");
    let toast_text = text_of(&toasts);

    assert!(
        toast_text.contains("已是最新版本") || toast_text.contains("发现新版本"),
        "clicking the badge must answer with the update state, got {toast_text}"
    );
    assert!(
        !toast_text.contains("失败") && !toast_text.contains("Failed"),
        "clicking the badge must not fail, got {toast_text}"
    );
    assert!(
        !toast_text.contains("\\\"]") || toast_text.matches("版本").count() == 1,
        "a reinjection must leave exactly one toast, got {toast_text}"
    );
}
