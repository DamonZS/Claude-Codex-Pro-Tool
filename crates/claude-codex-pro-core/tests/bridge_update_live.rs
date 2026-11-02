// 活体证据：注入页里的桥接能不能回答「检查更新」。
// 需要本机正在跑 Codex 桌面端（调试端口 9230）并已注入 CCP 桥接。
// 运行：cargo test -p claude-codex-pro-core --test bridge_update_live -- --include-ignored --nocapture
use claude_codex_pro_core::bridge;
use claude_codex_pro_core::cdp;

const DEBUG_PORT: u16 = 9230;

#[tokio::test]
#[ignore = "需要本机 Codex 正在运行且已注入桥接"]
async fn the_injected_page_can_answer_update_check_and_update_install() {
    let targets = cdp::list_targets(DEBUG_PORT)
        .await
        .expect("list cdp targets");
    let target = cdp::pick_injectable_codex_page_target(&targets).expect("pick codex page target");
    let ws = target
        .web_socket_debugger_url
        .clone()
        .expect("target websocket url");

    let has_bridge = bridge::evaluate_script_with_await_promise(
        &ws,
        "Boolean(window.__codexSessionDeleteBridge)",
        true,
    )
    .await
    .expect("probe bridge binding");
    println!("bridge binding: {has_bridge:?}");

    let check = bridge::evaluate_script_with_await_promise(
        &ws,
        r#"window.__codexSessionDeleteBridge("/update/check", {}).then((r) => JSON.stringify(r))"#,
        true,
    )
    .await
    .expect("call /update/check through the bridge");
    println!("update/check: {check}");
    let text = check.to_string();
    assert!(
        text.contains("\\\"status\\\":\\\"ok\\\"") || text.contains("\"status\":\"ok\""),
        "bridge must answer ok, got {text}"
    );
}
