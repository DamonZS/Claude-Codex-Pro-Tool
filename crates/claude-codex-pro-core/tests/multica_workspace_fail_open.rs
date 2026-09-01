use claude_codex_pro_core::assets;

fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing source section start: {start}"));
    rest.split_once(end)
        .map(|(section, _)| section)
        .unwrap_or_else(|| panic!("missing source section end: {end}"))
}

#[test]
fn multica_workspace_bridge_failure_keeps_local_board_open_and_retryable() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let open = source_between(
        workspace,
        "async function multicaWorkspaceOpen()",
        "function multicaWorkspaceHide()",
    );
    let loader = source_between(
        workspace,
        "async function multicaWorkspaceLoadCurrentRoute",
        "async function multicaWorkspaceOpen()",
    );
    let query = source_between(
        workspace,
        "async function multicaWorkspaceQuery(module, force = false, timeoutMs = 15000)",
        "async function multicaWorkspaceLoadCurrentRoute",
    );
    let bootstrap = source_between(
        workspace,
        "async function multicaWorkspaceLoadBootstrap(force = false, timeoutMs = 15000)",
        "async function multicaWorkspaceQuery(module, force = false, timeoutMs = 15000)",
    );
    let board = source_between(
        workspace,
        "function multicaWorkspaceRenderIssueBoard(content, module)",
        "function multicaWorkspaceSkillsInventoryReadOnly()",
    );
    let fail_open = source_between(
        workspace,
        "function multicaWorkspaceFailOpen(message = \"\")",
        "function multicaWorkspaceFeatureEnabled()",
    );
    let hide = source_between(
        workspace,
        "function multicaWorkspaceHide()",
        "async function multicaWorkspaceBackgroundSync()",
    );
    let request = source_between(
        workspace,
        "function multicaWorkspaceRequest(path, payload, timeoutMs = 15000)",
        "async function multicaWorkspaceCall",
    );

    assert!(open.contains("multicaWorkspaceState.opened = true;"));
    assert!(open.contains("const ready = await multicaWorkspaceLoadCurrentRoute(true, 2000, openSequence);"));
    assert!(!open.contains("await multicaWorkspaceLoadBootstrap"));
    assert!(!open.contains("await multicaWorkspaceQuery"));
    assert!(open.contains("readyMain.style.visibility = \"hidden\""));

    assert!(loader.contains("await multicaWorkspaceLoadBootstrap(force, timeoutMs)"));
    assert!(loader.contains("return multicaWorkspaceQuery(module, force, timeoutMs);"));
    assert!(!query.contains("multicaWorkspaceFailOpen"));
    assert!(query.contains("multicaWorkspaceSetEntryAvailability(\"启动器未连接，请通过 CCP 启动 Codex\")"));
    assert!(bootstrap.contains("if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();"));
    assert!(board.contains("multicaWorkspaceBoardColumns.forEach"));
    assert!(!board.contains("content.appendChild(page);\n      return;"));

    assert!(request.contains("setTimeout(() => resolve({ status: \"failed\", message: \"工作区请求超时\", timeout: true }), timeoutMs)"));
    assert!(fail_open.contains("multicaWorkspaceHide();"));
    assert!(fail_open.contains("multicaWorkspaceSetEntryAvailability(detail);"));
    assert!(hide.contains("multicaWorkspaceRestoreMain();"));
    assert!(hide.contains("multicaWorkspaceState.host.style.display = \"none\""));
    assert!(fail_open.contains("if (multicaWorkspaceState.opened)"));
    assert!(fail_open.contains("multicaWorkspaceRenderContent();"));
}

#[test]
fn multica_workspace_unavailable_entry_remains_retryable_and_is_not_empty_board() {
    let script = assets::injection_script(57321);
    let workspace = source_between(
        &script,
        "// The workspace is deliberately kept in this injection file",
        "function labelUnlockedPluginEntry",
    );
    let entry = source_between(
        workspace,
        "function multicaWorkspaceEnsureEntry(pluginButton)",
        "function multicaWorkspaceSetStatus",
    );
    let availability = source_between(
        workspace,
        "function multicaWorkspaceSetEntryAvailability(message = \"\")",
        "function multicaWorkspaceBridgeUnavailable",
    );

    assert!(entry.contains("multicaWorkspaceOpen();"));
    assert!(availability.contains("entry.dataset.ccpMulticaAvailability = \"unavailable\""));
    assert!(availability.contains("entry.setAttribute(\"data-state\", \"unavailable\")"));
    assert!(availability.contains("entry.setAttribute(\"aria-description\", `${detail}；点击重试`)"));
    assert!(availability.contains("entry.title = `我的任务（未连接，点击重试：${detail}）`"));
    assert!(!availability.contains("无任务"));
}
