use claude_codex_pro_core::assets;
use sha2::{Digest, Sha256};

#[test]
fn workflow_bundle_and_style_participate_in_live_renderer_fingerprint() {
    let bundle =
        include_str!("../../../apps/codex-workflow-surface/dist/codex-workflow-surface.js");
    let style =
        include_str!("../../../apps/codex-workflow-surface/dist/codex-workflow-surface.css");
    let mut hash = Sha256::new();
    hash.update(assets::renderer_script().as_bytes());
    hash.update(bundle.as_bytes());
    hash.update(style.as_bytes());
    assert_eq!(
        assets::renderer_fingerprint(),
        format!("sha256:{:x}", hash.finalize())
    );
    assert!(bundle.contains("__CODEX_WORKFLOW_SURFACE__"));
    assert!(!style.trim().is_empty());
}

#[test]
fn workflow_bootstrap_precedes_renderer_mount_and_uses_only_bundled_resources() {
    let script = assets::injection_script(57321);
    let style = script.find("window.__CODEX_WORKFLOW_STYLES__ =").unwrap();
    let bundle = script.find("__CODEX_WORKFLOW_SURFACE__").unwrap();
    let renderer = script
        .find("function multicaWorkspaceRenderUpstreamSurface")
        .unwrap();
    assert!(style < bundle && bundle < renderer);
    assert!(script.contains("workflow_generation_stale"));
    assert!(script.contains("multicaWorkspaceActivateNativeThread(threadId)"));
    assert!(script.contains("delete window.__CODEX_WORKFLOW_SURFACE__;\n}\n"));
}
