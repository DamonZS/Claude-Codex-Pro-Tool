(() => {
  // A reinjection must leave the previous workspace host, observers and
  // native-content snapshot behind before the new closure starts.
  try {
    window.__claudeCodexProMulticaWorkspaceCleanup?.();
  } catch (_) {}
  try {
    window.__claudeCodexProCodexPageHostCleanup?.();
  } catch (_) {}
  window.__claudeCodexProMulticaWorkspaceGeneration =
    (window.__claudeCodexProMulticaWorkspaceGeneration || 0) + 1;
  const claudeCodexProMulticaWorkspaceGeneration =
    window.__claudeCodexProMulticaWorkspaceGeneration;
  window.__claudeCodexProCodexPageHostGeneration =
    (window.__claudeCodexProCodexPageHostGeneration || 0) + 1;
  const claudeCodexProCodexPageHostGeneration =
    window.__claudeCodexProCodexPageHostGeneration;
  const helperBase = window.__CODEX_SESSION_DELETE_HELPER__ || "http://127.0.0.1:57321";
  // Per-process capability token injected by the launcher. The helper requires
  // it on the token-consuming proxy endpoints so that a random web page (which
  // never received this script) cannot drive the user's relay API key. Captured
  // in the closure, not left on the DOM, so page scripts cannot read it back.
  const helperToken = window.__CLAUDE_CODEX_PRO_HELPER_TOKEN__ || "";
  const helperTokenHeader = "x-claude-codex-pro-token";
  const withHelperToken = (headers) => {
    const merged = Object.assign({}, headers || {});
    if (helperToken) merged[helperTokenHeader] = helperToken;
    return merged;
  };
  const buttonClass = "codex-delete-button";
  const exportButtonClass = "codex-export-button";
  const projectMoveButtonClass = "codex-project-move-button";
  const projectMoveOverlayClass = "codex-project-move-overlay";
  const actionButtonClass = "codex-session-action-button";
  const actionGroupClass = "codex-session-actions";
  const moreButtonClass = "codex-session-more-button";
  const moreMenuClass = "codex-session-more-menu";
  const actionTooltipClass = "codex-session-action-tooltip";
  const timelineClass = "codex-conversation-timeline";
  const timelineTrackClass = "codex-conversation-timeline-track";
  const timelineMarkerClass = "codex-conversation-timeline-marker";
  const timelineTooltipClass = "codex-conversation-timeline-tooltip";
  const timelineTargetClass = "codex-conversation-timeline-target";
  const conversationViewMinWidth = 320;
  const conversationViewMaxAllowedWidth = 4000;
  const conversationViewDefaultWidth = 900;
  const conversationViewLegacyWidthKey = "claudeCodexPro.threadCenter.maxWidth";
  const zedRemoteButtonClass = "codex-zed-remote-button";
  const zedRemoteOpenInMenuItemClass = "codex-zed-open-in-menu-item";
  const zedRemoteToastClass = "codex-zed-remote-toast";
  const upstreamWorktreeDialogClass = "codex-upstream-worktree-dialog";
  const upstreamBranchOptionAttribute = "data-codex-upstream-branch-option";
  const upstreamBranchSelectionKey = "codexUpstreamBranchSelection";
  const upstreamProjectContextKey = "codexUpstreamProjectContext";
  const codexMemoryProjectContextKey = "claudeCodexProMemoryProjectContext";
  const zedRemoteOpenVersion = "1";
  const zedRemoteOpenInMenuVersion = "1";
  const zedRemoteOpenInMenuActivationWindowMs = 600;
  const timelineQuestionLimit = 40;
  const timelineMinTopPercent = 2;
  const timelineMaxTopPercent = 98;
  const timelineMaxMarkerGapPercent = 3.5;
  const projectMoveProjectionKey = "codexProjectMoveProjection";
  const legacyProjectMoveOverridesKey = "codexProjectMoveOverrides";
  const projectMoveProjectionTtlMs = 24 * 60 * 60 * 1000;
  const projectMoveProjectionSettleMs = 5 * 60 * 1000;
  const projectMoveRefreshDelaysMs = [50, 250, 750, 1500];
  const chatsSortRefreshIntervalMs = 1500;
  const chatsSortDbRefreshIntervalMs = 5000;
  const styleId = "codex-delete-style";
  const codexDeleteStyleVersion = "14";
  const claudeCodexProMenuId = "claude-codex-pro-menu";
  const claudeCodexProMenuFloatingClass = "claude-codex-pro-menu-floating";
  const claudeCodexProMenuVersion = "12";
  const claudeCodexProTriggerVersion = "6";
  const claudeCodexProModalTheme = "pangu-control-deck";
  const codexDeleteVersion = "7";
  const codexExportVersion = "1";
  const codexProjectMoveVersion = "1";
  const codexActionGroupVersion = "5";
  const codexArchiveRowActionsVersion = "1";
  const codexArchiveDeleteAllVersion = "2";
  const codexConversationTimelineVersion = "2";
  const codexConversationViewVersion = "1";
  const codexThreadScrollVersion = "1";
  const codexThreadServiceTierVersion = "1";
  const codexServiceTierBadgeClass = "codex-service-tier-badge";
  const codexServiceTierBadgeVersion = "3";
  const codexMemoryBadgeId = "codex-memory-assist-badge";
  const codexMemoryPanelId = "codex-memory-assist-panel";
  const codexMemoryAssistVersion = "1";
  let claudeCodexProVersion = window.__CLAUDE_CODEX_PRO_VERSION__ || "unknown";
  window.__CLAUDE_CODEX_PRO_MODAL_THEME__ = claudeCodexProModalTheme;
  const claudeCodexProBuild = window.__CLAUDE_CODEX_PRO_BUILD__ || "unknown";
  const claudeCodexProSupportPaymentQr = window.__CLAUDE_CODEX_PRO_SUPPORT_PAYMENT_QR__ || "";
  const claudeCodexProContactWechatQr = window.__CLAUDE_CODEX_PRO_CONTACT_WECHAT_QR__ || "";
  const claudeCodexProBundledAnnouncement = window.__CLAUDE_CODEX_PRO_ANNOUNCEMENT__ || { enabled: false, ads: [] };
  const claudeCodexProQqGroupPrimaryUrl = "https://qm.qq.com/cgi-bin/qm/qr?k=uwNon9opx0Arfovyo5qJQQ2jUvlxSpmf&jump_from=webapi&authKey=El8Xwz9ZqefrpE4BhW9xWQsEAUFvptw74MBsRKRJTw5x5QiEPiG0fmdVIf9VuMWg";
  const claudeCodexProQqGroupSecondaryUrl = "https://qm.qq.com/cgi-bin/qm/qr?k=cIeUYUFyy0ypTWMqo8CfgRwq8jU_OrXy&jump_from=webapi&authKey=njT7ceHMggvpptkiy9xD6FbBubVGCDof0cnX0adhLgUvi9kKZP4OY51M1xWZBy68";
  const claudeCodexProSettingsKey = "claudeCodexProSettings";
  const codexThreadScrollKey = "codexThreadScroll";
  const codexThreadServiceTierKey = "codexThreadServiceTierOverrides";
  const codexThreadServiceTierMaxEntries = 120;
  const codexThreadServiceTierDraftBindWindowMs = 60 * 1000;
  const codexServiceTierRequestOverrideVersion = "3";
  const codexPluginMarketplaceUnlockVersion = "13";
  const codexThreadScrollMaxEntries = 120;
  const codexThreadScrollSaveThrottleMs = 120;
  const codexThreadScrollRestoreWindowMs = 3200;
  const codexThreadScrollRestoreDelaysMs = [0, 80, 220, 500, 1000, 1800, 2800];
  const codexThreadScrollUserIntentWindowMs = 1200;
  const codexThreadScrollProgrammaticGuardVersion = "dispatcher:2";
  const codexThreadScrollRouteHooksVersion = "dispatcher:2";
  const codexThreadScrollListenerVersion = "4";
  const codexThreadScrollUserIntentVersion = "dispatcher:2";
  const codexForcePluginInstallRefreshIntervalMs = 1000;
  const claudeCodexProImageOverlayId = "claude-codex-pro-image-overlay";
  window.__codexProjectMoveRuntimeId = (window.__codexProjectMoveRuntimeId || 0) + 1;
  const codexProjectMoveRuntimeId = window.__codexProjectMoveRuntimeId;
  clearTimeout(window.__codexProjectMoveProjectionTimer);
  clearTimeout(window.__codexProjectMoveChatsSortTimer);
  window.__codexProjectMoveProjectionTimer = null;
  window.__codexProjectMoveChatsSortTimer = null;
  clearTimeout(window.__codexThreadScrollSaveTimer);
  window.__codexThreadScrollSaveTimer = null;
  (window.__codexThreadScrollRestoreTimers || []).forEach((timer) => clearTimeout(timer));
  window.__codexThreadScrollRestoreTimers = [];
  (window.__codexThreadScrollSyncTimers || []).forEach((timer) => clearTimeout(timer));
  window.__codexThreadScrollSyncTimers = [];
  window.__claudeCodexProBackendHeartbeatGeneration =
    (window.__claudeCodexProBackendHeartbeatGeneration || 0) + 1;
  const claudeCodexProBackendHeartbeatGeneration =
    window.__claudeCodexProBackendHeartbeatGeneration;
  window.__codexThreadScrollRestoreRevision = (window.__codexThreadScrollRestoreRevision || 0) + 1;

  function installClaudeCodexProImageOverlay() {
    if (!document?.getElementById || !document?.createElement || !document?.documentElement?.appendChild) return;
    const config = window.__CLAUDE_CODEX_PRO_IMAGE_OVERLAY__ || {};
    const existing = document.getElementById(claudeCodexProImageOverlayId);
    const source = config.dataUrl || "";
    if (!config.enabled || !source) {
      if (window.__claudeCodexProImageOverlayBlobUrl) {
        URL.revokeObjectURL(window.__claudeCodexProImageOverlayBlobUrl);
        window.__claudeCodexProImageOverlayBlobUrl = "";
      }
      if (existing) existing.remove();
      return;
    }
    const opacity = Math.min(1, Math.max(0.01, Number(config.opacity) || 0.35));
    const image = existing || document.createElement("img");
    image.id = claudeCodexProImageOverlayId;
    image.src = source;
    image.alt = "";
    image.setAttribute("aria-hidden", "true");
    Object.assign(image.style, {
      position: "fixed",
      inset: "0",
      width: "100vw",
      height: "100vh",
      objectFit: "contain",
      objectPosition: "center center",
      opacity: String(opacity),
      pointerEvents: "none",
      zIndex: "2147483646",
      userSelect: "none",
    });
    if (!existing) document.documentElement.appendChild(image);
    sendClaudeCodexProDiagnostic("image_overlay_installed", {
      opacity,
      sourceKind: source.startsWith("data:") ? "data-uri" : "unknown",
    });
  }

  function scheduleClaudeCodexProImageOverlay() {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", installClaudeCodexProImageOverlay, { once: true });
      return;
    }
    installClaudeCodexProImageOverlay();
    setTimeout(installClaudeCodexProImageOverlay, 250);
  }

  scheduleClaudeCodexProImageOverlay();
  window.__codexThreadScrollSyncRevision = (window.__codexThreadScrollSyncRevision || 0) + 1;
  window.__codexConversationTimelineNodeCounter = window.__codexConversationTimelineNodeCounter || 0;
  let upstreamBranchDefaultsCache = new Map();
  const upstreamBranchDefaultsCacheTtlMs = 5000;
  const upstreamRemoteBranchDefaultsCacheTtlMs = 30000;
  let upstreamBranchDefaultsInflight = new Map();
  const upstreamProjectContextTtlMs = 10 * 60 * 1000;
  const branchWorktreePathAttribute = "data-codex-branch-worktree-path";
  ["__claudeCodexProHtmlCenteredThreadWidth", "__claudeCodexProViewportCenteredThreadWidth", "__claudeCodexProBoundedThreadCenter"].forEach((key) => {
    try {
      window[key]?.cleanup?.();
    } catch (_) {}
  });
  try {
    window.__claudeCodexProConversationViewCleanup?.();
  } catch (_) {}
  window.__claudeCodexProConversationViewCleanup = null;
  const selectors = {
    sidebarThread: "[data-app-action-sidebar-thread-id]",
    threadTitle: "[data-thread-title]",
    appHeader: ".app-header-tint",
    nativeMenuBar: "[class*=\"ms-auto\"][class*=\"flex\"][class*=\"items-center\"]",
    headerContextMenuSurface: '[data-testid="app-shell-header-context-menu-surface"]',
    archiveNav: 'button[aria-label="已归档对话"], button[aria-label="Archived conversations"]',
    disabledInstallButton: 'button:disabled, button[aria-disabled="true"], [role="button"][aria-disabled="true"], button[data-disabled], [role="button"][data-disabled], button.cursor-not-allowed, [role="button"].cursor-not-allowed, button.pointer-events-none, [role="button"].pointer-events-none',
    // Codex has shipped both the current `sidebar-item` and the older token
    // navigation class. Keep the semantic label fallback as the final check.
    pluginNavButton: 'nav[role="navigation"] button.sidebar-item, aside.app-shell-left-panel button.sidebar-item, nav[role="navigation"] button.h-token-nav-row.w-full',
    // Keep the search scoped to navigation/aside containers. Recent Codex
    // builds sometimes omit the navigation role or the app-shell class.
    pluginAnchorButton: 'nav[role="navigation"] button, [role="navigation"] button, aside.app-shell-left-panel button, aside button',
    pluginAnchorRegion: 'nav[role="navigation"], [role="navigation"], aside.app-shell-left-panel, aside',
    pluginSvgPath: 'svg path[d^="M8.25031 1.46094"], svg path[d^="M7.94562 14.0277"]',
  };

  function installStyle() {
    const existingStyle = document.getElementById(styleId);
    if (existingStyle?.dataset.codexDeleteStyleVersion === codexDeleteStyleVersion) return;
    existingStyle?.remove();
    const style = document.createElement("style");
    style.id = styleId;
    style.dataset.codexDeleteStyleVersion = codexDeleteStyleVersion;
    style.textContent = `
      .${actionGroupClass} {
        position: absolute;
        right: var(--codex-session-actions-right, 28px);
        top: 50%;
        transform: translateY(-50%);
        z-index: 20;
        opacity: 0;
        pointer-events: none;
        display: inline-flex;
        align-items: center;
        gap: 6px;
        background: transparent;
      }
      .${actionButtonClass} {
        width: 26px;
        height: 26px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 0;
        border-radius: 6px;
        background: transparent;
        color: #d1d5db;
        font: 14px/1 system-ui, sans-serif;
        padding: 0;
        cursor: default;
        text-align: center;
      }
      .${actionButtonClass} svg {
        display: block;
        width: 16px;
        height: 16px;
      }
      .${actionButtonClass}:hover,
      .${actionButtonClass}:focus-visible {
        background: #363839;
        color: #f4f4f5;
        outline: none;
      }
      .${moreMenuClass} {
        position: fixed;
        z-index: 2147483201;
        min-width: 104px;
        border: 1px solid rgba(255,255,255,.1);
        border-radius: 10px;
        background: #242628;
        color: #f4f4f5;
        box-shadow: 0 14px 40px rgba(0,0,0,.28);
        padding: 5px;
      }
      .${moreMenuClass}[hidden] { display: none !important; }
      .${moreMenuClass}.codex-session-more-menu-open-up {
        transform: translateY(calc(-100% - 34px));
      }
      .codex-session-more-menu-item {
        width: 100%;
        border: 0;
        border-radius: 7px;
        background: transparent;
        color: inherit;
        cursor: default;
        display: flex;
        align-items: center;
        gap: 8px;
        font: 13px/18px system-ui, sans-serif;
        padding: 6px 8px;
        text-align: left;
      }
      .codex-session-more-menu-item:hover,
      .codex-session-more-menu-item:focus-visible {
        background: #363839;
        outline: none;
      }
      .codex-session-more-menu-icon {
        width: 16px;
        text-align: center;
      }
      .codex-archive-row-button {
        border: 1px solid #ef4444;
        border-radius: 7px;
        background: #f3f4f6;
        color: #374151;
        font: 12px system-ui, sans-serif;
        line-height: 16px;
        padding: 3px 8px;
        cursor: pointer;
      }
      .codex-archive-row-button.${buttonClass} {
        border-color: #ef4444;
        background: #fee2e2;
        color: #991b1b;
      }
      .codex-archive-row-button.${exportButtonClass} {
        border-color: #93c5fd;
        background: #dbeafe;
        color: #1d4ed8;
      }
      .codex-force-install-unlocked {
        border-color: #ef4444 !important;
        background: #fee2e2 !important;
        color: #991b1b !important;
        opacity: 1 !important;
      }
      .${zedRemoteButtonClass} {
        border: 1px solid #10a37f;
        border-radius: 7px;
        background: #d1fae5;
        color: #065f46;
        font: 12px system-ui, sans-serif;
        line-height: 16px;
        margin-left: 6px;
        padding: 2px 7px;
        cursor: pointer;
      }
      .${zedRemoteButtonClass}:hover,
      .${zedRemoteButtonClass}:focus-visible {
        background: #a7f3d0;
        outline: none;
      }
      .${zedRemoteOpenInMenuItemClass} {
        cursor: pointer;
      }
      .codex-zed-open-in-menu-icon {
        width: 18px;
        height: 18px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        object-fit: contain;
      }
      .${zedRemoteToastClass} {
        position: fixed;
        right: 18px;
        bottom: 58px;
        z-index: 2147483000;
        max-width: min(420px, calc(100vw - 36px));
        border-radius: 8px;
        background: #111827;
        color: #ffffff;
        font: 13px system-ui, sans-serif;
        line-height: 18px;
        padding: 10px 12px;
        box-shadow: 0 8px 30px rgba(0,0,0,.25);
        pointer-events: none;
      }
      [data-codex-delete-row="true"]:hover .${actionGroupClass} {
        opacity: 1;
        pointer-events: auto;
      }
      [data-codex-delete-row="true"].codex-session-more-open .${actionGroupClass} {
        opacity: 1;
        pointer-events: auto;
        z-index: 2147483201;
      }
      [data-codex-delete-row="true"]:hover [data-thread-title] {
        display: block;
        max-width: var(--codex-session-title-max-width, 100%);
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      [data-codex-delete-row="true"].codex-archive-confirm-visible .${actionGroupClass} {
        right: max(66px, var(--codex-session-actions-right, 28px));
      }
      .${actionTooltipClass} {
        position: fixed;
        z-index: 2147483201;
        max-width: min(220px, calc(100vw - 32px));
        border: 1px solid rgba(255,255,255,.1);
        border-radius: 12px;
        background: #242628;
        color: #f4f4f5;
        font: 14px/20px system-ui, sans-serif;
        padding: 9px 12px;
        box-shadow: 0 14px 40px rgba(0,0,0,.28);
        pointer-events: none;
        white-space: nowrap;
      }
      .${projectMoveOverlayClass} {
        position: fixed;
        inset: 0;
        z-index: 2147483200;
        background: rgba(15,23,42,.28);
      }
      .codex-project-move-panel {
        position: fixed;
        width: min(360px, calc(100vw - 32px));
        max-height: min(520px, calc(100vh - 32px));
        overflow: hidden;
        border: 1px solid rgba(15,23,42,.14);
        border-radius: 10px;
        background: #ffffff;
        color: #111827;
        font: 13px system-ui, sans-serif;
        box-shadow: 0 18px 60px rgba(15,23,42,.25);
      }
      .codex-project-move-header { border-bottom: 1px solid #e5e7eb; padding: 10px 12px; }
      .codex-project-move-title { font-weight: 650; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      .codex-project-move-list { max-height: min(440px, calc(100vh - 110px)); overflow-y: auto; padding: 6px; }
      .codex-project-move-item {
        display: block;
        width: 100%;
        border: 0;
        border-radius: 7px;
        background: transparent;
        color: #111827;
        padding: 8px 9px;
        text-align: left;
        cursor: pointer;
      }
      .codex-project-move-item:hover,
      .codex-project-move-item:focus-visible { background: #f3f4f6; outline: none; }
      .codex-project-move-item-title { font-weight: 550; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      .codex-project-move-item-path { margin-top: 2px; color: #6b7280; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      .codex-project-move-empty { padding: 18px 12px; color: #6b7280; text-align: center; }
      .codex-project-move-hidden { display: none !important; }
      [data-codex-project-move-injected-list="true"] { display: flex; flex-direction: column; }
      .codex-archive-delete-all {
        border: 1px solid #ef4444;
        border-radius: 7px;
        background: #fee2e2;
        color: #991b1b;
        font: 12px system-ui, sans-serif;
        line-height: 16px;
        padding: 3px 8px;
        cursor: pointer;
      }
      .codex-archive-action-bar {
        position: fixed;
        right: 28px;
        top: 86px;
        z-index: 2147482999;
        box-shadow: 0 8px 24px rgba(0,0,0,.18);
      }
      .codex-delete-toast {
        position: fixed;
        right: 18px;
        bottom: 18px;
        z-index: 2147483000;
        padding: 10px 12px;
        border-radius: 8px;
        background: #111827;
        color: white;
        font: 13px system-ui, sans-serif;
        box-shadow: 0 8px 30px rgba(0,0,0,.25);
        pointer-events: none;
      }
      .codex-delete-toast button { margin-left: 10px; pointer-events: auto; }
      .codex-delete-confirm-overlay {
        position: fixed;
        inset: 0;
        z-index: 2147483200;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(15,23,42,.28);
      }
      .codex-delete-confirm-content {
        width: min(420px, calc(100vw - 48px));
        border: 1px solid rgba(15,23,42,.12);
        border-radius: 12px;
        background: #ffffff;
        color: #111827;
        font: 14px system-ui, sans-serif;
        box-shadow: 0 24px 80px rgba(15,23,42,.22);
        padding: 18px;
      }
      .codex-delete-confirm-title { font-size: 16px; font-weight: 650; }
      .codex-delete-confirm-message { margin-top: 8px; color: #4b5563; line-height: 1.45; }
      .codex-delete-confirm-actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        margin-top: 18px;
      }
      .codex-delete-confirm-actions button {
        border: 1px solid #d1d5db;
        border-radius: 7px;
        padding: 6px 12px;
        background: #ffffff;
        color: #111827;
        font: 13px system-ui, sans-serif;
        cursor: pointer;
      }
      .codex-delete-confirm-actions [data-codex-delete-confirm="true"] {
        border-color: #ef4444;
        background: #dc2626;
        color: #ffffff;
      }
      /* Dark theme overrides for delete-confirm and project-move dialogs.
         Triggered either by Codex applying a "dark" class / data-theme="dark"
         on its document root, or by the OS-level prefers-color-scheme hint.
         Keep these legacy dialogs readable when Codex itself is in dark mode. */
      html.dark .codex-delete-confirm-overlay,
      html[data-theme="dark"] .codex-delete-confirm-overlay,
      :root[data-theme="dark"] .codex-delete-confirm-overlay {
        background: rgba(0,0,0,.55);
      }
      html.dark .codex-delete-confirm-content,
      html[data-theme="dark"] .codex-delete-confirm-content,
      :root[data-theme="dark"] .codex-delete-confirm-content {
        border-color: rgba(255,255,255,.12);
        background: #2b2b2b;
        color: #f3f4f6;
        box-shadow: 0 24px 80px rgba(0,0,0,.55);
      }
      html.dark .codex-delete-confirm-message,
      html[data-theme="dark"] .codex-delete-confirm-message,
      :root[data-theme="dark"] .codex-delete-confirm-message {
        color: #d1d5db;
      }
      html.dark .codex-delete-confirm-actions button,
      html[data-theme="dark"] .codex-delete-confirm-actions button,
      :root[data-theme="dark"] .codex-delete-confirm-actions button {
        border-color: rgba(255,255,255,.18);
        background: #3f3f46;
        color: #f3f4f6;
      }
      html.dark .codex-delete-confirm-actions [data-codex-delete-confirm="true"],
      html[data-theme="dark"] .codex-delete-confirm-actions [data-codex-delete-confirm="true"],
      :root[data-theme="dark"] .codex-delete-confirm-actions [data-codex-delete-confirm="true"] {
        border-color: #ef4444;
        background: #dc2626;
        color: #ffffff;
      }
      html.dark .${projectMoveOverlayClass},
      html[data-theme="dark"] .${projectMoveOverlayClass},
      :root[data-theme="dark"] .${projectMoveOverlayClass} {
        background: rgba(0,0,0,.55);
      }
      html.dark .codex-project-move-panel,
      html[data-theme="dark"] .codex-project-move-panel,
      :root[data-theme="dark"] .codex-project-move-panel {
        border-color: rgba(255,255,255,.12);
        background: #2b2b2b;
        color: #f3f4f6;
        box-shadow: 0 18px 60px rgba(0,0,0,.55);
      }
      html.dark .codex-project-move-header,
      html[data-theme="dark"] .codex-project-move-header,
      :root[data-theme="dark"] .codex-project-move-header {
        border-bottom-color: rgba(255,255,255,.1);
      }
      html.dark .codex-project-move-item,
      html[data-theme="dark"] .codex-project-move-item,
      :root[data-theme="dark"] .codex-project-move-item {
        color: #f3f4f6;
      }
      html.dark .codex-project-move-item:hover,
      html.dark .codex-project-move-item:focus-visible,
      html[data-theme="dark"] .codex-project-move-item:hover,
      html[data-theme="dark"] .codex-project-move-item:focus-visible,
      :root[data-theme="dark"] .codex-project-move-item:hover,
      :root[data-theme="dark"] .codex-project-move-item:focus-visible {
        background: rgba(255,255,255,.08);
      }
      html.dark .codex-project-move-item-path,
      html[data-theme="dark"] .codex-project-move-item-path,
      :root[data-theme="dark"] .codex-project-move-item-path,
      html.dark .codex-project-move-empty,
      html[data-theme="dark"] .codex-project-move-empty,
      :root[data-theme="dark"] .codex-project-move-empty {
        color: #9ca3af;
      }
      @media (prefers-color-scheme: dark) {
        html:not(.light):not([data-theme="light"]) .codex-delete-confirm-overlay {
          background: rgba(0,0,0,.55);
        }
        html:not(.light):not([data-theme="light"]) .codex-delete-confirm-content {
          border-color: rgba(255,255,255,.12);
          background: #2b2b2b;
          color: #f3f4f6;
          box-shadow: 0 24px 80px rgba(0,0,0,.55);
        }
        html:not(.light):not([data-theme="light"]) .codex-delete-confirm-message {
          color: #d1d5db;
        }
        html:not(.light):not([data-theme="light"]) .codex-delete-confirm-actions button {
          border-color: rgba(255,255,255,.18);
          background: #3f3f46;
          color: #f3f4f6;
        }
        html:not(.light):not([data-theme="light"]) .codex-delete-confirm-actions [data-codex-delete-confirm="true"] {
          border-color: #ef4444;
          background: #dc2626;
          color: #ffffff;
        }
        html:not(.light):not([data-theme="light"]) .${projectMoveOverlayClass} {
          background: rgba(0,0,0,.55);
        }
        html:not(.light):not([data-theme="light"]) .codex-project-move-panel {
          border-color: rgba(255,255,255,.12);
          background: #2b2b2b;
          color: #f3f4f6;
          box-shadow: 0 18px 60px rgba(0,0,0,.55);
        }
        html:not(.light):not([data-theme="light"]) .codex-project-move-header {
          border-bottom-color: rgba(255,255,255,.1);
        }
        html:not(.light):not([data-theme="light"]) .codex-project-move-item {
          color: #f3f4f6;
        }
        html:not(.light):not([data-theme="light"]) .codex-project-move-item:hover,
        html:not(.light):not([data-theme="light"]) .codex-project-move-item:focus-visible {
          background: rgba(255,255,255,.08);
        }
        html:not(.light):not([data-theme="light"]) .codex-project-move-item-path,
        html:not(.light):not([data-theme="light"]) .codex-project-move-empty {
          color: #9ca3af;
        }
      }
      #${claudeCodexProMenuId}.${claudeCodexProMenuFloatingClass} {
        position: fixed;
        top: var(--claude-codex-pro-menu-top, 8px);
        left: var(--claude-codex-pro-menu-left, calc(100vw - 220px));
        right: auto;
        transform: none;
        z-index: 2147483647;
        height: var(--claude-codex-pro-menu-height, 30px);
        color: #a9a4a9;
        font: 13px system-ui, sans-serif;
        text-align: left;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        pointer-events: auto;
        -webkit-app-region: no-drag;
      }
      #${claudeCodexProMenuId} {
        display: inline-flex;
        align-items: center;
        height: 100%;
        flex: 0 0 auto;
        pointer-events: auto;
        -webkit-app-region: no-drag;
      }
      .claude-codex-pro-trigger {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        border: 0;
        background: transparent;
        color: inherit;
        font: inherit;
        height: 100%;
        line-height: 1;
        padding: 0 4px;
        border-radius: 0;
        box-shadow: none;
        cursor: pointer;
        pointer-events: auto;
        -webkit-app-region: no-drag;
      }
      .claude-codex-pro-trigger:hover {
        background: transparent;
      }
      .claude-codex-pro-window-status-label {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        white-space: nowrap;
      }
      .claude-codex-pro-window-status-title {
        margin-right: 2px;
        color: inherit;
        font-weight: 650;
      }
      .claude-codex-pro-window-status-dot {
        width: 8px;
        height: 8px;
        border-radius: 999px;
        background: #ef4444;
        box-shadow: 0 0 0 3px rgba(239,68,68,.18);
        display: inline-block;
      }
      .claude-codex-pro-window-status-dot[data-status="ok"] {
        background: #34d399;
        box-shadow: 0 0 0 3px rgba(52,211,153,.18);
      }
      .claude-codex-pro-window-status-dot[data-status="checking"] {
        background: #ef4444;
        box-shadow: 0 0 0 3px rgba(239,68,68,.18);
      }
      .claude-codex-pro-modal-overlay {
        position: fixed;
        inset: 0;
        z-index: 2147483647;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(15,23,42,.32);
        pointer-events: auto;
        -webkit-app-region: no-drag;
      }
      .claude-codex-pro-modal-content {
        width: min(520px, calc(100vw - 48px));
        max-height: min(680px, calc(100vh - 40px));
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border: 1px solid #dce3ed;
        border-radius: 8px;
        background: #ffffff;
        color: #172033;
        font: 14px system-ui, sans-serif;
        box-shadow: 0 24px 80px rgba(15,23,42,.22);
        pointer-events: auto;
        -webkit-app-region: no-drag;
      }
      .claude-codex-pro-modal-content[data-claude-codex-pro-active-tab="support"],
      .claude-codex-pro-modal-content[data-claude-codex-pro-active-tab="contact"] { width: min(820px, calc(100vw - 48px)); }
      .claude-codex-pro-modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 16px 20px 8px;
        flex: 0 0 auto;
        -webkit-app-region: no-drag;
      }
      .claude-codex-pro-modal-title { display: flex; align-items: center; gap: 8px; font-size: 18px; font-weight: 650; }
      .claude-codex-pro-backend-indicator { width: 9px; height: 9px; border-radius: 999px; background: #94a3b8; display: inline-block; }
      .claude-codex-pro-backend-indicator[data-status="ok"] { background: #34d399; box-shadow: 0 0 8px rgba(52,211,153,.75); }
      .claude-codex-pro-backend-indicator[data-status="failed"] { background: #ef4444; box-shadow: 0 0 8px rgba(239,68,68,.75); }
      .claude-codex-pro-backend-indicator[data-status="checking"] { background: #fbbf24; }
      .claude-codex-pro-modal-close {
        border: 0;
        background: transparent;
        color: #64748b;
        font-size: 20px;
        cursor: pointer;
        pointer-events: auto;
        -webkit-app-region: no-drag;
      }
      .claude-codex-pro-modal-body {
        flex: 1 1 auto;
        min-height: 0;
        overflow-y: auto;
        overscroll-behavior: contain;
        scrollbar-gutter: stable;
        padding: 4px 20px 16px;
        scrollbar-width: thin;
        scrollbar-color: rgba(100,116,139,.36) transparent;
      }
      .claude-codex-pro-modal-body::-webkit-scrollbar { width: 10px; }
      .claude-codex-pro-modal-body::-webkit-scrollbar-track { background: transparent; }
      .claude-codex-pro-modal-body::-webkit-scrollbar-thumb {
        border: 2px solid transparent;
        border-radius: 999px;
        background: rgba(100,116,139,.36);
        background-clip: padding-box;
      }
      .claude-codex-pro-modal-body::-webkit-scrollbar-thumb:hover { background: rgba(100,116,139,.5); background-clip: padding-box; }
      .claude-codex-pro-row {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        padding: 10px 0;
        border-top: 1px solid #dce3ed;
      }
      .claude-codex-pro-row:first-child { border-top: 0; }
      .claude-codex-pro-row-title { font-weight: 550; line-height: 1.35; }
      .claude-codex-pro-row-description { margin-top: 2px; color: #64748b; font-size: 12px; line-height: 1.4; }
      .claude-codex-pro-model-compat-warning { margin-top: 6px; color: #fbbf24; font-size: 12px; line-height: 1.45; }
      .claude-codex-pro-toggle {
        width: 42px;
        height: 24px;
        border: 0;
        border-radius: 999px;
        background: #cbd5e1;
        padding: 2px;
      }
      .claude-codex-pro-toggle span {
        display: block;
        width: 20px;
        height: 20px;
        border-radius: 999px;
        background: white;
        transition: transform .12s ease;
      }
      .claude-codex-pro-toggle,
      .claude-codex-pro-action-button,
      .claude-codex-pro-issue-button,
      .claude-codex-pro-status-note {
        flex-shrink: 0;
        align-self: center;
      }
      .claude-codex-pro-toggle[data-enabled="true"] { background: #0f766e; }
      .claude-codex-pro-toggle[data-enabled="true"] span { transform: translateX(18px); }
      .claude-codex-pro-toggle[data-relay-unneeded="true"] { width: 72px; cursor: default; background: #eef6f5; color: #0f766e; }
      .claude-codex-pro-toggle[data-relay-unneeded="true"] span { display: none; }
      .claude-codex-pro-toggle[data-relay-unneeded="true"]::after { content: "无需开启"; font-size: 12px; font-weight: 650; line-height: 1; }
      .claude-codex-pro-width-control { display: flex; align-items: center; justify-content: flex-end; gap: 8px; min-width: 176px; align-self: center; }
      .claude-codex-pro-width-input {
        width: 78px;
        height: 26px;
        box-sizing: border-box;
        border: 1px solid #cbd5e1;
        border-radius: 7px;
        background: #ffffff;
        color: #172033;
        font: 12px system-ui, sans-serif;
        padding: 0 8px;
      }
      .claude-codex-pro-width-input:disabled { opacity: .55; cursor: not-allowed; }
      .claude-codex-pro-service-tier-control { display: grid; gap: 6px; min-width: 316px; justify-items: end; align-self: center; }
      .claude-codex-pro-service-tier-status { color: #64748b; font-size: 12px; line-height: 1.3; text-align: right; }
      .claude-codex-pro-service-tier-status[data-status="ok"] { color: #0f766e; }
      .claude-codex-pro-service-tier-status[data-status="failed"] { color: #dc2626; }
      .claude-codex-pro-service-tier-status[data-status="unsupported"] { color: #b45309; }
      .claude-codex-pro-service-tier-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
      .claude-codex-pro-service-tier-thread-actions { opacity: .88; align-items: center; }
      .claude-codex-pro-service-tier-thread-label { color: #64748b; font: 12px/1.2 system-ui, sans-serif; white-space: nowrap; }
      .claude-codex-pro-service-tier-button { border: 1px solid #cbd5e1; border-radius: 7px; background: #ffffff; color: #334155; font: 12px system-ui, sans-serif; padding: 5px 8px; white-space: nowrap; }
      .claude-codex-pro-service-tier-button[data-active="true"] { border-color: #0f766e; background: #eef6f5; color: #0f766e; }
      .claude-codex-pro-service-tier-button:disabled { opacity: .55; cursor: not-allowed; }
      .claude-codex-pro-control-deck {
        --ccp-deck-bg: rgba(5, 13, 14, .48);
        --ccp-deck-panel: rgba(9, 20, 21, .52);
        --ccp-deck-panel-raised: rgba(17, 32, 33, .72);
        --ccp-deck-line: rgba(204, 246, 238, .22);
        --ccp-deck-text: #e7f2ef;
        --ccp-deck-muted: #aec2bd;
        --ccp-deck-energy: #43d6b5;
        --ccp-deck-amber: #f3b85b;
        --ccp-deck-sheen: linear-gradient(132deg, rgba(255, 255, 255, .18), rgba(255, 255, 255, .035) 24%, transparent 49%, rgba(67, 214, 181, .055) 78%, rgba(255, 255, 255, .08));
      }
      .claude-codex-pro-modal-overlay:has(.claude-codex-pro-control-deck) {
        background: rgba(2, 7, 9, .34);
        backdrop-filter: blur(18px) saturate(1.18) contrast(1.04);
        -webkit-backdrop-filter: blur(18px) saturate(1.18) contrast(1.04);
        isolation: isolate;
      }
      .claude-codex-pro-modal-content.claude-codex-pro-control-deck {
        width: min(780px, calc(100vw - 72px));
        height: min(540px, calc(100vh - 64px));
        max-height: calc(100vh - 64px);
        display: grid;
        grid-template-columns: 148px minmax(0, 1fr);
        grid-template-rows: auto minmax(0, 1fr);
        position: relative;
        isolation: isolate;
        overflow: hidden;
        border: 1px solid rgba(222, 250, 245, .34);
        border-radius: 12px;
        background-color: var(--ccp-deck-bg);
        background-image: var(--ccp-deck-sheen);
        color: var(--ccp-deck-text);
        font-family: "Segoe UI Variable", "Segoe UI", "Microsoft YaHei", system-ui, sans-serif;
        text-rendering: geometricPrecision;
        -webkit-font-smoothing: antialiased;
        box-shadow: inset 0 1px 0 rgba(255, 255, 255, .52), inset 1px 0 0 rgba(255, 255, 255, .12), inset 0 -1px 0 rgba(0, 0, 0, .38), 0 38px 110px rgba(0, 0, 0, .46), 0 0 56px rgba(67, 214, 181, .08);
        backdrop-filter: blur(46px) saturate(1.72) contrast(1.08);
        -webkit-backdrop-filter: blur(46px) saturate(1.72) contrast(1.08);
      }
      .claude-codex-pro-modal-content.claude-codex-pro-control-deck::before {
        content: "";
        position: absolute;
        z-index: 3;
        inset: 1px;
        border: 1px solid rgba(255, 255, 255, .1);
        border-radius: 11px;
        pointer-events: none;
        mask-image: linear-gradient(145deg, #000 0 26%, transparent 58%);
      }
      .claude-codex-pro-control-deck .claude-codex-pro-modal-header {
        grid-column: 1 / -1;
        min-height: 60px;
        box-sizing: border-box;
        padding: 10px 14px;
        border-bottom: 1px solid var(--ccp-deck-line);
        background-color: rgba(7, 17, 18, .48);
        background-image: var(--ccp-deck-sheen);
        box-shadow: inset 0 1px 0 rgba(255, 255, 255, .12);
        backdrop-filter: blur(30px) saturate(1.55);
        -webkit-backdrop-filter: blur(30px) saturate(1.55);
      }
      .claude-codex-pro-deck-brand { display: flex; align-items: center; gap: 9px; min-width: 0; }
      .claude-codex-pro-deck-mark {
        width: 32px;
        height: 32px;
        display: grid;
        place-items: center;
        border: 1px solid rgba(67, 214, 181, .42);
        border-radius: 8px;
        background: linear-gradient(145deg, rgba(67, 214, 181, .18), rgba(67, 214, 181, .03));
        color: var(--ccp-deck-energy);
        font: 750 12px/1 ui-monospace, "Cascadia Code", monospace;
        letter-spacing: .04em;
        box-shadow: inset 0 1px 0 rgba(255, 255, 255, .24), inset 0 -1px 0 rgba(0, 0, 0, .25), 0 10px 28px rgba(0, 0, 0, .12);
      }
      .claude-codex-pro-deck-heading { min-width: 0; }
      .claude-codex-pro-deck-kicker {
        color: var(--ccp-deck-energy);
        font: 650 10px/1.3 ui-monospace, "Cascadia Code", monospace;
        letter-spacing: .16em;
      }
      .claude-codex-pro-control-deck .claude-codex-pro-modal-title {
        margin-top: 5px;
        gap: 8px;
        color: var(--ccp-deck-text);
        font-size: 15px;
        font-weight: 650;
      }
      .claude-codex-pro-deck-version { color: var(--ccp-deck-muted); font: 500 11px/1 ui-monospace, "Cascadia Code", monospace; }
      .claude-codex-pro-control-deck .claude-codex-pro-backend-indicator { width: 7px; height: 7px; }
      .claude-codex-pro-control-deck .claude-codex-pro-modal-close {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid rgba(142, 164, 159, .25);
        border-radius: 9px;
        background: rgba(255, 255, 255, .045);
        color: var(--ccp-deck-muted);
        font-size: 18px;
      }
      .claude-codex-pro-control-deck .claude-codex-pro-modal-close:hover { border-color: rgba(67, 214, 181, .45); color: var(--ccp-deck-text); background: rgba(67, 214, 181, .08); }
      .claude-codex-pro-control-deck .claude-codex-pro-tabs {
        grid-column: 1;
        grid-row: 2;
        display: flex;
        flex-direction: column;
        gap: 5px;
        min-width: 0;
        padding: 12px 9px;
        border-right: 1px solid var(--ccp-deck-line);
        background-color: rgba(6, 15, 16, .34);
        background-image: var(--ccp-deck-sheen);
        box-shadow: inset -1px 0 0 rgba(255, 255, 255, .06);
        backdrop-filter: blur(32px) saturate(1.5);
        -webkit-backdrop-filter: blur(32px) saturate(1.5);
      }
      .claude-codex-pro-control-deck .claude-codex-pro-tab-button {
        width: 100%;
        display: flex;
        align-items: center;
        gap: 9px;
        border: 1px solid transparent;
        border-radius: 8px;
        background: transparent;
        color: var(--ccp-deck-muted);
        font: 550 12px/1.2 "Segoe UI", "Microsoft YaHei", system-ui, sans-serif;
        padding: 8px 9px;
        text-align: left;
        cursor: pointer;
      }
      .claude-codex-pro-control-deck .claude-codex-pro-tab-button::before { content: ""; width: 5px; height: 5px; border-radius: 50%; background: #52635f; box-shadow: 0 0 0 3px rgba(82, 99, 95, .12); }
      .claude-codex-pro-control-deck .claude-codex-pro-tab-button:hover { color: var(--ccp-deck-text); background: rgba(255, 255, 255, .025); }
      .claude-codex-pro-control-deck .claude-codex-pro-tab-button[data-active="true"] {
        border-color: rgba(67, 214, 181, .2);
        background: linear-gradient(90deg, rgba(67, 214, 181, .14), rgba(67, 214, 181, .035));
        color: #dffbf4;
      }
      .claude-codex-pro-control-deck .claude-codex-pro-tab-button[data-active="true"]::before { background: var(--ccp-deck-energy); box-shadow: 0 0 10px rgba(67, 214, 181, .7); }
      .claude-codex-pro-deck-sidebar-note { margin: auto 8px 2px; color: #62746f; font: 500 9px/1.55 ui-monospace, "Cascadia Code", monospace; letter-spacing: .08em; }
      .claude-codex-pro-control-deck .claude-codex-pro-modal-body {
        grid-column: 2;
        grid-row: 2;
        min-height: 0;
        padding: 14px 16px 16px;
        background: rgba(8, 17, 18, .18);
        scrollbar-color: rgba(67, 214, 181, .28) transparent;
      }
      .claude-codex-pro-deck-hero {
        margin-bottom: 10px;
        padding: 12px 14px;
        border: 1px solid rgba(67, 214, 181, .2);
        border-radius: 8px;
        background-color: rgba(18, 37, 37, .58);
        background-image: var(--ccp-deck-sheen);
        box-shadow: inset 0 1px 0 rgba(255, 255, 255, .16), 0 16px 34px rgba(0, 0, 0, .12);
        backdrop-filter: blur(22px) saturate(1.38);
        -webkit-backdrop-filter: blur(22px) saturate(1.38);
      }
      .claude-codex-pro-deck-hero-label { color: var(--ccp-deck-energy); font: 650 10px/1.3 ui-monospace, "Cascadia Code", monospace; letter-spacing: .14em; }
      .claude-codex-pro-deck-hero h2 { margin: 5px 0 4px; color: #f2fbf8; font-size: 16px; line-height: 1.25; font-weight: 650; }
      .claude-codex-pro-deck-hero p { margin: 0; color: var(--ccp-deck-muted); font-size: 12px; line-height: 1.55; }
      .claude-codex-pro-deck-capabilities { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; margin-top: 10px; }
      .claude-codex-pro-deck-capabilities span { min-height: 28px; display: grid; place-items: center; border: 1px solid rgba(208, 246, 238, .2); border-radius: 6px; background: rgba(4, 10, 13, .28); color: #d2e7e2; font-size: 10px; font-weight: 600; padding: 3px 6px; box-shadow: inset 0 1px 0 rgba(255, 255, 255, .1); }
      .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; gap: 8px; }
      .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] > .claude-codex-pro-deck-hero,
      .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] > .claude-codex-pro-deck-section-title,
      .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] > [data-codex-service-tier-controls="true"] { grid-column: 1 / -1; }
      .claude-codex-pro-deck-section-title {
        display: flex;
        align-items: center;
        gap: 9px;
        margin: 10px 2px 6px;
        color: #bcd0cb;
        font: 650 10px/1.3 ui-monospace, "Cascadia Code", monospace;
        letter-spacing: .12em;
      }
      .claude-codex-pro-deck-section-title::after { content: ""; height: 1px; flex: 1; background: linear-gradient(90deg, var(--ccp-deck-line), transparent); }
      .claude-codex-pro-control-deck .claude-codex-pro-row {
        min-width: 0;
        min-height: 58px;
        margin: 0;
        padding: 9px 10px;
        border: 1px solid rgba(142, 164, 159, .12);
        border-radius: 8px;
        background-color: var(--ccp-deck-panel-raised);
        background-image: var(--ccp-deck-sheen);
        box-shadow: inset 0 1px 0 rgba(255, 255, 255, .11), inset 0 -1px 0 rgba(0, 0, 0, .18), 0 9px 22px rgba(0, 0, 0, .08);
        backdrop-filter: blur(18px) saturate(1.32);
        -webkit-backdrop-filter: blur(18px) saturate(1.32);
      }
      .claude-codex-pro-control-deck .claude-codex-pro-row:first-child { border-top: 1px solid rgba(142, 164, 159, .12); }
      .claude-codex-pro-control-deck .claude-codex-pro-row:hover { border-color: rgba(188, 245, 233, .34); background-color: rgba(25, 47, 46, .78); box-shadow: inset 0 1px 0 rgba(255, 255, 255, .18), 0 13px 30px rgba(0, 0, 0, .12); }
      .claude-codex-pro-control-deck .claude-codex-pro-row-title { color: #e2efec; font-weight: 600; }
      .claude-codex-pro-control-deck .claude-codex-pro-row-description,
      .claude-codex-pro-control-deck .claude-codex-pro-about,
      .claude-codex-pro-control-deck .claude-codex-pro-status-note,
      .claude-codex-pro-control-deck .claude-codex-pro-service-tier-status,
      .claude-codex-pro-control-deck .claude-codex-pro-service-tier-thread-label,
      .claude-codex-pro-control-deck .claude-codex-pro-support-text,
      .claude-codex-pro-control-deck .claude-codex-pro-contact-text,
      .claude-codex-pro-control-deck .claude-codex-pro-ad-description { color: var(--ccp-deck-muted); }
      .claude-codex-pro-control-deck .claude-codex-pro-toggle { border: 1px solid rgba(142, 164, 159, .28); background: #263630; cursor: pointer; }
      .claude-codex-pro-control-deck .claude-codex-pro-toggle[data-enabled="true"] { background: var(--ccp-deck-energy); box-shadow: 0 0 14px rgba(67, 214, 181, .18); }
      .claude-codex-pro-control-deck .claude-codex-pro-action-button,
      .claude-codex-pro-control-deck .claude-codex-pro-issue-button,
      .claude-codex-pro-control-deck .claude-codex-pro-backend-repair,
      .claude-codex-pro-control-deck .claude-codex-pro-service-tier-button,
      .claude-codex-pro-control-deck .claude-codex-pro-ad-link {
        border: 1px solid rgba(67, 214, 181, .24);
        border-radius: 7px;
        background: rgba(67, 214, 181, .07);
        color: #bcebe0;
        box-shadow: none;
      }
      .claude-codex-pro-control-deck .claude-codex-pro-service-tier-button[data-active="true"] { border-color: rgba(67, 214, 181, .55); background: rgba(67, 214, 181, .17); color: #dcfaf3; }
      .claude-codex-pro-control-deck .claude-codex-pro-width-input,
      .claude-codex-pro-control-deck .claude-codex-pro-form-field input { border-color: rgba(142, 164, 159, .25); background: #0d171b; color: var(--ccp-deck-text); box-shadow: none; }
      .claude-codex-pro-control-deck .claude-codex-pro-ad-card,
      .claude-codex-pro-control-deck .claude-codex-pro-support-qr-wrap,
      .claude-codex-pro-control-deck .claude-codex-pro-support-empty,
      .claude-codex-pro-control-deck .claude-codex-pro-contact-card,
      .claude-codex-pro-control-deck .claude-codex-pro-ad-empty { border-color: rgba(67, 214, 181, .18); background: rgba(19, 30, 35, .72); box-shadow: none; }
      .claude-codex-pro-control-deck :is(button, input, a):focus-visible { outline: 2px solid var(--ccp-deck-energy); outline-offset: 2px; }
      @media (max-width: 900px) {
        .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] { grid-template-columns: minmax(0, 1fr); }
        .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] > .claude-codex-pro-deck-hero,
        .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] > .claude-codex-pro-deck-section-title,
        .claude-codex-pro-panel[data-claude-codex-pro-panel="home"] > [data-codex-service-tier-controls="true"] { grid-column: 1; }
      }
      @media (max-width: 720px) {
        .claude-codex-pro-modal-content.claude-codex-pro-control-deck {
          width: min(calc(100vw - 32px), 620px);
          height: min(540px, calc(100vh - 32px));
          max-height: calc(100vh - 32px);
          grid-template-columns: minmax(0, 1fr);
          grid-template-rows: auto auto minmax(0, 1fr);
        }
        .claude-codex-pro-control-deck .claude-codex-pro-modal-header { grid-column: 1; padding: 12px 14px; }
        .claude-codex-pro-control-deck .claude-codex-pro-tabs { grid-column: 1; grid-row: 2; flex-direction: row; overflow-x: auto; padding: 9px 12px; border-right: 0; border-bottom: 1px solid var(--ccp-deck-line); }
        .claude-codex-pro-control-deck .claude-codex-pro-tab-button { width: auto; flex: 0 0 auto; padding: 8px 10px; }
        .claude-codex-pro-deck-sidebar-note { display: none; }
        .claude-codex-pro-control-deck .claude-codex-pro-modal-body { grid-column: 1; grid-row: 3; padding: 14px; }
        .claude-codex-pro-deck-kicker { letter-spacing: .1em; }
        .claude-codex-pro-control-deck .claude-codex-pro-service-tier-control { min-width: 0; }
        .claude-codex-pro-deck-capabilities { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      }
      @media (prefers-reduced-motion: reduce) {
        .claude-codex-pro-control-deck *, .claude-codex-pro-control-deck *::before, .claude-codex-pro-control-deck *::after { scroll-behavior: auto !important; transition: none !important; animation: none !important; }
      }
      .${codexServiceTierBadgeClass} {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        flex: 0 0 auto;
        height: 24px;
        min-width: 54px;
        box-sizing: border-box;
        border: 1px solid rgba(148,163,184,.28);
        border-radius: 999px;
        background: rgba(148,163,184,.12);
        color: #d4d4d8;
        font: 600 12px/1 system-ui, sans-serif;
        padding: 0 8px;
        white-space: nowrap;
        cursor: pointer;
      }
      .${codexServiceTierBadgeClass}:hover { border-color: rgba(16,163,127,.44); background: rgba(16,163,127,.13); }
      .${codexServiceTierBadgeClass}[data-tier="fast"] { border-color: rgba(16,163,127,.55); background: rgba(16,163,127,.18); color: #6ee7b7; }
      .${codexServiceTierBadgeClass}[data-tier="loading"] { color: #a1a1aa; }
      .${codexServiceTierBadgeClass}[data-tier="failed"] { border-color: rgba(248,113,113,.42); background: rgba(248,113,113,.12); color: #fca5a5; }
      .${codexServiceTierBadgeClass}[data-tier="unsupported"] { border-color: rgba(251,191,36,.48); background: rgba(251,191,36,.13); color: #fbbf24; }
      .${codexServiceTierBadgeClass}[data-disabled="true"] { cursor: not-allowed; opacity: .78; }
      #${codexMemoryBadgeId} {
        position: fixed;
        top: var(--codex-memory-badge-top, 8px);
        left: var(--codex-memory-badge-left, 44px);
        right: var(--codex-memory-badge-right, auto);
        transform: none;
        z-index: 2147483002;
        display: inline-flex;
        align-items: center;
        gap: 7px;
        max-width: min(520px, calc(100vw - 32px));
        height: 30px;
        box-sizing: border-box;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: #a9a4a9;
        box-shadow: none;
        font: 600 12px/1 system-ui, sans-serif;
        padding: 0 4px;
        cursor: pointer;
        backdrop-filter: none;
      }
      #${codexMemoryBadgeId}[data-status="ok"] { border-color: rgba(16,185,129,.44); color: #a9a4a9; }
      #${codexMemoryBadgeId}[data-status="failed"] { border-color: rgba(248,113,113,.5); color: #a9a4a9; }
      #${codexMemoryBadgeId}[data-status="disabled"] { opacity: .75; cursor: default; }
      #${codexMemoryBadgeId} .codex-memory-dot {
        width: 8px;
        height: 8px;
        border-radius: 999px;
        background: #60a5fa;
        box-shadow: 0 0 10px rgba(96,165,250,.75);
      }
      #${codexMemoryBadgeId}[data-status="ok"] .codex-memory-dot { background: #34d399; box-shadow: 0 0 10px rgba(52,211,153,.75); }
      #${codexMemoryBadgeId}[data-status="failed"] .codex-memory-dot { background: #f87171; box-shadow: 0 0 10px rgba(248,113,113,.75); }
      .codex-memory-count { color: inherit; font-weight: 700; }
      #${codexMemoryPanelId} {
        position: fixed;
        top: 50px;
        left: 50%;
        transform: translateX(-50%);
        z-index: 2147483003;
        width: min(520px, calc(100vw - 32px));
        max-height: min(620px, calc(100vh - 72px));
        overflow: hidden;
        border: 1px solid rgba(15,23,42,.14);
        border-radius: 10px;
        background: #ffffff;
        color: #111827;
        box-shadow: 0 20px 70px rgba(15,23,42,.28);
        font: 13px system-ui, sans-serif;
      }
      #${codexMemoryPanelId}[hidden] { display: none !important; }
      .codex-memory-panel-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        border-bottom: 1px solid #e5e7eb;
        padding: 12px 14px;
      }
      .codex-memory-panel-header strong { display: block; font-size: 14px; }
      .codex-memory-panel-header span { display: block; margin-top: 2px; color: #6b7280; font-size: 12px; }
      .codex-memory-panel-close { border: 0; background: transparent; color: #6b7280; font-size: 18px; cursor: pointer; }
      .codex-memory-panel-body { display: grid; gap: 10px; max-height: min(520px, calc(100vh - 150px)); overflow-y: auto; padding: 12px 14px 14px; }
      .codex-memory-actions { display: flex; flex-wrap: wrap; gap: 8px; }
      .codex-memory-actions button {
        border: 1px solid #d1d5db;
        border-radius: 6px;
        background: #f8fafc;
        color: #111827;
        font: 12px system-ui, sans-serif;
        padding: 6px 9px;
        cursor: pointer;
      }
      .codex-memory-actions button[data-primary="true"] { border-color: #2563eb; background: #2563eb; color: #ffffff; }
      .codex-memory-panel-body textarea,
      .codex-memory-panel-body input {
        width: 100%;
        box-sizing: border-box;
        border: 1px solid #d1d5db;
        border-radius: 6px;
        background: #ffffff;
        color: #111827;
        font: 13px system-ui, sans-serif;
        padding: 8px 9px;
      }
      .codex-memory-panel-body textarea { min-height: 78px; resize: vertical; }
      .codex-memory-list { display: grid; gap: 8px; }
      .codex-memory-card { border: 1px solid #e5e7eb; border-radius: 8px; background: #f9fafb; padding: 9px; }
      .codex-memory-card strong { display: block; margin-bottom: 4px; color: #374151; font-size: 12px; }
      .codex-memory-card p { margin: 0; color: #111827; line-height: 1.45; white-space: pre-wrap; }
      .codex-memory-card small { display: block; margin-top: 6px; color: #6b7280; }
      .codex-memory-message { min-height: 18px; color: #6b7280; font-size: 12px; }
      .codex-memory-message[data-status="ok"] { color: #059669; }
      .codex-memory-message[data-status="failed"] { color: #dc2626; }
      html.dark #${codexMemoryPanelId},
      html[data-theme="dark"] #${codexMemoryPanelId},
      :root[data-theme="dark"] #${codexMemoryPanelId} {
        border-color: rgba(255,255,255,.12);
        background: #27272a;
        color: #f4f4f5;
      }
      html.dark .codex-memory-panel-header,
      html[data-theme="dark"] .codex-memory-panel-header,
      :root[data-theme="dark"] .codex-memory-panel-header { border-bottom-color: rgba(255,255,255,.1); }
      html.dark .codex-memory-panel-header span,
      html[data-theme="dark"] .codex-memory-panel-header span,
      :root[data-theme="dark"] .codex-memory-panel-header span { color: #a1a1aa; }
      html.dark .codex-memory-card,
      html[data-theme="dark"] .codex-memory-card,
      :root[data-theme="dark"] .codex-memory-card {
        border-color: rgba(255,255,255,.1);
        background: #18181b;
      }
      html.dark .codex-memory-card p,
      html[data-theme="dark"] .codex-memory-card p,
      :root[data-theme="dark"] .codex-memory-card p { color: #f4f4f5; }
      .claude-codex-pro-about { color: #64748b; line-height: 1.5; }
      .claude-codex-pro-tabs { display: flex; gap: 8px; padding: 0 20px 6px; flex: 0 0 auto; }
      .claude-codex-pro-tab-button { border: 1px solid #cbd5e1; border-radius: 999px; background: #ffffff; color: #334155; font: 12px system-ui, sans-serif; padding: 5px 10px; }
      .claude-codex-pro-tab-button[data-active="true"] { background: #0f766e; color: white; border-color: #0f766e; }
      .claude-codex-pro-panel[hidden] { display: none; }
      .claude-codex-pro-action-button,
      .claude-codex-pro-issue-button { border: 1px solid #cbd5e1; border-radius: 7px; background: #ffffff; color: #334155; font: 12px system-ui, sans-serif; padding: 6px 8px; }
      .claude-codex-pro-worktree-actions {
        display: inline-flex;
        align-items: center;
        gap: 8px;
      }
      .claude-codex-pro-form-field {
        display: grid;
        gap: 4px;
        margin-top: 10px;
        color: #334155;
        font: 12px system-ui, sans-serif;
        text-align: left;
      }
      .claude-codex-pro-form-field input {
        width: min(520px, 72vw);
        border: 1px solid #cbd5e1;
        border-radius: 8px;
        background: #ffffff;
        color: #172033;
        padding: 8px 10px;
        font: 13px ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      }
      .claude-codex-pro-form-message {
        min-height: 18px;
        margin-top: 10px;
        color: #64748b;
        font: 12px system-ui, sans-serif;
        text-align: left;
      }
      .claude-codex-pro-form-message[data-status="ok"] { color: #0f766e; }
      .claude-codex-pro-form-message[data-status="failed"] { color: #dc2626; }
      .claude-codex-pro-form-message[data-status="loading"] { color: #b45309; }
      .claude-codex-pro-status-note { min-width: 132px; color: #64748b; font-size: 12px; text-align: right; }
      .claude-codex-pro-status-note[data-status="ok"] { color: #0f766e; }
      .claude-codex-pro-status-note[data-status="failed"] { color: #dc2626; }
      .claude-codex-pro-backend-repair { border: 1px solid #cbd5e1; border-radius: 7px; background: #ffffff; color: #334155; font: 12px system-ui, sans-serif; padding: 6px 8px; }
      .claude-codex-pro-backend-repair[hidden] { display: none; }
      .claude-codex-pro-ad-section { display: grid; gap: 10px; margin-top: 12px; }
      .claude-codex-pro-ad-section:first-of-type { margin-top: 0; }
      .claude-codex-pro-ad-section-title { color: #172033; font-size: 15px; margin: 0; }
      .claude-codex-pro-ad-list { display: grid; gap: 14px; }
      .claude-codex-pro-ad-card { border: 1px solid #dce3ed; border-radius: 8px; background: #ffffff; box-shadow: 0 10px 24px rgba(15,23,42,.05); }
      .claude-codex-pro-ad-content { padding: 14px; }
      .claude-codex-pro-ad-badge { margin-bottom: 6px; color: #0f766e; font-size: 11px; font-weight: 750; }
      .claude-codex-pro-ad-title { margin: 0; color: #172033; font-size: 17px; line-height: 1.35; }
      .claude-codex-pro-ad-description { margin: 6px 0 10px; color: #64748b; font-size: 13px; line-height: 1.55; }
      .claude-codex-pro-ad-highlights { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 12px; }
      .claude-codex-pro-ad-highlights span { border: 1px solid #bfe4d2; border-radius: 999px; background: #f0fdf4; color: #166534; font-size: 12px; padding: 4px 8px; }
      .claude-codex-pro-ad-link { display: inline-flex; align-items: center; justify-content: center; border-radius: 7px; background: #0f766e; color: #ffffff; font-size: 13px; font-weight: 650; text-decoration: none; padding: 8px 12px; }
      .claude-codex-pro-ad-empty { border: 1px dashed #cbd5e1; border-radius: 8px; color: #64748b; font-size: 13px; padding: 12px; text-align: center; }
      .claude-codex-pro-support-panel { display: grid; gap: 14px; justify-items: center; padding: 8px 0 4px; text-align: center; }
      .claude-codex-pro-support-title { margin: 0; color: #172033; font-size: 18px; line-height: 1.35; }
      .claude-codex-pro-support-text { margin: 0; max-width: 520px; color: #64748b; font-size: 13px; line-height: 1.55; }
      .claude-codex-pro-support-qr-wrap { display: grid; gap: 10px; justify-items: center; width: min(360px, 100%); border: 1px solid #dce3ed; border-radius: 8px; background: #f8fafc; padding: 14px; box-sizing: border-box; }
      .claude-codex-pro-support-qr { display: block; width: min(320px, 100%); aspect-ratio: 1 / 1; border-radius: 8px; background: #ffffff; object-fit: contain; }
      .claude-codex-pro-support-empty { border: 1px dashed #cbd5e1; border-radius: 8px; color: #64748b; font-size: 13px; padding: 12px; text-align: center; }
      .claude-codex-pro-contact-panel { display: grid; gap: 14px; padding: 8px 0 4px; }
      .claude-codex-pro-contact-title { margin: 0; color: #172033; font-size: 18px; line-height: 1.35; text-align: center; }
      .claude-codex-pro-contact-text { margin: 0; color: #64748b; font-size: 13px; line-height: 1.55; text-align: center; }
      .claude-codex-pro-contact-card { display: grid; gap: 12px; border: 1px solid #dce3ed; border-radius: 10px; background: #ffffff; padding: 14px; box-sizing: border-box; }
      .claude-codex-pro-contact-line { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 8px; color: #172033; font-size: 13px; line-height: 1.5; }
      .claude-codex-pro-contact-label { font-weight: 750; }
      .claude-codex-pro-contact-group-number { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-weight: 750; }
      .claude-codex-pro-contact-link { display: inline-flex; align-items: center; justify-content: center; border: 1px solid #0f766e; border-radius: 999px; background: #eef6f5; color: #0f766e; font-size: 12px; font-weight: 750; text-decoration: none; padding: 4px 9px; }
      .claude-codex-pro-contact-qr-wrap { display: grid; gap: 8px; justify-items: center; }
      .claude-codex-pro-contact-qr { display: block; width: min(220px, 80%); aspect-ratio: 1 / 1; border: 1px solid #dce3ed; border-radius: 12px; background: #ffffff; object-fit: contain; padding: 8px; box-sizing: border-box; }
      .claude-codex-pro-control-deck .claude-codex-pro-ad-section-title,
      .claude-codex-pro-control-deck .claude-codex-pro-ad-title,
      .claude-codex-pro-control-deck .claude-codex-pro-contact-title { color: #e8fff8; }
      .claude-codex-pro-control-deck .claude-codex-pro-ad-badge { color: #62e7c8; }
      .claude-codex-pro-control-deck .claude-codex-pro-contact-line,
      .claude-codex-pro-control-deck .claude-codex-pro-contact-label { color: #c8e1db; }
      .claude-codex-pro-control-deck .claude-codex-pro-contact-group-number { color: #e8fff8; }
      .${timelineClass} {
        position: fixed;
        top: calc(72px + 12px);
        right: 12px;
        bottom: calc(28px + 12px);
        width: 24px;
        z-index: 2147482500;
        pointer-events: none;
      }
      .${timelineTrackClass} {
        position: absolute;
        top: 0;
        bottom: 0;
        left: 50%;
        width: 2px;
        transform: translateX(-50%);
        border-radius: 999px;
        background: rgba(209, 213, 219, .55);
      }
      .${timelineMarkerClass} {
        position: absolute;
        left: 50%;
        width: 12px;
        height: 12px;
        border: 0;
        border-radius: 999px;
        transform: translate(-50%, -50%);
        background: #d1d5db;
        cursor: pointer;
        pointer-events: auto;
        box-shadow: 0 0 0 2px rgba(255, 255, 255, .92);
      }
      .${timelineMarkerClass}:hover,
      .${timelineMarkerClass}:focus-visible,
      .${timelineMarkerClass}.codex-conversation-timeline-marker-active {
        background: #8b8b8b;
        outline: none;
      }
      .${timelineTooltipClass} {
        position: absolute;
        right: 20px;
        top: 50%;
        display: block;
        box-sizing: border-box;
        width: max-content;
        max-width: min(320px, calc(100vw - 72px));
        transform: translateY(-50%);
        border-radius: 8px;
        background: rgba(80, 80, 80, .92);
        color: #ffffff;
        font: 600 13px system-ui, sans-serif;
        line-height: 18px;
        padding: 10px 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        box-shadow: 0 8px 24px rgba(0, 0, 0, .18);
        opacity: 0;
        visibility: hidden;
        pointer-events: none;
      }
      .${timelineMarkerClass}:hover .${timelineTooltipClass},
      .${timelineMarkerClass}:focus-visible .${timelineTooltipClass} {
        opacity: 1;
        visibility: visible;
        z-index: 2147482501;
      }
      .${timelineTargetClass} {
        animation: codex-conversation-timeline-pulse 1.2s ease-out;
      }
      @keyframes codex-conversation-timeline-pulse {
        0% { box-shadow: 0 0 0 0 rgba(16, 163, 127, .35); }
        100% { box-shadow: 0 0 0 14px rgba(16, 163, 127, 0); }
      }
    `;
    document.documentElement.appendChild(style);
  }

  function defaultClaudeCodexProSettings() {
    return { pluginEntryUnlock: true, pluginMarketplaceUnlock: true, forcePluginInstall: true, sessionDelete: true, markdownExport: true, projectMove: true, conversationTimeline: true, conversationView: false, conversationViewMaxWidth: conversationViewDefaultWidth, threadScrollRestore: true, zedRemoteOpen: true, upstreamWorktreeCreate: true, nativeMenuPlacement: true, chineseOverlayEnabled: false, serviceTierControls: false, memoryAssistEnabled: true, memoryAssistInjectEnabled: true, memoryAssistAutoSuggestEnabled: true, memoryAssistMaxInjectedItems: 5, multicaWorkspaceEnabled: true };
  }

  const claudeCodexProBackendSettingMap = {
    pluginEntryUnlock: "codexAppPluginEntryUnlock",
    pluginMarketplaceUnlock: "codexAppPluginMarketplaceUnlock",
    forcePluginInstall: "codexAppForcePluginInstall",
    sessionDelete: "codexAppSessionDelete",
    markdownExport: "codexAppMarkdownExport",
    projectMove: "codexAppProjectMove",
    conversationTimeline: "codexAppConversationTimeline",
    conversationView: "codexAppConversationView",
    threadScrollRestore: "codexAppThreadScrollRestore",
    zedRemoteOpen: "codexAppZedRemoteOpen",
    upstreamWorktreeCreate: "codexAppUpstreamWorktreeCreate",
    nativeMenuPlacement: "codexAppNativeMenuPlacement",
    serviceTierControls: "codexAppServiceTierControls",
    memoryAssistEnabled: "memoryAssistEnabled",
    memoryAssistInjectEnabled: "memoryAssistInjectEnabled",
    memoryAssistAutoSuggestEnabled: "memoryAssistAutoSuggestEnabled",
    multicaWorkspaceEnabled: "multicaWorkspaceEnabled",
  };

  function backendClaudeCodexProSettings() {
    const settings = {};
    Object.entries(claudeCodexProBackendSettingMap).forEach(([localKey, backendKey]) => {
      if (typeof claudeCodexProBackendSettings[backendKey] === "boolean") {
        settings[localKey] = claudeCodexProBackendSettings[backendKey];
      }
    });
    const maxInjectedItems = Number(claudeCodexProBackendSettings.memoryAssistMaxInjectedItems);
    if (Number.isFinite(maxInjectedItems)) {
      settings.memoryAssistMaxInjectedItems = Math.max(1, Math.min(20, Math.round(maxInjectedItems)));
    }
    return settings;
  }

  function claudeCodexProConfiguredSettings() {
    try {
      return { ...defaultClaudeCodexProSettings(), ...JSON.parse(localStorage.getItem(claudeCodexProSettingsKey) || "{}"), ...backendClaudeCodexProSettings() };
    } catch {
      return { ...defaultClaudeCodexProSettings(), ...backendClaudeCodexProSettings() };
    }
  }

  function hasAnyCodexFrontendEnhancementEnabled(settings) {
    return [
      "pluginEntryUnlock",
      "pluginMarketplaceUnlock",
      "forcePluginInstall",
      "sessionDelete",
      "markdownExport",
      "projectMove",
      "conversationTimeline",
      "conversationView",
      "threadScrollRestore",
      "zedRemoteOpen",
      "upstreamWorktreeCreate",
      "nativeMenuPlacement",
      "serviceTierControls",
      "memoryAssistEnabled",
      "memoryAssistInjectEnabled",
      "memoryAssistAutoSuggestEnabled",
      "multicaWorkspaceEnabled",
    ].some((key) => settings[key] === true);
  }

  function claudeCodexProSettings() {
    const relayPatchDisabled = claudeCodexProBackendSettings.launchMode === "relay";
    const settings = claudeCodexProConfiguredSettings();
    // Claude localization is owned by the Claude integration. Codex DOM content
    // can contain user input and project data, so it must never be translated.
    settings.chineseOverlayEnabled = false;
    if (claudeCodexProBackendSettings.enhancementsEnabled === false && !hasAnyCodexFrontendEnhancementEnabled(settings)) {
      return {
        ...settings,
        pluginEntryUnlock: false,
        pluginMarketplaceUnlock: false,
        forcePluginInstall: false,
        sessionDelete: false,
        markdownExport: false,
        projectMove: false,
        conversationTimeline: false,
        conversationView: false,
        conversationViewMaxWidth: conversationViewDefaultWidth,
        threadScrollRestore: false,
        zedRemoteOpen: false,
        upstreamWorktreeCreate: false,
        nativeMenuPlacement: false,
        chineseOverlayEnabled: false,
        serviceTierControls: false,
        memoryAssistEnabled: false,
        memoryAssistInjectEnabled: false,
        memoryAssistAutoSuggestEnabled: false,
      };
    }
    if (relayPatchDisabled) {
      settings.pluginEntryUnlock = false;
      settings.pluginMarketplaceUnlock = false;
      settings.forcePluginInstall = false;
    }
    return settings;
  }

  function setClaudeCodexProSetting(key, value) {
    const backendKey = claudeCodexProBackendSettingMap[key];
    if (backendKey) {
      setBackendSetting(backendKey, value);
      return;
    }
    let stored = {};
    try {
      stored = JSON.parse(localStorage.getItem(claudeCodexProSettingsKey) || "{}");
    } catch {
      stored = {};
    }
    const next = { ...stored, [key]: value };
    localStorage.setItem(claudeCodexProSettingsKey, JSON.stringify(next));
    if (key === "threadScrollRestore" && !value) {
      clearTimeout(window.__codexThreadScrollSaveTimer);
      window.__codexThreadScrollSaveTimer = null;
      window.__codexThreadScrollRestoreRevision = (window.__codexThreadScrollRestoreRevision || 0) + 1;
      window.__codexThreadScrollSyncRevision = (window.__codexThreadScrollSyncRevision || 0) + 1;
      (window.__codexThreadScrollRestoreTimers || []).forEach((timer) => clearTimeout(timer));
      window.__codexThreadScrollRestoreTimers = [];
      (window.__codexThreadScrollSyncTimers || []).forEach((timer) => clearTimeout(timer));
      window.__codexThreadScrollSyncTimers = [];
      window.__codexThreadScrollRuntime = null;
    }
    if (key === "serviceTierControls") {
      if (value) {
        void loadCodexServiceTierState();
      } else {
        removeCodexServiceTierBadges();
        refreshCodexServiceTierControls();
      }
    }
    if (key === "chineseOverlayEnabled") {
      claudeChineseOverlayFullRefreshDone = false;
      claudeChineseOverlayQueue.length = 0;
      refreshClaudeChineseOverlay();
    }
    renderClaudeCodexProMenu();
    scan();
  }

  function normalizeConversationViewWidth(value) {
    if (value === null || value === undefined || String(value).trim() === "") return null;
    const number = Number(value);
    if (!Number.isFinite(number)) return null;
    return Math.max(conversationViewMinWidth, Math.min(conversationViewMaxAllowedWidth, Math.round(number)));
  }

  function conversationViewWidth() {
    const settingsWidth = normalizeConversationViewWidth(claudeCodexProSettings().conversationViewMaxWidth);
    if (settingsWidth) return settingsWidth;
    const legacyWidth = normalizeConversationViewWidth(localStorage.getItem(conversationViewLegacyWidthKey));
    return legacyWidth || conversationViewDefaultWidth;
  }

  function refreshConversationViewControls() {
    const enabled = !!claudeCodexProSettings().conversationView;
    const width = conversationViewWidth();
    document.querySelectorAll("[data-claude-codex-pro-conversation-view-width]").forEach((input) => {
      input.value = String(width);
      input.disabled = !enabled;
    });
  }

  function setConversationViewWidth(value) {
    const width = normalizeConversationViewWidth(value);
    if (!width) return;
    setClaudeCodexProSetting("conversationViewMaxWidth", width);
  }

  function renderClaudeCodexProMenu() {
    const configuredSettings = claudeCodexProConfiguredSettings();
    document.querySelectorAll(".claude-codex-pro-toggle[data-claude-codex-pro-setting]").forEach((button) => {
      const key = button.getAttribute("data-claude-codex-pro-setting");
      button.dataset.enabled = String(!!configuredSettings[key]);
    });
    refreshConversationViewControls();
    refreshCodexServiceTierControls();
  }

  let claudeCodexProBackendSettings = { providerSyncEnabled: false, enhancementsEnabled: true, launchMode: "patch", codexAppVersion: "" };
  const codexPluginLegacyEntryUnlockBeforeVersion = "26.601.2237";
  const codexPluginBridgeRequestUnlockFromVersion = "26.616.0";

  function parseCodexVersionParts(version) {
    const raw = String(version || "").trim();
    if (!raw) return null;
    const match = raw.match(/\d+(?:\.\d+)*/);
    if (!match) return null;
    const parts = match[0].split(".").map((part) => Number(part));
    if (!parts.length || parts.some((part) => !Number.isInteger(part) || part < 0)) return null;
    return parts;
  }

  function compareCodexVersions(left, right) {
    const leftParts = parseCodexVersionParts(left);
    const rightParts = parseCodexVersionParts(right);
    if (!leftParts || !rightParts) return null;
    const length = Math.max(leftParts.length, rightParts.length);
    for (let index = 0; index < length; index += 1) {
      const leftPart = leftParts[index] || 0;
      const rightPart = rightParts[index] || 0;
      if (leftPart !== rightPart) return leftPart < rightPart ? -1 : 1;
    }
    return 0;
  }

  function codexPluginUnlockStrategy() {
    const version = String(claudeCodexProBackendSettings.codexAppVersion || "").trim();
    const comparison = compareCodexVersions(version, codexPluginLegacyEntryUnlockBeforeVersion);
    if (comparison == null) return "unknown";
    return comparison < 0 ? "legacy" : "modern";
  }

  function logCodexPluginUnlockStrategy(strategy) {
    const codexAppVersion = String(claudeCodexProBackendSettings.codexAppVersion || "").trim();
    const signature = `${strategy}:${codexAppVersion || "unknown"}`;
    if (window.__codexPluginUnlockStrategyLogged === signature) return;
    window.__codexPluginUnlockStrategyLogged = signature;
    sendClaudeCodexProDiagnostic("plugin_unlock_strategy_selected", {
      strategy,
      codexAppVersion,
      cutoff: codexPluginLegacyEntryUnlockBeforeVersion,
    });
  }

  let claudeCodexProBackendSettingsLoaded = false;
  let codexServiceTierState = {
    status: "loading",
    serviceTier: null,
    message: "正在读取…",
    fastTierValue: "priority",
    controlMode: "inherit",
    defaultMode: "inherit",
    activeThreadId: "",
    threadMode: "inherit",
    effectiveServiceTier: null,
    effectiveMode: "standard",
    fastModelName: "",
    fastSupported: false,
  };
  const codexDefaultServiceTierSetting = { key: "default-service-tier", default: null };
  const codexServiceTierFallbackFastValue = "priority";
  const codexServiceTierModulePromises = new Map();
  const codexServiceTierSupportedFastModels = new Set(["gpt-5.4", "gpt-5.5"]);
  const codexThreadServiceTierModes = new Set(["inherit", "standard", "fast"]);
  const codexServiceTierControlModes = new Set(["inherit", "global-standard", "global-fast", "custom"]);

  function codexAppAssetUrl(namePart) {
    const urls = [
      ...Array.from(document.scripts || []).map((script) => script.src),
      ...Array.from(document.querySelectorAll("link[href]") || []).map((link) => link.href),
      ...performance.getEntriesByType("resource").map((entry) => entry.name),
    ].filter(Boolean);
    return urls.find((url) => url.includes("/assets/") && url.includes(namePart) && url.split("?")[0].endsWith(".js")) || "";
  }

  async function loadCodexAppModule(namePart) {
    if (!codexServiceTierModulePromises.has(namePart)) {
      const promise = Promise.resolve().then(async () => {
        const url = codexAppAssetUrl(namePart);
        if (!url) throw new Error(`未找到 Codex App asset: ${namePart}`);
        return await import(url);
      }).catch((error) => {
        codexServiceTierModulePromises.delete(namePart);
        throw error;
      });
      codexServiceTierModulePromises.set(namePart, promise);
    }
    return await codexServiceTierModulePromises.get(namePart);
  }

  const codexPageHostAllowedMethods = new Set([
    "initialize",
    "skills/list",
    "thread/start",
    "thread/read",
    "thread/fork",
    "turn/start",
    "turn/interrupt",
  ]);
  let codexPageHostClientPromise = null;
  let codexPageHostClient = null;
  let codexPageHostInitializeResponse = null;

  function codexPageHostStillCurrent() {
    return window.__claudeCodexProCodexPageHostGeneration === claudeCodexProCodexPageHostGeneration;
  }

  function codexPageHostCandidates(module) {
    const candidates = [];
    const seen = new Set();
    const append = (value) => {
      if (!value || typeof value !== "object" || seen.has(value) || typeof value.sendRequest !== "function") return;
      seen.add(value);
      candidates.push(value);
    };
    // A signal/store export represents the client owned by the currently
    // mounted page. Prefer its live value over unrelated exported objects.
    Object.values(module || {}).forEach((exported) => {
      if (!exported || typeof exported !== "object" || typeof exported.get !== "function") return;
      try { append(exported.get()); } catch (_) {}
    });
    Object.values(module || {}).forEach(append);
    return candidates;
  }

  function codexPageHostInitializeResponseValid(response) {
    if (!response || typeof response !== "object" || Array.isArray(response)) return false;
    if (response.error || response.status === "failed") return false;
    const provider = response.provider || response.serverInfo?.provider;
    return !provider || String(provider).toLowerCase() === "codex";
  }

  function codexPageHostAppScopeValid(appScope) {
    if (!appScope || typeof appScope !== "object") return false;
    const hasScopeGetter = typeof appScope.get === "function";
    const hasScopeNode = typeof appScope.node === "function" ||
      (!!appScope.node && typeof appScope.node === "object");
    const hasQueryClient = !!appScope.queryClient && typeof appScope.queryClient === "object";
    // Codex changed the React scope shape across releases.  FRt only needs
    // one live scope handle; requiring all legacy fields made the native Host
    // appear unavailable even though the page-owned client was present.
    return hasScopeGetter || hasScopeNode || hasQueryClient;
  }

  function codexPageHostReactRootFiber() {
    const configuredRoot = window.__codexRoot?._internalRoot?.current;
    if (configuredRoot) return configuredRoot;
    // Codex does not expose a stable global React root in newer builds. The
    // renderer still owns the root container, whose private React key is the
    // authoritative mounted tree. Discover it without touching React stores.
    const candidates = [document.documentElement, document.body, ...Array.from(document.querySelectorAll?.('*') || []).slice(0, 5000)];
    for (const element of candidates) {
      if (!element || typeof element !== "object") continue;
      const keys = Object.keys(element);
      for (const key of keys) {
        if (key.startsWith("__reactContainer$") || key.startsWith("__reactRootContainer$")) {
          const value = element[key];
          const fiber = value?.current || value?._internalRoot?.current;
          if (fiber && typeof fiber === "object") return fiber;
        }
      }
    }
    return null;
  }

  function codexPageHostAppScopeFromReactRoot() {
    const rootFiber = codexPageHostReactRootFiber();
    if (!rootFiber || typeof rootFiber !== "object") return null;
    const queue = [rootFiber];
    const seen = new Set();
    let cursor = 0;
    while (cursor < queue.length && seen.size < 20000) {
      const fiber = queue[cursor++];
      if (!fiber || typeof fiber !== "object" || seen.has(fiber)) continue;
      seen.add(fiber);
      for (let hook = fiber.memoizedState, count = 0;
        hook && typeof hook === "object" && count < 256;
        hook = hook.next, count += 1) {
        const state = hook.memoizedState;
        if (codexPageHostAppScopeValid(state?.appScope)) return state.appScope;
      }
      const props = fiber.memoizedProps;
      if (codexPageHostAppScopeValid(props?.appScope)) return props.appScope;
      if (fiber.child) queue.push(fiber.child);
      if (fiber.sibling) queue.push(fiber.sibling);
    }
    return null;
  }

  function codexPageHostIdFromActiveThread() {
    const rows = Array.from(document.querySelectorAll?.("[data-app-action-sidebar-thread-id]") || []);
    const activeRow = rows.find((row) => row.matches?.('[data-app-action-sidebar-thread-active="true"], [aria-current="page"]') ||
      row.querySelector?.('[data-app-action-sidebar-thread-active="true"], [aria-current="page"]'));
    return String(activeRow?.getAttribute?.("data-app-action-sidebar-thread-host-id") || "").trim();
  }

  async function codexPageHostClientFromAppInitial() {
    const module = await loadCodexAppModule("app-initial-");
    const appScope = codexPageHostAppScopeFromReactRoot();
    if (!appScope) throw new Error("codex_page_host_app_scope_unavailable");
    if (typeof module?.FRt !== "function") throw new Error("codex_page_host_factory_unavailable");
    const hostId = codexPageHostIdFromActiveThread() ||
      String(appScope.currentHostId || appScope.hostId || "local");
    const client = await Promise.resolve(module.FRt(appScope, hostId));
    if (!client || typeof client.sendRequest !== "function") {
      throw new Error("codex_page_host_client_unavailable");
    }
    // The page-owned client is already initialized. Probe a read-only method
    // instead of sending a second initialize request, which Codex rejects.
    const skills = await client.sendRequest("skills/list", {});
    if (!skills || typeof skills !== "object" || Array.isArray(skills) || skills.error) {
      throw new Error("codex_page_host_probe_failed");
    }
    return {
      client,
      initializeResponse: {
        provider: "codex",
        protocolVersion: "current-page",
        serverInfo: { provider: "codex" },
        // `skills/list` proves inventory only.  Do not infer subagent or task
        // execution support from a page-owned client that has no live
        // initialize capability response; the Rust adapter will keep those
        // operations unsupported until the primary host reports them.
        capabilities: [],
        pageHostProbe: { skillsList: true, nativeTaskHost: false },
      },
    };
  }

  async function currentCodexPageHostClient(initializeParams) {
    if (codexPageHostClient) {
      return { client: codexPageHostClient, initializeResponse: codexPageHostInitializeResponse };
    }
    if (!codexPageHostClientPromise) {
      codexPageHostClientPromise = Promise.resolve().then(async () => {
        let primaryError = null;
        try {
          const module = await loadCodexAppModule("app-server-manager-signals-");
          const candidates = codexPageHostCandidates(module);
          let lastError = null;
          for (const candidate of candidates) {
            if (!codexPageHostStillCurrent()) throw new Error("codex_page_host_generation_stale");
            try {
              const response = await candidate.sendRequest("initialize", initializeParams);
              if (!codexPageHostInitializeResponseValid(response)) continue;
              codexPageHostClient = candidate;
              codexPageHostInitializeResponse = response;
              return { client: candidate, initializeResponse: response };
            } catch (error) {
              lastError = error;
            }
          }
          primaryError = lastError || new Error("codex_page_host_unavailable");
        } catch (error) {
          primaryError = error;
        }
        try {
          const selected = await codexPageHostClientFromAppInitial();
          if (!codexPageHostStillCurrent()) throw new Error("codex_page_host_generation_stale");
          codexPageHostClient = selected.client;
          codexPageHostInitializeResponse = selected.initializeResponse;
          return selected;
        } catch (fallbackError) {
          sendClaudeCodexProDiagnostic("codex_page_host_probe_failed", {
            primaryError: primaryError?.message || String(primaryError || ""),
            fallbackError: fallbackError?.message || String(fallbackError || ""),
            hasRoot: !!codexPageHostReactRootFiber(),
            generation: claudeCodexProCodexPageHostGeneration,
          });
          const error = new Error("codex_page_host_unavailable");
          error.primaryError = primaryError;
          error.fallbackError = fallbackError;
          throw error;
        }
      }).catch((error) => {
        codexPageHostClientPromise = null;
        throw error;
      });
    }
    return await codexPageHostClientPromise;
  }

  async function codexPageHostRequest(method, params = {}) {
    const normalizedMethod = String(method || "");
    if (!codexPageHostAllowedMethods.has(normalizedMethod)) {
      throw new Error("codex_page_host_method_unsupported");
    }
    if (!params || typeof params !== "object" || Array.isArray(params)) {
      throw new Error("codex_page_host_params_invalid");
    }
    if (!codexPageHostStillCurrent()) throw new Error("codex_page_host_generation_stale");
    const initializeParams = normalizedMethod === "initialize"
      ? params
      : { clientInfo: { name: "claude-codex-pro-tool", version: "page-host-probe" } };
    const selected = await currentCodexPageHostClient(initializeParams);
    if (!codexPageHostStillCurrent()) throw new Error("codex_page_host_generation_stale");
    if (normalizedMethod === "initialize" && selected.initializeResponse) {
      return selected.initializeResponse;
    }
    const client = selected.client;
    return await client.sendRequest(normalizedMethod, params);
  }

  function cleanupCodexPageHostRequest() {
    codexPageHostClient = null;
    codexPageHostClientPromise = null;
    codexPageHostInitializeResponse = null;
    if (window.__claudeCodexProCodexPageHostRequest === codexPageHostRequest) {
      try { delete window.__claudeCodexProCodexPageHostRequest; } catch (_) {
        window.__claudeCodexProCodexPageHostRequest = null;
      }
    }
    if (window.__claudeCodexProCodexPageHostCleanup === cleanupCodexPageHostRequest) {
      window.__claudeCodexProCodexPageHostCleanup = null;
    }
  }

  // Called only through CCP's CDP bridge. It reuses the request client owned
  // by the currently open Codex page and never starts or registers a runtime.
  window.__claudeCodexProCodexPageHostRequest = codexPageHostRequest;
  window.__claudeCodexProCodexPageHostCleanup = cleanupCodexPageHostRequest;

  async function codexSettingStorageModule() {
    const module = await loadCodexAppModule("setting-storage-");
    if (typeof module.n !== "function" || typeof module.s !== "function") {
      throw new Error("Codex setting-storage 接口不可用");
    }
    return module;
  }

  async function getCodexServiceTierSetting() {
    try {
      const settingStorage = await codexSettingStorageModule();
      return await settingStorage.n(codexDefaultServiceTierSetting);
    } catch (error) {
      if (typeof codexStateCall === "function") {
        const result = await codexStateCall("get-setting", { params: { key: codexDefaultServiceTierSetting.key } });
        return result && Object.prototype.hasOwnProperty.call(result, "value") ? result.value : codexDefaultServiceTierSetting.default;
      }
      throw error;
    }
  }

  function isFastServiceTierValue(value) {
    const normalized = String(value || "").trim().toLowerCase();
    return normalized === "fast" || normalized === "priority";
  }

  function codexFastServiceTierValue() {
    return codexServiceTierState.fastTierValue || codexServiceTierFallbackFastValue;
  }

  function codexServiceTierFastModelListLabel() {
    return Array.from(codexServiceTierSupportedFastModels).join(" / ");
  }

  function normalizeCodexServiceTierModelName(model) {
    return String(model || "").trim().toLowerCase();
  }

  function codexServiceTierModelFromValue(value, visited = new WeakSet(), depth = 0) {
    if (typeof value === "string") return value.trim();
    if (!value || typeof value !== "object" || visited.has(value) || depth > 3) return "";
    visited.add(value);
    for (const key of ["model", "modelId", "model_id", "selectedModel", "selected_model", "defaultModel", "default_model"]) {
      const model = codexServiceTierModelFromValue(value[key], visited, depth + 1);
      if (model) return model;
    }
    for (const key of ["params", "request", "payload", "body", "config", "options"]) {
      const model = codexServiceTierModelFromValue(value[key], visited, depth + 1);
      if (model) return model;
    }
    return "";
  }

  function codexServiceTierCurrentModelName() {
    return codexServiceTierModelFromValue(codexModelCatalog.model) || codexServiceTierModelFromValue(codexModelCatalog.default_model);
  }

  function codexServiceTierModelForRequest(params, modelHint = "") {
    return codexServiceTierModelFromValue(params) || codexServiceTierModelFromValue(modelHint) || codexServiceTierCurrentModelName();
  }

  function codexServiceTierFastSupportedForModel(modelName) {
    return codexServiceTierSupportedFastModels.has(normalizeCodexServiceTierModelName(modelName));
  }

  function codexServiceTierFastUnsupportedMessage(modelName = codexServiceTierCurrentModelName()) {
    const modelText = modelName ? `当前模型 ${modelName} 不支持` : "当前模型未读取";
    return `Fast 仅支持 ${codexServiceTierFastModelListLabel()}，${modelText}`;
  }

  function codexServiceTierMaybeLoadModelCatalog(force = false) {
    if (codexModelCatalogPromise) return;
    if (!force && codexModelCatalog.status === "failed") return;
    if (!force && codexModelCatalogLoadedAt && Date.now() - codexModelCatalogLoadedAt < 10000) return;
    loadCodexModelCatalog(force).then(() => {
      refreshCodexServiceTierControls();
    }).catch(() => {
      refreshCodexServiceTierControls();
    });
  }

  function codexServiceTierFastAvailability(modelName = codexServiceTierCurrentModelName()) {
    const normalizedModel = normalizeCodexServiceTierModelName(modelName);
    return {
      modelName: modelName || "",
      supported: !!normalizedModel && codexServiceTierSupportedFastModels.has(normalizedModel),
    };
  }

  function codexServiceTierValueForMode(mode) {
    if (mode === "fast") return codexFastServiceTierValue();
    if (mode === "standard") return null;
    return codexServiceTierState.serviceTier || null;
  }

  function codexServiceTierDefaultModeForControlMode(controlMode, fallback = "inherit") {
    if (controlMode === "global-fast") return "fast";
    if (controlMode === "global-standard") return "standard";
    if (controlMode === "inherit") return "inherit";
    return normalizeCodexThreadServiceTierMode(fallback);
  }

  function codexServiceTierControlModeForDefaultMode(defaultMode) {
    if (defaultMode === "fast") return "global-fast";
    if (defaultMode === "standard") return "global-standard";
    return "inherit";
  }

  function codexServiceTierEffectiveThreadMode(threadMode = "inherit", defaultMode = "inherit") {
    const normalizedThreadMode = normalizeCodexThreadServiceTierMode(threadMode);
    if (normalizedThreadMode !== "inherit") return normalizedThreadMode;
    return normalizeCodexThreadServiceTierMode(defaultMode);
  }

  function codexServiceTierValueForControlMode(controlMode, threadMode = "inherit", defaultMode = "inherit") {
    if (controlMode === "global-fast") return codexFastServiceTierValue();
    if (controlMode === "global-standard") return null;
    if (controlMode === "custom") return codexServiceTierValueForMode(codexServiceTierEffectiveThreadMode(threadMode, defaultMode));
    return codexServiceTierState.serviceTier || null;
  }

  function codexServiceTierEffectiveMode(value) {
    return isFastServiceTierValue(value) ? "fast" : "standard";
  }

  function normalizeCodexThreadServiceTierMode(mode) {
    const normalized = String(mode || "").trim().toLowerCase();
    return codexThreadServiceTierModes.has(normalized) ? normalized : "inherit";
  }

  function normalizeCodexServiceTierControlMode(mode) {
    const normalized = String(mode || "").trim().toLowerCase();
    return codexServiceTierControlModes.has(normalized) ? normalized : "inherit";
  }

  function serviceTierGlobalStatusMessage(serviceTier) {
    if (isFastServiceTierValue(serviceTier)) return "Fast 已开启";
    if (!serviceTier) return "默认服务模式";
    return `当前：${serviceTier}`;
  }

  function serviceTierStatusMessage(
    controlMode = codexServiceTierState.controlMode || "inherit",
    threadMode = codexServiceTierState.threadMode || "inherit",
    effectiveMode = codexServiceTierState.effectiveMode || "standard",
    defaultMode = codexServiceTierState.defaultMode || "inherit"
  ) {
    if (codexServiceTierState.status === "loading") return "正在读取…";
    if (codexServiceTierState.status === "failed") return "读取失败";
    if (controlMode === "inherit") return `继承 config.toml：${effectiveMode}`;
    if (controlMode === "global-standard") return "全局 Standard";
    if (controlMode === "global-fast") return "全局 Fast";
    if (threadMode === "inherit") return `自定义：默认 ${defaultMode}`;
    return `自定义：当前 thread ${threadMode}`;
  }

  function readThreadServiceTierState() {
    try {
      const parsed = JSON.parse(localStorage.getItem(codexThreadServiceTierKey) || "{}");
      const rawEntries = parsed?.version === codexThreadServiceTierVersion && parsed?.entries && typeof parsed.entries === "object"
        ? parsed.entries
        : {};
      const entries = Object.create(null);
      Object.entries(rawEntries).forEach(([key, value]) => {
        const safeKey = typeof validThreadScrollSessionKey === "function" ? validThreadScrollSessionKey(key) : String(key || "");
        const mode = normalizeCodexThreadServiceTierMode(value?.mode);
        if (safeKey && mode !== "inherit") entries[safeKey] = { mode, at: finiteNonNegativeNumber(value?.at) || Date.now() };
      });
      const draft = normalizeThreadServiceTierDraft(parsed?.draft);
      const hasCustomState = !!draft || Object.keys(entries).length > 0;
      const mode = parsed?.mode ? normalizeCodexServiceTierControlMode(parsed.mode) : (hasCustomState ? "custom" : "inherit");
      return {
        mode,
        defaultMode: normalizeCodexThreadServiceTierMode(parsed?.defaultMode || codexServiceTierDefaultModeForControlMode(mode)),
        entries,
        draft,
      };
    } catch (_) {
      return { mode: "inherit", defaultMode: "inherit", entries: Object.create(null), draft: null };
    }
  }

  function writeThreadServiceTierState(state) {
    const mode = normalizeCodexServiceTierControlMode(state?.mode);
    const defaultMode = normalizeCodexThreadServiceTierMode(state?.defaultMode || codexServiceTierDefaultModeForControlMode(mode));
    const rawEntries = state?.entries && typeof state.entries === "object" ? state.entries : {};
    const entries = Object.create(null);
    Object.entries(rawEntries)
      .map(([key, value]) => {
        const safeKey = validThreadScrollSessionKey(key);
        const mode = normalizeCodexThreadServiceTierMode(value?.mode);
        return safeKey && mode !== "inherit" ? [safeKey, { mode, at: finiteNonNegativeNumber(value?.at) || Date.now() }] : null;
      })
      .filter(Boolean)
      .sort((left, right) => right[1].at - left[1].at)
      .slice(0, codexThreadServiceTierMaxEntries)
      .forEach(([key, value]) => {
        entries[key] = value;
      });
    const draft = normalizeThreadServiceTierDraft(state?.draft);
    try {
      localStorage.setItem(codexThreadServiceTierKey, JSON.stringify({
        version: codexThreadServiceTierVersion,
        mode,
        defaultMode,
        entries,
        ...(draft ? { draft } : {}),
      }));
    } catch (_) {}
  }

  function normalizeThreadServiceTierDraft(value) {
    if (!value || typeof value !== "object") return null;
    const mode = normalizeCodexThreadServiceTierMode(value.mode);
    if (mode === "inherit") return null;
    const at = finiteNonNegativeNumber(value.at) || Date.now();
    return { mode, at };
  }

  function codexThreadServiceTierOverride(threadId) {
    const key = validThreadScrollSessionKey(threadId);
    if (!key) return null;
    const entry = readThreadServiceTierState().entries[key];
    const mode = normalizeCodexThreadServiceTierMode(entry?.mode);
    return mode === "inherit" ? null : { mode, at: finiteNonNegativeNumber(entry?.at) || 0 };
  }

  function codexThreadServiceTierDraft() {
    const draft = readThreadServiceTierState().draft;
    if (!draft) return null;
    if (Date.now() - draft.at > codexThreadServiceTierDraftBindWindowMs) return null;
    return draft;
  }

  function setCodexThreadServiceTierOverride(threadId, mode) {
    const normalizedMode = normalizeCodexThreadServiceTierMode(mode);
    const state = readThreadServiceTierState();
    state.mode = "custom";
    const key = validThreadScrollSessionKey(threadId);
    if (key) {
      if (normalizedMode === "inherit") {
        delete state.entries[key];
      } else {
        state.entries[key] = { mode: normalizedMode, at: Date.now() };
      }
    } else if (normalizedMode === "inherit") {
      state.draft = null;
    } else {
      state.draft = { mode: normalizedMode, at: Date.now() };
    }
    writeThreadServiceTierState(state);
  }

  function bindDraftServiceTierToThread(threadId) {
    const key = validThreadScrollSessionKey(threadId);
    const draft = codexThreadServiceTierDraft();
    if (!key || !draft) return false;
    const state = readThreadServiceTierState();
    if (normalizeCodexServiceTierControlMode(state.mode) !== "custom") {
      state.draft = null;
      writeThreadServiceTierState(state);
      return false;
    }
    if (!state.entries[key]) state.entries[key] = { mode: draft.mode, at: Date.now() };
    state.draft = null;
    writeThreadServiceTierState(state);
    return true;
  }

  function setCodexServiceTierControlMode(mode) {
    if (claudeCodexProBackendStatus.status !== "ok") {
      showToast("后端未连接，无法切换服务模式", null);
      refreshCodexServiceTierControls();
      return;
    }
    const normalizedMode = normalizeCodexServiceTierControlMode(mode);
    if (normalizedMode === "global-fast") {
      const fastAvailability = codexServiceTierFastAvailability();
      if (!fastAvailability.supported) {
        codexServiceTierMaybeLoadModelCatalog(true);
        showToast(codexServiceTierFastUnsupportedMessage(fastAvailability.modelName), null);
        refreshCodexServiceTierControls();
        return;
      }
    }
    const state = readThreadServiceTierState();
    state.mode = normalizedMode;
    if (normalizedMode !== "custom") {
      state.defaultMode = codexServiceTierDefaultModeForControlMode(normalizedMode);
      state.entries = Object.create(null);
      state.draft = null;
    } else {
      state.defaultMode = normalizeCodexThreadServiceTierMode(state.defaultMode);
    }
    writeThreadServiceTierState(state);
    refreshCodexServiceTierControls();
    const labels = {
      inherit: "继承 config.toml",
      "global-standard": "全局 Standard",
      "global-fast": "全局 Fast",
      custom: "自定义",
    };
    showToast(`服务模式：${labels[normalizedMode] || normalizedMode}`, null);
  }

  function syncCodexServiceTierEffectiveState() {
    if (!claudeCodexProSettings().serviceTierControls) {
      codexServiceTierState = {
        ...codexServiceTierState,
        activeThreadId: "",
        threadMode: "inherit",
        effectiveServiceTier: codexServiceTierState.serviceTier || null,
        effectiveMode: codexServiceTierEffectiveMode(codexServiceTierState.serviceTier),
        message: "未启用",
      };
      return;
    }
    const activeThreadId = validThreadScrollSessionKey(currentSessionRef().session_id);
    if (activeThreadId) bindDraftServiceTierToThread(activeThreadId);
    const storedState = readThreadServiceTierState();
    const controlMode = normalizeCodexServiceTierControlMode(storedState.mode);
    const defaultMode = normalizeCodexThreadServiceTierMode(storedState.defaultMode);
    const override = activeThreadId ? codexThreadServiceTierOverride(activeThreadId) : codexThreadServiceTierDraft();
    const threadMode = normalizeCodexThreadServiceTierMode(override?.mode);
    const effectiveServiceTier = codexServiceTierValueForControlMode(controlMode, threadMode, defaultMode);
    const effectiveMode = codexServiceTierEffectiveMode(effectiveServiceTier);
    const fastAvailability = codexServiceTierFastAvailability();
    const message = effectiveMode === "fast" && !fastAvailability.supported
      ? codexServiceTierFastUnsupportedMessage(fastAvailability.modelName)
      : serviceTierStatusMessage(controlMode, threadMode, effectiveMode, defaultMode);
    codexServiceTierState = {
      ...codexServiceTierState,
      controlMode,
      defaultMode,
      activeThreadId,
      threadMode,
      effectiveServiceTier,
      effectiveMode,
      fastModelName: fastAvailability.modelName,
      fastSupported: fastAvailability.supported,
      message,
    };
  }

  function codexServiceTierBadgeState() {
    if (claudeCodexProBackendStatus.status === "checking") return { tier: "loading", label: "...", disabled: true, title: "服务模式：正在检查连接" };
    if (claudeCodexProBackendStatus.status && claudeCodexProBackendStatus.status !== "ok") return { tier: "failed", label: "未连接", disabled: true, title: "服务模式：未连接，无法切换" };
    if (codexServiceTierState.status === "loading") return { tier: "loading", label: "...", title: "服务模式：正在读取" };
    if (codexServiceTierState.status === "failed") return { tier: "failed", label: "?", title: "服务模式：读取失败" };
    const fastAvailability = codexServiceTierFastAvailability();
    const effectiveMode = codexServiceTierState.effectiveMode || "standard";
    const scope = codexServiceTierState.controlMode === "custom" && codexServiceTierState.threadMode !== "inherit"
      ? `当前 thread：${codexServiceTierState.threadMode}`
      : serviceTierStatusMessage(codexServiceTierState.controlMode, codexServiceTierState.threadMode, effectiveMode, codexServiceTierState.defaultMode);
    const title = [
      `服务模式：${scope}`,
      "Standard：使用标准处理；不在请求上设置 priority。",
      `Fast：仅支持 ${codexServiceTierFastModelListLabel()}；对支持模型使用 service_tier=\"priority\"，官方说明其延迟更低且更一致，但会按更高价格计费；rate limit 与 Standard 共享，流量快速上涨时可能回落到 Standard。`,
    ].join("\n");
    if (effectiveMode === "fast" && !fastAvailability.supported) {
      return { tier: "unsupported", label: "不支持", title: `${title}\n${codexServiceTierFastUnsupportedMessage(fastAvailability.modelName)}；当前请求会按 Standard 发送。` };
    }
    if (effectiveMode === "fast") return { tier: "fast", label: "fast", title };
    return { tier: "standard", label: "standard", title };
  }

  function refreshCodexServiceTierBadges() {
    const state = codexServiceTierBadgeState();
    document.querySelectorAll(`[data-codex-service-tier-badge="true"]`).forEach((node) => {
      node.dataset.tier = state.tier;
      node.dataset.disabled = String(!!state.disabled);
      node.textContent = state.label;
      node.title = state.title;
      node.setAttribute("aria-label", state.title);
    });
  }

  function refreshCodexServiceTierControls() {
    syncCodexServiceTierEffectiveState();
    const featureEnabled = !!claudeCodexProSettings().serviceTierControls;
    const backendConnected = claudeCodexProBackendStatus.status === "ok";
    const backendChecking = claudeCodexProBackendStatus.status === "checking";
    if (featureEnabled && backendConnected) codexServiceTierMaybeLoadModelCatalog();
    const fastAvailability = codexServiceTierFastAvailability();
    const fastDisabled = !featureEnabled || !backendConnected || codexServiceTierState.status === "loading" || !fastAvailability.supported;
    const fastTitle = fastAvailability.supported
      ? "Fast：使用 service_tier=\"priority\""
      : codexServiceTierFastUnsupportedMessage(fastAvailability.modelName);
    const fastUnsupportedActive = codexServiceTierState.effectiveMode === "fast" && !fastAvailability.supported;
    document.querySelectorAll("[data-codex-service-tier-controls]").forEach((node) => {
      node.hidden = !featureEnabled;
    });
    document.querySelectorAll("[data-codex-service-tier-status]").forEach((node) => {
      node.dataset.status = fastUnsupportedActive ? "unsupported" : (featureEnabled && backendConnected ? (codexServiceTierState.status || "loading") : (backendChecking ? "loading" : "failed"));
      node.textContent = featureEnabled
        ? (backendConnected ? (codexServiceTierState.message || "未读取") : (backendChecking ? "正在检查连接…" : "未连接"))
        : "未启用";
    });
    document.querySelectorAll("[data-codex-service-tier-inherit]").forEach((button) => {
      button.disabled = !featureEnabled || !backendConnected || codexServiceTierState.status === "loading";
      button.dataset.active = String(codexServiceTierState.controlMode === "inherit");
    });
    document.querySelectorAll("[data-codex-service-tier-standard]").forEach((button) => {
      button.disabled = !featureEnabled || !backendConnected || codexServiceTierState.status === "loading";
      button.dataset.active = String(codexServiceTierState.controlMode === "global-standard");
    });
    document.querySelectorAll("[data-codex-service-tier-fast]").forEach((button) => {
      button.disabled = fastDisabled;
      button.dataset.active = String(codexServiceTierState.controlMode === "global-fast");
      button.title = fastTitle;
    });
    document.querySelectorAll("[data-codex-service-tier-custom]").forEach((button) => {
      button.disabled = !featureEnabled || !backendConnected || codexServiceTierState.status === "loading";
      button.dataset.active = String(codexServiceTierState.controlMode === "custom");
    });
    document.querySelectorAll("[data-codex-service-tier-thread-inherit]").forEach((button) => {
      button.disabled = !featureEnabled || !backendConnected || codexServiceTierState.status === "loading";
      button.dataset.active = String(codexServiceTierState.controlMode === "custom" && codexServiceTierState.threadMode === "inherit");
      button.title = `当前 thread 不单独覆盖，继承自定义默认 ${codexServiceTierState.defaultMode || "inherit"}`;
    });
    document.querySelectorAll("[data-codex-service-tier-thread-standard]").forEach((button) => {
      button.disabled = !featureEnabled || !backendConnected || codexServiceTierState.status === "loading";
      button.dataset.active = String(codexServiceTierState.controlMode === "custom" && codexServiceTierState.threadMode === "standard");
    });
    document.querySelectorAll("[data-codex-service-tier-thread-fast]").forEach((button) => {
      button.disabled = fastDisabled;
      button.dataset.active = String(codexServiceTierState.controlMode === "custom" && codexServiceTierState.threadMode === "fast");
      button.title = fastTitle;
    });
    refreshCodexServiceTierBadges();
  }

  async function loadCodexServiceTierState() {
    if (!claudeCodexProSettings().serviceTierControls) {
      codexServiceTierState = { ...codexServiceTierState, status: "idle", message: "未启用" };
      refreshCodexServiceTierControls();
      return;
    }
    codexServiceTierState = { ...codexServiceTierState, status: "loading", message: "正在读取…" };
    refreshCodexServiceTierControls();
    try {
      const serviceTier = await getCodexServiceTierSetting();
      codexServiceTierState = {
        ...codexServiceTierState,
        status: "ok",
        serviceTier,
        message: serviceTierGlobalStatusMessage(serviceTier),
      };
    } catch (error) {
      codexServiceTierState = {
        ...codexServiceTierState,
        status: "failed",
        message: "读取失败",
      };
      sendClaudeCodexProDiagnostic("service_tier_read_failed", {
        errorName: error?.name || "",
        errorMessage: error?.message || String(error),
      });
    } finally {
      refreshCodexServiceTierControls();
    }
  }

  function setCodexThreadServiceTierMode(mode) {
    if (claudeCodexProBackendStatus.status !== "ok") {
      showToast("后端未连接，无法切换服务模式", null);
      refreshCodexServiceTierControls();
      return;
    }
    const normalizedMode = normalizeCodexThreadServiceTierMode(mode);
    if (normalizedMode === "fast") {
      const fastAvailability = codexServiceTierFastAvailability();
      if (!fastAvailability.supported) {
        codexServiceTierMaybeLoadModelCatalog(true);
        showToast(codexServiceTierFastUnsupportedMessage(fastAvailability.modelName), null);
        refreshCodexServiceTierControls();
        return;
      }
    }
    const threadId = validThreadScrollSessionKey(currentSessionRef().session_id);
    setCodexThreadServiceTierOverride(threadId, normalizedMode);
    refreshCodexServiceTierControls();
    const target = threadId ? "当前 thread" : "新 thread 草稿";
    showToast(`${target}服务模式：${normalizedMode === "inherit" ? "继承" : normalizedMode}`, null);
  }

  function toggleCodexServiceTierFromBadge() {
    if (claudeCodexProBackendStatus.status !== "ok") {
      showToast("后端未连接，无法切换服务模式", null);
      refreshCodexServiceTierControls();
      return;
    }
    syncCodexServiceTierEffectiveState();
    const nextMode = codexServiceTierState.effectiveMode === "fast" ? "standard" : "fast";
    if (nextMode === "fast") {
      const fastAvailability = codexServiceTierFastAvailability();
      if (!fastAvailability.supported) {
        codexServiceTierMaybeLoadModelCatalog(true);
        showToast(codexServiceTierFastUnsupportedMessage(fastAvailability.modelName), null);
        refreshCodexServiceTierControls();
        return;
      }
    }
    setCodexThreadServiceTierMode(nextMode);
  }

  function codexServiceTierRequestMethods() {
    return new Set(["thread/start", "thread/resume", "turn/start"]);
  }

  function codexServiceTierThreadIdForRequest(method, params, threadIdHint = "") {
    if (method === "thread/start") return validThreadScrollSessionKey(params?.threadId || threadIdHint);
    return validThreadScrollSessionKey(params?.threadId || params?.conversationId || threadIdHint || currentSessionRef().session_id);
  }

  function codexServiceTierOverrideResult(method, params, threadIdHint, mode, requestedServiceTier, modelHint = "") {
    const threadId = codexServiceTierThreadIdForRequest(method, params, threadIdHint);
    const requestedFast = isFastServiceTierValue(requestedServiceTier);
    const modelName = codexServiceTierModelForRequest(params, modelHint);
    const fastSupported = !requestedFast || codexServiceTierFastSupportedForModel(modelName);
    return {
      threadId,
      mode,
      serviceTier: requestedFast && fastSupported ? codexFastServiceTierValue() : null,
      requestedServiceTier: requestedServiceTier || null,
      modelName,
      fastSupported,
      fastBlocked: requestedFast && !fastSupported,
    };
  }

  function codexServiceTierOverrideForRequest(method, params, threadIdHint = "") {
    if (!claudeCodexProSettings().serviceTierControls) return null;
    if (!codexServiceTierRequestMethods().has(method) || !params || typeof params !== "object") return null;
    const state = readThreadServiceTierState();
    const controlMode = normalizeCodexServiceTierControlMode(state.mode);
    const defaultMode = normalizeCodexThreadServiceTierMode(state.defaultMode);
    if (controlMode === "inherit") {
      const inheritedServiceTier = params.serviceTier ?? params.service_tier ?? codexServiceTierState.serviceTier;
      const override = codexServiceTierOverrideResult(method, params, threadIdHint, "inherit", inheritedServiceTier);
      return override.fastBlocked ? override : null;
    }
    if (controlMode === "global-standard" || controlMode === "global-fast") {
      return codexServiceTierOverrideResult(
        method,
        params,
        threadIdHint,
        controlMode,
        controlMode === "global-fast" ? codexFastServiceTierValue() : null
      );
    }
    const threadId = codexServiceTierThreadIdForRequest(method, params, threadIdHint);
    const override = threadId ? codexThreadServiceTierOverride(threadId) : codexThreadServiceTierDraft();
    const mode = codexServiceTierEffectiveThreadMode(override?.mode, defaultMode);
    if (mode === "inherit") {
      const inheritedServiceTier = params.serviceTier ?? params.service_tier ?? codexServiceTierState.serviceTier;
      const inheritedOverride = codexServiceTierOverrideResult(method, params, threadIdHint, "inherit", inheritedServiceTier);
      return inheritedOverride.fastBlocked ? { ...inheritedOverride, threadId, mode } : null;
    }
    return {
      ...codexServiceTierOverrideResult(method, params, threadIdHint, mode, mode === "fast" ? codexFastServiceTierValue() : null),
      threadId,
      mode,
    };
  }

  function applyCodexServiceTierRequestOverride(method, params, threadIdHint = "") {
    const override = codexServiceTierOverrideForRequest(method, params, threadIdHint);
    if (!override) return params;
    const nextParams = { ...(params || {}), serviceTier: override.serviceTier };
    if (Object.prototype.hasOwnProperty.call(nextParams, "service_tier") || override.fastBlocked) {
      nextParams.service_tier = override.serviceTier;
    }
    sendClaudeCodexProDiagnostic("service_tier_request_override_applied", {
      method,
      threadId: override.threadId || "",
      mode: override.mode,
      serviceTier: override.serviceTier || "standard",
      model: override.modelName || "",
      fastSupported: override.fastSupported !== false,
      fastBlocked: !!override.fastBlocked,
    });
    return nextParams;
  }

  function codexServiceTierRequestOverride(message) {
    if (!message || typeof message !== "object") return message;
    if (message.type === "send-cli-request-for-host") {
      const method = String(message.method || "");
      const params = applyCodexRequestOverrides(method, message.params);
      return params === message.params ? message : { ...message, params };
    }
    if (message.type === "mcp-request" && message.request && typeof message.request === "object") {
      const method = String(message.request.method || "");
      const params = applyCodexRequestOverrides(method, message.request.params);
      if (params === message.request.params) return message;
      return { ...message, request: { ...message.request, params } };
    }
    if (message.type === "worker-request" && message.request && typeof message.request === "object") {
      const method = String(message.request.method || "");
      const params = applyCodexRequestOverrides(method, message.request.params);
      if (params === message.request.params) return message;
      return { ...message, request: { ...message.request, params } };
    }
    if (message.type === "thread-prewarm-start" && message.request && typeof message.request === "object") {
      const params = applyCodexRequestOverrides("thread/start", message.request.params);
      if (params === message.request.params) return message;
      return { ...message, request: { ...message.request, params } };
    }
    if (message.type === "start-conversation") {
      const nextMessage = applyCodexRequestOverrides("thread/start", message);
      return nextMessage === message ? message : nextMessage;
    }
    if (message.type === "prewarm-thread-start-for-host" && message.params && typeof message.params === "object") {
      const params = applyCodexRequestOverrides("thread/start", message.params);
      return params === message.params ? message : { ...message, params };
    }
    if (message.type === "start-thread-for-host") {
      const params = applyCodexRequestOverrides("thread/start", message);
      return params === message ? message : params;
    }
    if (message.type === "start-turn-for-host" && message.params && typeof message.params === "object") {
      const params = applyCodexRequestOverrides("turn/start", message.params, message.conversationId);
      return params === message.params ? message : { ...message, params };
    }
    return message;
  }

  function installCodexServiceTierDispatcherPatch() {
    if (window.__codexServiceTierRequestOverrideInstalled === codexServiceTierRequestOverrideVersion) return;
    const patch = async () => {
      try {
        const module = await loadCodexAppModule("setting-storage-");
        const dispatcherClass = typeof module.v === "function" && String(module.v).includes("dispatchMessage") ? module.v : null;
        const dispatcher = dispatcherClass?.getInstance?.();
        if (!dispatcher || typeof dispatcher.dispatchMessage !== "function") throw new Error("Codex dispatcher unavailable");
        if (dispatcher.__codexServiceTierOriginalDispatchMessage) {
          window.__codexServiceTierRequestOverrideInstalled = codexServiceTierRequestOverrideVersion;
          return;
        }
        dispatcher.__codexServiceTierOriginalDispatchMessage = dispatcher.dispatchMessage.bind(dispatcher);
        dispatcher.dispatchMessage = (type, payload) => {
          const message = codexServiceTierRequestOverride({ ...(payload || {}), type });
          const nextType = message?.type || type;
          const { type: _type, ...nextPayload } = message || {};
          return dispatcher.__codexServiceTierOriginalDispatchMessage(nextType, nextPayload);
        };
        window.__codexServiceTierRequestOverrideInstalled = codexServiceTierRequestOverrideVersion;
        sendClaudeCodexProDiagnostic("service_tier_dispatcher_patch_installed", {});
      } catch (error) {
        sendClaudeCodexProDiagnostic("service_tier_dispatcher_patch_failed", {
          errorName: error?.name || "",
          errorMessage: error?.message || String(error),
        });
      }
    };
    void patch();
  }

  async function loadBackendSettings() {
    try {
      const settings = await postJson("/settings/get", {});
      if (!settings || typeof settings !== "object" || (!("launchMode" in settings) && !("enhancementsEnabled" in settings) && !("providerSyncEnabled" in settings))) {
        throw new Error("invalid backend settings response");
      }
      claudeCodexProBackendSettings = { ...claudeCodexProBackendSettings, ...settings };
      claudeCodexProBackendSettingsLoaded = true;
      refreshClaudeCodexProBackendToggles();
      return true;
    } catch (_) {
      refreshClaudeCodexProBackendToggles();
      return false;
    }
  }

  function loadBackendSettingsForStartup(attempt = 0) {
    loadBackendSettings().then((loaded) => {
      if (loaded) {
        scan();
        codexMemoryUpdateBadge();
        void codexMemoryLoadSession(true);
        void codexMemoryMaybeSuggestCandidate();
        return;
      }
      if (attempt < 60) {
        setTimeout(() => loadBackendSettingsForStartup(attempt + 1), 250);
      }
    });
  }

  async function setBackendSetting(key, value) {
    const deferMulticaWorkspaceRuntime = key === "multicaWorkspaceEnabled";
    const previousValue = claudeCodexProBackendSettings[key];
    claudeCodexProBackendSettings = { ...claudeCodexProBackendSettings, [key]: value };
    // The workspace toggle owns its runtime lifecycle below. Do not let the
    // optimistic local value make a scan remove the settings page before the
    // persistence request has completed.
    refreshClaudeCodexProBackendToggles({ skipMulticaWorkspaceRuntime: deferMulticaWorkspaceRuntime });
    try {
      const settings = await postJson("/settings/set", { [key]: value });
      if (deferMulticaWorkspaceRuntime && settings?.[key] !== value) {
        throw new Error(settings?.message || "设置保存结果未确认");
      }
      claudeCodexProBackendSettings = { ...claudeCodexProBackendSettings, ...settings };
      return settings;
    } catch (error) {
      if (deferMulticaWorkspaceRuntime) {
        claudeCodexProBackendSettings = { ...claudeCodexProBackendSettings, [key]: previousValue };
      }
      throw error;
    } finally {
      refreshClaudeCodexProBackendToggles({ skipMulticaWorkspaceRuntime: deferMulticaWorkspaceRuntime });
    }
  }

  function refreshClaudeCodexProBackendToggles(options = {}) {
    document.querySelectorAll(".claude-codex-pro-toggle[data-codex-backend-setting]").forEach((button) => {
      const key = button.getAttribute("data-codex-backend-setting");
      button.dataset.enabled = String(!!claudeCodexProBackendSettings[key]);
    });
    renderClaudeCodexProMenu();
    if (!options.skipMulticaWorkspaceRuntime) scan();
  }

  let claudeCodexProBackendStatus = { status: "checking", message: "正在检查连接…" };
  let claudeCodexProBackendCheckSeq = 0;
  let claudeCodexProBackendConsecutiveFailures = 0;
  const claudeCodexProBackendFailureThreshold = 3;
  let claudeChineseOverlayObserver = null;
  let claudeChineseOverlayScheduled = false;
  let claudeChineseOverlayFullRefreshDone = false;
  let claudeChineseOverlaySortedMap = null;
  let claudeChineseOverlayDirectMap = null;
  const claudeChineseOverlayQueue = [];
  const claudeChineseOverlayQueued = new Set();
  const claudeChineseOverlayBatchSize = 80;
  const claudeChineseOverlayProtectedBrands = [
    ["Claude Code", "__CCP_BRAND_CLAUDE_CODE__"],
    ["Codex", "__CCP_BRAND_CODEX__"],
    ["Claude", "__CCP_BRAND_CLAUDE__"],
  ];

  const claudeChineseOverlayMap = [
    ["Settings", "设置"],
    ["General settings", "通用设置"],
    ["Account settings", "账户设置"],
    ["App settings", "应用设置"],
    ["Language", "语言"],
    ["Display language", "显示语言"],
    ["Progress", "进度"],
    ["Working folder", "工作目录"],
    ["Working directory", "工作目录"],
    ["Current folder", "当前目录"],
    ["Context", "上下文"],
    ["Context window", "上下文窗口"],
    ["Context left", "剩余上下文"],
    ["New task", "新建任务"],
    ["New chat", "新建聊天"],
    ["New conversation", "新建对话"],
    ["Start new chat", "开始新聊天"],
    ["Start a new chat", "开始新聊天"],
    ["Projects", "项目"],
    ["Project", "项目"],
    ["Create project", "创建项目"],
    ["New project", "新建项目"],
    ["Project knowledge", "项目知识"],
    ["Project instructions", "项目指令"],
    ["Gateway", "网关"],
    ["Configure third-party inference", "配置第三方推理"],
    ["Inference configuration", "推理配置"],
    ["Third-party inference", "第三方推理"],
    ["Connection", "连接"],
    ["Choose where Claude Desktop sends inference requests.", "选择 Claude Desktop 将推理请求发送到哪里。"],
    ["Workspace restrictions", "工作区限制"],
    ["Allowed surfaces", "允许的界面"],
    ["Cowork", "协作"],
    ["General restrictions", "通用限制"],
    ["Allowed egress hosts", "允许出站主机"],
    ["Hostnames the agent's tools may reach from the Cowork and Code tabs.", "代理工具可从协作和代码页访问的主机名。"],
    ["Also surfaced under Egress Requirements.", "也会显示在出站要求中。"],
    ["Allowed workspace folders", "允许的工作区文件夹"],
    ["Folders users may attach as a workspace. Leave unset for unrestricted access.", "用户可附加为工作区的文件夹。留空则表示不限制访问。"],
    ["Disabled built-in tools", "已禁用内置工具"],
    ["Built-in tools removed from Cowork.", "从协作中移除的内置工具。"],
    ["Built-in tool policy", "内置工具策略"],
    ["Per-tool approval policy", "按工具审批策略"],
    ["ask\" requires user approval before each call; \"allow\" is", "“ask” 每次调用前都需要用户批准；“allow” 则会"],
    ["Add policy", "添加策略"],
    ["Connectors & extensions", "连接器与扩展"],
    ["MCP SERVERS", "MCP 服务器"],
    ["Managed MCP servers", "受管理的 MCP 服务器"],
    ["Org-pushed MCP servers: remote (HTTP/SSE) or local (stdio command). May embed bearer tokens.", "组织推送的 MCP 服务器：远程（HTTP/SSE）或本地（stdio 命令）。可能会嵌入 Bearer 令牌。"],
    ["Add server", "添加服务器"],
    ["Allow user-added MCP servers", "允许用户添加的 MCP 服务器"],
    ["Local stdio servers added via the Developer settings. Remote servers come from the managed list above, or plugins mounted to a user's computer by an organization admin.", "通过开发者设置添加的本地 stdio 服务器。远程服务器来自上方的受管列表，或者由组织管理员挂载到用户电脑上的插件。"],
    ["EXTENSIONS", "扩展"],
    ["Allow desktop extensions", "允许桌面扩展"],
    [".dxt and .mcpb installs.", ".dxt 和 .mcpb 安装。"],
    ["Require signed extensions", "要求已签名扩展"],
    ["Reject desktop extensions that are not signed by a trusted publisher.", "拒绝未由可信发布者签名的桌面扩展。"],
    ["Telemetry & updates", "遥测与更新"],
    ["ANTHROPIC TELEMETRY", "Anthropic 遥测"],
    ["Organization UUID", "组织 UUID"],
    ["Tags telemetry events with your organization's UUID so Anthropic support can find them. Not used for auth.", "使用组织的 UUID 标记遥测事件，以便 Anthropic 支持查找。不用于认证。"],
    ["Block essential telemetry", "屏蔽必要遥测"],
    ["Crash and performance reports to Anthropic.", "发送给 Anthropic 的崩溃和性能报告。"],
    ["Block nonessential telemetry", "屏蔽非必要遥测"],
    ["Product-usage analytics and diagnostic-report uploads. No message content.", "产品使用分析和诊断报告上传。不包含消息内容。"],
    ["Block nonessential services", "屏蔽非必要服务"],
    ["Favicon fetch and the artifact-preview iframe origin. Artifacts will not render.", "图标获取和产物预览 iframe 源。产物将不会渲染。"],
    ["Usage limits", "使用限制"],
    ["Max tokens per window", "每窗口最大 tokens"],
    ["Per-user soft cap, counted client-side over the duration below. Not a server-enforced quota.", "按用户计算的软上限，在下方周期内由客户端统计。不属于服务器强制配额。"],
    ["Plugins & skills", "插件与技能"],
    ["Organization plugins", "组织插件"],
    ["No organization plugins found", "未找到组织插件"],
    ["Mount plugin bundles to this folder using your device-management tool and Cowork will load them at launch. The folder is read-only; tool policies you set below are saved in this configuration.", "使用设备管理工具将插件包挂载到此文件夹，协作将在启动时加载它们。该文件夹为只读；你在下方设置的工具策略会保存在此配置中。"],
    ["Copy", "复制"],
    ["Add server policy", "添加服务器策略"],
    ["Egress Requirements", "出站要求"],
    ["FIREWALL ALLOWLIST", "防火墙允许列表"],
    ["Test connectivity", "测试连通性"],
    ["Copy hostnames", "复制主机名"],
    ["Download .txt", "下载 .txt"],
    ["CORE (VM BUNDLE + CLAUDE CLI BINARY)", "核心（VM 包 + Claude CLI 二进制）"],
    ["AUTO-UPDATES", "自动更新"],
    ["ESSENTIAL TELEMETRY", "必要遥测"],
    ["NONESSENTIAL TELEMETRY", "非必要遥测"],
    ["NONESSENTIAL SERVICES", "非必要服务"],
    ["Source", "来源"],
    ["BOOTSTRAP CONFIG URL", "启动配置 URL"],
    ["Bootstrap config URL", "启动配置 URL"],
    ["HTTPS endpoint that returns a per-user JSON config overlay. Values from the response override local settings and become read-only.", "返回每个用户 JSON 配置覆盖的 HTTPS 端点。响应中的值会覆盖本地设置并变为只读。"],
    ["Search settings", "搜索设置"],
    ["GATEWAY CREDENTIALS", "网关凭据"],
    ["Credential kind", "凭据类型"],
    ["Selects the credential source. When set, only that source is used (no fallback).", "选择凭据来源。设置后只使用该来源（无回退）。"],
    ["Static API key", "静态 API Key"],
    ["Gateway base URL", "网关基础 URL"],
    ["Full URL of the inference gateway endpoint.", "推理网关端点的完整 URL。"],
    ["Gateway API key", "网关 API Key"],
    ["Gateway auth scheme", "网关认证方案"],
    ["How the gateway credential is sent on the wire", "网关凭据在请求中的发送方式"],
    ["Authorization: Bearer vs x-api-key header", "Authorization: Bearer 与 x-api-key 请求头"],
    ["Custom inference headers", "自定义推理请求头"],
    ["Extra HTTP headers sent on every inference request to the configured provider.", "每个推理请求都会发送到已配置提供商的额外 HTTP 请求头。"],
    ["For tenant routing, org IDs, Bedrock Guardrails, etc.", "用于租户路由、组织 ID、Bedrock Guardrails 等。"],
    ["Add header", "添加请求头"],
    ["Test connection", "测试连接"],
    ["Apply Changes", "应用更改"],
    ["Export", "导出"],
    ["Learn more", "了解更多"],
    ["Sign out", "退出登录"],
    ["Sign in", "登录"],
    ["Log in", "登录"],
    ["Log out", "退出登录"],
    ["Account", "账户"],
    ["Profile", "个人资料"],
    ["Email", "邮箱"],
    ["Name", "名称"],
    ["Model", "模型"],
    ["Models", "模型"],
    ["Select a model", "选择模型"],
    ["Choose a model", "选择模型"],
    ["Switch model", "切换模型"],
    ["Default model", "默认模型"],
    ["Search", "搜索"],
    ["Search chats", "搜索会话"],
    ["Search conversations", "搜索对话"],
    ["Search projects", "搜索项目"],
    ["Chats", "会话"],
    ["Chat", "会话"],
    ["Conversation", "对话"],
    ["Conversations", "对话"],
    ["Messages", "消息"],
    ["Message", "消息"],
    ["Folders", "文件夹"],
    ["Folder", "文件夹"],
    ["Archived conversations", "已归档对话"],
    ["Archived chats", "已归档会话"],
    ["Archive", "归档"],
    ["Archived", "已归档"],
    ["Unarchive", "取消归档"],
    ["Pin", "置顶"],
    ["Pinned conversations", "置顶对话"],
    ["Pinned chats", "置顶会话"],
    ["Pinned", "置顶"],
    ["Unpin", "取消置顶"],
    ["Preferences", "偏好设置"],
    ["Appearance", "外观"],
    ["Interface", "界面"],
    ["Accessibility", "辅助功能"],
    ["Language and region", "语言和地区"],
    ["Theme", "主题"],
    ["Light", "浅色"],
    ["Dark", "深色"],
    ["System", "跟随系统"],
    ["System theme", "系统主题"],
    ["Light theme", "浅色主题"],
    ["Dark theme", "深色主题"],
    ["Color scheme", "配色方案"],
    ["Notifications", "通知"],
    ["Notification", "通知"],
    ["Desktop notifications", "桌面通知"],
    ["Email notifications", "邮件通知"],
    ["Sound", "声音"],
    ["Mute", "静音"],
    ["Data controls", "数据控制"],
    ["Data settings", "数据设置"],
    ["Conversation history", "会话历史"],
    ["Chat history", "聊天历史"],
    ["Usage data", "使用数据"],
    ["Auto save", "自动保存"],
    ["Auto-update", "自动更新"],
    ["Privacy", "隐私"],
    ["Privacy settings", "隐私设置"],
    ["Personalization", "个性化"],
    ["Security", "安全"],
    ["Security settings", "安全设置"],
    ["Billing", "账单"],
    ["Payment", "支付"],
    ["Payments", "支付"],
    ["Plan", "套餐"],
    ["Plans", "套餐"],
    ["Subscription", "订阅"],
    ["Manage subscription", "管理订阅"],
    ["Usage limit", "使用限制"],
    ["Usage limits", "使用限制"],
    ["Account settings", "账户设置"],
    ["Account plan", "账户套餐"],
    ["Profile settings", "个人资料设置"],
    ["Connected apps", "已连接应用"],
    ["Connected apps and integrations", "已连接应用与集成"],
    ["Experimental features", "实验功能"],
    ["Experimental", "实验性"],
    ["Feature preview", "功能预览"],
    ["Preview", "预览"],
    ["Activity", "活动"],
    ["Usage", "用量"],
    ["Tasks", "任务"],
    ["Task", "任务"],
    ["Artifacts", "产物"],
    ["Artifact", "产物"],
    ["Live artifacts", "实时产物"],
    ["Connectors", "连接器"],
    ["Connector", "连接器"],
    ["Integrations", "集成"],
    ["Integration", "集成"],
    ["MCP servers", "MCP 服务器"],
    ["MCP server", "MCP 服务器"],
    ["Tools", "工具"],
    ["Tool", "工具"],
    ["Customize", "自定义"],
    ["Custom instructions", "自定义指令"],
    ["Instructions", "指令"],
    ["Cancel", "取消"],
    ["Close", "关闭"],
    ["Save", "保存"],
    ["Save changes", "保存更改"],
    ["Saving", "保存中"],
    ["Saved", "已保存"],
    ["Confirm", "确认"],
    ["Done", "完成"],
    ["Apply", "应用"],
    ["Continue", "继续"],
    ["Back", "返回"],
    ["Next", "下一步"],
    ["Previous", "上一步"],
    ["Copy", "复制"],
    ["Copied", "已复制"],
    ["Copy link", "复制链接"],
    ["Paste", "粘贴"],
    ["Delete", "删除"],
    ["Delete chat", "删除会话"],
    ["Delete conversation", "删除对话"],
    ["Delete project", "删除项目"],
    ["Remove", "移除"],
    ["Edit", "编辑"],
    ["Rename", "重命名"],
    ["Duplicate", "复制一份"],
    ["Share", "分享"],
    ["Share chat", "分享会话"],
    ["Export", "导出"],
    ["Import", "导入"],
    ["Download", "下载"],
    ["Upload", "上传"],
    ["Submit", "提交"],
    ["Send", "发送"],
    ["Stop", "停止"],
    ["Stop response", "停止回复"],
    ["Regenerate", "重新生成"],
    ["Regenerate response", "重新生成回复"],
    ["Retry", "重试"],
    ["Retrying", "重试中"],
    ["Thinking", "思考中"],
    ["Thinking...", "思考中..."],
    ["Thinking…", "思考中…"],
    ["Generating", "生成中"],
    ["Generating...", "生成中..."],
    ["Generating…", "生成中…"],
    ["Loading", "加载中"],
    ["Loading...", "加载中..."],
    ["Loading…", "加载中…"],
    ["Please wait", "请稍候"],
    ["Try again", "请重试"],
    ["Something went wrong", "出了点问题"],
    ["Unable to load", "无法加载"],
    ["Connection lost", "连接已断开"],
    ["Reconnect", "重新连接"],
    ["No results", "没有结果"],
    ["No conversations", "没有会话"],
    ["No chats", "没有会话"],
    ["No projects", "没有项目"],
    ["No items", "没有项目"],
    ["Empty", "为空"],
    ["Write a message...", "输入消息..."],
    ["Type a message...", "输入消息..."],
    ["Message Claude", "给 Claude 发消息"],
    ["Ask anything", "请输入内容"],
    ["How can I help you today?", "今天我能帮你什么？"],
    ["What can I help you with?", "我能帮你什么？"],
    ["What are you working on?", "你正在做什么？"],
    ["Send a message", "发送消息"],
    ["Upload files", "上传文件"],
    ["Attach files", "附加文件"],
    ["Add files", "添加文件"],
    ["Drag and drop files", "拖放文件"],
    ["File upload", "文件上传"],
    ["View", "视图"],
    ["Help", "帮助"],
    ["Support", "支持"],
    ["Keyboard shortcuts", "键盘快捷键"],
    ["Shortcut keys", "快捷键"],
    ["Hotkeys", "热键"],
    ["About", "关于"],
    ["More", "更多"],
    ["More options", "更多操作"],
    ["Advanced", "高级"],
    ["Advanced settings", "高级设置"],
    ["General", "通用"],
    ["General preferences", "通用偏好设置"],
    ["General settings", "通用设置"],
    ["Labs", "实验功能"],
    ["Beta", "测试版"],
    ["File", "文件"],
    ["Edit", "编辑"],
    ["View", "视图"],
    ["Help", "帮助"],
    ["Open Documentation", "打开文档"],
    ["Check for Updates...", "检查更新..."],
    ["Troubleshooting", "故障排查"],
    ["Show Logs in Explorer", "在资源管理器中显示日志"],
    ["Show Cowork Session Data in Explorer", "在资源管理器中显示协作会话数据"],
    ["Copy Installation ID", "复制安装 ID"],
    ["Generate Diagnostic Report", "生成诊断报告"],
    ["Record Net Log (30s)", "录制网络日志（30 秒）"],
    ["Enable Developer Mode", "启用开发者模式"],
    ["Disable Hardware Acceleration", "禁用硬件加速"],
    ["Enable Cowork VM Debug Logging", "启用协作 VM 调试日志"],
    ["Enable Cowork SDK Debugging", "启用协作 SDK 调试"],
    ["Free Up Cowork Disk Space...", "释放协作磁盘空间..."],
    ["Delete Cowork VM Bundle and Restart...", "删除协作 VM 包并重启..."],
    ["Delete Cowork VM Sessions and Restart...", "删除协作 VM 会话并重启..."],
    ["Clear Cache and Restart", "清除缓存并重启"],
    ["Reset App Data...", "重置应用数据..."],
    ["Overview", "概览"],
    ["History", "历史"],
    ["All", "全部"],
    ["Recent", "最近"],
    ["Recents", "最近"],
    ["Today", "今天"],
    ["Yesterday", "昨天"],
    ["Last 7 days", "最近 7 天"],
    ["Last 30 days", "最近 30 天"],
    ["Draft", "草稿"],
    ["Drafts", "草稿"],
    ["Error", "错误"],
    ["Failed", "失败"],
    ["Success", "成功"],
    ["Ready", "就绪"],
    ["Running", "运行中"],
    ["Stopped", "已停止"],
    ["Offline", "离线"],
    ["Online", "在线"],
    ["Open", "打开"],
    ["Open in new window", "在新窗口打开"],
    ["Read only", "只读"],
    ["Developer mode", "开发者模式"],
    ["Release notes", "更新日志"],
    ["What's new", "新功能"],
    ["Feedback", "反馈"],
    ["Report a bug", "报告问题"],
    ["Terms of service", "服务条款"],
    ["Privacy policy", "隐私政策"],
  ];

  function refreshClaudeChineseOverlay() {
    if (!claudeCodexProSettings().chineseOverlayEnabled) return;
    if (claudeChineseOverlayFullRefreshDone) return;
    claudeChineseOverlayFullRefreshDone = true;
    const roots = [document.body, document.querySelector("main"), document.querySelector("aside")].filter(Boolean);
    roots.forEach(queueClaudeChineseOverlaySubtree);
    scheduleClaudeChineseOverlayRefresh();
  }

  function scheduleClaudeChineseOverlayRefresh() {
    if (claudeChineseOverlayScheduled) return;
    claudeChineseOverlayScheduled = true;
    requestAnimationFrame(() => {
      claudeChineseOverlayScheduled = false;
      flushClaudeChineseOverlayQueue();
    });
  }

  function ensureClaudeChineseOverlayObserver() {
    if (claudeChineseOverlayObserver || !window.MutationObserver || !document.documentElement) return;
    claudeChineseOverlayObserver = new MutationObserver((mutations) => {
      if (!claudeCodexProSettings().chineseOverlayEnabled) return;
      mutations.forEach(queueClaudeChineseOverlayMutation);
      scheduleClaudeChineseOverlayRefresh();
    });
    claudeChineseOverlayObserver.observe(document.documentElement, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ["title", "aria-label", "placeholder", "alt", "data-placeholder"],
    });
  }

  function translateClaudeChineseText(value) {
    return String(value || "");
  }

  function protectClaudeChineseOverlayBrands(value) {
    let next = String(value || "");
    const tokens = [];
    claudeChineseOverlayProtectedBrands.forEach(([brand, token]) => {
      if (!next.includes(brand)) return;
      next = next.replaceAll(brand, token);
      tokens.push([token, brand]);
    });
    return { value: next, tokens };
  }

  function restoreClaudeChineseOverlayBrands(value, tokens) {
    let next = String(value || "");
    (tokens || []).forEach(([token, brand]) => {
      next = next.replaceAll(token, brand);
    });
    return next;
  }

  function queueClaudeChineseOverlayNode(node) {
    if (!node || claudeChineseOverlayQueued.has(node)) return;
    if (node.nodeType !== Node.ELEMENT_NODE && node.nodeType !== Node.TEXT_NODE) return;
    if (node.nodeType === Node.ELEMENT_NODE && isExtensionUiNode(node)) return;
    claudeChineseOverlayQueued.add(node);
    claudeChineseOverlayQueue.push(node);
  }

  function queueClaudeChineseOverlaySubtree(root) {
    queueClaudeChineseOverlayNode(root);
    if (!root || root.nodeType !== Node.ELEMENT_NODE) return;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) queueClaudeChineseOverlayNode(node);
  }

  function queueClaudeChineseOverlayMutation(mutation) {
    queueClaudeChineseOverlayNode(mutation.target);
    mutation.addedNodes.forEach(queueClaudeChineseOverlaySubtree);
  }

  function flushClaudeChineseOverlayQueue() {
    if (!claudeCodexProSettings().chineseOverlayEnabled) {
      claudeChineseOverlayQueue.length = 0;
      return;
    }
    let remaining = claudeChineseOverlayBatchSize;
    while (remaining > 0 && claudeChineseOverlayQueue.length) {
      const node = claudeChineseOverlayQueue.shift();
      claudeChineseOverlayQueued.delete(node);
      translateClaudeChineseOverlayNode(node);
      remaining -= 1;
    }
    if (claudeChineseOverlayQueue.length) scheduleClaudeChineseOverlayRefresh();
  }

  function translateClaudeChineseOverlayNode(current) {
    if (!current) return;
    if (current.nodeType === Node.TEXT_NODE) {
      const original = current.nodeValue || "";
      const next = translateClaudeChineseText(original);
      if (next !== original) current.nodeValue = next;
      return;
    }
    if (!(current instanceof Element) || isExtensionUiNode(current)) return;
    if (current.childNodes.length === 1 && current.firstChild?.nodeType === Node.TEXT_NODE) {
      const original = current.textContent || "";
      const next = translateClaudeChineseText(original);
      if (next !== original) current.textContent = next;
    }
    ["title", "aria-label", "placeholder", "data-placeholder", "alt"].forEach((attr) => {
      const value = current.getAttribute(attr);
      if (!value) return;
      const nextValue = translateClaudeChineseText(value);
      if (nextValue !== value) current.setAttribute(attr, nextValue);
    });
  }

  function setClaudeCodexProTriggerLabel(trigger) {
    if (!trigger) return;
    trigger.setAttribute("aria-label", `CCP ${claudeCodexProVersion}`);
    trigger.title = `CCP ${claudeCodexProVersion}`;
    const hasRenderableStatusLabel = !!trigger.querySelector("[data-codex-backend-indicator]")
      && Array.from(trigger.querySelectorAll(".claude-codex-pro-window-status-title"))
        .some((node) => String(node.textContent || "").trim().startsWith("CCP"));
    if (trigger.dataset.claudeCodexProTriggerLabel === "ccp-status-v2" && hasRenderableStatusLabel) return;
    trigger.dataset.claudeCodexProTriggerLabel = "ccp-status-v2";
    trigger.textContent = "";
    const indicator = document.createElement("span");
    indicator.className = "claude-codex-pro-window-status-dot";
    indicator.dataset.codexBackendIndicator = "true";
    indicator.dataset.status = claudeCodexProBackendStatus.status || "checking";
    const title = document.createElement("span");
    title.className = "claude-codex-pro-window-status-title";
    title.textContent = `CCP ${claudeCodexProVersion}`;
    trigger.append(indicator, title);
  }

  function ensureClaudeCodexProTriggerIndicator(trigger) {
    if (!trigger) return null;
    let indicator = trigger.querySelector("[data-codex-backend-indicator]");
    if (!indicator) {
      indicator = document.createElement("span");
      indicator.className = "claude-codex-pro-window-status-dot";
      indicator.dataset.codexBackendIndicator = "true";
      trigger.prepend(indicator);
    }
    return indicator;
  }

  function renderBackendStatus() {
    const status = claudeCodexProBackendStatus.status || "failed";
    const statusMessage = claudeCodexProBackendStatus.message || (status === "ok" ? "已连接" : status === "checking" ? "检查中" : "未连接");
    const label = document.querySelector("[data-codex-backend-status]");
    if (label) {
      label.dataset.status = status;
      label.textContent = statusMessage;
    }
    document.querySelectorAll("[data-codex-backend-indicator]").forEach((indicator) => {
      indicator.dataset.status = status;
      indicator.title = statusMessage;
    });
    document.querySelectorAll(".claude-codex-pro-trigger").forEach((trigger) => {
      trigger.title = `CCP ${claudeCodexProVersion}：${statusMessage}`;
    });
    const repair = document.querySelector("[data-codex-backend-repair]");
    if (repair) repair.hidden = status === "ok" || status === "checking";
    refreshCodexServiceTierControls();
  }

  function withBackendTimeout(request) {
    return Promise.race([
      request,
      new Promise((resolve) => setTimeout(() => resolve({ status: "failed", message: "连接检查超时", timeout: true }), 3000)),
    ]);
  }

  async function checkBackendStatus() {
    const seq = ++claudeCodexProBackendCheckSeq;
    const nextStatus = await withBackendTimeout(postJson("/backend/status", {}));
    if (seq !== claudeCodexProBackendCheckSeq) return;
    if (claudeCodexProBackendHeartbeatGeneration !== window.__claudeCodexProBackendHeartbeatGeneration) return;
    if (nextStatus?.status === "ok") {
      claudeCodexProBackendConsecutiveFailures = 0;
      claudeCodexProBackendStatus = nextStatus;
    } else {
      claudeCodexProBackendConsecutiveFailures += 1;
      sendClaudeCodexProDiagnostic("backend_check_failed", {
        status: nextStatus?.status || "unknown",
        message: nextStatus?.message || "",
        timeout: !!nextStatus?.timeout,
        consecutiveFailures: claudeCodexProBackendConsecutiveFailures,
      });
      if (claudeCodexProBackendConsecutiveFailures >= claudeCodexProBackendFailureThreshold) {
        claudeCodexProBackendStatus = nextStatus;
      } else if (claudeCodexProBackendStatus.status !== "ok") {
        claudeCodexProBackendStatus = { status: "checking", message: "正在确认连接…" };
      }
    }
    renderBackendStatus();
  }

  async function repairBackend() {
    claudeCodexProBackendStatus = { status: "checking", message: "正在修复连接…" };
    renderBackendStatus();
    try {
      claudeCodexProBackendStatus = await postJson("/backend/repair", {});
      if (claudeCodexProBackendStatus?.status === "ok") {
        claudeCodexProBackendConsecutiveFailures = 0;
      }
    } catch (error) {
      claudeCodexProBackendStatus = { status: "failed", message: "连接修复失败" };
    }
    renderBackendStatus();
  }

  function scheduleBackendHeartbeat() {
    clearInterval(window.__claudeCodexProBackendHeartbeat);
    document.removeEventListener("visibilitychange", window.__claudeCodexProBackendVisibilityHandler);
    window.__claudeCodexProBackendVisibilityHandler = () => {
      if (document.visibilityState !== "visible") return;
      void checkBackendStatus();
    };
    document.addEventListener("visibilitychange", window.__claudeCodexProBackendVisibilityHandler);
    window.__claudeCodexProBackendHeartbeat = window.setInterval(() => {
      void checkBackendStatus();
    }, 5000);
    void checkBackendStatus();
  }

  let claudeCodexProAds = [];
  let claudeCodexProAdsLoaded = false;

  function isClaudeCodexProAdExpired(ad) {
    if (!ad.expires_at) return false;
    const expiresAt = Date.parse(ad.expires_at);
    return Number.isFinite(expiresAt) && expiresAt < Date.now();
  }

  function isSafeClaudeCodexProAdUrl(value) {
    try {
      const parsed = new URL(String(value || ""));
      return parsed.protocol === "https:" || parsed.protocol === "http:";
    } catch (_) {
      return false;
    }
  }

  function normalizeClaudeCodexProAds(payload) {
    if (!payload || payload.enabled !== true) return [];
    const remoteAds = payload && Array.isArray(payload.ads) ? payload.ads : [];
    const seen = new Set();
    return remoteAds.filter((ad) => {
      return ad && ad.type === "normal" && ad.title && ad.description && isSafeClaudeCodexProAdUrl(ad.url) && !isClaudeCodexProAdExpired(ad);
    }).map((ad) => ({
      id: String(ad.id || ad.title),
      type: ad.type,
      badge: ad.badge ? String(ad.badge) : "",
      title: String(ad.title),
      description: String(ad.description),
      buttonLabel: ad.buttonLabel ? String(ad.buttonLabel) : "查看详情",
      url: String(ad.url),
      expires_at: ad.expires_at ? String(ad.expires_at) : "",
      highlights: Array.isArray(ad.highlights) ? ad.highlights.map((item) => String(item)).filter(Boolean) : [],
    })).filter((ad) => {
      const key = `${ad.id}\n${ad.url}`;
      if (seen.has(ad.id) || seen.has(ad.url) || seen.has(key)) return false;
      seen.add(ad.id);
      seen.add(ad.url);
      seen.add(key);
      return true;
    });
  }

  function renderClaudeCodexProAdGroup(type, emptyText) {
    const ads = claudeCodexProAds.filter((ad) => ad.type === type);
    if (!ads.length) return `<div class="claude-codex-pro-ad-empty">${escapeHtml(emptyText)}</div>`;
    return ads.map((ad) => `
      <article class="claude-codex-pro-ad-card">
        <div class="claude-codex-pro-ad-content">
          ${ad.badge ? `<div class="claude-codex-pro-ad-badge">${escapeHtml(ad.badge)}</div>` : ""}
          <h3 class="claude-codex-pro-ad-title">${escapeHtml(ad.title)}</h3>
          <p class="claude-codex-pro-ad-description">${escapeHtml(ad.description)}</p>
          <div class="claude-codex-pro-ad-highlights">
            ${ad.highlights.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}
          </div>
          <a class="claude-codex-pro-ad-link" href="${escapeHtml(ad.url)}" target="_blank" rel="noreferrer">${escapeHtml(ad.buttonLabel)}</a>
        </div>
      </article>
    `).join("");
  }

  function renderClaudeCodexProAds() {
    if (!claudeCodexProAdsLoaded) return `<div class="claude-codex-pro-ad-empty">推荐内容加载中...</div>`;
    const normalAds = claudeCodexProAds.filter((ad) => ad.type === "normal");
    if (!normalAds.length) return `<div class="claude-codex-pro-ad-empty">暂无推荐内容。</div>`;
    return `
      <section class="claude-codex-pro-ad-section">
        <h3 class="claude-codex-pro-ad-section-title">推荐内容</h3>
        <div class="claude-codex-pro-ad-list">${renderClaudeCodexProAdGroup("normal", "暂无推荐内容。")}</div>
      </section>
    `;
  }
  function cacheBustClaudeCodexProAdUrl(url, version) {
    return `${url}${url.includes("?") ? "&" : "?"}v=${version}`;
  }

  async function directFetchClaudeCodexProAds() {
    const urls = [
      "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main/assets/config/announcement.json",
      "https://cdn.jsdelivr.net/gh/DamonZS/Claude-Codex-Pro-Tool@main/assets/config/announcement.json",
    ];
    let lastError = null;
    const cacheBust = Date.now();
    for (const url of urls) {
      try {
        const response = await fetch(cacheBustClaudeCodexProAdUrl(url, cacheBust), {
          headers: { "Accept": "application/json" },
          cache: "no-store",
        });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        return await response.json();
      } catch (error) {
        lastError = error;
      }
    }
    throw lastError || new Error("ad list unavailable");
  }

  async function fetchClaudeCodexProAds() {
    try {
      claudeCodexProAds = normalizeClaudeCodexProAds(await directFetchClaudeCodexProAds());
    } catch (error) {
      sendClaudeCodexProDiagnostic("ads_fetch_failed", {
        errorName: error?.name || "",
        errorMessage: error?.message || String(error),
      });
      claudeCodexProAds = normalizeClaudeCodexProAds(claudeCodexProBundledAnnouncement);
    } finally {
      claudeCodexProAdsLoaded = true;
      const panel = document.querySelector('[data-claude-codex-pro-panel="recommendations"] .claude-codex-pro-ad-remote');
      if (panel) panel.innerHTML = renderClaudeCodexProAds();
    }
  }

  function selectClaudeCodexProTab(tab) {
    document.querySelectorAll(".claude-codex-pro-modal-content").forEach((modal) => {
      modal.dataset.claudeCodexProActiveTab = tab;
    });
    document.querySelectorAll("[data-claude-codex-pro-tab]").forEach((button) => {
      button.dataset.active = String(button.getAttribute("data-claude-codex-pro-tab") === tab);
    });
    document.querySelectorAll("[data-claude-codex-pro-panel]").forEach((panel) => {
      panel.hidden = panel.getAttribute("data-claude-codex-pro-panel") !== tab;
    });
  }

  function openClaudeCodexProModal() {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
    document.dispatchEvent(new KeyboardEvent("keyup", { key: "Escape", code: "Escape", bubbles: true }));
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    document.querySelectorAll(".claude-codex-pro-modal-overlay").forEach((node) => node.remove());
    document.querySelectorAll('[data-claude-codex-pro-dialog="true"]').forEach((node) => node.remove());
    const overlay = document.createElement("div");
    overlay.className = "claude-codex-pro-modal-overlay";
    overlay.innerHTML = `
      <div class="claude-codex-pro-modal-content claude-codex-pro-control-deck" data-claude-codex-pro-dialog="true" role="dialog" aria-modal="true" aria-label="盘古本地控制舱">
        <div class="claude-codex-pro-modal-header">
          <div class="claude-codex-pro-deck-brand">
            <div class="claude-codex-pro-deck-mark" aria-hidden="true">CCP</div>
            <div class="claude-codex-pro-deck-heading">
              <div class="claude-codex-pro-deck-kicker">PANGU LOCAL CONTROL DECK</div>
              <div class="claude-codex-pro-modal-title"><span class="claude-codex-pro-backend-indicator" data-codex-backend-indicator="true" data-status="checking"></span><span>盘古本地控制舱</span><span class="claude-codex-pro-deck-version" data-claude-codex-pro-version="true">${claudeCodexProVersion}</span></div>
            </div>
          </div>
          <button type="button" class="claude-codex-pro-modal-close" aria-label="关闭">×</button>
        </div>
        <div class="claude-codex-pro-tabs" role="tablist" aria-label="Claude Codex Pro">
          <button type="button" class="claude-codex-pro-tab-button" data-claude-codex-pro-tab="home" data-active="true">主页</button>
          <button type="button" class="claude-codex-pro-tab-button" data-claude-codex-pro-tab="recommendations" data-active="false">推荐内容</button>
          <button type="button" class="claude-codex-pro-tab-button" data-claude-codex-pro-tab="support" data-active="false">支持</button>
          <button type="button" class="claude-codex-pro-tab-button" data-claude-codex-pro-tab="contact" data-active="false">合作请联系微信</button>
          <div class="claude-codex-pro-deck-sidebar-note">LOCAL FIRST<br>REVIEWABLE · REVERSIBLE</div>
        </div>
        <div class="claude-codex-pro-modal-body">
          <div class="claude-codex-pro-panel" data-claude-codex-pro-panel="home">
            <div class="claude-codex-pro-deck-hero">
              <div class="claude-codex-pro-deck-hero-label">CCP / LOCAL OPERATIONS</div>
              <h2>把模型、记忆与工作流留在你的控制范围内</h2>
              <p>这是 CCP 的本地能力控制面：集中查看桥接状态，按需启用增强，并保留可审查、可修复与可回退的操作路径。</p>
              <div class="claude-codex-pro-deck-capabilities" aria-label="核心能力"><span>本机运行</span><span>模型桥接</span><span>盘古记忆</span><span>可审查回退</span></div>
            </div>
            <div class="claude-codex-pro-deck-section-title">模型与插件通道</div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">连接状态</div><div class="claude-codex-pro-row-description">状态灯同步显示当前注入连接，异常时可尝试修复运行。</div></div>
              <div class="claude-codex-pro-status-note" data-codex-backend-status="true" data-status="checking">检查中</div>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">页面功能增强</div><div class="claude-codex-pro-row-description">关闭后停用删除、导出、移动、Timeline、插件相关和菜单位置增强。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-codex-backend-setting="enhancementsEnabled"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">插件市场解锁</div><div class="claude-codex-pro-row-description">${claudeCodexProBackendSettings.launchMode === "relay" ? "兼容增强模式下无需开启；ChatGPT 登录态会保留官方插件市场。" : "API Key 模式下扩展插件市场请求，尽量显示完整插件列表。"}</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="pluginMarketplaceUnlock" ${claudeCodexProBackendSettings.launchMode === "relay" ? 'disabled data-relay-unneeded="true"' : ""}><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">强制解锁入口</div><div class="claude-codex-pro-row-description">${claudeCodexProBackendSettings.launchMode === "relay" ? "兼容增强模式下无需开启；官方登录态会保留插件入口。" : "恢复 1.1.9 的入口解锁方式，强制显示并启用插件入口。"}</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="pluginEntryUnlock" ${claudeCodexProBackendSettings.launchMode === "relay" ? 'disabled data-relay-unneeded="true"' : ""}><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">特殊插件强制安装</div><div class="claude-codex-pro-row-description">${claudeCodexProBackendSettings.launchMode === "relay" ? "兼容增强模式下无需开启；不会改插件安装入口。" : "解除 App unavailable / 应用不可用导致的前端安装禁用。"}</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="forcePluginInstall" ${claudeCodexProBackendSettings.launchMode === "relay" ? 'disabled data-relay-unneeded="true"' : ""}><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">Fast 按钮</div><div class="claude-codex-pro-row-description">显示服务模式切换按钮；Fast 仅支持 ${codexServiceTierFastModelListLabel()}，其他模型按 Standard 发送。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="serviceTierControls"><span></span></button>
            </div>
            <div class="claude-codex-pro-row" data-codex-service-tier-controls="true">
              <div><div class="claude-codex-pro-row-title">服务模式</div><div class="claude-codex-pro-row-description">继承使用 config.toml 的 service tier；全局模式覆盖全部 thread；自定义允许按 thread 覆盖。</div></div>
              <div class="claude-codex-pro-service-tier-control">
                <div class="claude-codex-pro-service-tier-status" data-codex-service-tier-status="true" data-status="loading">正在读取…</div>
                <div class="claude-codex-pro-service-tier-actions">
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-inherit="true">继承</button>
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-standard="true">全局 Standard</button>
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-fast="true">全局 Fast</button>
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-custom="true">自定义</button>
                </div>
                <div class="claude-codex-pro-service-tier-actions claude-codex-pro-service-tier-thread-actions">
                  <span class="claude-codex-pro-service-tier-thread-label">当前 thread 覆盖</span>
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-thread-inherit="true" title="当前 thread 不单独覆盖，继承 config.toml">继承</button>
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-thread-standard="true" title="仅当前 thread 使用 Standard，并切到自定义模式">Standard</button>
                  <button type="button" class="claude-codex-pro-service-tier-button" data-codex-service-tier-thread-fast="true" title="仅当前 thread 使用 Fast，并切到自定义模式">Fast</button>
                </div>
              </div>
            </div>
            <div class="claude-codex-pro-deck-section-title">会话与工作流</div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">会话删除</div><div class="claude-codex-pro-row-description">在会话列表悬停显示删除按钮，并支持撤销。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="sessionDelete"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">Markdown 导出</div><div class="claude-codex-pro-row-description">在会话列表显示导出按钮，按本地 rollout 导出带时间戳的 Markdown。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="markdownExport"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">会话项目移动</div><div class="claude-codex-pro-row-description">在会话列表悬停显示移动按钮，可移动到普通对话或其他本地项目。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="projectMove"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">对话 Timeline</div><div class="claude-codex-pro-row-description">在对话右侧显示用户提问时间线，悬停查看摘要，点击跳转。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="conversationTimeline"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">对话居中宽度</div><div class="claude-codex-pro-row-description">开启后把主对话和输入框限制到固定最大宽度，适合大屏阅读。</div></div>
              <div class="claude-codex-pro-width-control">
                <input class="claude-codex-pro-width-input" data-claude-codex-pro-conversation-view-width="true" min="${conversationViewMinWidth}" max="${conversationViewMaxAllowedWidth}" step="10" type="number" value="${conversationViewWidth()}">
                <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="conversationView"><span></span></button>
              </div>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">切换对话保留位置</div><div class="claude-codex-pro-row-description">开启后在不同 thread 之间切换时恢复到上一次浏览位置，不再自动跳到底部。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="threadScrollRestore"><span></span></button>
            </div>
            <div class="claude-codex-pro-deck-section-title">本地运维与诊断</div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">Zed Remote open</div><div class="claude-codex-pro-row-description">Open supported remote SSH file references in Zed without patching Codex.app.</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="zedRemoteOpen"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">Upstream worktree</div><div class="claude-codex-pro-row-description">Create a Git worktree from a fresh upstream branch, equivalent to git worktree add -b branch path upstream/base.</div></div>
              <div class="claude-codex-pro-worktree-actions">
                <button type="button" class="claude-codex-pro-action-button" data-codex-upstream-worktree-open="true">创建</button>
                <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="upstreamWorktreeCreate"><span></span></button>
              </div>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">历史会话修复</div><div class="claude-codex-pro-row-description">切换官方登录、混合 API 或纯 API 后，让旧对话重新显示在当前模式下。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-codex-backend-setting="providerSyncEnabled"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">页面增强模式</div><div class="claude-codex-pro-row-description">${claudeCodexProBackendSettings.launchMode === "relay" ? "兼容增强：保留会话删除、导出、项目移动和 Timeline，仅关闭插件入口相关增强。" : "完整增强：加载插件入口、强制安装、项目路径移动等全部页面能力。"}</div></div>
              <button type="button" class="claude-codex-pro-action-button" data-codex-backend-repair="true">修复运行</button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">原生菜单栏位置</div><div class="claude-codex-pro-row-description">把 Claude Codex Pro 菜单插入顶部原生菜单栏；默认关闭以避免页面重渲染冲突。</div></div>
              <button type="button" class="claude-codex-pro-toggle" data-claude-codex-pro-setting="nativeMenuPlacement"><span></span></button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">打开 DevTools</div><div class="claude-codex-pro-row-description">打开当前 Codex 页面开发者工具，方便排查前端增强运行状态。</div></div>
              <button type="button" class="claude-codex-pro-action-button" data-codex-open-devtools="true">打开 DevTools</button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">关于 Claude Codex Pro</div><div class="claude-codex-pro-about">Claude Codex Pro 是通过外部 launcher 注入的增强菜单，不修改 Codex App 原始安装文件。<br>Build: <span data-claude-codex-pro-build="true">${claudeCodexProBuild}</span><br>GitHub: <a href="https://github.com/DamonZS/Claude-Codex-Pro-Tool" target="_blank" rel="noreferrer">https://github.com/DamonZS/Claude-Codex-Pro-Tool</a><br>Discord: <a href="https://discord.gg/Q9cbMaWsb" target="_blank" rel="noreferrer">https://discord.gg/Q9cbMaWsb</a></div></div>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">Discord 社区</div><div class="claude-codex-pro-row-description">加入 Discord 获取更新消息、反馈问题或交流使用体验。</div></div>
              <button type="button" class="claude-codex-pro-action-button" data-claude-codex-pro-discord="true">打开 Discord</button>
            </div>
            <div class="claude-codex-pro-row">
              <div><div class="claude-codex-pro-row-title">提出问题</div><div class="claude-codex-pro-row-description">打开 GitHub Issues 反馈问题或建议。</div></div>
              <button type="button" class="claude-codex-pro-issue-button" data-claude-codex-pro-issue="true">提出问题</button>
            </div>
          </div>
          <div class="claude-codex-pro-panel" data-claude-codex-pro-panel="recommendations" hidden>
            <div class="claude-codex-pro-ad-remote">
              ${renderClaudeCodexProAds()}
            </div>
          </div>
          <div class="claude-codex-pro-panel" data-claude-codex-pro-panel="support" hidden>
            <div class="claude-codex-pro-support-panel">
              <h3 class="claude-codex-pro-support-title">支持项目</h3>
              <p class="claude-codex-pro-support-text">如果这个工具帮到了你，可以通过下面的支付二维码支持后续维护。</p>
              ${claudeCodexProSupportPaymentQr
                ? `<div class="claude-codex-pro-support-qr-wrap"><img class="claude-codex-pro-support-qr" src="${escapeHtml(claudeCodexProSupportPaymentQr)}" alt="支付二维码"></div>`
                : `<div class="claude-codex-pro-support-empty">支付二维码未加载。</div>`}
            </div>
          </div>
          <div class="claude-codex-pro-panel" data-claude-codex-pro-panel="contact" hidden>
            <div class="claude-codex-pro-contact-panel">
              <h3 class="claude-codex-pro-contact-title">合作请联系微信</h3>
              <div class="claude-codex-pro-contact-card">
                <div class="claude-codex-pro-contact-line">
                  <span class="claude-codex-pro-contact-label">官方QQ群：</span>
                  <span class="claude-codex-pro-contact-group-number">10061615</span>
                  <a class="claude-codex-pro-contact-link" target="_blank" rel="noreferrer" href="${escapeHtml(claudeCodexProQqGroupPrimaryUrl)}">一键添加</a>
                  <span class="claude-codex-pro-contact-group-number">1076215359</span>
                  <a class="claude-codex-pro-contact-link" target="_blank" rel="noreferrer" href="${escapeHtml(claudeCodexProQqGroupSecondaryUrl)}">一键添加</a>
                </div>
                <div class="claude-codex-pro-contact-qr-wrap">
                  <p class="claude-codex-pro-contact-text">合作请联系微信</p>
                  ${claudeCodexProContactWechatQr
                    ? `<img class="claude-codex-pro-contact-qr" src="${escapeHtml(claudeCodexProContactWechatQr)}" alt="合作代理微信二维码">`
                    : `<div class="claude-codex-pro-support-empty">微信二维码未加载。</div>`}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
    const closeButton = overlay.querySelector(".claude-codex-pro-modal-close");
    closeButton?.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      overlay.remove();
    }, true);
    overlay.addEventListener("input", (event) => {
      const target = event.target instanceof Element ? event.target : event.target?.parentElement;
      const widthInput = target?.closest("[data-claude-codex-pro-conversation-view-width]");
      if (widthInput) setConversationViewWidth(widthInput.value);
    }, true);
    overlay.addEventListener("change", (event) => {
      const target = event.target instanceof Element ? event.target : event.target?.parentElement;
      const widthInput = target?.closest("[data-claude-codex-pro-conversation-view-width]");
      if (widthInput) {
        const width = normalizeConversationViewWidth(widthInput.value);
        widthInput.value = String(width || conversationViewWidth());
        setConversationViewWidth(widthInput.value);
      }
    }, true);
    overlay.addEventListener("click", (event) => {
      const target = event.target instanceof Element ? event.target : event.target?.parentElement;
      if (event.target === overlay || target?.closest(".claude-codex-pro-modal-close")) {
        overlay.remove();
        return;
      }
      const tabButton = target?.closest("[data-claude-codex-pro-tab]");
      if (tabButton) {
        selectClaudeCodexProTab(tabButton.getAttribute("data-claude-codex-pro-tab"));
        return;
      }
      if (target?.closest("[data-codex-open-devtools]")) {
        postJson("/devtools/open", {});
        return;
      }
      if (target?.closest("[data-claude-codex-pro-discord]")) {
        window.open("https://discord.gg/Q9cbMaWsb", "_blank");
        return;
      }
      if (target?.closest("[data-codex-backend-repair]")) {
        repairBackend();
        return;
      }
      const issueButton = target?.closest("[data-claude-codex-pro-issue]");
      if (issueButton) {
        const issueUrl = "https://github.com/DamonZS/Claude-Codex-Pro-Tool/issues";
        window.open(issueUrl, "_blank");
        return;
      }
      if (target?.closest("[data-codex-service-tier-inherit]")) {
        setCodexServiceTierControlMode("inherit");
        return;
      }
      if (target?.closest("[data-codex-service-tier-standard]")) {
        setCodexServiceTierControlMode("global-standard");
        return;
      }
      if (target?.closest("[data-codex-service-tier-fast]")) {
        setCodexServiceTierControlMode("global-fast");
        return;
      }
      if (target?.closest("[data-codex-service-tier-custom]")) {
        setCodexServiceTierControlMode("custom");
        return;
      }
      if (target?.closest("[data-codex-service-tier-thread-inherit]")) {
        setCodexThreadServiceTierMode("inherit");
        return;
      }
      if (target?.closest("[data-codex-service-tier-thread-standard]")) {
        setCodexThreadServiceTierMode("standard");
        return;
      }
      if (target?.closest("[data-codex-service-tier-thread-fast]")) {
        setCodexThreadServiceTierMode("fast");
        return;
      }
      if (target?.closest("[data-codex-upstream-worktree-open]")) {
        if (!claudeCodexProSettings().upstreamWorktreeCreate) {
          showToast("Upstream worktree enhancement is disabled", null);
          return;
        }
        openUpstreamWorktreeDialog();
        return;
      }
      const toggle = target?.closest("[data-claude-codex-pro-setting]");
      if (toggle) {
        if (toggle.disabled) return;
        const key = toggle.getAttribute("data-claude-codex-pro-setting");
        setClaudeCodexProSetting(key, !claudeCodexProConfiguredSettings()[key]);
        return;
      }
      const backendToggle = target?.closest("[data-codex-backend-setting]");
      if (backendToggle) {
        const key = backendToggle.getAttribute("data-codex-backend-setting");
        setBackendSetting(key, !claudeCodexProBackendSettings[key]);
        return;
      }
    }, true);
    document.body.appendChild(overlay);
    if (!claudeCodexProAdsLoaded) fetchClaudeCodexProAds();
    selectClaudeCodexProTab("home");
    renderClaudeCodexProMenu();
    refreshClaudeCodexProBackendToggles();
    renderBackendStatus();
    void loadCodexServiceTierState();
  }

  function findNativeMenuInsertionPoint() {
    if (!claudeCodexProSettings().nativeMenuPlacement) return null;
    const header = document.querySelector(selectors.appHeader);
    const menuBar = header?.querySelector(selectors.nativeMenuBar);
    if (menuBar) {
      const buttons = Array.from(menuBar.querySelectorAll("button")).filter((button) => !button.closest(`#${claudeCodexProMenuId}`));
      return { parent: menuBar, before: buttons[buttons.length - 1]?.nextSibling || null, nativeButtonClass: buttons[buttons.length - 1]?.className || "" };
    }
    const contextSurface = header?.querySelector(selectors.headerContextMenuSurface);
    const buttons = Array.from(contextSurface?.querySelectorAll?.("button") || [])
      .filter((button) => !button.closest(`#${claudeCodexProMenuId}`) && button.getBoundingClientRect().width > 0 && button.getBoundingClientRect().height > 0);
    const nativeButton = buttons.find((button) => !button.parentElement?.classList?.contains("inline-flex")) || buttons[0];
    const parent = nativeButton?.parentElement;
    if (!parent) return null;
    return { parent, before: nativeButton, nativeButtonClass: nativeButton.className || "" };
  }

  function removeDuplicateClaudeCodexProMenus(keep) {
    document.querySelectorAll(`#${claudeCodexProMenuId}, [data-claude-codex-pro-menu="true"]`).forEach((node) => {
      if (node !== keep) node.remove();
    });
    Array.from(document.querySelectorAll("button")).forEach((button) => {
      if ((button.textContent || "").trim() === `Claude Codex Pro ${claudeCodexProVersion}` && !button.closest(`#${claudeCodexProMenuId}`)) {
        button.remove();
      }
    });
  }

  function normalizeClaudeCodexProTriggerClassName(className) {
    const classes = String(className || "").split(/\s+/).filter(Boolean);
    const incompatibleNativeGroupClasses = new Set(["gap-0", "rounded-l-none", "border-l-0", "pl-0.5", "pr-1.5"]);
    const hasIncompatibleNativeGroupClass = classes.some((name) => incompatibleNativeGroupClasses.has(name));
    const normalized = classes.filter((name) => !incompatibleNativeGroupClasses.has(name));
    if (hasIncompatibleNativeGroupClass) {
      ["gap-1", "rounded-lg", "border-l", "px-2"].forEach((name) => {
        if (!normalized.includes(name)) normalized.push(name);
      });
    }
    return normalized.join(" ");
  }

  function configureClaudeCodexProTrigger(menu, trigger, nativeButtonClass) {
    if (!trigger) return;
    trigger.className = "claude-codex-pro-trigger";
    setClaudeCodexProTriggerLabel(trigger);
    if (trigger.dataset.claudeCodexProTriggerInstalled === claudeCodexProTriggerVersion) return;
    trigger.dataset.claudeCodexProTriggerInstalled = claudeCodexProTriggerVersion;
    trigger.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      openClaudeCodexProModal();
    }, true);
  }

  function visibleRectForCodexStatusAnchor(node, headerRect) {
    if (!(node instanceof Element) || node.closest?.(`#${claudeCodexProMenuId}`) || isExtensionUiNode(node)) return null;
    const rect = node.getBoundingClientRect();
    if (!(rect.width > 0 && rect.height > 0)) return null;
    if (rect.right < headerRect.left || rect.left > headerRect.right) return null;
    if (rect.bottom < headerRect.top || rect.top > headerRect.bottom) return null;
    if (!String(node.textContent || node.getAttribute("aria-label") || node.getAttribute("title") || "").trim() && !node.querySelector?.("svg")) return null;
    return rect;
  }

  function codexTitlebarControlLabel(node) {
    if (!(node instanceof Element)) return "";
    const raw = [
      node.textContent,
      node.getAttribute?.("aria-label"),
      node.getAttribute?.("title"),
      node.getAttribute?.("data-testid"),
    ]
      .filter(Boolean)
      .join(" ");
    return String(raw || "")
      .replace(/\s+/g, " ")
      .trim()
      .toLowerCase();
  }

  function codexWindowControlsOverlayAnchor(headerRect) {
    const windowControlsOverlay = navigator.windowControlsOverlay;
    if (!windowControlsOverlay?.getTitlebarAreaRect) return null;
    let overlayRect = null;
    try {
      overlayRect = windowControlsOverlay.getTitlebarAreaRect();
    } catch {
      return null;
    }
    if (!(overlayRect?.width > 0 && overlayRect.height > 0)) return null;
    if (!(overlayRect.right < window.innerWidth - 1)) return null;
    if (overlayRect.bottom < headerRect.top || overlayRect.top > headerRect.bottom) return null;
    if (overlayRect.right < headerRect.left + headerRect.width * 0.5 || overlayRect.right > headerRect.right) return null;
    return {
      node: null,
      label: "window-controls-overlay",
      rect: {
        left: overlayRect.right,
        right: window.innerWidth,
        top: overlayRect.top,
        bottom: overlayRect.bottom,
        width: window.innerWidth - overlayRect.right,
        height: overlayRect.height,
      },
    };
  }

  function findCodexStatusRightAnchor(header, headerRect) {
    const overlayAnchor = codexWindowControlsOverlayAnchor(headerRect);
    if (overlayAnchor) return overlayAnchor;
    const minimizeKeywords = ["minimize", "最小化"];
    const selector = [
      "button",
      "a",
      '[role="button"]',
      '[aria-label]',
      '[title]',
      '[data-testid]',
    ].join(",");
    const headerCandidates = Array.from(header?.querySelectorAll?.(selector) || []);
    const windowControlCandidates = Array.from(document.querySelectorAll?.('[aria-label], [title], [data-testid]') || [])
      .filter((node) => minimizeKeywords.some((keyword) => codexTitlebarControlLabel(node).includes(keyword)));
    const candidates = [...new Set([...windowControlCandidates, ...headerCandidates])];
    const entries = candidates
      .map((node) => ({ node, rect: visibleRectForCodexStatusAnchor(node, headerRect), label: codexTitlebarControlLabel(node) }))
      .filter((entry) => entry.rect && entry.rect.left >= headerRect.left + headerRect.width * 0.5);
    const minimize = entries
      .filter((entry) => minimizeKeywords.some((keyword) => entry.label.includes(keyword)))
      .sort((a, b) => a.rect.left - b.rect.left || a.rect.top - b.rect.top)[0];
    if (minimize) return minimize;
    const toolbarEntries = entries.filter((entry) => isHeaderToolbarButton(entry.node, header, entry.rect));
    return (toolbarEntries.length ? toolbarEntries : entries)
      .sort((a, b) => a.rect.left - b.rect.left || a.rect.top - b.rect.top)[0] || null;
  }

  function numericCssValue(value) {
    const parsed = Number.parseFloat(value || "");
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function setCssPropIfChanged(menu, prop, value) {
    if (menu.style.getPropertyValue(prop) !== value) {
      menu.style.setProperty(prop, value);
    }
  }

  function headerTitleRegion(header) {
    const candidates = Array.from(header?.querySelectorAll?.('[data-state], [class*="truncate"], [class*="text-base"]') || []);
    return candidates.find((node) => {
      if (!node?.querySelector?.('[data-state], button')) return false;
      if (!node.textContent?.trim()) return false;
      return node.closest?.(".draggable") || node.closest?.('[class*="grid-cols-[minmax(0,1fr)]"]');
    }) || null;
  }

  function isHeaderToolbarButton(button, header, rect) {
    if (!button || button.closest?.(`#${claudeCodexProMenuId}`)) return false;
    if (!(rect.width > 0 && rect.height > 0 && rect.left > window.innerWidth / 2)) return false;
    const buttonCluster = button.closest(".ms-auto.flex.shrink-0.items-center");
    if (buttonCluster && header?.contains(buttonCluster)) return true;
    const titleRegion = headerTitleRegion(header);
    if (titleRegion?.contains?.(button)) return false;
    return !!button.closest?.('[class*="ms-auto"][class*="shrink-0"][class*="items-center"]');
  }

  function updateFloatingClaudeCodexProMenuPosition(menu) {
    if (!menu?.classList?.contains(claudeCodexProMenuFloatingClass)) return;
    // Windows title-bar controls are outside the WebView. Anchor the injected
    // marker to the title-bar row, immediately left of native minimize.
    if (/Windows/i.test(navigator.userAgent || "")) {
      const menuWidth = menu.getBoundingClientRect().width || 168;
      const memoryBadge = document.getElementById(codexMemoryBadgeId);
      const memoryRect = memoryBadge?.getBoundingClientRect?.();
      const left = memoryRect && memoryRect.width > 0
        ? memoryRect.left - menuWidth - 8
        : window.innerWidth - menuWidth - 260;
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-top", "4px");
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-left", `${Math.max(8, left)}px`);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-height", "24px");
      updateCodexMemoryBadgePosition();
      return;
    }
    const header = document.querySelector(selectors.appHeader) || document.querySelector("header");
    if (!header) {
      const menuWidth = menu.getBoundingClientRect().width || 168;
      const fallbackLeft = Math.max(8, window.innerWidth - menuWidth - 16);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-top", "8px");
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-left", `${fallbackLeft}px`);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-height", "30px");
      return;
    }
    const headerRect = header.getBoundingClientRect();
    if (headerRect.height) {
      const anchor = findCodexStatusRightAnchor(header, headerRect);
      const anchorRect = anchor?.rect || null;
      const menuWidth = menu.getBoundingClientRect().width || 168;
      const minLeft = Math.max(8, headerRect.left + 8);
      const maxLeft = Math.max(minLeft, Math.min(window.innerWidth - menuWidth - 8, headerRect.right - menuWidth - 8));
      const anchorLeft = anchorRect ? anchorRect.left - menuWidth - 8 : headerRect.right - menuWidth - 16;
      const left = Math.max(minLeft, Math.min(anchorLeft, maxLeft));
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-top", `${headerRect.top}px`);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-left", `${left}px`);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-height", `${headerRect.height}px`);
    } else {
      const menuWidth = menu.getBoundingClientRect().width || 168;
      const fallbackLeft = Math.max(8, window.innerWidth - menuWidth - 16);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-top", "8px");
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-left", `${fallbackLeft}px`);
      setCssPropIfChanged(menu, "--claude-codex-pro-menu-height", "30px");
    }
    updateCodexMemoryBadgePosition();
  }

  function updateCodexMemoryBadgePosition() {
    const badge = document.getElementById(codexMemoryBadgeId);
    if (!badge) return;
    const minimize = Array.from(document.querySelectorAll("button, [role=button]"))
      .find((node) => /minimi[sz]e|最小化/i.test(`${node.getAttribute("aria-label") || ""} ${node.getAttribute("title") || ""} ${node.textContent || ""}`));
    const minimizeRect = minimize?.getBoundingClientRect?.();
    if (minimizeRect && minimizeRect.width > 0 && minimizeRect.height > 0) {
      const badgeWidth = badge.getBoundingClientRect().width || 150;
      badge.style.setProperty("--codex-memory-badge-left", `${Math.max(8, minimizeRect.left - badgeWidth - 8)}px`);
      badge.style.setProperty("--codex-memory-badge-right", "auto");
      badge.style.setProperty("--codex-memory-badge-top", `${minimizeRect.top}px`);
      badge.style.height = `${minimizeRect.height}px`;
      return;
    }
    // Windows title-bar controls are outside the WebView DOM. Keep the
    // injection marker in the title-bar row, immediately left of minimize.
    if (/Windows/i.test(navigator.userAgent || "")) {
      badge.style.setProperty("--codex-memory-badge-left", "auto");
      badge.style.setProperty("--codex-memory-badge-right", "104px");
      badge.style.setProperty("--codex-memory-badge-top", "4px");
      badge.style.height = "24px";
      return;
    }
    const statusMenu = document.getElementById(claudeCodexProMenuId);
    const statusRect = statusMenu?.getBoundingClientRect?.();
    if (statusRect && statusRect.width > 0 && statusRect.height > 0) {
      const badgeWidth = badge.getBoundingClientRect().width || 150;
      const left = Math.max(8, statusRect.left - badgeWidth - 8);
      badge.style.setProperty("--codex-memory-badge-left", `${left}px`);
      badge.style.setProperty("--codex-memory-badge-right", "auto");
      badge.style.setProperty("--codex-memory-badge-top", `${statusRect.top}px`);
      badge.style.height = `${statusRect.height}px`;
      return;
    }
    const badgeWidth = badge.getBoundingClientRect().width || 150;
    badge.style.setProperty("--codex-memory-badge-left", `${Math.max(8, window.innerWidth - badgeWidth - 192)}px`);
    badge.style.setProperty("--codex-memory-badge-right", "auto");
    badge.style.setProperty("--codex-memory-badge-top", "8px");
    badge.style.height = "30px";
  }

  function installClaudeCodexProMenu() {
    const existing = document.getElementById(claudeCodexProMenuId);
    removeDuplicateClaudeCodexProMenus(existing);
    let insertionPoint = findNativeMenuInsertionPoint();
    if (existing && existing.dataset.claudeCodexProMenuVersion !== claudeCodexProMenuVersion) {
      existing.remove();
      insertionPoint = findNativeMenuInsertionPoint();
    } else if (existing && insertionPoint && existing.parentElement === insertionPoint.parent) {
      configureClaudeCodexProTrigger(existing, existing.querySelector("button"), insertionPoint.nativeButtonClass);
      removeDuplicateClaudeCodexProMenus(existing);
      existing.className = claudeCodexProMenuFloatingClass;
      updateFloatingClaudeCodexProMenuPosition(existing);
      return;
    } else if (existing && insertionPoint) {
      configureClaudeCodexProTrigger(existing, existing.querySelector("button"), insertionPoint.nativeButtonClass);
      existing.className = claudeCodexProMenuFloatingClass;
      document.documentElement.appendChild(existing);
      updateFloatingClaudeCodexProMenuPosition(existing);
      removeDuplicateClaudeCodexProMenus(existing);
      return;
    }
    const menu = document.createElement("div");
    menu.id = claudeCodexProMenuId;
    menu.dataset.claudeCodexProMenu = "true";
    menu.dataset.claudeCodexProMenuVersion = claudeCodexProMenuVersion;
    const trigger = document.createElement("button");
    trigger.type = "button";
    const indicator = ensureClaudeCodexProTriggerIndicator(trigger);
    if (indicator) indicator.dataset.status = claudeCodexProBackendStatus.status || "checking";
    setClaudeCodexProTriggerLabel(trigger);
    const nativeButtonClass = insertionPoint?.nativeButtonClass || "claude-codex-pro-trigger";
    configureClaudeCodexProTrigger(menu, trigger, nativeButtonClass);
    menu.appendChild(trigger);
    menu.className = claudeCodexProMenuFloatingClass;
    document.documentElement.appendChild(menu);
    updateFloatingClaudeCodexProMenuPosition(menu);
    removeDuplicateClaudeCodexProMenus(menu);
  }

  function patchPluginMarketplaceRequestParams(method, params) {
    if (method === "list-plugins") {
      if (!params || typeof params !== "object") return params;
    } else {
      return params;
    }
    const next = { ...params };
    const hadMarketplaceKinds = Object.prototype.hasOwnProperty.call(next, "marketplaceKinds");
    if (hadMarketplaceKinds) delete next.marketplaceKinds;
    sendClaudeCodexProDiagnostic("plugin_marketplace_request_expanded", {
      hadMarketplaceKinds,
      cwdCount: Array.isArray(next.cwds) ? next.cwds.length : 0,
    });
    return next;
  }

  function pluginMarketplaceAliasForName(name) {
    if (name === "openai-bundled") return "";
    if (name === "openai-curated") return "claude-codex-pro-openai-curated";
    if (name === "openai-api-curated") return "claude-codex-pro-openai-api-curated";
    if (name === "openai-primary-runtime") return "claude-codex-pro-openai-primary-runtime";
    return "";
  }

  function displayNameForPluginMarketplaceName(name, fallback) {
    if (name === "openai-bundled" || name === "claude-codex-pro-openai-bundled") return "OpenAI插件1(Claude Codex Pro)";
    if (name === "openai-curated" || name === "claude-codex-pro-openai-curated") return "OpenAI插件2(Claude Codex Pro)";
    if (name === "openai-api-curated" || name === "claude-codex-pro-openai-api-curated") return "OpenAI插件2(Claude Codex Pro)";
    if (name === "openai-primary-runtime" || name === "claude-codex-pro-openai-primary-runtime") return "OpenAI插件3(Claude Codex Pro)";
    return fallback;
  }

  function patchPluginMarketplaceObject(marketplace) {
    if (!marketplace || typeof marketplace !== "object" || marketplace.__claudeCodexProMarketplaceUnlockPatched) return false;
    const alias = pluginMarketplaceAliasForName(marketplace.name);
    if (alias) marketplace.name = alias;
    const displayName = displayNameForPluginMarketplaceName(marketplace.name, marketplace.displayName || marketplace.title || marketplace.label || marketplace.name);
    if (!displayName || displayName === marketplace.name) return false;
    marketplace.displayName = displayName;
    marketplace.title = displayName;
    marketplace.label = displayName;
    if (marketplace.interface && typeof marketplace.interface === "object") {
      marketplace.interface = {
        ...marketplace.interface,
        displayName,
        name: displayName,
        title: displayName,
        label: displayName,
      };
    } else {
      marketplace.interface = { displayName, name: displayName, title: displayName, label: displayName };
    }
    marketplace.__claudeCodexProMarketplaceUnlockPatched = true;
    return true;
  }

  function restorePluginMarketplaceName(name) {
    if (name === "claude-codex-pro-openai-bundled") return "openai-bundled";
    if (name === "claude-codex-pro-openai-curated") return "openai-curated";
    if (name === "claude-codex-pro-openai-api-curated") return "openai-api-curated";
    if (name === "claude-codex-pro-openai-primary-runtime") return "openai-primary-runtime";
    return name;
  }

  function codexPluginOfficialMarketplaceName(name) {
    const restored = restorePluginMarketplaceName(name);
    return restored === "openai-bundled" || restored === "openai-curated" || restored === "openai-api-curated" || restored === "openai-primary-runtime";
  }

  function codexPluginMarketplaceRequestPatchStrategy() {
    const pluginStrategy = codexPluginUnlockStrategy();
    if (pluginStrategy === "legacy") return "none";
    const version = String(claudeCodexProBackendSettings.codexAppVersion || "").trim();
    const comparison = compareCodexVersions(version, codexPluginBridgeRequestUnlockFromVersion);
    if (comparison == null) return "unknown";
    return comparison >= 0 ? "bridge" : "client";
  }

  function cloneLocalPluginMarketplace(marketplace) {
    try {
      return JSON.parse(JSON.stringify(marketplace));
    } catch {
      return null;
    }
  }

  function localPluginMarketplaces() {
    const marketplaces = window.__CLAUDE_CODEX_PRO_PLUGIN_MARKETPLACES__;
    return Array.isArray(marketplaces) ? marketplaces : [];
  }

  function pluginMarketplaceKey(marketplace) {
    return restorePluginMarketplaceName(String(marketplace?.name || marketplace?.marketplaceName || ""));
  }

  function pluginKey(plugin, marketplaceName) {
    const rawId = String(plugin?.id || plugin?.pluginId || plugin?.name || "");
    const pluginName = rawId.includes("@") ? rawId.split("@")[0] : rawId;
    return pluginName + "@" + restorePluginMarketplaceName(String(plugin?.marketplaceName || marketplaceName || ""));
  }

  function prepareLocalPluginMarketplace(marketplace) {
    const next = cloneLocalPluginMarketplace(marketplace);
    if (!next || typeof next !== "object") return null;
    const marketplaceName = String(next.name || "");
    if (Array.isArray(next.plugins)) {
      next.plugins.forEach((plugin) => {
        if (plugin && typeof plugin === "object" && !plugin.marketplaceName) plugin.marketplaceName = marketplaceName;
      });
    }
    return next;
  }

  function mergeLocalPluginMarketplaces(result) {
    if (!result || typeof result !== "object" || !Array.isArray(result.marketplaces)) return 0;
    const locals = localPluginMarketplaces().map(prepareLocalPluginMarketplace).filter(Boolean);
    if (locals.length === 0) return 0;
    const byName = new Map(result.marketplaces.map((marketplace) => [pluginMarketplaceKey(marketplace), marketplace]));
    let mergedCount = 0;
    locals.forEach((local) => {
      const key = pluginMarketplaceKey(local);
      if (!key) return;
      const existing = byName.get(key);
      if (!existing) {
        result.marketplaces.push(local);
        byName.set(key, local);
        mergedCount += 1;
        return;
      }
      if (!Array.isArray(local.plugins)) return;
      if (!Array.isArray(existing.plugins)) existing.plugins = [];
      const existingPlugins = new Set(existing.plugins.map((plugin) => pluginKey(plugin, existing.name)));
      local.plugins.forEach((plugin) => {
        const key = pluginKey(plugin, local.name);
        if (key && !existingPlugins.has(key)) {
          existing.plugins.push(plugin);
          existingPlugins.add(key);
          mergedCount += 1;
        }
      });
    });
    if (mergedCount > 0) {
      sendClaudeCodexProDiagnostic("plugin_marketplace_local_merged", {
        marketplaceCount: locals.length,
        mergedCount,
      });
    }
    return mergedCount;
  }

  function pluginMarketplaceMatchesQuery(plugin, query) {
    const normalizedQuery = String(query || "").trim().toLowerCase();
    if (!normalizedQuery) return true;
    const tokens = normalizedQuery.split(/\s+/).filter(Boolean);
    const haystack = [
      plugin?.name,
      plugin?.title,
      plugin?.displayName,
      plugin?.description,
      plugin?.shortDescription,
      plugin?.id,
      plugin?.pluginId,
      plugin?.category,
      plugin?.marketplaceName,
      plugin?.interface?.displayName,
      plugin?.interface?.name,
      plugin?.interface?.title,
      plugin?.interface?.shortDescription,
      plugin?.interface?.longDescription,
      plugin?.interface?.description,
      plugin?.interface?.category,
      ...(Array.isArray(plugin?.keywords) ? plugin.keywords : []),
    ].map((value) => String(value || "").toLowerCase()).join(" ");
    return tokens.every((token) => haystack.includes(token));
  }

  function expandVisibleOfficialMarketplacePlugins(result) {
    if (!result || typeof result !== "object" || !Array.isArray(result.marketplaces)) return 0;
    if (!Array.isArray(result.plugins)) result.plugins = [];
    const query = String(result.query || result.search || result.filter || "").trim();
    const existing = new Set(result.plugins.map((plugin) => pluginKey(plugin, plugin?.marketplaceName)));
    let added = 0;
    result.marketplaces.forEach((marketplace) => {
      const marketplaceName = restorePluginMarketplaceName(String(marketplace?.name || ""));
      if (!codexPluginOfficialMarketplaceName(marketplaceName) || !Array.isArray(marketplace?.plugins)) return;
      marketplace.plugins.forEach((plugin) => {
        if (!plugin || typeof plugin !== "object") return;
        if (!plugin.marketplaceName) plugin.marketplaceName = marketplaceName;
        if (!pluginMarketplaceMatchesQuery(plugin, query)) return;
        const key = pluginKey(plugin, marketplaceName);
        if (!key || existing.has(key)) return;
        result.plugins.push(plugin);
        existing.add(key);
        added += 1;
      });
    });
    return added;
  }

  function isCodexPluginBuildFlavorFilter(callback, sample) {
    if (!Array.isArray(sample) || sample.length === 0 || typeof callback !== "function") return false;
    let source = "";
    try {
      source = Function.prototype.toString.call(callback);
    } catch {
      return false;
    }
    const isKnownFilterSource = source.includes("!u(e.marketplaceName)||e.marketplaceName===r")
      || source.includes("!ne(e.marketplaceName)||e.marketplaceName===n");
    if (!isKnownFilterSource) return false;
    if (!sample.some((plugin) => codexPluginOfficialMarketplaceName(plugin?.marketplaceName))) return false;
    return sample.some((plugin) => codexPluginOfficialMarketplaceName(plugin?.marketplaceName) && !callback(plugin));
  }

  function isCodexPluginMarketplaceHiddenFilter(callback, sample) {
    if (!Array.isArray(sample) || sample.length === 0 || typeof callback !== "function") return false;
    let source = "";
    try {
      source = Function.prototype.toString.call(callback);
    } catch {
      return false;
    }
    if (!source.includes("!t.includes(e.name)")) return false;
    if (!sample.some((marketplace) => codexPluginOfficialMarketplaceName(marketplace?.name))) return false;
    return sample.some((marketplace) => codexPluginOfficialMarketplaceName(marketplace?.name) && !callback(marketplace));
  }

  function installPluginBuildFlavorFilterPatch() {
    if (window.__codexPluginBuildFlavorFilterPatch === codexPluginMarketplaceUnlockVersion) return;
    if (pluginPatchDisabledInRelayMode()) return;
    if (!claudeCodexProSettings().pluginMarketplaceUnlock) return;
    const originalFilter = Array.prototype.__codexPluginBuildFlavorOriginalFilter || Array.prototype.filter;
    if (!Array.prototype.__codexPluginBuildFlavorOriginalFilter) {
      Object.defineProperty(Array.prototype, "__codexPluginBuildFlavorOriginalFilter", {
        value: originalFilter,
        configurable: true,
        writable: true,
      });
    }
    if (Array.prototype.filter.__codexPluginBuildFlavorPatched === codexPluginMarketplaceUnlockVersion) {
      window.__codexPluginBuildFlavorFilterPatch = codexPluginMarketplaceUnlockVersion;
      return;
    }
    const patchedFilter = function codexPluginBuildFlavorFilterPatch(callback, thisArg) {
      if (isCodexPluginBuildFlavorFilter(callback, this)) {
        sendClaudeCodexProDiagnostic("plugin_build_flavor_filter_bypassed", { pluginCount: this.length });
        return Array.from(this);
      }
      if (isCodexPluginMarketplaceHiddenFilter(callback, this)) {
        sendClaudeCodexProDiagnostic("plugin_marketplace_hidden_filter_bypassed", { marketplaceCount: this.length });
        return Array.from(this);
      }
      return originalFilter.call(this, callback, thisArg);
    };
    patchedFilter.__codexPluginBuildFlavorPatched = codexPluginMarketplaceUnlockVersion;
    Array.prototype.filter = patchedFilter;
    window.__codexPluginBuildFlavorFilterPatch = codexPluginMarketplaceUnlockVersion;
    sendClaudeCodexProDiagnostic("plugin_build_flavor_filter_patch_installed", {});
  }

  function restorePluginMarketplaceRequestParams(params, method = "") {
    if (!params || typeof params !== "object") return params;
    let next = params;
    if (Array.isArray(params.marketplaceKinds)) {
      const nextKinds = params.marketplaceKinds.map((kind) => {
        if (kind === "remote:openai-curated") return "openai-curated";
        return restorePluginMarketplaceName(kind);
      });
      next = { ...next, marketplaceKinds: Array.from(new Set(nextKinds)) };
    }
    if (method === "install-plugin") {
      next = next === params ? { ...params } : { ...next };
      if (next.remoteMarketplaceName) next.remoteMarketplaceName = restorePluginMarketplaceName(next.remoteMarketplaceName);
      if (typeof next.marketplacePath === "string" && next.marketplacePath.startsWith("remote:")) {
        const remoteMarketplaceName = next.marketplacePath.slice("remote:".length);
        delete next.marketplacePath;
        next.remoteMarketplaceName = restorePluginMarketplaceName(remoteMarketplaceName);
      }
    }
    return next;
  }

  function patchPluginMarketplaceResult(method, result) {
    if (method !== "list-plugins") return result;
    let patchedCount = 0;
    try {
      const pluginMarketplaceCounts = {};
      if (Array.isArray(result?.marketplaces)) {
        patchedCount += mergeLocalPluginMarketplaces(result);
        patchedCount += expandVisibleOfficialMarketplacePlugins(result);
        result.marketplaces.forEach((marketplace) => {
          if (Array.isArray(marketplace?.plugins)) {
            marketplace.plugins.forEach((plugin) => {
              if (plugin && typeof plugin === "object" && !plugin.marketplaceName) plugin.marketplaceName = marketplace?.name || "";
              const name = plugin?.marketplaceName || marketplace?.name || "";
              if (name) pluginMarketplaceCounts[name] = (pluginMarketplaceCounts[name] || 0) + 1;
            });
          }
          if (patchPluginMarketplaceObject(marketplace)) patchedCount += 1;
        });
        sendClaudeCodexProDiagnostic("plugin_marketplace_response_debug", {
          marketplaces: result.marketplaces.map((marketplace) => ({
            name: marketplace?.name || "",
            path: marketplace?.path || null,
            displayName: marketplace?.displayName || marketplace?.interface?.displayName || null,
            pluginCount: Array.isArray(marketplace?.plugins) ? marketplace.plugins.length : null,
            remoteMarketplaceName: marketplace?.remoteMarketplaceName || null,
          })),
          pluginMarketplaceCounts,
        });
      }
      if (patchedCount > 0) {
        sendClaudeCodexProDiagnostic("plugin_marketplace_response_expanded", { patchedCount });
      }
    } catch (error) {
      sendClaudeCodexProDiagnostic("plugin_marketplace_response_patch_failed", {
        errorName: error?.name || "",
        errorMessage: error?.message || String(error),
      });
    }
    return result;
  }

  function looksLikePluginMarketplaceResult(value) {
    if (!value || typeof value !== "object") return false;
    if (Array.isArray(value.marketplaces)) return true;
    if (Array.isArray(value.plugins)) return true;
    if (value.data && typeof value.data === "object") return looksLikePluginMarketplaceResult(value.data);
    return false;
  }

  function appServerModelRequestMethod(method, params) {
    if (method === "send-cli-request-for-host" && params?.method) return String(params.method);
    if (method === "vscode://codex/list-plugins") return "list-plugins";
    if (method === "vscode://codex/plugin/install") return "install-plugin";
    if (method === "vscode://codex/plugin/uninstall") return "uninstall-plugin";
    if (method === "plugin/list") return "list-plugins";
    if (method === "plugin/install") return "install-plugin";
    if (method === "plugin/uninstall") return "uninstall-plugin";
    return String(method || "");
  }

  function patchPluginMarketplaceRequestClient(client) {
    if (!client || typeof client.sendRequest !== "function") return false;
    if (client.__codexPluginMarketplaceUnlockPatch === codexPluginMarketplaceUnlockVersion) return true;
    const originalSendRequest = client.__codexPluginMarketplaceOriginalSendRequest || client.sendRequest.bind(client);
    client.__codexPluginMarketplaceOriginalSendRequest = originalSendRequest;
    client.sendRequest = async function codexPluginMarketplacePatchedSendRequest(method, params, options) {
      const requestMethod = appServerModelRequestMethod(String(method || ""), params);
      const requestParams = patchPluginMarketplaceRequestParams(requestMethod, restorePluginMarketplaceRequestParams(params, requestMethod));
      if (requestMethod === "install-plugin") {
        sendClaudeCodexProDiagnostic("plugin_install_request_debug", {
          method: String(method || ""),
          requestMethod,
          originalMarketplacePath: params?.marketplacePath || null,
          originalRemoteMarketplaceName: params?.remoteMarketplaceName || null,
          originalPluginName: params?.pluginName || null,
          requestMarketplacePath: requestParams?.marketplacePath || null,
          requestRemoteMarketplaceName: requestParams?.remoteMarketplaceName || null,
          requestPluginName: requestParams?.pluginName || null,
        });
      }
      try {
        const result = await originalSendRequest(method, requestParams, options);
        return patchPluginMarketplaceResult(requestMethod, result);
      } catch (error) {
        if (requestMethod === "install-plugin") {
          sendClaudeCodexProDiagnostic("plugin_install_request_failed", {
            method: String(method || ""),
            requestMethod,
            requestMarketplacePath: requestParams?.marketplacePath || null,
            requestRemoteMarketplaceName: requestParams?.remoteMarketplaceName || null,
            requestPluginName: requestParams?.pluginName || null,
            errorName: error?.name || "",
            errorMessage: error?.message || String(error),
          });
        }
        throw error;
      }
    };
    client.__codexPluginMarketplaceUnlockPatch = codexPluginMarketplaceUnlockVersion;
    return true;
  }

  function patchPluginMarketplaceRequestMessage(message) {
    if (!message || typeof message !== "object") return message;
    if (message.type === "fetch" && typeof message.url === "string") {
      const requestMethod = appServerModelRequestMethod(message.url, message.body);
      if (requestMethod !== "list-plugins" && requestMethod !== "install-plugin") return message;
      let requestBody = message.body;
      let params = null;
      if (typeof requestBody === "string" && requestBody.trim()) {
        try {
          params = JSON.parse(requestBody);
        } catch {
          params = null;
        }
      } else if (requestBody && typeof requestBody === "object") {
        params = requestBody;
      }
      const requestParams = patchPluginMarketplaceRequestParams(
        requestMethod,
        restorePluginMarketplaceRequestParams(params, requestMethod)
      );
      if (requestMethod === "list-plugins" && message.requestId != null) {
        window.__codexPluginMarketplaceFetchRequestIds = window.__codexPluginMarketplaceFetchRequestIds || new Set();
        window.__codexPluginMarketplaceFetchRequestIds.add(String(message.requestId));
      }
      if (requestParams === params) return message;
      if (requestMethod === "install-plugin") {
        sendClaudeCodexProDiagnostic("plugin_install_request_debug", {
          method: message.url,
          requestMethod,
          originalMarketplacePath: params?.marketplacePath || null,
          originalRemoteMarketplaceName: params?.remoteMarketplaceName || null,
          originalPluginName: params?.pluginName || null,
          requestMarketplacePath: requestParams?.marketplacePath || null,
          requestRemoteMarketplaceName: requestParams?.remoteMarketplaceName || null,
          requestPluginName: requestParams?.pluginName || null,
        });
      }
      return {
        ...message,
        body: typeof requestBody === "string" ? JSON.stringify(requestParams) : requestParams,
      };
    }
    if (message.type === "mcp-request" && message.request && typeof message.request === "object") {
      const requestMethod = appServerModelRequestMethod(String(message.request.method || ""), message.request.params);
      if (requestMethod !== "list-plugins" && requestMethod !== "install-plugin") return message;
      const requestParams = patchPluginMarketplaceRequestParams(
        requestMethod,
        restorePluginMarketplaceRequestParams(message.request.params, requestMethod)
      );
      if (requestMethod === "list-plugins" && message.request.id != null) {
        window.__codexPluginMarketplaceRequestIds = window.__codexPluginMarketplaceRequestIds || new Set();
        window.__codexPluginMarketplaceRequestIds.add(String(message.request.id));
      }
      if (requestParams === message.request.params) return message;
      if (requestMethod === "install-plugin") {
        sendClaudeCodexProDiagnostic("plugin_install_request_debug", {
          method: String(message.request.method || ""),
          requestMethod,
          originalMarketplacePath: message.request.params?.marketplacePath || null,
          originalRemoteMarketplaceName: message.request.params?.remoteMarketplaceName || null,
          originalPluginName: message.request.params?.pluginName || null,
          requestMarketplacePath: requestParams?.marketplacePath || null,
          requestRemoteMarketplaceName: requestParams?.remoteMarketplaceName || null,
          requestPluginName: requestParams?.pluginName || null,
        });
      }
      return { ...message, request: { ...message.request, params: requestParams } };
    }
    return message;
  }

  function patchPluginMarketplaceResponseData(data) {
    if (data?.type === "fetch-response") {
      const requestId = data.requestId != null ? String(data.requestId) : "";
      const requestIds = window.__codexPluginMarketplaceFetchRequestIds;
      if (requestIds instanceof Set && requestIds.size > 0) {
        if (!requestIds.has(requestId)) return false;
        requestIds.delete(requestId);
      }
      if (typeof data.bodyJsonString !== "string" || !data.bodyJsonString.trim()) return false;
      try {
        const result = JSON.parse(data.bodyJsonString);
        if (!looksLikePluginMarketplaceResult(result)) return false;
        if (result && typeof result === "object") {
          patchPluginMarketplaceResult("list-plugins", result);
          patchPluginMarketplaceResult("list-plugins", result.data);
        }
        data.bodyJsonString = JSON.stringify(result);
        return true;
      } catch (error) {
        sendClaudeCodexProDiagnostic("plugin_marketplace_fetch_response_patch_failed", {
          errorName: error?.name || "",
          errorMessage: error?.message || String(error),
        });
      }
      return false;
    }
    if (data?.type !== "mcp-response") return false;
    const message = data.message || data.response;
    const method = String(message?.method || data.method || "");
    if (appServerModelRequestMethod(method) === "install-plugin") {
      clearPluginMarketplaceQueryCache();
    }
    const requestId = message?.id != null ? String(message.id) : "";
    const requestIds = window.__codexPluginMarketplaceRequestIds;
    if (requestIds instanceof Set && requestIds.size > 0) {
      if (!requestIds.has(requestId)) return false;
      requestIds.delete(requestId);
    }
    const result = message?.result;
    if (!result || typeof result !== "object") return false;
    patchPluginMarketplaceResult("list-plugins", result);
    patchPluginMarketplaceResult("list-plugins", result.data);
    return true;
  }

  function clearPluginMarketplaceQueryCache() {
    try {
      const queryClient = window.__REACT_QUERY_CLIENT__ || window.__codexQueryClient;
      if (queryClient && typeof queryClient.invalidateQueries === "function") {
        queryClient.invalidateQueries({ queryKey: ["plugins"] });
      }
    } catch {
    }
  }

  function installPluginMarketplaceBridgePatch() {
    if (window.__codexPluginMarketplaceBridgePatch === codexPluginMarketplaceUnlockVersion) return;
    if (pluginPatchDisabledInRelayMode()) return;
    if (!claudeCodexProSettings().pluginMarketplaceUnlock) return;
    installPluginMarketplaceWindowEventPatchOnly();
    const bridge = window.electronBridge;
    if (!bridge || typeof bridge.sendMessageFromView !== "function") {
      sendClaudeCodexProDiagnostic("plugin_marketplace_bridge_patch_not_found", {});
      return;
    }
    if (!bridge.__codexPluginMarketplaceOriginalSendMessageFromView) {
      bridge.__codexPluginMarketplaceOriginalSendMessageFromView = bridge.sendMessageFromView.bind(bridge);
      bridge.sendMessageFromView = function claudeCodexProPluginMarketplacePatchedSendMessageFromView(message) {
        let nextMessage = message;
        try {
          nextMessage = patchPluginMarketplaceRequestMessage(message);
        } catch (error) {
          sendClaudeCodexProDiagnostic("plugin_marketplace_bridge_request_patch_failed", {
            errorName: error?.name || "",
            errorMessage: error?.message || String(error),
          });
        }
        return bridge.__codexPluginMarketplaceOriginalSendMessageFromView(nextMessage);
      };
    }
    bridge.__codexPluginMarketplaceBridgePatch = codexPluginMarketplaceUnlockVersion;
    window.__codexPluginMarketplaceBridgePatch = codexPluginMarketplaceUnlockVersion;
    sendClaudeCodexProDiagnostic("plugin_marketplace_bridge_patch_installed", {});
  }

  function installPluginMarketplaceWindowEventPatchOnly() {
    if (window.__codexPluginMarketplaceWindowEventPatch === codexPluginMarketplaceUnlockVersion) return;
    if (pluginPatchDisabledInRelayMode()) return;
    if (!claudeCodexProSettings().pluginMarketplaceUnlock) return;
    const originalDispatchEvent = window.__codexPluginMarketplaceOriginalDispatchEvent || window.dispatchEvent;
    if (!window.__codexPluginMarketplaceOriginalDispatchEvent) {
      window.__codexPluginMarketplaceOriginalDispatchEvent = originalDispatchEvent;
      window.dispatchEvent = function claudeCodexProPluginMarketplacePatchedDispatchEvent(event) {
        try {
          const detail = event?.detail;
          if (event?.type === "codex-message-from-view" && detail?.type === "mcp-request") {
            const patched = patchPluginMarketplaceRequestMessage(detail);
            if (patched !== detail) {
              Object.keys(detail).forEach((key) => delete detail[key]);
              Object.assign(detail, patched);
            }
          }
          if (event?.type === "message") patchPluginMarketplaceResponseData(event.data);
        } catch (error) {
          sendClaudeCodexProDiagnostic("plugin_marketplace_dispatch_event_patch_failed", {
            errorName: error?.name || "",
            errorMessage: error?.message || String(error),
          });
        }
        return originalDispatchEvent.call(this, event);
      };
    }
    if (!window.__codexPluginMarketplaceResponseListenerInstalled) {
      window.__codexPluginMarketplaceResponseListenerInstalled = true;
      window.addEventListener("message", (event) => {
        try {
          patchPluginMarketplaceResponseData(event?.data);
        } catch (error) {
          sendClaudeCodexProDiagnostic("plugin_marketplace_response_message_patch_failed", {
            errorName: error?.name || "",
            errorMessage: error?.message || String(error),
          });
        }
      }, true);
    }
    window.__codexPluginMarketplaceWindowEventPatch = codexPluginMarketplaceUnlockVersion;
  }

  function installPluginMarketplaceRequestPatch() {
    if (window.__codexPluginMarketplaceUnlockInstalled === codexPluginMarketplaceUnlockVersion) return;
    if (pluginPatchDisabledInRelayMode()) return;
    if (!claudeCodexProSettings().pluginMarketplaceUnlock) return;
    const patch = async () => {
      try {
        const module = await loadCodexAppModule("app-server-manager-signals-");
        const candidates = Object.values(module).filter((value) => value && typeof value === "object");
        let patchedCount = 0;
        for (const candidate of candidates) {
          if (patchPluginMarketplaceRequestClient(candidate)) patchedCount += 1;
          if (typeof candidate.sendRequest !== "function" && typeof candidate.get === "function") {
            try {
              if (patchPluginMarketplaceRequestClient(candidate.get())) patchedCount += 1;
            } catch {
            }
          }
        }
        if (patchedCount > 0) {
          window.__codexPluginMarketplaceUnlockInstalled = codexPluginMarketplaceUnlockVersion;
          sendClaudeCodexProDiagnostic("plugin_marketplace_request_patch_installed", {
            candidateCount: candidates.length,
            patchedCount,
          });
        } else {
          sendClaudeCodexProDiagnostic("plugin_marketplace_request_patch_not_found", {
            exportCount: Object.keys(module || {}).length,
            candidateCount: candidates.length,
          });
        }
      } catch (error) {
        sendClaudeCodexProDiagnostic("plugin_marketplace_request_patch_failed", {
          errorName: error?.name || "",
          errorMessage: error?.message || String(error),
        });
      }
    };
    void patch();
  }

  function reactFiberFrom(element) {
    const fiberKey = Object.keys(element).find((key) => key.startsWith("__reactFiber"));
    return fiberKey ? element[fiberKey] : null;
  }

  function authContextValueFrom(element) {
    for (let fiber = reactFiberFrom(element); fiber; fiber = fiber.return) {
      for (const value of [fiber.memoizedProps?.value, fiber.pendingProps?.value]) {
        if (value && typeof value === "object" && typeof value.setAuthMethod === "function" && "authMethod" in value) {
          return value;
        }
      }
    }
    return null;
  }

  function spoofChatGPTAuthMethod(element) {
    const auth = authContextValueFrom(element);
    if (!auth || auth.authMethod === "chatgpt") return false;
    auth.setAuthMethod("chatgpt");
    return true;
  }

  function multicaPluginButtonLabelMatches(button) {
    const normalize = (value) => String(value || "").replace(/\s+/g, " ").trim();
    return [
      button?.textContent,
      button?.getAttribute?.("aria-label"),
      button?.getAttribute?.("title"),
    ].map(normalize).some((value) => /^(插件|Plugins)(?:\s*[-|·:]\s*.*)?$/i.test(value));
  }

  function multicaPluginButtonLooksLike(button) {
    return !!button && button.nodeType === 1 && button.tagName === "BUTTON" &&
      !button?.dataset?.ccpMulticaNav &&
      (!!button.querySelector?.(selectors.pluginSvgPath) || multicaPluginButtonLabelMatches(button));
  }

  function isMulticaPluginAnchorButton(button) {
    return multicaPluginButtonLooksLike(button) &&
      !!button.closest?.(selectors.pluginAnchorRegion);
  }

  function multicaPluginAnchorMutationNode(node) {
    const element = node?.nodeType === 1 ? node : node?.parentElement;
    if (!element) return false;
    for (let current = element; current; current = current.parentElement) {
      if (isMulticaPluginAnchorButton(current)) return true;
    }
    // A removed plugin button is detached before MutationObserver receives the
    // record, so retain the signature check for that node only.
    if (!element.isConnected && multicaPluginButtonLooksLike(element)) return true;
    const inAnchorRegion = element.matches?.(selectors.pluginAnchorRegion) ||
      element.closest?.(selectors.pluginAnchorRegion);
    if (!inAnchorRegion && element.isConnected) return false;
    return Array.from(element.querySelectorAll?.("button") || []).some((button) =>
      isMulticaPluginAnchorButton(button) || (!button.isConnected && multicaPluginButtonLooksLike(button))
    );
  }

  function pluginEntryButton() {
    const candidates = Array.from(document.querySelectorAll(selectors.pluginAnchorButton))
      .filter(isMulticaPluginAnchorButton);
    const byIcon = candidates.find((button) => button.querySelector?.(selectors.pluginSvgPath));
    if (byIcon) return byIcon;
    return candidates.find(multicaPluginButtonLabelMatches) || null;
  }

  // The workspace is deliberately kept in this injection file instead of
  // taking over Codex's React tree. It owns one body-level host and only
  // snapshots/restores the native main surface while it is visible.
  const multicaWorkspaceModules = Object.freeze([
    { key: "my-issues", resource: "my_tasks", label: "我的任务" },
    { key: "issues", resource: "issues", label: "任务" },
    { key: "comments", resource: "comments", label: "评论" },
    { key: "labels", resource: "labels", label: "标签" },
    { key: "subscribers", resource: "subscribers", label: "订阅者" },
    { key: "reactions", resource: "reactions", label: "反应" },
    { key: "activities", resource: "activities", label: "活动" },
    { key: "projects", resource: "projects", label: "项目" },
    { key: "project-resources", resource: "project_resources", label: "项目资源" },
    { key: "autopilots", resource: "autopilots", label: "自动化" },
    { key: "agents", resource: "agents", label: "智能体" },
    { key: "squads", resource: "squads", label: "小队" },
    { key: "usage", resource: "statistics", label: "统计" },
    { key: "runtimes", resource: "runtimes", label: "运行时" },
    { key: "skills", resource: "skills", label: "Skills" },
    { key: "settings", resource: "settings", label: "设置" },
  ]);
  const multicaWorkspaceBoardColumns = Object.freeze([
    { key: "backlog", label: "待规划", tone: "neutral" },
    { key: "todo", label: "待办", tone: "neutral" },
    { key: "in_progress", label: "进行中", tone: "warning" },
    { key: "in_review", label: "审核中", tone: "success" },
    { key: "done", label: "已完成", tone: "info" },
    { key: "blocked", label: "已阻塞", tone: "danger" },
    { key: "cancelled", label: "已取消", tone: "muted" },
  ]);
  const multicaWorkspaceIssueFilters = Object.freeze([
    { key: "all", label: "全部" },
    { key: "assigned", label: "已分配" },
    { key: "created", label: "我创建的" },
    { key: "agents", label: "我的智能体和小队" },
    { key: "working", label: "工作中" },
  ]);
  const multicaWorkspaceBackgroundIntervalMs = 5000;
  // Background refreshes must use the same bounded budget as the foreground
  // preflight. A shorter timeout made normal CDP/IPC jitter look like a lost
  // connection and overwrote a previously healthy entry state.
  const multicaWorkspaceBackgroundTimeoutMs = 15000;
  const multicaWorkspaceSavedViewsKey = "claudeCodexProMulticaSavedIssueViews";
  const multicaWorkspaceState = {
    entry: null,
    host: null,
    shadow: null,
    root: null,
    main: null,
    mainSnapshot: null,
    mainResizeObserver: null,
    navHandler: null,
    anchorTimer: null,
    anchorAttempts: 0,
    anchorDiagnosticSent: false,
    opened: false,
    opening: false,
    openSeq: 0,
    entryAvailabilityMessage: "",
    route: "my-issues",
    issueFilter: "assigned",
    boardCompact: false,
    issueViewMode: "board",
    savedIssueViews: [],
    activeIssueViewId: "",
    moduleMenuOpen: false,
    draggedIssue: null,
    nativeThreadActivation: false,
    backgroundTimer: null,
    backgroundStarted: false,
    backgroundBusy: false,
    workspaceId: "",
    bootstrap: null,
    bootstrapLoading: false,
    bootstrapError: "",
    querySeq: 0,
    bootstrapSeq: 0,
    queryRequest: null,
    bootstrapRequest: null,
    activeRequests: new Set(),
    loading: new Set(),
    issueFilterDependencies: {
      loading: false,
      error: "",
      sequence: 0,
    },
    collections: new Map(),
    errors: new Map(),
    editor: null,
    mutationNotice: null,
    mutationBusy: false,
    executions: [],
    executionsLoading: false,
    executionsError: "",
    executionSeq: 0,
    executionDraft: null,
    executionBusy: new Set(),
    executionNotice: null,
    selectedSkillIds: new Set(),
    skillResolution: null,
    skillReview: null,
    skillBindings: [],
    skillBindingsLoading: false,
    skillBindingsError: "",
    skillBindingDraft: null,
    settingsSave: null,
  };
  const multicaWorkspaceVersion = "2";

  function multicaWorkspaceLoadSavedIssueViews() {
    try {
      const parsed = JSON.parse(localStorage.getItem(multicaWorkspaceSavedViewsKey) || "[]");
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((view) => view && typeof view === "object")
        .map((view) => ({
          id: String(view.id || "").trim(),
          name: String(view.name || "").trim().slice(0, 80),
          scope: multicaWorkspaceIssueFilters.some((filter) => filter.key === view.scope) ? view.scope : "assigned",
          issueViewMode: ["board", "list", "table", "swimlane"].includes(view.issueViewMode) ? view.issueViewMode : "board",
          boardCompact: view.boardCompact === true,
          revision: Number(view.revision) > 0 ? Number(view.revision) : 1,
        }))
        .filter((view) => view.id && view.name);
    } catch (_) {
      return [];
    }
  }

  function multicaWorkspaceWriteSavedIssueViews() {
    try {
      localStorage.setItem(multicaWorkspaceSavedViewsKey, JSON.stringify(multicaWorkspaceState.savedIssueViews));
      return true;
    } catch (_) {
      return false;
    }
  }

  async function multicaWorkspaceSaveCurrentIssueView() {
    const name = String(window.prompt("保存本机视图名称", "我的任务视图") || "").trim().slice(0, 80);
    if (!name) return;
    const view = {
      id: `local-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name,
      scope: multicaWorkspaceState.issueFilter,
      issueViewMode: multicaWorkspaceState.issueViewMode,
      boardCompact: multicaWorkspaceState.boardCompact === true,
      revision: 1,
    };
    multicaWorkspaceState.savedIssueViews = [
      ...multicaWorkspaceState.savedIssueViews.filter((item) => item.name !== name),
      view,
    ];
    multicaWorkspaceState.activeIssueViewId = view.id;
    let backendSaved = false;
    if (multicaWorkspaceState.workspaceId) {
      try {
        const backendView = {
          id: view.id,
          workspace_id: multicaWorkspaceState.workspaceId,
          name: view.name,
          scope_type: "my",
          scope_variant: view.scope === "working" ? "assigned" : view.scope,
          visibility: "private",
          definition_version: 1,
          query: { local_filter: view.scope },
          display: { issue_view_mode: view.issueViewMode, board_compact: view.boardCompact },
          revision: 1,
        };
        await multicaWorkspaceCall("/multica/workspace/upsert", {
          resource: "issue_views",
          entity: backendView,
        });
        backendSaved = true;
      } catch (_) {
        // The local cache remains usable when an older binary lacks the new
        // resource; do not claim remote/server synchronization.
      }
    }
    if (!multicaWorkspaceWriteSavedIssueViews()) {
      multicaWorkspaceState.mutationNotice = { state: "error", message: "本机视图保存失败，请检查浏览器存储空间" };
    } else {
      multicaWorkspaceState.mutationNotice = { state: "ok", message: `本机视图“${name}”已保存${backendSaved ? "（控制面已记录）" : ""}` };
    }
    multicaWorkspaceRenderContent();
  }

  async function multicaWorkspaceLoadSavedIssueViewsFromControlPlane() {
    if (!multicaWorkspaceState.workspaceId) return;
    try {
      const result = await multicaWorkspaceCall("/multica/workspace/query", {
        resource: "issue_views",
        limit: 100,
        offset: 0,
      });
      const items = Array.isArray(result?.collection?.items)
        ? result.collection.items
        : Array.isArray(result?.items) ? result.items : [];
      const views = items.map((item) => {
        const display = item.display && typeof item.display === "object" ? item.display : {};
        const query = item.query && typeof item.query === "object" ? item.query : {};
        return {
          id: String(item.id || "").trim(),
          name: String(item.name || "").trim().slice(0, 80),
          scope: ["all", "assigned", "created", "agents", "working"].includes(query.local_filter) ? query.local_filter : (item.scope_variant || "assigned"),
          issueViewMode: ["board", "list", "table", "swimlane"].includes(display.issue_view_mode) ? display.issue_view_mode : "board",
          boardCompact: display.board_compact === true,
          revision: Number(item.revision) > 0 ? Number(item.revision) : 1,
        };
      }).filter((view) => view.id && view.name);
      if (views.length > 0) {
        multicaWorkspaceState.savedIssueViews = views;
        multicaWorkspaceWriteSavedIssueViews();
        if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
      }
    } catch (_) {
      // Older builds may not expose issue_views; local cache remains valid.
    }
  }

  function multicaWorkspaceApplySavedIssueView(viewId) {
    const view = multicaWorkspaceState.savedIssueViews.find((item) => item.id === viewId);
    if (!view) return;
    multicaWorkspaceState.activeIssueViewId = view.id;
    multicaWorkspaceState.issueFilter = view.scope;
    multicaWorkspaceState.issueViewMode = view.issueViewMode;
    multicaWorkspaceState.boardCompact = view.boardCompact;
    multicaWorkspaceRenderContent();
    multicaWorkspaceRefreshBoardSource(false);
  }

  async function multicaWorkspaceDeleteActiveIssueView() {
    const view = multicaWorkspaceState.savedIssueViews.find((item) => item.id === multicaWorkspaceState.activeIssueViewId);
    if (!view || !window.confirm(`删除本机视图“${view.name}”？`)) return;
    if (multicaWorkspaceState.workspaceId) {
      try {
        await multicaWorkspaceCall("/multica/workspace/delete", {
          resource: "issue_views",
          entityId: view.id,
          expectedRevision: view.revision || 1,
        });
      } catch (_) {
        multicaWorkspaceState.mutationNotice = { state: "error", message: "控制面删除失败，已保留本机视图" };
        multicaWorkspaceRenderContent();
        return;
      }
    }
    multicaWorkspaceState.savedIssueViews = multicaWorkspaceState.savedIssueViews.filter((item) => item.id !== view.id);
    multicaWorkspaceState.activeIssueViewId = "";
    multicaWorkspaceWriteSavedIssueViews();
    multicaWorkspaceState.mutationNotice = { state: "ok", message: `本机视图“${view.name}”已删除` };
    multicaWorkspaceRenderContent();
  }

  multicaWorkspaceState.savedIssueViews = multicaWorkspaceLoadSavedIssueViews();

  function multicaWorkspaceEl(tag, className, text) {
    const element = document.createElement(tag);
    if (className) element.className = className;
    if (text !== undefined) element.textContent = String(text);
    return element;
  }

  function multicaWorkspaceClear(element) {
    if (!element) return;
    while (element.firstChild) element.removeChild(element.firstChild);
  }

  function multicaWorkspaceVisible(element) {
    if (!element?.isConnected) return false;
    const rect = element.getBoundingClientRect?.();
    if (!rect || rect.width <= 0 || rect.height <= 0) return false;
    if (typeof getComputedStyle !== "function") return true;
    const style = getComputedStyle(element);
    return style.display !== "none" && style.visibility !== "hidden";
  }

  function multicaWorkspaceNativeMain(pluginButton) {
    // The native main is intentionally hidden while the workspace is open.
    // Reuse the bound node before checking visibility so a scan cannot lose
    // the anchor simply because our own overlay made it invisible.
    if (multicaWorkspaceState.opened && multicaWorkspaceState.main?.isConnected) {
      return multicaWorkspaceState.main;
    }
    const shellParent = pluginButton?.closest?.("aside")?.parentElement;
    const directMain = Array.from(shellParent?.children || [])
      .find((child) => child?.tagName?.toLowerCase?.() === "main");
    if (directMain && (multicaWorkspaceVisible(directMain) || directMain === multicaWorkspaceState.main)) return directMain;
    const candidates = Array.from(document.querySelectorAll("main"))
      .filter((main) => !isExtensionUiNode(main) && multicaWorkspaceVisible(main));
    candidates.sort((left, right) => {
      const a = left.getBoundingClientRect();
      const b = right.getBoundingClientRect();
      return (b.width * b.height) - (a.width * a.height);
    });
    return candidates[0] || null;
  }

  function multicaWorkspaceCaptureMain(main) {
    if (!main || !main.style) return null;
    return {
      main,
      visibility: main.style.visibility,
      pointerEvents: main.style.pointerEvents,
      inertAttribute: main.getAttribute?.("inert"),
      ariaHidden: main.getAttribute?.("aria-hidden"),
      inertProperty: "inert" in main ? !!main.inert : null,
    };
  }

  function multicaWorkspaceRestoreMain() {
    const snapshot = multicaWorkspaceState.mainSnapshot;
    const main = snapshot?.main;
    if (!main?.style) return;
    main.style.visibility = snapshot.visibility;
    main.style.pointerEvents = snapshot.pointerEvents;
    if (snapshot.inertAttribute === null || snapshot.inertAttribute === undefined) {
      main.removeAttribute?.("inert");
    } else {
      main.setAttribute?.("inert", snapshot.inertAttribute);
    }
    if (snapshot.ariaHidden === null || snapshot.ariaHidden === undefined) {
      main.removeAttribute?.("aria-hidden");
    } else {
      main.setAttribute?.("aria-hidden", snapshot.ariaHidden);
    }
    if (snapshot.inertProperty !== null && "inert" in main) {
      try { main.inert = snapshot.inertProperty; } catch (_) {}
    }
  }

  function multicaWorkspaceBindMain(main) {
    if (!main) return false;
    if (multicaWorkspaceState.main === main && multicaWorkspaceState.mainSnapshot) return true;
    multicaWorkspaceRestoreMain();
    multicaWorkspaceState.main = main;
    multicaWorkspaceState.mainSnapshot = multicaWorkspaceCaptureMain(main);
    if (multicaWorkspaceState.mainResizeObserver) {
      try { multicaWorkspaceState.mainResizeObserver.disconnect(); } catch (_) {}
    }
    multicaWorkspaceState.mainResizeObserver = null;
    if (typeof ResizeObserver === "function") {
      multicaWorkspaceState.mainResizeObserver = new ResizeObserver(() => multicaWorkspaceUpdateGeometry());
      multicaWorkspaceState.mainResizeObserver.observe(main);
    }
    if (multicaWorkspaceState.opened) {
      main.style.visibility = "hidden";
      main.style.pointerEvents = "none";
      main.setAttribute?.("inert", "");
      main.setAttribute?.("aria-hidden", "true");
    }
    return true;
  }

  function multicaWorkspaceUpdateGeometry() {
    const host = multicaWorkspaceState.host;
    const main = multicaWorkspaceState.main;
    if (!host || !main || !main.getBoundingClientRect) return;
    const rect = main.getBoundingClientRect();
    const width = Math.max(0, Math.round(rect.width));
    const height = Math.max(0, Math.round(rect.height));
    host.style.left = `${Math.round(rect.left)}px`;
    host.style.top = `${Math.round(rect.top)}px`;
    host.style.width = `${width}px`;
    host.style.height = `${height}px`;
    host.style.display = multicaWorkspaceState.opened && width > 0 && height > 0 ? "block" : "none";
    if (typeof getComputedStyle === "function") {
      const style = getComputedStyle(main);
      host.style.setProperty("--ccp-multica-bg", style.backgroundColor || "#181b1a");
      host.style.setProperty("--ccp-multica-fg", style.color || "#f2f5f3");
    }
  }

  function multicaWorkspaceInstallShadow(host) {
    if (!host?.attachShadow) return null;
    const shadow = host.attachShadow({ mode: "open" });
    const style = multicaWorkspaceEl("style");
    style.textContent = `
      :host { color-scheme: light dark; }
      .ccp-multica-shell {
        box-sizing: border-box; width: 100%; height: 100%; min-width: 280px;
        display: flex; flex-direction: column; overflow: hidden;
        background: var(--ccp-multica-bg, #181b1a); color: var(--ccp-multica-fg, #f2f5f3);
        font: 13px/1.4 system-ui, -apple-system, "Segoe UI", sans-serif;
      }
      .ccp-multica-button { min-height: 30px; border: 1px solid color-mix(in srgb, currentColor 20%, transparent); border-radius: 6px; padding: 4px 10px; background: color-mix(in srgb, currentColor 7%, transparent); color: inherit; font: inherit; cursor: pointer; }
      .ccp-multica-button:hover, .ccp-multica-button:focus-visible { background: color-mix(in srgb, #4fb995 20%, transparent); outline: none; }
      .ccp-multica-button:disabled { cursor: wait; opacity: .55; }
      .ccp-multica-button[data-variant="primary"] { border-color: color-mix(in srgb, #4fb995 70%, transparent); background: color-mix(in srgb, #4fb995 22%, transparent); }
      .ccp-multica-button[data-variant="danger"] { border-color: color-mix(in srgb, #e17d83 58%, transparent); color: #e17d83; }
      .ccp-multica-button:focus-visible, .ccp-multica-module-item:focus-visible, .ccp-multica-filter:focus-visible, .ccp-multica-icon-button:focus-visible, .ccp-multica-card:focus-visible { box-shadow: 0 0 0 2px #4fb995; outline: none; }
      .ccp-multica-content { box-sizing: border-box; width: 100%; min-width: 0; min-height: 0; flex: 1; overflow: auto; padding: 16px 18px 24px; }
      .ccp-multica-content[data-route="my-issues"] { display: flex; overflow: hidden; padding: 0; }
      .ccp-multica-content-header { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
      .ccp-multica-content-title { min-width: 0; flex: 1; margin: 0; font-size: 15px; font-weight: 620; letter-spacing: 0; }
      .ccp-multica-count { color: color-mix(in srgb, currentColor 58%, transparent); white-space: nowrap; }
      .ccp-multica-toolbar { position: relative; display: flex; align-items: center; flex-wrap: wrap; gap: 8px; min-width: 0; }
      .ccp-multica-toolbar-group { display: inline-flex; align-items: center; gap: 5px; min-width: 0; }
      .ccp-multica-toolbar-spacer { min-width: 10px; flex: 1 1 auto; }
      .ccp-multica-filter, .ccp-multica-icon-button { box-sizing: border-box; min-height: 30px; border: 1px solid color-mix(in srgb, currentColor 18%, transparent); border-radius: 6px; padding: 4px 9px; background: color-mix(in srgb, currentColor 5%, transparent); color: color-mix(in srgb, currentColor 70%, transparent); font: inherit; cursor: pointer; white-space: nowrap; }
      .ccp-multica-filter:hover, .ccp-multica-icon-button:hover { background: color-mix(in srgb, currentColor 9%, transparent); color: inherit; }
      .ccp-multica-filter[aria-pressed="true"], .ccp-multica-icon-button[aria-pressed="true"] { border-color: color-mix(in srgb, #4fb995 55%, transparent); background: color-mix(in srgb, #4fb995 16%, transparent); color: inherit; }
      .ccp-multica-icon-button { display: inline-flex; align-items: center; justify-content: center; min-width: 30px; padding-inline: 8px; }
      .ccp-multica-module-menu { position: relative; }
      .ccp-multica-module-popover { position: absolute; z-index: 8; top: calc(100% + 6px); right: 0; width: min(220px, calc(100vw - 36px)); max-height: min(420px, calc(100vh - 150px)); overflow: auto; border: 1px solid color-mix(in srgb, currentColor 20%, transparent); border-radius: 7px; padding: 6px; background: var(--ccp-multica-bg, #181b1a); box-shadow: 0 12px 30px color-mix(in srgb, #000 35%, transparent); }
      .ccp-multica-module-item { display: flex; align-items: center; width: 100%; min-height: 34px; border: 0; border-radius: 5px; padding: 6px 9px; background: transparent; color: inherit; font: inherit; text-align: left; cursor: pointer; }
      .ccp-multica-module-item:hover { background: color-mix(in srgb, currentColor 9%, transparent); }
      .ccp-multica-module-item[aria-current="page"] { background: color-mix(in srgb, #4fb995 17%, transparent); }
      .ccp-multica-board-page { display: flex; flex: 1; flex-direction: column; min-width: 0; min-height: 0; overflow: hidden; }
      .ccp-multica-board-heading { display: flex; align-items: center; gap: 9px; min-height: 50px; padding: 8px 14px; border-bottom: 1px solid color-mix(in srgb, currentColor 14%, transparent); }
      .ccp-multica-board-title { margin: 0; font-size: 15px; font-weight: 650; letter-spacing: 0; white-space: nowrap; }
      .ccp-multica-board-toolbar { padding: 8px 12px; border-bottom: 1px solid color-mix(in srgb, currentColor 12%, transparent); }
      .ccp-multica-native-inventory { display: grid; grid-template-columns: minmax(0, 2fr) minmax(220px, 1fr); gap: 12px; padding: 10px 12px; border-bottom: 1px solid color-mix(in srgb, currentColor 12%, transparent); }
      .ccp-multica-native-inventory-title { grid-column: 1 / -1; margin: 0; font-size: 13px; }
      .ccp-multica-native-inventory-label { margin: 0 0 6px; font-size: 12px; opacity: .72; }
      .ccp-multica-native-session-list, .ccp-multica-native-agent-list { display: flex; flex-wrap: wrap; gap: 6px; }
      .ccp-multica-native-session, .ccp-multica-native-agent { max-width: 260px; padding: 4px 8px; border: 1px solid color-mix(in srgb, currentColor 16%, transparent); border-radius: 6px; background: transparent; color: inherit; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      .ccp-multica-native-session { cursor: pointer; text-align: left; }
      .ccp-multica-native-session[aria-current="page"] { border-color: #56b9a6; }
      .ccp-multica-working-count { color: color-mix(in srgb, currentColor 66%, transparent); white-space: nowrap; }
      .ccp-multica-board-scroll { min-width: 0; min-height: 0; flex: 1; overflow-x: auto; overflow-y: hidden; scrollbar-gutter: stable; }
      .ccp-multica-board { box-sizing: border-box; display: grid; grid-template-columns: repeat(7, 280px); gap: 12px; width: max-content; min-width: 100%; height: 100%; min-height: 360px; padding: 12px; }
      .ccp-multica-board-column { box-sizing: border-box; display: flex; flex-direction: column; width: 280px; min-width: 280px; min-height: 0; border: 1px solid color-mix(in srgb, currentColor 10%, transparent); border-radius: 7px; background: color-mix(in srgb, currentColor 4%, transparent); overflow: hidden; }
      .ccp-multica-board-column[data-tone="warning"] { background: color-mix(in srgb, #b78923 8%, var(--ccp-multica-bg, #181b1a)); }
      .ccp-multica-board-column[data-tone="success"] { background: color-mix(in srgb, #2f9a68 7%, var(--ccp-multica-bg, #181b1a)); }
      .ccp-multica-board-column[data-tone="info"] { background: color-mix(in srgb, #2f79a8 8%, var(--ccp-multica-bg, #181b1a)); }
      .ccp-multica-board-column[data-tone="danger"] { background: color-mix(in srgb, #a24a5a 8%, var(--ccp-multica-bg, #181b1a)); }
      .ccp-multica-board-column[data-tone="muted"] { background: color-mix(in srgb, currentColor 3%, var(--ccp-multica-bg, #181b1a)); }
      .ccp-multica-column-header { display: flex; align-items: center; gap: 7px; min-height: 42px; padding: 7px 10px; }
      .ccp-multica-column-dot { width: 8px; height: 8px; flex: 0 0 8px; border: 1px solid currentColor; border-radius: 50%; color: color-mix(in srgb, currentColor 64%, transparent); }
      .ccp-multica-column-title { min-width: 0; flex: 1; margin: 0; font-size: 13px; font-weight: 620; letter-spacing: 0; }
      .ccp-multica-column-count { color: color-mix(in srgb, currentColor 58%, transparent); }
      .ccp-multica-column-actions { display: inline-flex; align-items: center; gap: 2px; }
      .ccp-multica-column-actions .ccp-multica-icon-button { min-width: 26px; min-height: 26px; border-color: transparent; padding: 2px 5px; background: transparent; }
      .ccp-multica-column-list { min-height: 0; flex: 1; overflow-y: auto; padding: 4px 8px 10px; }
      .ccp-multica-column-empty { display: flex; align-items: center; justify-content: center; min-height: 120px; color: color-mix(in srgb, currentColor 54%, transparent); }
      .ccp-multica-card { box-sizing: border-box; display: grid; gap: 8px; width: 100%; min-width: 0; margin-bottom: 8px; border: 1px solid color-mix(in srgb, currentColor 18%, transparent); border-radius: 7px; padding: 10px; background: color-mix(in srgb, currentColor 6%, var(--ccp-multica-bg, #181b1a)); color: inherit; }
      .ccp-multica-card[draggable="true"] { cursor: grab; }
      .ccp-multica-card[data-dragging="true"] { opacity: .48; }
      .ccp-multica-card-id { color: color-mix(in srgb, currentColor 58%, transparent); font-size: 11px; overflow-wrap: anywhere; }
      .ccp-multica-card-title { margin: 0; font-size: 13px; font-weight: 620; letter-spacing: 0; overflow-wrap: anywhere; }
      .ccp-multica-card-summary { display: -webkit-box; overflow: hidden; color: color-mix(in srgb, currentColor 64%, transparent); -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow-wrap: anywhere; }
      .ccp-multica-card-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 5px 9px; color: color-mix(in srgb, currentColor 58%, transparent); font-size: 11px; }
      .ccp-multica-card-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 5px; }
      .ccp-multica-card-actions .ccp-multica-button { min-height: 26px; padding: 2px 7px; font-size: 12px; }
      .ccp-multica-board-page[data-compact="true"] .ccp-multica-card-summary { display: none; }
      .ccp-multica-issue-list, .ccp-multica-table, .ccp-multica-swimlane { display: grid; gap: 8px; min-width: 0; padding: 12px; }
      .ccp-multica-issue-list-row { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 12px; min-width: 0; padding: 10px 12px; border: 1px solid color-mix(in srgb, currentColor 14%, transparent); border-radius: 6px; }
      .ccp-multica-issue-list-title { min-width: min(240px, 100%); flex: 1 1 240px; overflow-wrap: anywhere; }
      .ccp-multica-table { overflow-x: auto; }
      .ccp-multica-table-row { display: grid; grid-template-columns: minmax(220px, 2fr) 110px 90px minmax(120px, 1fr) minmax(140px, 1fr) 140px; gap: 10px; min-width: 850px; align-items: center; padding: 9px 10px; border-bottom: 1px solid color-mix(in srgb, currentColor 12%, transparent); }
      .ccp-multica-table-header { position: sticky; top: 0; border-bottom-width: 2px; color: color-mix(in srgb, currentColor 65%, transparent); font-size: 12px; font-weight: 600; background: var(--ccp-multica-bg, #181b1a); }
      .ccp-multica-table-cell { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      .ccp-multica-swimlane { grid-template-columns: repeat(2, minmax(280px, 1fr)); align-items: start; }
      .ccp-multica-swimlane-lane { min-width: 0; border: 1px solid color-mix(in srgb, currentColor 13%, transparent); border-radius: 6px; padding: 10px; }
      .ccp-multica-state { padding: 30px 8px; color: color-mix(in srgb, currentColor 66%, transparent); text-align: center; }
      .ccp-multica-state strong { display: block; margin-bottom: 8px; color: inherit; font-weight: 600; }
      .ccp-multica-list { display: grid; gap: 0; }
      .ccp-multica-item { min-width: 0; padding: 12px 0; border-top: 1px solid color-mix(in srgb, currentColor 13%, transparent); }
      .ccp-multica-item:first-child { border-top: 0; padding-top: 0; }
      .ccp-multica-item-heading { display: flex; align-items: flex-start; gap: 10px; min-width: 0; }
      .ccp-multica-item-heading .ccp-multica-item-title { min-width: 0; flex: 1; }
      .ccp-multica-item-actions { display: inline-flex; flex: 0 0 auto; flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
      .ccp-multica-badge { display: inline-flex; align-items: center; min-height: 22px; border: 1px solid color-mix(in srgb, currentColor 18%, transparent); border-radius: 5px; padding: 1px 7px; color: color-mix(in srgb, currentColor 70%, transparent); font-size: 12px; }
      .ccp-multica-form { display: grid; gap: 12px; margin: 0 0 18px; padding: 14px 0 18px; border-bottom: 1px solid color-mix(in srgb, currentColor 15%, transparent); }
      .ccp-multica-form-title { margin: 0; font-size: 14px; font-weight: 650; }
      .ccp-multica-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px 12px; }
      .ccp-multica-form-field { display: grid; align-content: start; gap: 5px; min-width: 0; }
      .ccp-multica-form-field[data-wide="true"] { grid-column: 1 / -1; }
      .ccp-multica-form-field > span { color: color-mix(in srgb, currentColor 66%, transparent); font-size: 12px; }
      .ccp-multica-input, .ccp-multica-select, .ccp-multica-textarea { box-sizing: border-box; width: 100%; min-width: 0; border: 1px solid color-mix(in srgb, currentColor 20%, transparent); border-radius: 5px; padding: 7px 9px; background: color-mix(in srgb, currentColor 6%, transparent); color: inherit; font: inherit; letter-spacing: 0; }
      .ccp-multica-input, .ccp-multica-select { min-height: 34px; }
      .ccp-multica-textarea { min-height: 84px; resize: vertical; }
      .ccp-multica-form-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 7px; }
      .ccp-multica-inline-message { min-width: 0; color: color-mix(in srgb, currentColor 65%, transparent); overflow-wrap: anywhere; }
      .ccp-multica-inline-message[data-state="error"] { color: #e17d83; }
      .ccp-multica-inline-message[data-state="warning"] { color: #d7aa59; }
      .ccp-multica-executions { display: grid; gap: 7px; margin-top: 10px; padding-left: 12px; border-left: 2px solid color-mix(in srgb, #4fb995 36%, transparent); }
      .ccp-multica-execution { display: grid; gap: 6px; }
      .ccp-multica-execution-summary { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; color: color-mix(in srgb, currentColor 68%, transparent); }
      .ccp-multica-skill-heading { display: flex; align-items: center; gap: 10px; min-width: 0; }
      .ccp-multica-skill-heading .ccp-multica-item-title { min-width: 0; flex: 1; }
      .ccp-multica-skill-actions { display: inline-flex; flex: 0 0 auto; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 6px; }
      .ccp-multica-skill-bindings { display: grid; gap: 6px; margin-top: 8px; }
      .ccp-multica-skill-binding { display: flex; align-items: center; gap: 8px; min-width: 0; color: color-mix(in srgb, currentColor 68%, transparent); }
      .ccp-multica-skill-binding span { min-width: 0; overflow-wrap: anywhere; }
      .ccp-multica-skill-binding-form { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; margin-top: 8px; }
      .ccp-multica-skill-binding-form select, .ccp-multica-skill-binding-form input { min-height: 30px; max-width: 100%; border: 1px solid color-mix(in srgb, currentColor 20%, transparent); border-radius: 5px; padding: 4px 8px; background: color-mix(in srgb, currentColor 7%, transparent); color: inherit; font: inherit; }
      .ccp-multica-skill-binding-form input { flex: 1 1 180px; min-width: 120px; }
      .ccp-multica-item-title { margin: 0 0 5px; font-weight: 600; overflow-wrap: anywhere; }
      .ccp-multica-item-subtitle { color: color-mix(in srgb, currentColor 62%, transparent); overflow-wrap: anywhere; }
      .ccp-multica-fields { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-top: 7px; color: color-mix(in srgb, currentColor 58%, transparent); }
      .ccp-multica-field { max-width: 100%; overflow-wrap: anywhere; }
      .ccp-multica-field-key { color: color-mix(in srgb, currentColor 45%, transparent); margin-right: 4px; }
      .ccp-multica-stale { color: #d7aa59; }
      .ccp-multica-setting-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-width: 0; padding: 14px 0; border-top: 1px solid color-mix(in srgb, currentColor 13%, transparent); }
      .ccp-multica-setting-copy { min-width: 0; }
      .ccp-multica-setting-title { margin: 0 0 4px; font-weight: 600; }
      .ccp-multica-setting-description { color: color-mix(in srgb, currentColor 62%, transparent); overflow-wrap: anywhere; }
      .ccp-multica-toggle { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 auto; min-width: 58px; min-height: 30px; border: 1px solid color-mix(in srgb, currentColor 20%, transparent); border-radius: 6px; padding: 4px 10px; background: color-mix(in srgb, currentColor 7%, transparent); color: inherit; font: inherit; cursor: pointer; }
      .ccp-multica-toggle[data-enabled="true"] { border-color: color-mix(in srgb, #4fb995 65%, transparent); background: color-mix(in srgb, #4fb995 22%, transparent); }
      .ccp-multica-toggle:disabled { cursor: wait; opacity: .62; }
      @media (max-width: 620px) {
        .ccp-multica-content { padding: 14px 12px 20px; }
        .ccp-multica-content[data-route="my-issues"] { padding: 0; }
        .ccp-multica-board-heading { align-items: flex-start; flex-direction: column; }
        .ccp-multica-toolbar-spacer { display: none; }
        .ccp-multica-setting-row { align-items: flex-start; }
        .ccp-multica-form-grid { grid-template-columns: 1fr; }
        .ccp-multica-item-heading { align-items: stretch; flex-direction: column; }
        .ccp-multica-item-actions { justify-content: flex-start; }
      }
      @media (prefers-reduced-motion: reduce) { *, *::before, *::after { transition: none !important; animation: none !important; } }
    `;
    shadow.appendChild(style);
    const shell = multicaWorkspaceEl("section", "ccp-multica-shell");
    shell.setAttribute("role", "region");
    shell.setAttribute("aria-label", "我的任务");
    const content = multicaWorkspaceEl("main", "ccp-multica-content");
    content.id = "ccp-multica-content";
    shell.appendChild(content);
    shadow.appendChild(shell);
    shell.addEventListener("keydown", (event) => {
      if (event.key !== "Escape" || !multicaWorkspaceState.moduleMenuOpen) return;
      multicaWorkspaceState.moduleMenuOpen = false;
      multicaWorkspaceRenderContent();
    });
    shell.addEventListener("click", (event) => {
      if (!multicaWorkspaceState.moduleMenuOpen || event.target?.closest?.(".ccp-multica-module-menu")) return;
      multicaWorkspaceState.moduleMenuOpen = false;
      multicaWorkspaceRenderContent();
    });
    multicaWorkspaceState.shadow = shadow;
    multicaWorkspaceState.root = { shell, content };
    return shadow;
  }

  function moduleForMulticaWorkspace(route) {
    return multicaWorkspaceModules.find((module) => module.key === route) || multicaWorkspaceModules[0];
  }

  function multicaWorkspaceSelectRoute(route) {
    const module = multicaWorkspaceModules.find((candidate) => candidate.key === route);
    if (!module) return;
    multicaWorkspaceState.route = module.key;
    multicaWorkspaceState.moduleMenuOpen = false;
    multicaWorkspaceState.querySeq += 1;
    multicaWorkspaceCancelQuery();
    multicaWorkspaceState.loading.clear();
    multicaWorkspaceState.editor = null;
    multicaWorkspaceState.executionDraft = null;
    multicaWorkspaceRenderContent();
    if (module.key !== "settings") void multicaWorkspaceQuery(module, false);
  }

  function multicaWorkspaceAppendModuleMenu(parent) {
    const wrapper = multicaWorkspaceEl("div", "ccp-multica-module-menu");
    const trigger = multicaWorkspaceEl("button", "ccp-multica-icon-button", "≡");
    trigger.type = "button";
    trigger.title = "工作区模块";
    trigger.setAttribute("aria-label", "工作区模块");
    trigger.setAttribute("aria-haspopup", "menu");
    trigger.setAttribute("aria-expanded", String(multicaWorkspaceState.moduleMenuOpen));
    trigger.addEventListener("click", () => {
      multicaWorkspaceState.moduleMenuOpen = !multicaWorkspaceState.moduleMenuOpen;
      multicaWorkspaceRenderContent();
    });
    wrapper.appendChild(trigger);
    if (multicaWorkspaceState.moduleMenuOpen) {
      const menu = multicaWorkspaceEl("div", "ccp-multica-module-popover");
      menu.setAttribute("role", "menu");
      multicaWorkspaceModules.forEach((module) => {
        const item = multicaWorkspaceEl("button", "ccp-multica-module-item", module.label);
        item.type = "button";
        item.dataset.multicaRoute = module.key;
        item.setAttribute("role", "menuitem");
        if (module.key === multicaWorkspaceState.route) item.setAttribute("aria-current", "page");
        item.addEventListener("click", () => multicaWorkspaceSelectRoute(module.key));
        menu.appendChild(item);
      });
      wrapper.appendChild(menu);
    }
    parent.appendChild(wrapper);
  }

  function multicaWorkspaceEnsureHost() {
    document.querySelectorAll?.("#ccp-multica-workspace-root").forEach((candidate) => {
      if (candidate !== multicaWorkspaceState.host) candidate.remove();
    });
    if (multicaWorkspaceState.host?.isConnected && multicaWorkspaceState.root) return multicaWorkspaceState.host;
    if (!document.body || !document.createElement) return null;
    const host = document.createElement("div");
    host.id = "ccp-multica-workspace-root";
    host.dataset.ccpMulticaWorkspace = "true";
    host.dataset.ccpMulticaWorkspaceVersion = multicaWorkspaceVersion;
    Object.assign(host.style, {
      position: "fixed",
      display: "none",
      overflow: "hidden",
      zIndex: "2147481000",
      contain: "layout paint style",
    });
    if (!multicaWorkspaceInstallShadow(host)) return null;
    document.body.appendChild(host);
    multicaWorkspaceState.host = host;
    multicaWorkspaceRenderContent();
    return host;
  }

  function multicaWorkspaceEnsureEntryAvailabilityBadge(entry) {
    let badge = entry.querySelector?.('[data-ccp-multica-nav-availability="true"]');
    if (badge || !document.createElement) return badge || null;
    badge = document.createElement("span");
    badge.dataset.ccpMulticaNavAvailability = "true";
    badge.setAttribute("aria-hidden", "true");
    Object.assign(badge.style, {
      display: "none",
      alignItems: "center",
      flex: "0 0 auto",
      minHeight: "18px",
      marginLeft: "auto",
      padding: "0 5px",
      borderRadius: "4px",
      background: "rgba(225, 125, 131, .18)",
      color: "#e17d83",
      fontSize: "10px",
      fontWeight: "600",
      lineHeight: "18px",
      whiteSpace: "nowrap",
    });
    entry.appendChild(badge);
    return badge;
  }

  function multicaWorkspaceEnsureEntry(pluginButton) {
    if (!pluginButton?.parentElement || !document.createElement) return null;
    const entries = Array.from(document.querySelectorAll('[data-ccp-multica-nav="true"]'));
    let entry = multicaWorkspaceState.entry?.isConnected ? multicaWorkspaceState.entry : entries[0];
    entries.forEach((candidate) => {
      if (candidate !== entry) candidate.remove();
    });
    if (!entry) {
      entry = document.createElement("button");
      entry.type = "button";
      entry.className = pluginButton.className || "sidebar-item flex w-full";
      entry.dataset.ccpMulticaNav = "true";
      entry.dataset.ccpMulticaNavVersion = multicaWorkspaceVersion;
      entry.setAttribute("aria-label", "我的任务");
      entry.title = "我的任务";
      const icon = document.createElement("span");
      icon.textContent = "M";
      Object.assign(icon.style, {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "16px",
        height: "16px",
        flex: "0 0 16px",
        fontWeight: "700",
        fontSize: "11px",
        lineHeight: "16px",
      });
      const label = document.createElement("span");
      label.textContent = "我的任务";
      label.dataset.ccpMulticaNavLabel = "true";
      label.style.minWidth = "0";
      label.style.overflow = "hidden";
      label.style.textOverflow = "ellipsis";
      label.style.whiteSpace = "nowrap";
      entry.append(icon, label);
      multicaWorkspaceEnsureEntryAvailabilityBadge(entry);
    }
    // React and reinjection can preserve the DOM node across generations.
    // Always bind the current generation's handler so an unavailable entry
    // remains retryable instead of retaining a stale closure.
    if (entry.__ccpMulticaClickHandler) {
      entry.removeEventListener("click", entry.__ccpMulticaClickHandler, true);
    }
    entry.__ccpMulticaClickHandler = (event) => {
      event.preventDefault();
      event.stopPropagation();
      void multicaWorkspaceOpen();
    };
    entry.addEventListener("click", entry.__ccpMulticaClickHandler, true);
    // Reused entries can survive a partial reinjection. Keep their accessible
    // name and visible label in sync with the current navigation contract.
    entry.setAttribute("aria-label", "我的任务");
    entry.title = "我的任务";
    const label = entry.querySelector?.('[data-ccp-multica-nav-label="true"]')
      || entry.lastElementChild;
    if (label && label !== entry) {
      label.textContent = "我的任务";
      label.dataset.ccpMulticaNavLabel = "true";
    }
    multicaWorkspaceEnsureEntryAvailabilityBadge(entry);
    if (entry.parentElement !== pluginButton.parentElement || entry.previousElementSibling !== pluginButton) {
      pluginButton.parentElement.insertBefore(entry, pluginButton.nextSibling);
    }
    entry.setAttribute("aria-current", multicaWorkspaceState.opened ? "page" : "false");
    entry.setAttribute("data-state", multicaWorkspaceState.opened ? "active" : "inactive");
    multicaWorkspaceState.entry = entry;
    multicaWorkspaceSetEntryAvailability(multicaWorkspaceState.entryAvailabilityMessage);
    return entry;
  }

  function multicaWorkspaceSetStatus(text, state) {
    const host = multicaWorkspaceState.host;
    if (!host) return;
    host.dataset.ccpMulticaStatus = state || "";
    host.dataset.ccpMulticaStatusText = String(text || "").slice(0, 160);
  }

  function multicaWorkspaceSetEntryAvailability(message = "") {
    const entry = multicaWorkspaceState.entry;
    const detail = String(message || "").trim();
    multicaWorkspaceState.entryAvailabilityMessage = detail;
    if (!entry) return;
    const badge = multicaWorkspaceEnsureEntryAvailabilityBadge(entry);
    if (detail) {
      entry.dataset.ccpMulticaAvailability = "unavailable";
      entry.setAttribute("data-state", "unavailable");
      entry.setAttribute("aria-label", "我的任务，未连接，点击重试");
      entry.setAttribute("aria-description", `${detail}；点击重试`);
      entry.title = `我的任务（未连接，点击重试：${detail}）`;
      if (badge) {
        badge.textContent = "未连接";
        badge.style.display = "inline-flex";
      }
      return;
    }
    delete entry.dataset.ccpMulticaAvailability;
    entry.setAttribute("aria-label", "我的任务");
    entry.removeAttribute("aria-description");
    entry.setAttribute("data-state", multicaWorkspaceState.opened ? "active" : "inactive");
    entry.title = "我的任务";
    if (badge) {
      badge.textContent = "";
      badge.style.display = "none";
    }
  }

  function multicaWorkspaceBridgeUnavailable(error) {
    if (error?.timeout === true) return true;
    const detail = `${error?.code || ""} ${error?.message || error || ""}`;
    return /bridge|桥接|启动器|后端检查超时|工作区请求超时|未连接|network|unreachable|connection/i.test(detail);
  }

  function multicaWorkspaceFailOpen(message = "") {
    const detail = String(message || "启动器未连接，请通过 CCP 启动 Codex").trim();
    // A failed preflight must leave Codex's native surface usable. If the
    // board is already open, keep it visible and only expose the recoverable
    // bridge status; never hide a working local board because a later poll
    // or host probe failed.
    if (multicaWorkspaceState.opened) {
      multicaWorkspaceState.opening = false;
      multicaWorkspaceSetStatus(detail, "error");
      multicaWorkspaceSetEntryAvailability(detail);
      multicaWorkspaceRenderContent();
      return;
    }
    // Keep the entry usable even when the local bridge is temporarily down.
    // Show the local shell with its retry state instead of hiding it and
    // leaving Codex on a blank native surface.
    multicaWorkspaceState.opened = true;
    multicaWorkspaceState.opening = false;
    multicaWorkspaceRestoreMain();
    if (multicaWorkspaceState.host) {
      multicaWorkspaceState.host.style.display = "block";
      multicaWorkspaceState.host.style.visibility = "visible";
      multicaWorkspaceState.host.style.pointerEvents = "auto";
    }
    multicaWorkspaceSetStatus(detail, "error");
    multicaWorkspaceSetEntryAvailability(detail);
    multicaWorkspaceRenderContent();
  }

  function multicaWorkspaceFeatureEnabled() {
    const explicit = window.__claudeCodexProMulticaWorkspaceEnabled;
    if (typeof explicit === "boolean") return explicit;
    for (const key of ["multicaWorkspaceEnabled", "codexMulticaWorkspaceEnabled"]) {
      if (typeof claudeCodexProBackendSettings?.[key] === "boolean") {
        return claudeCodexProBackendSettings[key];
      }
    }
    // Existing installations do not have this optional setting. Keep the
    // integration enabled by default without changing settings storage.
    return true;
  }

  function multicaWorkspacePermissionError(error) {
    const status = Number(error?.httpStatus ?? error?.statusCode ?? error?.status);
    if (status === 401 || status === 403) return true;
    const value = `${error?.code || ""} ${error?.message || error || ""}`;
    return /unauthorized|forbidden|permission|access[ _-]?denied|needs?_login|未授权|无权|权限|登录|认证失败/i.test(value);
  }

  function multicaWorkspaceErrorFromResult(result, fallback) {
    const error = new Error(result?.message || fallback);
    if (result && typeof result === "object") {
      error.status = result.status;
      error.code = result.code;
      error.httpStatus = result.httpStatus ?? result.http_status ?? result.statusCode;
      error.timeout = result.timeout === true;
      error.cancelled = result.cancelled === true;
    }
    return error;
  }

  function multicaWorkspaceRequest(path, payload, timeoutMs = 15000) {
    let cancelRequest;
    let timer = null;
    const request = { promise: null, cancel: null };
    const cancelled = new Promise((resolve) => {
      cancelRequest = () => resolve({ status: "failed", message: "工作区请求已取消", cancelled: true });
    });
    const timedOut = new Promise((resolve) => {
      timer = setTimeout(() => resolve({ status: "failed", message: "工作区请求超时", timeout: true }), timeoutMs);
    });
    request.promise = Promise.race([postJson(path, payload), cancelled, timedOut]).finally(() => {
      if (timer) clearTimeout(timer);
      timer = null;
      multicaWorkspaceState.activeRequests.delete(request);
    });
    request.cancel = () => cancelRequest?.();
    multicaWorkspaceState.activeRequests.add(request);
    return request;
  }

  async function multicaWorkspaceCall(path, payload, timeoutMs = 15000) {
    const request = multicaWorkspaceRequest(path, payload, timeoutMs);
    const result = await request.promise;
    if (!result || result.status === "failed") {
      throw multicaWorkspaceErrorFromResult(result, "workspace request failed");
    }
    return result;
  }

  function multicaWorkspaceCancelQuery() {
    const request = multicaWorkspaceState.queryRequest;
    multicaWorkspaceState.queryRequest = null;
    request?.cancel?.();
  }

  function multicaWorkspaceCancelBootstrap() {
    const request = multicaWorkspaceState.bootstrapRequest;
    multicaWorkspaceState.bootstrapRequest = null;
    request?.cancel?.();
  }

  function multicaWorkspaceSafeKey(key) {
    return !/(token|secret|authorization|cookie|api[_-]?key|password|prompt|session[_-]?body|access[_-]?key|base[_-]?url|\burl\b|uri|headers?|\bmodel\b|\bprovider\b|runtime[_-]?id|shell|command|process[_-]?id|\bpid\b|environment)/i.test(key);
  }

  function multicaWorkspaceValue(value) {
    if (value === null || value === undefined) return "";
    if (typeof value === "string") return value.length > 280 ? `${value.slice(0, 277)}…` : value;
    if (typeof value === "number" || typeof value === "boolean") return String(value);
    if (Array.isArray(value)) return `${value.length} 项`;
    if (typeof value === "object") return Object.keys(value).filter(multicaWorkspaceSafeKey).slice(0, 8).join(", ") || "对象";
    return String(value);
  }

  function multicaWorkspaceItemTitle(item) {
    if (!item || typeof item !== "object") return multicaWorkspaceValue(item);
    for (const key of ["title", "name", "display_name", "displayName", "slug", "id", "key"]) {
      const value = item[key];
      if (typeof value === "string" && value.trim()) return value;
    }
    return "未命名记录";
  }

  function multicaWorkspaceAppendItem(parent, item) {
    const article = multicaWorkspaceEl("article", "ccp-multica-item");
    const title = multicaWorkspaceEl("h3", "ccp-multica-item-title", multicaWorkspaceItemTitle(item));
    article.appendChild(title);
    if (!item || typeof item !== "object" || Array.isArray(item)) {
      const value = multicaWorkspaceEl("div", "ccp-multica-item-subtitle", multicaWorkspaceValue(item));
      article.appendChild(value);
      parent.appendChild(article);
      return;
    }
    const preferred = ["status", "status_category", "status_name", "version", "priority", "labels", "reactions", "comment_count", "activity_count", "last_activity_at", "trigger_kinds", "next_run_at", "last_run_status", "subscribers", "timeline", "trust_state", "inventory_source", "workspace_slug"];
    const keys = preferred.concat(Object.keys(item)).filter((key, index, all) => all.indexOf(key) === index)
      .filter((key) => multicaWorkspaceSafeKey(key) && !["title", "name", "display_name", "displayName", "slug", "id", "key"].includes(key))
      .slice(0, 8);
    const fields = multicaWorkspaceEl("div", "ccp-multica-fields");
    keys.forEach((key) => {
      const value = multicaWorkspaceValue(item[key]);
      if (!value) return;
      const field = multicaWorkspaceEl("span", "ccp-multica-field");
      const labels = {
        status: "状态", status_category: "状态分类", status_name: "状态名称", version: "版本",
        priority: "优先级", labels: "标签", reactions: "反应", comment_count: "评论数",
        activity_count: "活动数", last_activity_at: "最近活动", trigger_kinds: "触发方式",
        next_run_at: "下次运行", last_run_status: "最近运行", subscribers: "订阅者",
        timeline: "时间线", trust_state: "信任状态", inventory_source: "清单来源", workspace_slug: "工作区"
      };
      field.append(multicaWorkspaceEl("span", "ccp-multica-field-key", `${labels[key] || key}:`), document.createTextNode(value));
      fields.appendChild(field);
    });
    if (fields.childNodes.length) article.appendChild(fields);
    parent.appendChild(article);
  }

  const multicaWorkspaceWritableResources = new Set(["issues", "comments", "labels", "subscribers", "reactions", "projects", "project_resources", "agents", "squads", "autopilots"]);
  const multicaWorkspaceTerminalExecutionStates = new Set(["completed", "failed", "cancelled"]);

  function multicaWorkspaceWritableResource(module) {
    const resource = module?.resource === "my_tasks" ? "issues" : module?.resource;
    return multicaWorkspaceWritableResources.has(resource) ? resource : "";
  }

  function multicaWorkspaceObjectValue(value, ...keys) {
    for (const key of keys) {
      if (value && value[key] !== undefined && value[key] !== null) return value[key];
    }
    return undefined;
  }

  function multicaWorkspaceEntityId(item) {
    return String(multicaWorkspaceObjectValue(item, "id") || "").trim();
  }

  function multicaWorkspaceEntityRevision(item) {
    const value = Number(multicaWorkspaceObjectValue(item, "revision"));
    return Number.isSafeInteger(value) && value >= 0 ? value : 0;
  }

  function multicaWorkspaceNewId(resource) {
    const prefix = ({ issues: "issue", comments: "comment", labels: "label", subscribers: "subscriber", reactions: "reaction", projects: "project", project_resources: "resource", agents: "agent", squads: "squad", autopilots: "autopilot" })[resource] || "item";
    const random = globalThis.crypto?.randomUUID?.().replaceAll("-", "").slice(0, 12)
      || Math.random().toString(36).slice(2, 14);
    return `${prefix}-${Date.now().toString(36)}-${random}`;
  }

  function multicaWorkspaceCommandId(kind, stableId) {
    const random = globalThis.crypto?.randomUUID?.().replaceAll("-", "")
      || `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`;
    return `${kind}:${String(stableId || "item").slice(0, 80)}:${random}`;
  }

  function multicaWorkspaceEditableEntity(item, resource) {
    const entity = {};
    if (!item || typeof item !== "object" || Array.isArray(item)) return entity;
    const allowed = new Set(({
      // Keep both wire names from Multica and the legacy local aliases. The
      // embedded store is deliberately schema-tolerant, while the editor must
      // preserve fields returned by newer upstream servers.
      issues: ["id", "title", "description", "status", "status_category", "status_name", "priority", "project_id", "projectId", "parent_issue_id", "parentIssueId", "assignee_type", "assigneeKind", "assignee_id", "assigneeId", "creator_type", "creator_id", "creatorId", "position", "stage", "start_date", "due_date", "label_ids", "labelIds", "metadata", "properties"],
      comments: ["id", "issue_id", "issueId", "author_type", "author_id", "content", "type", "parent_id", "parentId", "reactions", "attachments", "resolved_at", "resolved_by_type", "resolved_by_id", "source_task_id"],
      labels: ["id", "workspace_id", "resource_type", "resourceType", "name", "description", "color", "usage_count"],
      subscribers: ["id", "issue_id", "user_type", "user_id", "reason"],
      reactions: ["id", "comment_id", "actor_type", "actor_id", "emoji", "created_at"],
      activities: ["id", "issue_id", "actor_type", "actor_id", "action", "details", "created_at"],
      projects: ["id", "title", "name", "description", "icon", "status", "priority", "lead_type", "lead_id", "start_date", "due_date", "resources", "members", "progress"],
      project_resources: ["id", "project_id", "projectId", "workspace_id", "resource_type", "resourceType", "resource_ref", "resourceRef", "label", "position"],
      agents: ["id", "name", "description", "instructions", "enabled", "status", "runtime_id", "runtime_mode", "provider", "visibility", "permission_mode", "invocation_targets", "max_concurrent_tasks", "concurrency_limit", "concurrencyLimit", "model", "thinking_level", "service_tier", "skills", "label_ids", "labelIds", "disabled_runtime_skills", "runtime_bound", "conversation_starters"],
      squads: ["id", "name", "description", "instructions", "avatar_url", "leader_id", "leaderAgentId", "memberAgentIds", "members", "member_preview", "activity"],
      autopilots: ["id", "title", "name", "description", "project_id", "assignee_type", "assignee_id", "trigger_kind", "triggerKind", "execution_mode", "issue_title_template", "schedule", "enabled", "status", "subscribers", "collaborators", "triggers", "runs", "last_run_status", "next_run_at"],
    })[resource] || ["id"]);
    for (const [key, value] of Object.entries(item)) {
      if (!allowed.has(key) || !multicaWorkspaceSafeKey(key)) continue;
      entity[key] = value;
    }
    return entity;
  }

  function multicaWorkspaceFieldDefinitions(resource) {
    if (resource === "comments") return [
      { key: "issue_id", label: "任务 ID", required: true },
      { key: "content", label: "评论内容", type: "textarea", wide: true, required: true },
      { key: "type", label: "类型", type: "select", options: [["comment", "评论"], ["status_change", "状态变更"], ["progress_update", "进度更新"]] },
      { key: "parent_id", label: "父评论 ID" },
      { key: "author_type", label: "作者类型", type: "select", options: [["member", "成员"], ["agent", "智能体"], ["system", "系统"]] },
      { key: "author_id", label: "作者 ID" },
      { key: "resolved_at", label: "解析时间" },
      { key: "resolved_by_type", label: "解析者类型" },
      { key: "resolved_by_id", label: "解析者 ID" },
      { key: "source_task_id", label: "来源任务 ID" },
    ];
    if (resource === "labels") return [
      { key: "name", label: "标签名称", required: true },
      { key: "resource_type", label: "资源类型", type: "select", options: [["issue", "任务"], ["agent", "智能体"], ["skill", "Skill"]] },
      { key: "description", label: "描述", type: "textarea", wide: true },
      { key: "color", label: "颜色（#RRGGBB）" },
    ];
    if (resource === "subscribers") return [
      { key: "issue_id", label: "任务 ID", required: true },
      { key: "user_type", label: "用户类型", type: "select", options: [["member", "成员"], ["agent", "智能体"]] },
      { key: "user_id", label: "用户 ID", required: true },
      { key: "reason", label: "订阅原因" },
    ];
    if (resource === "reactions") return [
      { key: "comment_id", label: "评论 ID", required: true },
      { key: "actor_type", label: "操作者类型" },
      { key: "actor_id", label: "操作者 ID", required: true },
      { key: "emoji", label: "表情", required: true },
    ];
    if (resource === "activities") return [];
    if (resource === "issues") return [
      { key: "title", label: "标题", required: true, wide: true },
      { key: "description", label: "描述", type: "textarea", wide: true },
      { key: "status", label: "状态", type: "select", options: [["backlog", "待规划"], ["todo", "待办"], ["in_progress", "进行中"], ["in_review", "审核中"], ["done", "已完成"], ["blocked", "已阻塞"], ["cancelled", "已取消"]] },
      { key: "priority", label: "优先级", type: "select", options: [["none", "无"], ["low", "低"], ["medium", "中"], ["high", "高"], ["urgent", "紧急"]] },
      { key: "project_id", label: "项目 ID" },
      { key: "parent_issue_id", label: "父任务 ID" },
      { key: "assignee_type", label: "执行者类型", type: "select", optional: true, options: [["agent", "智能体"], ["squad", "小队"], ["member", "成员"]] },
      { key: "assignee_id", label: "执行者 ID" },
      { key: "start_date", label: "开始日期（YYYY-MM-DD）" },
      { key: "due_date", label: "截止日期（YYYY-MM-DD）" },
      { key: "label_ids", label: "标签 ID（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "resources", label: "项目资源（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "members", label: "项目成员（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "progress", label: "项目进度（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: {} },
      { key: "metadata", label: "流程元数据（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: {} },
      { key: "properties", label: "自定义属性（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: {} },
    ];
    if (resource === "projects") return [
      { key: "title", label: "项目名称", required: true, wide: true },
      { key: "description", label: "描述", type: "textarea", wide: true },
      { key: "status", label: "状态", type: "select", options: [["planned", "计划中"], ["in_progress", "进行中"], ["paused", "已暂停"], ["completed", "已完成"], ["cancelled", "已取消"]] },
      { key: "priority", label: "优先级", type: "select", options: [["none", "无"], ["low", "低"], ["medium", "中"], ["high", "高"], ["urgent", "紧急"]] },
      { key: "lead_type", label: "负责人类型", type: "select", optional: true, options: [["member", "成员"], ["agent", "智能体"]] },
      { key: "lead_id", label: "负责人 ID" },
      { key: "start_date", label: "开始日期（YYYY-MM-DD）" },
      { key: "due_date", label: "截止日期（YYYY-MM-DD）" },
    ];
    if (resource === "project_resources") return [
      { key: "project_id", label: "项目 ID", required: true },
      { key: "resource_type", label: "资源类型", type: "select", required: true, options: [["github_repo", "GitHub 仓库"], ["local_directory", "本地目录"]] },
      { key: "resource_ref", label: "资源引用（JSON）", type: "textarea", wide: true, required: true, valueType: "json", jsonEmpty: {} },
      { key: "label", label: "显示名称" },
      { key: "position", label: "排序", type: "number" },
    ];
    if (resource === "agents") return [
      { key: "name", label: "智能体名称", required: true, wide: true },
      { key: "description", label: "职责", type: "textarea", wide: true },
      { key: "instructions", label: "执行指令", type: "textarea", wide: true },
      { key: "status", label: "状态", type: "select", options: [["active", "启用"], ["paused", "暂停"], ["archived", "归档"]] },
      { key: "runtime_id", label: "运行时 ID" },
      { key: "runtime_mode", label: "运行模式", type: "select", options: [["local", "本地"], ["cloud", "云端"]] },
      { key: "provider", label: "运行时提供方" },
      { key: "visibility", label: "可见性", type: "select", options: [["workspace", "工作区"], ["private", "私有"]] },
      { key: "permission_mode", label: "调用权限", type: "select", options: [["private", "仅所有者"], ["public_to", "按授权目标"]] },
      { key: "invocation_targets", label: "授权目标（JSON）", type: "textarea", wide: true, valueType: "json" },
      { key: "max_concurrent_tasks", label: "并发任务上限", type: "number" },
      { key: "model", label: "模型" },
      { key: "thinking_level", label: "思考级别" },
      { key: "service_tier", label: "服务层级" },
      { key: "label_ids", label: "标签 ID（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "disabled_runtime_skills", label: "禁用运行时 Skills（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "conversation_starters", label: "对话起始项（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
    ];
    if (resource === "squads") return [
      { key: "name", label: "小队名称", required: true, wide: true },
      { key: "description", label: "分工说明", type: "textarea", wide: true },
      { key: "instructions", label: "小队指令", type: "textarea", wide: true },
      { key: "leader_id", label: "负责人智能体 ID" },
      { key: "memberAgentIds", label: "成员智能体 ID（逗号分隔）", valueType: "list", wide: true },
      { key: "members", label: "成员与分工（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
    ];
    return [
      { key: "title", label: "自动化名称", required: true, wide: true },
      { key: "description", label: "描述", type: "textarea", wide: true },
      { key: "trigger_kind", label: "触发方式", type: "select", options: [["schedule", "定时"], ["webhook", "Webhook"], ["api", "API"]] },
      { key: "schedule", label: "调度表达式" },
      { key: "assignee_type", label: "执行者类型", type: "select", options: [["agent", "智能体"], ["squad", "小队"]] },
      { key: "assignee_id", label: "执行者 ID" },
      { key: "execution_mode", label: "执行模式", type: "select", options: [["create_issue", "创建任务"], ["run_only", "仅运行"]] },
      { key: "issue_title_template", label: "任务标题模板" },
      { key: "status", label: "状态", type: "select", options: [["active", "启用"], ["paused", "暂停"], ["archived", "归档"]] },
      { key: "pause_reason", label: "暂停原因" },
      { key: "subscribers", label: "订阅者（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "collaborators", label: "协作者（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
      { key: "triggers", label: "触发器（JSON）", type: "textarea", wide: true, valueType: "json", jsonEmpty: [] },
    ];
  }

  function multicaWorkspaceDefaultEntity(resource) {
    const common = { id: multicaWorkspaceNewId(resource) };
    if (resource === "issues") return { ...common, title: "", description: "", status: "todo", priority: "medium", assignee_type: "", assignee_id: "", metadata: {}, properties: {} };
    if (resource === "comments") return { ...common, issue_id: "", content: "", type: "comment", parent_id: null, author_type: "member", author_id: "" };
    if (resource === "labels") return { ...common, name: "", resource_type: "issue", description: "", color: "#6B7280" };
    if (resource === "subscribers") return { ...common, issue_id: "", user_type: "member", user_id: "", reason: "manual" };
    if (resource === "reactions") return { ...common, comment_id: "", actor_type: "member", actor_id: "", emoji: "👍" };
    if (resource === "projects") return { ...common, title: "", description: "", status: "planned", priority: "none", lead_type: "", lead_id: "" };
    if (resource === "project_resources") return { ...common, project_id: "", resource_type: "github_repo", resource_ref: {}, label: "", position: 0 };
    if (resource === "agents") return { ...common, name: "", description: "", instructions: "", enabled: true, runtime_id: "", runtime_mode: "local", visibility: "workspace", permission_mode: "private", invocation_targets: [], max_concurrent_tasks: 1, model: "", thinking_level: "", service_tier: "" };
    if (resource === "squads") return { ...common, name: "", description: "", instructions: "", leader_id: "", memberAgentIds: [] };
    return { ...common, title: "", description: "", trigger_kind: "schedule", schedule: "", assignee_type: "agent", assignee_id: "", execution_mode: "create_issue", status: "active", collaborators: [], triggers: [], runs: [] };
  }

  function multicaWorkspaceNormalizeEditableEntity(resource, values) {
    const normalized = { ...(values || {}) };
    const copy = (canonical, legacy) => {
      if ((normalized[canonical] === undefined || normalized[canonical] === null || normalized[canonical] === "") && normalized[legacy] !== undefined) {
        normalized[canonical] = normalized[legacy];
      }
      delete normalized[legacy];
    };
    if (resource === "issues") {
      copy("project_id", "projectId");
      copy("parent_issue_id", "parentIssueId");
      copy("assignee_type", "assigneeKind");
      copy("assignee_id", "assigneeId");
      copy("creator_id", "creatorId");
    } else if (resource === "projects") {
      copy("title", "name");
    } else if (resource === "agents") {
      copy("concurrency_limit", "concurrencyLimit");
    } else if (resource === "squads") {
      copy("leader_id", "leaderAgentId");
    } else if (resource === "autopilots") {
      copy("title", "name");
      copy("trigger_kind", "triggerKind");
    }
    return normalized;
  }

  function multicaWorkspaceOpenEditor(module, item = null, defaults = {}) {
    const resource = multicaWorkspaceWritableResource(module);
    if (!resource || multicaWorkspaceState.mutationBusy) return;
    const values = multicaWorkspaceNormalizeEditableEntity(
      resource,
      item ? multicaWorkspaceEditableEntity(item, resource) : { ...multicaWorkspaceDefaultEntity(resource), ...defaults },
    );
    if (!item && resource === "issues") {
      const localUserId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
      if (localUserId) {
        values.creator_id = localUserId;
        if (module.key === "my-issues") {
          values.assignee_type = "member";
          values.assignee_id = localUserId;
        }
      }
    }
    multicaWorkspaceState.editor = {
      resource,
      entityId: multicaWorkspaceEntityId(values) || multicaWorkspaceNewId(resource),
      expectedRevision: item ? multicaWorkspaceEntityRevision(item) : 0,
      original: item ? multicaWorkspaceEditableEntity(item) : null,
      values,
      message: "",
    };
    multicaWorkspaceState.mutationNotice = null;
    multicaWorkspaceRenderContent();
  }

  function multicaWorkspaceCloseEditor() {
    if (multicaWorkspaceState.mutationBusy) return;
    multicaWorkspaceState.editor = null;
    multicaWorkspaceRenderContent();
  }

  function multicaWorkspaceFormControl(field, rawValue, onChange) {
    let control;
    if (field.type === "textarea") {
      control = multicaWorkspaceEl("textarea", "ccp-multica-textarea");
      const displayValue = field.valueType === "json" && rawValue != null && typeof rawValue !== "string"
        ? JSON.stringify(rawValue, null, 2)
        : rawValue;
      control.value = displayValue == null ? "" : String(displayValue);
      control.addEventListener("input", () => {
        if (field.valueType === "json") {
          try { onChange(control.value.trim() ? JSON.parse(control.value) : (field.jsonEmpty ?? [])); }
          catch { onChange(control.value); }
        } else onChange(control.value);
      });
    } else if (field.type === "select") {
      control = multicaWorkspaceEl("select", "ccp-multica-select");
      if (field.optional) {
        const empty = multicaWorkspaceEl("option", "", "未设置");
        empty.value = "";
        control.appendChild(empty);
      }
      field.options.forEach(([value, label]) => {
        const option = multicaWorkspaceEl("option", "", label);
        option.value = value;
        control.appendChild(option);
      });
      control.value = rawValue == null ? "" : String(rawValue);
      control.addEventListener("change", () => onChange(field.valueType === "boolean" ? control.value === "true" : control.value));
    } else {
      control = multicaWorkspaceEl("input", "ccp-multica-input");
      control.type = field.type === "number" ? "number" : "text";
      if (field.type === "number") control.min = "1";
      const displayValue = field.valueType === "list" && Array.isArray(rawValue) ? rawValue.join(", ") : rawValue;
      control.value = displayValue == null ? "" : String(displayValue);
      control.addEventListener("input", () => {
        if (field.valueType === "list") onChange(control.value.split(",").map((value) => value.trim()).filter(Boolean));
        else if (field.type === "number") onChange(Math.max(1, Number(control.value) || 1));
        else onChange(control.value);
      });
    }
    control.setAttribute("aria-label", field.label);
    if (field.required) control.required = true;
    return control;
  }

  function multicaWorkspaceRenderEditor(content, module) {
    const editor = multicaWorkspaceState.editor;
    const resource = multicaWorkspaceWritableResource(module);
    if (!editor || editor.resource !== resource) return;
    const form = multicaWorkspaceEl("form", "ccp-multica-form");
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void multicaWorkspaceSaveEditor(module);
    });
    form.appendChild(multicaWorkspaceEl("h3", "ccp-multica-form-title", editor.expectedRevision ? "编辑记录" : "新建记录"));
    const grid = multicaWorkspaceEl("div", "ccp-multica-form-grid");
    multicaWorkspaceFieldDefinitions(resource).forEach((field) => {
      const wrapper = multicaWorkspaceEl("label", "ccp-multica-form-field");
      if (field.wide) wrapper.dataset.wide = "true";
      wrapper.appendChild(multicaWorkspaceEl("span", "", field.label));
      wrapper.appendChild(multicaWorkspaceFormControl(field, editor.values[field.key], (value) => {
        if (multicaWorkspaceState.editor === editor) {
          editor.values[field.key] = value;
          editor.message = "";
        }
      }));
      grid.appendChild(wrapper);
    });
    form.appendChild(grid);
    const actions = multicaWorkspaceEl("div", "ccp-multica-form-actions");
    const save = multicaWorkspaceEl("button", "ccp-multica-button", multicaWorkspaceState.mutationBusy ? "保存中…" : "保存");
    save.type = "submit";
    save.dataset.variant = "primary";
    save.disabled = multicaWorkspaceState.mutationBusy;
    const cancel = multicaWorkspaceEl("button", "ccp-multica-button", "取消");
    cancel.type = "button";
    cancel.disabled = multicaWorkspaceState.mutationBusy;
    cancel.addEventListener("click", multicaWorkspaceCloseEditor);
    actions.append(save, cancel);
    if (editor.message) {
      const message = multicaWorkspaceEl("span", "ccp-multica-inline-message", editor.message);
      message.dataset.state = "error";
      actions.appendChild(message);
    }
    form.appendChild(actions);
    content.appendChild(form);
  }

  async function multicaWorkspaceRefreshMutationResource(module, resource) {
    if (resource !== "issues") {
      await multicaWorkspaceQuery(module, true);
      return;
    }
    for (const route of ["issues", "my-issues"]) {
      await multicaWorkspaceQuery(moduleForMulticaWorkspace(route), true);
    }
  }

  async function multicaWorkspaceSaveEditor(module) {
    const editor = multicaWorkspaceState.editor;
    if (!editor || multicaWorkspaceState.mutationBusy) return;
    const requiredKey = (editor.resource === "issues" || editor.resource === "projects" || editor.resource === "autopilots") ? "title" : (editor.resource === "project_resources" ? "project_id" : (editor.resource === "comments" ? "issue_id" : "name"));
    if (!String(editor.values[requiredKey] || "").trim()) {
      editor.message = requiredKey === "title" ? "请输入标题" : (requiredKey === "project_id" ? "请输入项目 ID" : (requiredKey === "issue_id" ? "请输入任务 ID" : "请输入名称"));
      multicaWorkspaceRenderContent();
      return;
    }
    const invalidJsonField = multicaWorkspaceFieldDefinitions(editor.resource).find((field) =>
      field.valueType === "json" && typeof editor.values[field.key] === "string"
    );
    if (invalidJsonField) {
      editor.message = `${invalidJsonField.label}必须是有效 JSON`;
      multicaWorkspaceRenderContent();
      return;
    }
    const entity = { ...(editor.original || {}), ...editor.values, id: editor.entityId };
    Object.keys(entity).forEach((key) => {
      if (!multicaWorkspaceSafeKey(key) || ["workspaceId", "workspace_id", "revision", "createdAtMs", "created_at_ms", "updatedAtMs", "updated_at_ms"].includes(key)) delete entity[key];
    });
    multicaWorkspaceState.mutationBusy = true;
    editor.message = "";
    multicaWorkspaceRenderContent();
    try {
      await multicaWorkspaceCall("/multica/workspace/upsert", {
        resource: editor.resource,
        entity,
        expectedRevision: editor.expectedRevision,
      });
      multicaWorkspaceState.editor = null;
      multicaWorkspaceState.mutationNotice = { state: "ok", message: "已保存" };
      await multicaWorkspaceRefreshMutationResource(module, editor.resource);
    } catch (error) {
      editor.message = multicaWorkspaceErrorMessage(error);
      multicaWorkspaceState.mutationNotice = { state: "error", message: editor.message };
    } finally {
      multicaWorkspaceState.mutationBusy = false;
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceDeleteEntity(module, item) {
    const resource = multicaWorkspaceWritableResource(module);
    const entityId = multicaWorkspaceEntityId(item);
    const expectedRevision = multicaWorkspaceEntityRevision(item);
    if (!resource || !entityId || !expectedRevision || multicaWorkspaceState.mutationBusy) return;
    if (typeof window.confirm === "function" && !window.confirm(`确认删除“${multicaWorkspaceItemTitle(item)}”？`)) return;
    multicaWorkspaceState.mutationBusy = true;
    multicaWorkspaceState.mutationNotice = { state: "loading", message: "正在删除…" };
    multicaWorkspaceRenderContent();
    try {
      await multicaWorkspaceCall("/multica/workspace/delete", { resource, entityId, expectedRevision });
      multicaWorkspaceState.mutationNotice = { state: "ok", message: "已删除" };
      if (multicaWorkspaceState.editor?.entityId === entityId) multicaWorkspaceState.editor = null;
      await multicaWorkspaceRefreshMutationResource(module, resource);
    } catch (error) {
      multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
    } finally {
      multicaWorkspaceState.mutationBusy = false;
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspacePatchEntity(module, item, patch, successMessage) {
    const resource = multicaWorkspaceWritableResource(module);
    if (!resource || multicaWorkspaceState.mutationBusy) return;
    const entity = { ...multicaWorkspaceEditableEntity(item, resource), ...patch, id: multicaWorkspaceEntityId(item) };
    multicaWorkspaceState.mutationBusy = true;
    try {
      await multicaWorkspaceCall("/multica/workspace/upsert", { resource, entity, expectedRevision: multicaWorkspaceEntityRevision(item) });
      multicaWorkspaceState.mutationNotice = { state: "ok", message: successMessage };
      await multicaWorkspaceRefreshMutationResource(module, resource);
    } catch (error) {
      multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
    } finally {
      multicaWorkspaceState.mutationBusy = false;
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceToggleIssueSubscription(module, item) {
    const issueId = multicaWorkspaceEntityId(item);
    const userId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
    if (!issueId || !userId || multicaWorkspaceState.mutationBusy) return;
    const collection = multicaWorkspaceState.collections.get("subscribers");
    const current = (collection?.items || []).find((entry) =>
      String(multicaWorkspaceObjectValue(entry, "issue_id", "issueId") || "") === issueId
      && String(multicaWorkspaceObjectValue(entry, "user_id", "userId") || "") === userId,
    );
    if (current) {
      try {
        await multicaWorkspaceDeleteEntity({ key: "subscribers", resource: "subscribers" }, current);
      } catch (error) {
        multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
      }
    } else {
      const entity = { id: multicaWorkspaceNewId("subscriber"), issue_id: issueId, user_type: "member", user_id: userId, reason: "manual" };
      multicaWorkspaceState.mutationBusy = true;
      try {
        await multicaWorkspaceCall("/multica/workspace/upsert", { resource: "subscribers", entity, expectedRevision: 0 });
        await multicaWorkspaceRefreshMutationResource({ key: "subscribers", resource: "subscribers" }, "subscribers");
      } catch (error) {
        multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
      } finally {
        multicaWorkspaceState.mutationBusy = false;
        if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
      }
    }
  }

  async function multicaWorkspaceUnsubscribeIssueSubtree(item) {
    const rootId = multicaWorkspaceEntityId(item);
    const userId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
    if (!rootId || !userId || multicaWorkspaceState.mutationBusy) return;
    const issues = multicaWorkspaceState.collections.get("issues")?.items || [];
    const children = new Map();
    issues.forEach((issue) => {
      const parent = String(multicaWorkspaceObjectValue(issue, "parent_issue_id", "parentIssueId") || "").trim();
      const id = multicaWorkspaceEntityId(issue);
      if (!id || !parent) return;
      const list = children.get(parent) || [];
      list.push(id);
      children.set(parent, list);
    });
    const ids = [rootId];
    for (let index = 0; index < ids.length; index += 1) ids.push(...(children.get(ids[index]) || []));
    const subscribers = multicaWorkspaceState.collections.get("subscribers")?.items || [];
    const matches = subscribers.filter((entry) => ids.includes(String(multicaWorkspaceObjectValue(entry, "issue_id", "issueId") || ""))
      && String(multicaWorkspaceObjectValue(entry, "user_id", "userId") || "") === userId);
    if (!matches.length) return;
    multicaWorkspaceState.mutationBusy = true;
    try {
      for (const subscriber of matches) {
        await multicaWorkspaceCall("/multica/workspace/delete", {
          resource: "subscribers",
          entityId: multicaWorkspaceEntityId(subscriber),
          expectedRevision: multicaWorkspaceEntityRevision(subscriber),
        });
      }
      multicaWorkspaceState.mutationNotice = { state: "ok", message: "已取消任务树订阅" };
      await multicaWorkspaceRefreshMutationResource({ key: "subscribers", resource: "subscribers" }, "subscribers");
    } catch (error) {
      multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
    } finally {
      multicaWorkspaceState.mutationBusy = false;
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceToggleReaction(targetType, item, emoji = "👍") {
    const targetId = multicaWorkspaceEntityId(item);
    const userId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
    if (!targetId || !userId || multicaWorkspaceState.mutationBusy) return;
    const collection = multicaWorkspaceState.collections.get("reactions");
    const current = (collection?.items || []).find((entry) =>
      String(multicaWorkspaceObjectValue(entry, targetType === "issue" ? "issue_id" : "comment_id") || "") === targetId
      && String(multicaWorkspaceObjectValue(entry, "actor_id") || "") === userId
      && String(multicaWorkspaceObjectValue(entry, "emoji") || "") === emoji,
    );
    const module = { key: "reactions", resource: "reactions" };
    if (current) {
      try {
        await multicaWorkspaceDeleteEntity(module, current);
      } catch (error) {
        multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
      }
      return;
    }
    multicaWorkspaceState.mutationBusy = true;
    try {
      await multicaWorkspaceCall("/multica/workspace/upsert", {
        resource: "reactions",
        entity: { id: multicaWorkspaceNewId("reaction"), [`${targetType}_id`]: targetId, actor_type: "member", actor_id: userId, emoji },
        expectedRevision: 0,
      });
      await multicaWorkspaceRefreshMutationResource(module, "reactions");
    } catch (error) {
      multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
    } finally {
      multicaWorkspaceState.mutationBusy = false;
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceToggleLabel(module, item) {
    const resource = multicaWorkspaceWritableResource(module);
    if (!(resource === "issues" || resource === "agents") || multicaWorkspaceState.mutationBusy) return;
    const labelId = String(window.prompt?.("输入要切换的 label_id", "") || "").trim();
    if (!labelId) return;
    const current = multicaWorkspaceObjectValue(item, "label_ids", "labelIds");
    const labelIds = Array.isArray(current) ? current.map((value) => String(value)).filter(Boolean) : [];
    const index = labelIds.indexOf(labelId);
    if (index >= 0) labelIds.splice(index, 1); else labelIds.push(labelId);
    await multicaWorkspacePatchEntity(module, item, { label_ids: labelIds }, index >= 0 ? "已移除标签" : "已添加标签");
  }

  async function multicaWorkspaceTriggerAutopilot(module, item) {
    const resource = multicaWorkspaceWritableResource(module);
    if (resource !== "autopilots" || multicaWorkspaceState.mutationBusy) return;
    const autopilotId = multicaWorkspaceEntityId(item);
    try {
      await multicaWorkspaceCall("/multica/autopilots/trigger", { autopilotId, source: "manual" });
      await multicaWorkspaceRefreshMutationResource(module, resource);
      multicaWorkspaceState.mutationNotice = { state: "ok", message: "已创建待执行的 Autopilot 运行记录" };
    } catch (error) {
      multicaWorkspaceState.mutationNotice = { state: "error", message: multicaWorkspaceErrorMessage(error) };
    }
    if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
  }

  async function multicaWorkspaceCreateAutopilotTrigger(module, item) {
    if (multicaWorkspaceWritableResource(module) !== "autopilots" || multicaWorkspaceState.mutationBusy) return;
    const kind = String(window.prompt?.("触发器类型：schedule / webhook / api", "schedule") || "").trim().toLowerCase();
    if (!["schedule", "webhook", "api"].includes(kind)) return;
    const trigger = {
      id: multicaWorkspaceNewId("autopilot-trigger"),
      kind,
      enabled: true,
      ...(kind === "schedule" ? {
        cron_expression: String(window.prompt?.("cron_expression", "0 * * * *") || "").trim(),
        timezone: String(window.prompt?.("timezone", "UTC") || "UTC").trim() || "UTC",
      } : {}),
      label: String(window.prompt?.("label（可选）", "") || "").trim(),
      ...(kind === "webhook" ? { event_filters: [] } : {}),
    };
    const triggers = Array.isArray(item.triggers) ? item.triggers.slice() : [];
    triggers.push(trigger);
    await multicaWorkspacePatchEntity(module, item, { triggers }, "已创建自动化触发器");
  }

  async function multicaWorkspaceDeleteAutopilotTrigger(module, item) {
    if (multicaWorkspaceWritableResource(module) !== "autopilots" || multicaWorkspaceState.mutationBusy) return;
    const triggers = Array.isArray(item.triggers) ? item.triggers.slice() : [];
    if (!triggers.length) return;
    const triggerId = String(window.prompt?.("输入要删除的 trigger_id", String(triggers[0].id || "")) || "").trim();
    if (!triggerId) return;
    const next = triggers.filter((trigger) => String(trigger?.id || "") !== triggerId);
    if (next.length === triggers.length) return;
    await multicaWorkspacePatchEntity(module, item, { triggers: next }, "已删除自动化触发器");
  }

  async function multicaWorkspaceUpdateAutopilotTrigger(module, item) {
    if (multicaWorkspaceWritableResource(module) !== "autopilots" || multicaWorkspaceState.mutationBusy) return;
    const triggers = Array.isArray(item.triggers) ? item.triggers.slice() : [];
    if (!triggers.length) return;
    const triggerId = String(window.prompt?.("输入要编辑的 trigger_id", String(triggers[0].id || "")) || "").trim();
    const index = triggers.findIndex((trigger) => String(trigger?.id || "") === triggerId);
    if (index < 0) return;
    const current = triggers[index];
    const enabled = String(window.prompt?.("enabled：true / false", String(current.enabled !== false)) || "").trim().toLowerCase();
    if (!["true", "false"].includes(enabled)) return;
    const next = { ...current, enabled: enabled === "true" };
    if (next.kind === "schedule") {
      const cron = String(window.prompt?.("cron_expression", String(current.cron_expression || "")) || "").trim();
      if (!cron) return;
      next.cron_expression = cron;
      const timezone = String(window.prompt?.("timezone", String(current.timezone || "UTC")) || "").trim();
      if (!timezone) return;
      next.timezone = timezone;
    }
    const label = String(window.prompt?.("label（留空清除）", String(current.label || "")) || "").trim();
    if (label) next.label = label; else delete next.label;
    if (next.kind === "webhook") {
      const rawFilters = String(window.prompt?.("event_filters（JSON，留空清除）", current.event_filters ? JSON.stringify(current.event_filters) : "") || "").trim();
      if (rawFilters) {
        try {
          const parsed = JSON.parse(rawFilters);
          if (!Array.isArray(parsed) || parsed.some((entry) => !entry || typeof entry !== "object" || typeof entry.event !== "string" || !entry.event.trim() || (entry.actions !== undefined && (!Array.isArray(entry.actions) || entry.actions.some((action) => typeof action !== "string" || !action.trim()))))) return;
          next.event_filters = parsed;
        } catch (_) { return; }
      } else {
        delete next.event_filters;
      }
    }
    triggers[index] = next;
    await multicaWorkspacePatchEntity(module, item, { triggers }, "已更新自动化触发器");
  }

  async function multicaWorkspaceToggleAutopilotCollaborator(module, item) {
    if (multicaWorkspaceWritableResource(module) !== "autopilots" || multicaWorkspaceState.mutationBusy) return;
    const userId = String(window.prompt?.("输入协作者 user_id（再次输入可移除）", "") || "").trim();
    if (!userId || userId.length > 240 || /[\u0000\s]/.test(userId)) return;
    const collaborators = Array.isArray(item.collaborators) ? item.collaborators.slice() : [];
    const index = collaborators.findIndex((entry) => String(entry?.user_id || entry?.userId || entry || "") === userId);
    if (index >= 0) collaborators.splice(index, 1); else collaborators.push({ user_id: userId, role: "collaborator" });
    await multicaWorkspacePatchEntity(module, item, { collaborators }, index >= 0 ? "已移除协作者" : "已添加协作者");
  }

  function multicaWorkspaceIssuePrompt(item) {
    const title = String(multicaWorkspaceObjectValue(item, "title", "name") || "").trim();
    const description = String(multicaWorkspaceObjectValue(item, "description") || "").trim();
    return [title, description].filter(Boolean).join("\n\n").slice(0, 32000);
  }

  function multicaWorkspaceExecutionBindingId(binding) {
    return String(multicaWorkspaceObjectValue(binding, "bindingId", "binding_id") || "").trim();
  }

  function multicaWorkspaceExecutionState(binding) {
    return String(multicaWorkspaceObjectValue(binding, "state") || "unknown").toLowerCase();
  }

  function multicaWorkspaceExecutionsForIssue(issueId) {
    return multicaWorkspaceState.executions
      .filter((binding) => String(multicaWorkspaceObjectValue(binding, "issueId", "issue_id") || "") === issueId)
      .sort((left, right) => Number(multicaWorkspaceObjectValue(right, "attemptNo", "attempt_no") || 0) - Number(multicaWorkspaceObjectValue(left, "attemptNo", "attempt_no") || 0));
  }

  function multicaWorkspaceMergeExecution(binding) {
    if (!binding || typeof binding !== "object") return;
    const bindingId = multicaWorkspaceExecutionBindingId(binding);
    if (!bindingId) return;
    const index = multicaWorkspaceState.executions.findIndex((item) => multicaWorkspaceExecutionBindingId(item) === bindingId);
    if (index >= 0) multicaWorkspaceState.executions[index] = binding;
    else multicaWorkspaceState.executions.unshift(binding);
  }

  async function multicaWorkspaceLoadExecutions(force = false) {
    if (!multicaWorkspaceState.workspaceId || (multicaWorkspaceState.executionsLoading && !force)) return;
    const sequence = ++multicaWorkspaceState.executionSeq;
    multicaWorkspaceState.executionsLoading = true;
    multicaWorkspaceState.executionsError = "";
    if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    try {
      const result = await multicaWorkspaceCall("/multica/executions/list", {
        workspaceId: multicaWorkspaceState.workspaceId,
        limit: 100,
        offset: 0,
      });
      if (sequence !== multicaWorkspaceState.executionSeq) return;
      if (!Array.isArray(result.items)) throw new Error("execution_list_invalid");
      multicaWorkspaceState.executions = result.items;
    } catch (error) {
      if (sequence === multicaWorkspaceState.executionSeq) multicaWorkspaceState.executionsError = multicaWorkspaceErrorMessage(error);
    } finally {
      if (sequence === multicaWorkspaceState.executionSeq) {
        multicaWorkspaceState.executionsLoading = false;
        if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
      }
    }
  }

  function multicaWorkspaceOpenExecutionDraft(mode, issue, binding = null) {
    const issueId = multicaWorkspaceEntityId(issue);
    if (!issueId) return;
    multicaWorkspaceState.executionDraft = {
      mode,
      issue,
      issueId,
      binding,
      prompt: mode === "continue" ? "" : multicaWorkspaceIssuePrompt(issue),
      idempotencyKey: multicaWorkspaceCommandId(mode, issueId),
      message: "",
    };
    multicaWorkspaceState.executionNotice = null;
    multicaWorkspaceRenderContent();
  }

  function multicaWorkspaceRenderExecutionDraft(parent, issue) {
    const draft = multicaWorkspaceState.executionDraft;
    if (!draft || draft.issueId !== multicaWorkspaceEntityId(issue)) return;
    const form = multicaWorkspaceEl("form", "ccp-multica-form");
    form.appendChild(multicaWorkspaceEl("h4", "ccp-multica-form-title", ({ create: "执行任务", continue: "继续执行", retry: "创建新 attempt" })[draft.mode] || "执行"));
    const field = multicaWorkspaceEl("label", "ccp-multica-form-field");
    field.dataset.wide = "true";
    field.appendChild(multicaWorkspaceEl("span", "", "本次指令"));
    const prompt = multicaWorkspaceEl("textarea", "ccp-multica-textarea");
    prompt.value = draft.prompt;
    prompt.maxLength = 32000;
    prompt.setAttribute("aria-label", "本次执行指令");
    prompt.addEventListener("input", () => { draft.prompt = prompt.value; draft.message = ""; });
    field.appendChild(prompt);
    form.appendChild(field);
    const actions = multicaWorkspaceEl("div", "ccp-multica-form-actions");
    const submit = multicaWorkspaceEl("button", "ccp-multica-button", "确认执行");
    submit.type = "submit";
    submit.dataset.variant = "primary";
    const close = multicaWorkspaceEl("button", "ccp-multica-button", "取消");
    close.type = "button";
    close.addEventListener("click", () => { multicaWorkspaceState.executionDraft = null; multicaWorkspaceRenderContent(); });
    actions.append(submit, close);
    if (draft.message) {
      const message = multicaWorkspaceEl("span", "ccp-multica-inline-message", draft.message);
      message.dataset.state = "error";
      actions.appendChild(message);
    }
    form.appendChild(actions);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void multicaWorkspaceSubmitExecutionDraft();
    });
    parent.appendChild(form);
  }

  async function multicaWorkspaceSubmitExecutionDraft() {
    const draft = multicaWorkspaceState.executionDraft;
    if (!draft) return;
    const prompt = String(draft.prompt || "").trim();
    if (!prompt) {
      draft.message = "请输入本次执行指令";
      multicaWorkspaceRenderContent();
      return;
    }
    if (draft.mode === "continue") {
      await multicaWorkspaceRunExecutionAction("continue", draft.issue, draft.binding, { prompt, idempotencyKey: draft.idempotencyKey });
    } else {
      await multicaWorkspaceRunExecutionAction("create", draft.issue, draft.binding, { prompt, idempotencyKey: draft.idempotencyKey });
    }
  }

  function multicaWorkspaceThreadIdMatches(value, threadId) {
    const candidate = String(value || "").trim();
    const expected = String(threadId || "").trim();
    return !!candidate && !!expected && (candidate === expected || candidate.endsWith(`:${expected}`));
  }

  function multicaWorkspaceNativeThreadRow(threadId) {
    return Array.from(document.querySelectorAll?.("[data-app-action-sidebar-thread-id]") || [])
      .find((row) => multicaWorkspaceThreadIdMatches(row.getAttribute("data-app-action-sidebar-thread-id"), threadId)) || null;
  }

  function multicaWorkspaceNativeThreadIsActive(row) {
    if (!row?.isConnected) return false;
    const active = row.matches?.('[data-app-action-sidebar-thread-active="true"], [aria-current="page"]')
      || row.querySelector?.('[data-app-action-sidebar-thread-active="true"], [aria-current="page"]');
    return !!active;
  }

  function multicaWorkspaceCurrentNativeThreadId() {
    const active = Array.from(document.querySelectorAll?.("[data-app-action-sidebar-thread-id]") || [])
      .find(multicaWorkspaceNativeThreadIsActive);
    return active?.getAttribute?.("data-app-action-sidebar-thread-id") || "";
  }

  async function multicaWorkspaceActivateNativeThread(threadId) {
    const row = multicaWorkspaceNativeThreadRow(threadId);
    if (!row) throw new Error("未在 Codex 侧栏找到该对话");
    const clickTarget = row.matches?.('button, a[href], [role="button"], [role="link"]')
      ? row
      : row.closest?.('button, a[href], [role="button"], [role="link"]') || row;
    multicaWorkspaceState.nativeThreadActivation = true;
    try {
      clickTarget.click?.();
      const deadline = Date.now() + 3000;
      while (Date.now() < deadline) {
        const current = multicaWorkspaceNativeThreadRow(threadId) || row;
        const activeThreadId = multicaWorkspaceCurrentNativeThreadId();
        if (multicaWorkspaceNativeThreadIsActive(current) &&
            multicaWorkspaceThreadIdMatches(activeThreadId, threadId)) {
          multicaWorkspaceHide();
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, 80));
      }
      throw new Error("Codex 未激活目标对话");
    } finally {
      multicaWorkspaceState.nativeThreadActivation = false;
    }
  }

  async function multicaWorkspaceRunExecutionAction(action, issue, binding, extras = {}) {
    const issueId = multicaWorkspaceEntityId(issue);
    const bindingId = multicaWorkspaceExecutionBindingId(binding);
    const busyKey = `${action}:${bindingId || issueId}`;
    if (!issueId || multicaWorkspaceState.executionBusy.has(busyKey)) return;
    let path;
    let payload;
    if (action === "create") {
      path = "/multica/executions/create";
      const assigneeKind = String(multicaWorkspaceObjectValue(issue, "assignee_type", "assigneeKind", "assignee_kind") || "").toLowerCase();
      const assigneeId = String(multicaWorkspaceObjectValue(issue, "assignee_id", "assigneeId") || "").trim();
      const parentThreadId = multicaWorkspaceCurrentNativeThreadId();
      payload = {
        workspaceId: multicaWorkspaceState.workspaceId,
        issueId,
        prompt: extras.prompt,
        idempotencyKey: extras.idempotencyKey,
      };
      // Match Multica's native assignment flow: an agent-owned issue is
      // dispatched as a child of the currently open Codex conversation.
      if (assigneeKind === "agent" && assigneeId) {
        payload.executionKind = "subagent";
        payload.agentId = assigneeId;
        payload.parentThreadId = parentThreadId || undefined;
      }
    } else if (action === "continue") {
      path = "/multica/executions/continue";
      payload = {
        bindingId,
        prompt: extras.prompt,
        idempotencyKey: extras.idempotencyKey,
        expectedRevision: multicaWorkspaceEntityRevision(binding),
      };
    } else if (action === "cancel") {
      path = "/multica/executions/cancel";
      payload = { bindingId, idempotencyKey: multicaWorkspaceCommandId("cancel", bindingId), expectedRevision: multicaWorkspaceEntityRevision(binding) };
    } else if (action === "status") {
      path = "/multica/executions/status";
      payload = { bindingId };
    } else {
      path = "/multica/executions/open";
      payload = { bindingId };
    }
    if (!path || (action !== "create" && !bindingId)) return;
    multicaWorkspaceState.executionBusy.add(busyKey);
    multicaWorkspaceState.executionNotice = { state: "loading", message: "正在处理…" };
    multicaWorkspaceRenderContent();
    try {
      const result = await multicaWorkspaceCall(path, payload, action === "create" || action === "continue" ? 30000 : 15000);
      if (result.binding) multicaWorkspaceMergeExecution(result.binding);
      if (action === "create" || action === "continue") multicaWorkspaceState.executionDraft = null;
      if (action === "open") {
        const threadId = String(multicaWorkspaceObjectValue(
          result.handle,
          "threadId",
          "thread_id",
        ) || multicaWorkspaceObjectValue(
          result.binding || binding,
          "codexThreadId",
          "codex_thread_id",
        ) || "").trim();
        if (!threadId) throw new Error("执行记录缺少 Codex 对话 ID");
        await multicaWorkspaceActivateNativeThread(threadId);
      }
      multicaWorkspaceState.executionNotice = { state: "ok", message: ({ create: "已派发", continue: "已继续执行", cancel: "取消请求已提交", status: "状态已更新", open: "已打开原对话" })[action] };
      if (action !== "open") await multicaWorkspaceLoadExecutions(true);
    } catch (error) {
      const message = multicaWorkspaceErrorMessage(error);
      if (multicaWorkspaceState.executionDraft && (action === "create" || action === "continue")) multicaWorkspaceState.executionDraft.message = message;
      multicaWorkspaceState.executionNotice = { state: "error", message };
    } finally {
      multicaWorkspaceState.executionBusy.delete(busyKey);
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    }
  }

  function multicaWorkspaceAppendExecutionAttempts(parent, issue) {
    const issueId = multicaWorkspaceEntityId(issue);
    const attempts = multicaWorkspaceExecutionsForIssue(issueId);
    const draft = multicaWorkspaceState.executionDraft;
    if (!attempts.length && (!draft || draft.issueId !== issueId)) return;
    const section = multicaWorkspaceEl("div", "ccp-multica-executions");
    attempts.forEach((binding) => {
      const bindingId = multicaWorkspaceExecutionBindingId(binding);
      const state = multicaWorkspaceExecutionState(binding);
      const terminal = multicaWorkspaceTerminalExecutionStates.has(state);
      const row = multicaWorkspaceEl("div", "ccp-multica-execution");
      const summary = multicaWorkspaceEl("div", "ccp-multica-execution-summary");
      const attemptNo = multicaWorkspaceObjectValue(binding, "attemptNo", "attempt_no") || 1;
      summary.append(
        multicaWorkspaceEl("span", "ccp-multica-badge", `Attempt ${attemptNo}`),
        multicaWorkspaceEl("span", "ccp-multica-badge", state),
      );
      const agentId = multicaWorkspaceObjectValue(binding, "agentId", "agent_id");
      const parentThreadId = multicaWorkspaceObjectValue(binding, "parentThreadId", "parent_thread_id");
      if (agentId) summary.appendChild(multicaWorkspaceEl("span", "ccp-multica-badge", `Agent ${agentId}`));
      if (parentThreadId) summary.appendChild(multicaWorkspaceEl("span", "ccp-multica-badge", `父会话 ${parentThreadId}`));
      const errorCode = multicaWorkspaceObjectValue(binding, "lastErrorCode", "last_error_code");
      if (errorCode) summary.appendChild(multicaWorkspaceEl("span", "ccp-multica-stale", String(errorCode)));
      row.appendChild(summary);
      const actions = multicaWorkspaceEl("div", "ccp-multica-item-actions");
      const addAction = (label, action, enabled = true) => {
        const button = multicaWorkspaceEl("button", "ccp-multica-button", label);
        button.type = "button";
        button.disabled = !enabled || multicaWorkspaceState.executionBusy.has(`${action}:${bindingId || issueId}`);
        button.addEventListener("click", () => {
          if (action === "continue" || action === "retry") multicaWorkspaceOpenExecutionDraft(action, issue, binding);
          else void multicaWorkspaceRunExecutionAction(action, issue, binding);
        });
        actions.appendChild(button);
      };
      addAction("打开对话", "open", !!multicaWorkspaceObjectValue(binding, "codexThreadId", "codex_thread_id"));
      addAction("刷新状态", "status", !!bindingId);
      if (!terminal) {
        addAction("继续", "continue", !!multicaWorkspaceObjectValue(binding, "codexThreadId", "codex_thread_id"));
        addAction("取消", "cancel", !!bindingId);
      } else {
        addAction("重跑", "retry", true);
      }
      row.appendChild(actions);
      section.appendChild(row);
    });
    multicaWorkspaceRenderExecutionDraft(section, issue);
    parent.appendChild(section);
  }

  function multicaWorkspaceAppendEntityItem(parent, item, module) {
    const resource = multicaWorkspaceWritableResource(module);
    const article = multicaWorkspaceEl("article", "ccp-multica-item");
    const heading = multicaWorkspaceEl("div", "ccp-multica-item-heading");
    heading.appendChild(multicaWorkspaceEl("h3", "ccp-multica-item-title", multicaWorkspaceItemTitle(item)));
    const actions = multicaWorkspaceEl("div", "ccp-multica-item-actions");
    const add = (label, handler, variant) => {
      const button = multicaWorkspaceEl("button", "ccp-multica-button", label);
      button.type = "button";
      button.disabled = multicaWorkspaceState.mutationBusy;
      if (variant) button.dataset.variant = variant;
      button.addEventListener("click", handler);
      actions.appendChild(button);
    };
    if (resource === "issues") {
      const attempts = multicaWorkspaceExecutionsForIssue(multicaWorkspaceEntityId(item));
      const active = attempts.find((binding) => !multicaWorkspaceTerminalExecutionStates.has(multicaWorkspaceExecutionState(binding)));
      if (!active) add("执行", () => multicaWorkspaceOpenExecutionDraft("create", item), "primary");
      const archived = String(multicaWorkspaceObjectValue(item, "status") || "") === "archived";
      add(archived ? "恢复" : "归档", () => void multicaWorkspacePatchEntity(module, item, { status: archived ? "todo" : "archived" }, archived ? "已恢复" : "已归档"));
      const localUserId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
      if (localUserId) {
        const subscribed = (multicaWorkspaceState.collections.get("subscribers")?.items || []).some((entry) =>
          String(multicaWorkspaceObjectValue(entry, "issue_id", "issueId") || "") === multicaWorkspaceEntityId(item)
          && String(multicaWorkspaceObjectValue(entry, "user_id", "userId") || "") === localUserId,
        );
        add(subscribed ? "取消订阅" : "订阅", () => void multicaWorkspaceToggleIssueSubscription(module, item));
        if (subscribed) add("取消树订阅", () => void multicaWorkspaceUnsubscribeIssueSubtree(item));
      }
      if (localUserId) add("👍", () => void multicaWorkspaceToggleReaction("issue", item));
      add("标签", () => void multicaWorkspaceToggleLabel(module, item));
    } else if (resource === "comments") {
      const resolvedAt = multicaWorkspaceObjectValue(item, "resolved_at", "resolvedAt");
      if (resolvedAt) {
        add("取消解决", () => void multicaWorkspacePatchEntity(module, item, {
          resolved_at: null,
          resolved_by_type: null,
          resolved_by_id: null,
        }, "已取消解决"));
      } else {
        const userId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
        add("标记已解决", () => void multicaWorkspacePatchEntity(module, item, {
          resolved_at: new Date().toISOString(),
          resolved_by_type: "member",
          resolved_by_id: userId || null,
        }, "已标记为已解决"));
      }
      if (String(multicaWorkspaceState.bootstrap?.user?.id || "").trim()) {
        add("👍", () => void multicaWorkspaceToggleReaction("comment", item));
      }
    } else if (resource === "agents") {
      const enabled = multicaWorkspaceObjectValue(item, "enabled") !== false;
      add(enabled ? "停用" : "启用", () => void multicaWorkspacePatchEntity(module, item, { enabled: !enabled }, enabled ? "已停用" : "已启用"));
      add("标签", () => void multicaWorkspaceToggleLabel(module, item));
    } else if (resource === "autopilots") {
      const status = String(multicaWorkspaceObjectValue(item, "status") || "active").toLowerCase();
      const paused = status === "paused";
      const archived = status === "archived";
      const canWrite = multicaWorkspaceObjectValue(item, "can_write", "canWrite");
      const canManageAccess = multicaWorkspaceObjectValue(item, "can_manage_access", "canManageAccess");
      const writeAllowed = canWrite === undefined || canWrite === null || canWrite === true;
      const accessAllowed = canManageAccess === undefined || canManageAccess === null || canManageAccess === true;
      if (!archived) {
        if (writeAllowed) {
          add(paused ? "启用" : "暂停", () => void multicaWorkspacePatchEntity(module, item, { status: paused ? "active" : "paused" }, paused ? "已启用" : "已暂停"));
          add("立即触发", () => void multicaWorkspaceTriggerAutopilot(module, item), "primary");
          add("新增触发器", () => void multicaWorkspaceCreateAutopilotTrigger(module, item));
          if (Array.isArray(item.triggers) && item.triggers.length) {
            add("编辑触发器", () => void multicaWorkspaceUpdateAutopilotTrigger(module, item));
            add("删除触发器", () => void multicaWorkspaceDeleteAutopilotTrigger(module, item));
          }
        }
        if (accessAllowed) {
          add("管理协作者", () => void multicaWorkspaceToggleAutopilotCollaborator(module, item));
        }
      }
    }
    add("编辑", () => multicaWorkspaceOpenEditor(module, item));
    add("删除", () => void multicaWorkspaceDeleteEntity(module, item), "danger");
    heading.appendChild(actions);
    article.appendChild(heading);
    const fields = multicaWorkspaceEl("div", "ccp-multica-fields");
    const keys = resource === "issues"
      ? ["status", "priority", "project_id", "projectId", "assignee_type", "assigneeKind", "assignee_id", "assigneeId", "revision"]
      : resource === "projects" ? ["status", "priority", "lead_type", "lead_id", "revision"]
        : resource === "agents" ? ["enabled", "runtime_mode", "provider", "concurrency_limit", "revision"]
          : resource === "squads" ? ["leader_id", "leaderAgentId", "revision"]
            : ["status", "trigger_kind", "triggerKind", "execution_mode", "assignee_type", "assignee_id", "last_run_status", "last_run_at", "next_run_at", "revision"];
    keys.forEach((key) => {
      const value = multicaWorkspaceObjectValue(item, key, key.replace(/[A-Z]/g, (match) => `_${match.toLowerCase()}`));
      if (value === undefined || value === null || value === "") return;
      const field = multicaWorkspaceEl("span", "ccp-multica-field");
      field.append(multicaWorkspaceEl("span", "ccp-multica-field-key", `${key}:`), document.createTextNode(multicaWorkspaceValue(value)));
      fields.appendChild(field);
    });
    if (fields.childNodes.length) article.appendChild(fields);
    if (resource === "autopilots" && Array.isArray(item.runs) && item.runs.length) {
      const history = multicaWorkspaceEl("div", "ccp-multica-inline-message", `运行历史：${item.runs.slice(0, 5).map((run) => `${run.status || "unknown"} @ ${run.triggered_at || ""}`).join("；")}`);
      article.appendChild(history);
    }
    if (resource === "issues") multicaWorkspaceAppendExecutionAttempts(article, item);
    parent.appendChild(article);
  }

  function multicaWorkspaceIssueStatus(item) {
    const raw = String(multicaWorkspaceObjectValue(item, "status", "state") || "backlog").toLowerCase();
    if (raw === "archived") return "cancelled";
    return multicaWorkspaceBoardColumns.some((column) => column.key === raw) ? raw : "backlog";
  }

  function multicaWorkspaceIssueSource() {
    const filter = multicaWorkspaceState.issueFilter;
    const sourceKey = filter === "assigned" ? "my-issues" : "issues";
    const collection = multicaWorkspaceState.collections.get(sourceKey);
    const error = multicaWorkspaceState.errors.get(sourceKey);
    let items = Array.isArray(collection?.items) ? collection.items : [];
    const localUserId = String(multicaWorkspaceState.bootstrap?.user?.id || "").trim();
    if (filter === "created") {
      items = items.filter((item) => String(multicaWorkspaceObjectValue(item, "creatorId", "creator_id", "createdBy", "created_by") || "") === localUserId);
    } else if (filter === "agents") {
      const localAssignees = new Set();
      for (const key of ["agents", "squads"]) {
        const values = multicaWorkspaceState.collections.get(key)?.items;
        if (!Array.isArray(values)) continue;
        values.forEach((item) => {
          const id = multicaWorkspaceEntityId(item);
          if (id) localAssignees.add(id);
        });
      }
      items = items.filter((item) => {
        const kind = String(multicaWorkspaceObjectValue(item, "assignee_type", "assigneeKind", "assignee_kind") || "").toLowerCase();
        const id = String(multicaWorkspaceObjectValue(item, "assigneeId", "assignee_id") || "");
        return (kind === "agent" || kind === "squad") && localAssignees.has(id);
      });
    } else if (filter === "working") {
      // Upstream exposes /api/working-agents as a separate projection. The
      // local control plane has no remote endpoint, so derive this filter only
      // from persisted non-terminal execution bindings and label it locally.
      const workingIssueIds = new Set(multicaWorkspaceState.executions
        .filter((binding) => !multicaWorkspaceTerminalExecutionStates.has(multicaWorkspaceExecutionState(binding)))
        .map((binding) => String(multicaWorkspaceObjectValue(binding, "issueId", "issue_id") || "").trim())
        .filter(Boolean));
      items = items.filter((item) => workingIssueIds.has(multicaWorkspaceEntityId(item)));
    }
    return { sourceKey, collection, error, items };
  }

  function multicaWorkspaceFormatUpdatedAt(item) {
    const value = Number(multicaWorkspaceObjectValue(item, "updatedAtMs", "updated_at_ms", "updatedAt", "updated_at"));
    if (!Number.isFinite(value) || value <= 0) return "";
    const date = new Date(value < 100000000000 ? value * 1000 : value);
    if (!Number.isFinite(date.getTime())) return "";
    try {
      return date.toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
    } catch (_) {
      return date.toISOString().slice(0, 16).replace("T", " ");
    }
  }

  async function multicaWorkspaceLoadAgentFilterDependencies(force = true) {
    const dependencies = multicaWorkspaceState.issueFilterDependencies;
    const sequence = ++dependencies.sequence;
    dependencies.loading = true;
    dependencies.error = "";
    if (multicaWorkspaceState.opened && multicaWorkspaceState.route === "my-issues") {
      multicaWorkspaceRenderContent();
    }

    const failures = [];
    // multicaWorkspaceQuery owns one cancellation slot. These reads must remain
    // serial so every directory snapshot is settled before the issue filter runs.
    for (const dependency of [
      { key: "agents", label: "智能体" },
      { key: "squads", label: "小队" },
      { key: "issues", label: "任务" },
    ]) {
      if (sequence !== dependencies.sequence ||
          multicaWorkspaceState.issueFilter !== "agents" ||
          window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) {
        return;
      }
      await multicaWorkspaceQuery(moduleForMulticaWorkspace(dependency.key), force);
      const error = multicaWorkspaceState.errors.get(dependency.key);
      if (error) failures.push(`${dependency.label}：${multicaWorkspaceErrorMessage(error)}`);
    }

    if (sequence !== dependencies.sequence ||
        multicaWorkspaceState.issueFilter !== "agents" ||
        window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) {
      return;
    }
    dependencies.loading = false;
    dependencies.error = failures.length ? failures.join("；") : "";
    if (multicaWorkspaceState.opened && multicaWorkspaceState.route === "my-issues") {
      multicaWorkspaceRenderContent();
    }
  }

  function multicaWorkspaceRefreshBoardSource(force = true) {
    if (multicaWorkspaceState.issueFilter === "agents") {
      void multicaWorkspaceLoadAgentFilterDependencies(force);
      return;
    }
    const dependencies = multicaWorkspaceState.issueFilterDependencies;
    dependencies.sequence += 1;
    dependencies.loading = false;
    dependencies.error = "";
    const source = multicaWorkspaceIssueSource();
    const module = moduleForMulticaWorkspace(source.sourceKey);
    void multicaWorkspaceQuery(module, force);
  }

  function multicaWorkspaceAppendBoardCard(parent, issue, module) {
    const issueId = multicaWorkspaceEntityId(issue);
    const article = multicaWorkspaceEl("article", "ccp-multica-card");
    article.tabIndex = 0;
    article.dataset.multicaIssueId = issueId;
    article.dataset.multicaIssueStatus = multicaWorkspaceIssueStatus(issue);
    article.draggable = !multicaWorkspaceState.mutationBusy;
    article.addEventListener("dragstart", (event) => {
      if (!article.draggable || !issueId) {
        event.preventDefault();
        return;
      }
      multicaWorkspaceState.draggedIssue = issue;
      article.dataset.dragging = "true";
      event.dataTransfer?.setData("text/plain", issueId);
      if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
    });
    article.addEventListener("dragend", () => {
      multicaWorkspaceState.draggedIssue = null;
      delete article.dataset.dragging;
    });
    article.appendChild(multicaWorkspaceEl("div", "ccp-multica-card-id", issueId || "未编号"));
    article.appendChild(multicaWorkspaceEl("h3", "ccp-multica-card-title", multicaWorkspaceItemTitle(issue)));
    const summary = String(multicaWorkspaceObjectValue(issue, "description", "summary") || "").trim();
    if (summary) article.appendChild(multicaWorkspaceEl("div", "ccp-multica-card-summary", summary.slice(0, 280)));
    const meta = multicaWorkspaceEl("div", "ccp-multica-card-meta");
    const assignee = String(multicaWorkspaceObjectValue(issue, "assigneeId", "assignee_id") || "未分配");
    meta.appendChild(multicaWorkspaceEl("span", "", assignee));
    const updatedAt = multicaWorkspaceFormatUpdatedAt(issue);
    if (updatedAt) meta.appendChild(multicaWorkspaceEl("span", "", updatedAt));
    const attempts = multicaWorkspaceExecutionsForIssue(issueId);
    const latest = attempts[0] || null;
    if (latest) meta.appendChild(multicaWorkspaceEl("span", "ccp-multica-badge", multicaWorkspaceExecutionState(latest)));
    article.appendChild(meta);
    const actions = multicaWorkspaceEl("div", "ccp-multica-card-actions");
    const addAction = (label, handler, options = {}) => {
      const button = multicaWorkspaceEl("button", "ccp-multica-button", label);
      button.type = "button";
      button.disabled = !!options.disabled || multicaWorkspaceState.mutationBusy;
      if (options.variant) button.dataset.variant = options.variant;
      if (options.title) button.title = options.title;
      button.addEventListener("click", handler);
      actions.appendChild(button);
    };
    const active = attempts.find((binding) => !multicaWorkspaceTerminalExecutionStates.has(multicaWorkspaceExecutionState(binding)));
    const pageHostAvailable = multicaWorkspaceState.bootstrap?.runtime?.available !== false;
    const pageHostUnavailableTitle = "当前 Codex 页面执行能力不可用；本地任务仍可查看、编辑和流转";
    if (!active) addAction("执行", () => multicaWorkspaceOpenExecutionDraft("create", issue), {
      variant: "primary",
      disabled: !pageHostAvailable,
      title: pageHostAvailable ? "创建一次 Codex 执行" : pageHostUnavailableTitle,
    });
    if (latest && multicaWorkspaceObjectValue(latest, "codexThreadId", "codex_thread_id")) {
      addAction("打开对话", () => void multicaWorkspaceRunExecutionAction("open", issue, latest), {
        disabled: !pageHostAvailable,
        title: pageHostAvailable ? "打开已绑定的 Codex 对话" : pageHostUnavailableTitle,
      });
    }
    addAction("编辑", () => multicaWorkspaceOpenEditor(module, issue));
    const cancelled = multicaWorkspaceIssueStatus(issue) === "cancelled";
    addAction(cancelled ? "恢复" : "取消", () => void multicaWorkspacePatchEntity(module, issue, { status: cancelled ? "todo" : "cancelled" }, cancelled ? "已恢复" : "已取消"));
    article.appendChild(actions);
    if (multicaWorkspaceState.executionDraft?.issueId === issueId) {
      multicaWorkspaceRenderExecutionDraft(article, issue);
    }
    parent.appendChild(article);
  }

  function multicaWorkspaceNativeSessionRows() {
    return Array.from(document.querySelectorAll?.("[data-app-action-sidebar-thread-id]") || [])
      .filter((row) => String(row.getAttribute?.("data-app-action-sidebar-thread-id") || "").trim())
      .slice(0, 100);
  }

  function multicaWorkspaceRenderNativeInventory(parent) {
    const section = multicaWorkspaceEl("section", "ccp-multica-native-inventory");
    section.appendChild(multicaWorkspaceEl("h3", "ccp-multica-native-inventory-title", "Codex 原生会话与智能体"));
    const projectGroup = multicaWorkspaceEl("div", "ccp-multica-native-inventory-group");
    const projects = nativeProjectTargets();
    const nativeProjectSnapshot = multicaWorkspaceState.bootstrap?.collections?.codex_native_projects?.items || [];
    const projectCount = Math.max(projects.length, nativeProjectSnapshot.length);
    projectGroup.appendChild(multicaWorkspaceEl("h4", "ccp-multica-native-inventory-label", `原生项目（${projectCount}）`));
    if (projects.length === 0 && nativeProjectSnapshot.length === 0) {
      projectGroup.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "当前页面没有可读取的原生项目"));
    } else {
      const list = multicaWorkspaceEl("div", "ccp-multica-native-session-list");
      projects.forEach((project) => {
        const button = multicaWorkspaceEl("button", "ccp-multica-native-session", normalizeProjectLabel(project.label) || displayProjectName(project.path));
        button.type = "button";
        button.title = `打开原生项目 ${project.path}`;
        button.addEventListener("click", () => {
          if (!project.row?.isConnected) {
            button.disabled = true;
            button.title = "原生项目已不可用，请刷新页面";
            return;
          }
          project.row.click();
        });
        list.appendChild(button);
      });
      if (projects.length === 0) {
        nativeProjectSnapshot.slice(0, 100).forEach((project) => {
          const path = String(project.path || "").trim();
          const item = multicaWorkspaceEl("span", "ccp-multica-native-session", displayProjectName(path));
          item.title = `来自 Codex 本机状态库的只读项目快照：${path}`;
          list.appendChild(item);
        });
      }
      projectGroup.appendChild(list);
    }
    section.appendChild(projectGroup);
    const nativeThreads = multicaWorkspaceState.bootstrap?.collections?.codex_native_threads?.items || [];
    const sessions = multicaWorkspaceNativeSessionRows();
    const sessionGroup = multicaWorkspaceEl("div", "ccp-multica-native-inventory-group");
    const sessionCount = Math.max(sessions.length, nativeThreads.length);
    sessionGroup.appendChild(multicaWorkspaceEl("h4", "ccp-multica-native-inventory-label", `原生会话（${sessionCount}）`));
    if (sessions.length === 0 && nativeThreads.length === 0) {
      sessionGroup.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "当前页面没有可读取的原生会话"));
    } else {
      const list = multicaWorkspaceEl("div", "ccp-multica-native-session-list");
      sessions.forEach((row) => {
        const threadId = String(row.getAttribute("data-app-action-sidebar-thread-id") || "").trim();
        const label = String(row.getAttribute("aria-label") || row.textContent || threadId).replace(/\s+/g, " ").trim().slice(0, 160) || threadId;
        const button = multicaWorkspaceEl("button", "ccp-multica-native-session", label);
        button.type = "button";
        button.title = `打开原生会话 ${threadId}`;
        button.dataset.threadId = threadId;
        button.setAttribute("aria-current", row.matches?.('[aria-current="page"], [data-app-action-sidebar-thread-active="true"]') ? "page" : "false");
        button.addEventListener("click", () => {
          if (!row.isConnected) {
            button.disabled = true;
            button.title = "原生会话已不可用，请刷新页面";
            return;
          }
          row.click();
        });
        list.appendChild(button);
      });
      if (sessions.length === 0) {
        nativeThreads.slice(0, 100).forEach((thread) => {
          const id = String(thread.id || "").trim();
          const title = String(thread.title || id).replace(/\s+/g, " ").trim().slice(0, 160) || id;
          const item = multicaWorkspaceEl("span", "ccp-multica-native-session", `${title} · ${id}`);
          item.title = "来自 Codex 本机状态库的只读快照；当前页面没有可点击的原生行";
          list.appendChild(item);
        });
      }
      sessionGroup.appendChild(list);
    }
    section.appendChild(sessionGroup);
    const toolCalls = multicaWorkspaceState.bootstrap?.collections?.codex_native_tool_calls?.items || [];
    const toolGroup = multicaWorkspaceEl("div", "ccp-multica-native-inventory-group");
    toolGroup.appendChild(multicaWorkspaceEl("h4", "ccp-multica-native-inventory-label", `原生工具/智能体事件（${toolCalls.length}）`));
    if (toolCalls.length === 0) {
      toolGroup.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "当前本机历史库没有可读取的工具或智能体事件"));
    } else {
      const list = multicaWorkspaceEl("div", "ccp-multica-native-session-list");
      toolCalls.slice(0, 100).forEach((call) => {
        const name = String(call.name || "未命名工具").trim();
        const threadId = String(call.thread_id || "").trim();
        const type = String(call.item_type || "tool").trim();
        const item = multicaWorkspaceEl("span", "ccp-multica-native-session", `${name} · ${type}${threadId ? ` · ${threadId}` : ""}`);
        item.title = "来自 Codex thread_history 的只读原生事件摘要";
        list.appendChild(item);
      });
      toolGroup.appendChild(list);
    }
    section.appendChild(toolGroup);
    const nativeEvents = multicaWorkspaceState.bootstrap?.collections?.codex_native_events?.items || [];
    const eventGroup = multicaWorkspaceEl("div", "ccp-multica-native-inventory-group");
    eventGroup.appendChild(multicaWorkspaceEl("h4", "ccp-multica-native-inventory-label", `原生事件时间线（${nativeEvents.length}）`));
    if (nativeEvents.length === 0) {
      eventGroup.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "当前本机历史库没有可读取的原生事件"));
    } else {
      const list = multicaWorkspaceEl("div", "ccp-multica-native-session-list");
      nativeEvents.slice(0, 100).forEach((event) => {
        const type = String(event.item_type || "event").trim();
        const summary = String(event.summary || "").replace(/\s+/g, " ").trim();
        const threadId = String(event.thread_id || "").trim();
        const label = `${type}${summary ? ` · ${summary}` : ""}${threadId ? ` · ${threadId}` : ""}`.slice(0, 220);
        const item = multicaWorkspaceEl("span", "ccp-multica-native-session", label);
        item.title = "来自 Codex thread_history 的只读原生事件；不可作为可调度任务直接执行";
        list.appendChild(item);
      });
      eventGroup.appendChild(list);
    }
    section.appendChild(eventGroup);
    const nativeSkills = multicaWorkspaceState.bootstrap?.collections?.codex_native_skills?.items || [];
    const skillGroup = multicaWorkspaceEl("div", "ccp-multica-native-inventory-group");
    skillGroup.appendChild(multicaWorkspaceEl("h4", "ccp-multica-native-inventory-label", `原生 Skills（${nativeSkills.length}）`));
    if (nativeSkills.length === 0) {
      skillGroup.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "当前本机没有可读取的 Codex Skill 清单"));
    } else {
      const list = multicaWorkspaceEl("div", "ccp-multica-native-session-list");
      nativeSkills.slice(0, 100).forEach((skill) => {
        const name = String(skill.title || skill.name || "未命名 Skill").trim();
        const id = String(skill.name || "").trim();
        const item = multicaWorkspaceEl("span", "ccp-multica-native-session", id ? `${name} · ${id}` : name);
        item.title = String(skill.description || "来自 ~/.codex/skills 的只读元数据");
        list.appendChild(item);
      });
      skillGroup.appendChild(list);
    }
    section.appendChild(skillGroup);
    const agentGroup = multicaWorkspaceEl("div", "ccp-multica-native-inventory-group");
    const nativeAgents = multicaWorkspaceState.bootstrap?.collections?.codex_native_agents?.items || [];
    agentGroup.appendChild(multicaWorkspaceEl("h4", "ccp-multica-native-inventory-label", `原生智能体（${nativeAgents.length}）`));
    const agents = multicaWorkspaceState.collections.get("agents")?.items || [];
    // Only project agents that have a persisted Codex thread mapping.  A
    // local Agent definition alone is not evidence that the current Codex
    // Host can execute it, so unbound definitions must stay out of this
    // native inventory surface.
    const boundAgentIds = new Set(multicaWorkspaceState.executions
      .filter((binding) => String(multicaWorkspaceObjectValue(binding, "codexThreadId", "codex_thread_id") || "").trim())
      .map((binding) => multicaWorkspaceObjectValue(binding, "agentId", "agent_id"))
      .filter(Boolean));
    const boundAgents = agents.filter((agent) => boundAgentIds.has(multicaWorkspaceObjectValue(agent, "id")));
    if (boundAgents.length === 0 && nativeAgents.length === 0) {
      const supported = multicaWorkspaceState.bootstrap?.runtime?.multiAgentSupported === true;
      agentGroup.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", supported ? "暂无已绑定的本地智能体" : "当前页面未提供可核实的原生智能体绑定"));
    } else {
      const list = multicaWorkspaceEl("div", "ccp-multica-native-agent-list");
      nativeAgents.slice(0, 100).forEach((agent) => {
        const label = String(agent.title || agent.id || "未命名智能体").trim();
        const item = multicaWorkspaceEl("span", "ccp-multica-native-agent", `${label} · Codex 原生子智能体`);
        item.title = "来自 Codex 原生线程父子关系的只读智能体投影";
        list.appendChild(item);
      });
      boundAgents.slice(0, 100).forEach((agent) => {
        const label = String(multicaWorkspaceObjectValue(agent, "name", "title") || multicaWorkspaceObjectValue(agent, "id") || "未命名智能体");
        const item = multicaWorkspaceEl("span", "ccp-multica-native-agent", `${label} · 已绑定原生执行`);
        item.title = "该智能体已有 Codex 原生执行映射";
        list.appendChild(item);
      });
      agentGroup.appendChild(list);
    }
    section.appendChild(agentGroup);
    parent.appendChild(section);
  }

  function multicaWorkspaceRenderIssueBoard(content, module) {
    const source = multicaWorkspaceIssueSource();
    const filterDependencies = multicaWorkspaceState.issueFilter === "agents"
      ? multicaWorkspaceState.issueFilterDependencies
      : null;
    const page = multicaWorkspaceEl("section", "ccp-multica-board-page");
    page.dataset.compact = String(multicaWorkspaceState.boardCompact);
    const heading = multicaWorkspaceEl("div", "ccp-multica-board-heading");
    heading.appendChild(multicaWorkspaceEl("h2", "ccp-multica-board-title", "我的任务"));
    const headingSpacer = multicaWorkspaceEl("span", "ccp-multica-toolbar-spacer");
    heading.appendChild(headingSpacer);
    if (source.collection && Number.isFinite(Number(source.collection.total))) {
      heading.appendChild(multicaWorkspaceEl("span", "ccp-multica-count", `${source.items.length} 条`));
    }
    multicaWorkspaceAppendModuleMenu(heading);
    page.appendChild(heading);
    if (multicaWorkspaceState.bootstrapLoading && !multicaWorkspaceState.workspaceId) {
      page.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "正在连接本地任务…"));
    }
    if (multicaWorkspaceState.bootstrapError && !multicaWorkspaceState.workspaceId) {
      const state = multicaWorkspaceEl("div", "ccp-multica-inline-message", multicaWorkspaceState.bootstrapError);
      state.dataset.state = "error";
      const retry = multicaWorkspaceEl("button", "ccp-multica-button", "重试");
      retry.type = "button";
      retry.addEventListener("click", () => void multicaWorkspaceLoadCurrentRoute(true));
      state.appendChild(document.createTextNode(" "));
      state.appendChild(retry);
      page.appendChild(state);
    }
    if (source.error && !source.collection) {
      const state = multicaWorkspaceEl("div", "ccp-multica-inline-message", multicaWorkspaceErrorMessage(source.error));
      state.dataset.state = "error";
      const retry = multicaWorkspaceEl("button", "ccp-multica-button", "重试");
      retry.type = "button";
      retry.addEventListener("click", () => void multicaWorkspaceQuery(moduleForMulticaWorkspace(source.sourceKey), true));
      state.appendChild(document.createTextNode(" "));
      state.appendChild(retry);
      page.appendChild(state);
    }
    if (multicaWorkspaceState.loading.has(source.sourceKey) && !source.collection) {
      page.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "正在读取任务…"));
    }
    const toolbar = multicaWorkspaceEl("div", "ccp-multica-toolbar ccp-multica-board-toolbar");
    const filters = multicaWorkspaceEl("div", "ccp-multica-toolbar-group");
    filters.setAttribute("role", "group");
    filters.setAttribute("aria-label", "任务筛选");
    multicaWorkspaceIssueFilters.forEach((filter) => {
      const button = multicaWorkspaceEl("button", "ccp-multica-filter", filter.label);
      button.type = "button";
      button.dataset.multicaIssueFilter = filter.key;
      button.setAttribute("aria-pressed", String(multicaWorkspaceState.issueFilter === filter.key));
      button.addEventListener("click", () => {
        if (multicaWorkspaceState.issueFilter === filter.key) return;
        multicaWorkspaceState.issueFilter = filter.key;
        multicaWorkspaceRenderContent();
        multicaWorkspaceRefreshBoardSource(false);
      });
      filters.appendChild(button);
    });
    toolbar.appendChild(filters);
    toolbar.appendChild(multicaWorkspaceEl("span", "ccp-multica-toolbar-spacer"));
    const working = multicaWorkspaceState.executions.filter((binding) => !multicaWorkspaceTerminalExecutionStates.has(multicaWorkspaceExecutionState(binding))).length;
    toolbar.appendChild(multicaWorkspaceEl("span", "ccp-multica-working-count", `${working} 个智能体工作中`));
    const queue = multicaWorkspaceState.bootstrap?.collections?.agent_task_queue;
    if (queue && Number.isFinite(Number(queue.total))) {
      toolbar.appendChild(multicaWorkspaceEl("span", "ccp-multica-working-count", `队列 ${queue.total} 条`));
    }
    const display = multicaWorkspaceEl("button", "ccp-multica-filter", "显示");
    display.type = "button";
    display.title = "切换卡片摘要";
    display.setAttribute("aria-pressed", String(multicaWorkspaceState.boardCompact));
    display.addEventListener("click", () => {
      multicaWorkspaceState.boardCompact = !multicaWorkspaceState.boardCompact;
      multicaWorkspaceRenderContent();
    });
    toolbar.appendChild(display);
    [
      ["board", "看板"],
      ["list", "列表"],
      ["table", "表格"],
      ["swimlane", "泳道"],
    ].forEach(([mode, label]) => {
      const modeButton = multicaWorkspaceEl("button", "ccp-multica-filter", label);
      modeButton.type = "button";
      modeButton.setAttribute("aria-pressed", String(multicaWorkspaceState.issueViewMode === mode));
      modeButton.addEventListener("click", () => {
        if (multicaWorkspaceState.issueViewMode === mode) return;
        multicaWorkspaceState.issueViewMode = mode;
        multicaWorkspaceRenderContent();
      });
      toolbar.appendChild(modeButton);
    });
    const savedViews = multicaWorkspaceEl("select", "ccp-multica-filter");
    savedViews.title = "选择本机保存视图";
    savedViews.setAttribute("aria-label", "本机保存视图");
    const emptyOption = document.createElement("option");
    emptyOption.value = "";
    emptyOption.textContent = "本机视图";
    savedViews.appendChild(emptyOption);
    multicaWorkspaceState.savedIssueViews.forEach((view) => {
      const option = document.createElement("option");
      option.value = view.id;
      option.textContent = view.name;
      savedViews.appendChild(option);
    });
    savedViews.value = multicaWorkspaceState.activeIssueViewId;
    savedViews.addEventListener("change", () => {
      if (savedViews.value) multicaWorkspaceApplySavedIssueView(savedViews.value);
    });
    toolbar.appendChild(savedViews);
    const saveView = multicaWorkspaceEl("button", "ccp-multica-filter", "保存本机视图");
    saveView.type = "button";
    saveView.title = "将当前任务筛选和显示方式保存到本机浏览器";
    saveView.addEventListener("click", multicaWorkspaceSaveCurrentIssueView);
    toolbar.appendChild(saveView);
    if (multicaWorkspaceState.activeIssueViewId) {
      const deleteView = multicaWorkspaceEl("button", "ccp-multica-icon-button", "×");
      deleteView.type = "button";
      deleteView.title = "删除当前本机视图";
      deleteView.setAttribute("aria-label", "删除当前本机视图");
      deleteView.addEventListener("click", multicaWorkspaceDeleteActiveIssueView);
      toolbar.appendChild(deleteView);
    }
    const refresh = multicaWorkspaceEl("button", "ccp-multica-icon-button", "↻");
    refresh.type = "button";
    refresh.title = "刷新任务";
    refresh.setAttribute("aria-label", "刷新任务");
    refresh.addEventListener("click", () => multicaWorkspaceRefreshBoardSource(true));
    toolbar.appendChild(refresh);
    page.appendChild(toolbar);
    multicaWorkspaceRenderNativeInventory(page);
    const queueItems = multicaWorkspaceState.bootstrap?.collections?.agent_task_queue?.items || [];
    if (queueItems.length > 0) {
      const queueSection = multicaWorkspaceEl("section", "ccp-multica-native-inventory-group");
      queueSection.appendChild(multicaWorkspaceEl("h3", "ccp-multica-native-inventory-label", "Codex 任务队列"));
      const queueList = multicaWorkspaceEl("div", "ccp-multica-native-session-list");
      queueItems.slice(0, 50).forEach((task) => {
        const status = String(task.status || "unknown");
        const attempt = String(task.attempt || "1");
        const failure = String(task.failure_reason || "").trim();
        const row = multicaWorkspaceEl("div", "ccp-multica-native-session", `${status} · 第 ${attempt} 次尝试${failure ? ` · ${failure}` : ""}`);
        row.title = `队列项 ${String(task.id || "")}，来源：${String(task.source || "")}`;
        queueList.appendChild(row);
      });
      queueSection.appendChild(queueList);
      page.appendChild(queueSection);
    }
    if (multicaWorkspaceState.mutationNotice?.message) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", multicaWorkspaceState.mutationNotice.message);
      notice.setAttribute("role", "status");
      if (multicaWorkspaceState.mutationNotice.state === "error") notice.dataset.state = "error";
      page.appendChild(notice);
    }
    if (multicaWorkspaceState.executionNotice?.message) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", multicaWorkspaceState.executionNotice.message);
      notice.setAttribute("role", "status");
      if (multicaWorkspaceState.executionNotice.state === "error") notice.dataset.state = "error";
      page.appendChild(notice);
    }
    if (multicaWorkspaceState.executionsError) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", `执行记录：${multicaWorkspaceState.executionsError}`);
      notice.dataset.state = "error";
      page.appendChild(notice);
    }
    if (multicaWorkspaceState.bootstrap?.runtime?.available === false) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", "当前 Codex 执行能力不可用，本地任务仍可查看和编辑");
      notice.dataset.state = "warning";
      page.appendChild(notice);
    }
    if (filterDependencies?.loading) {
      page.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message", "正在读取智能体和小队…"));
    } else if (filterDependencies?.error) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", `读取智能体和小队失败：${filterDependencies.error}`);
      notice.dataset.state = "error";
      const retry = multicaWorkspaceEl("button", "ccp-multica-button", "重试");
      retry.type = "button";
      retry.addEventListener("click", () => multicaWorkspaceRefreshBoardSource(true));
      notice.appendChild(document.createTextNode(" "));
      notice.appendChild(retry);
      page.appendChild(notice);
    }
    if (source.collection?.stale) {
      page.appendChild(multicaWorkspaceEl("div", "ccp-multica-inline-message ccp-multica-stale", "数据待同步"));
    }
    const assignedFilterEmpty = multicaWorkspaceState.issueFilter === "assigned" &&
      !source.error && source.collection && Array.isArray(source.collection.items) && source.items.length === 0;
    if (assignedFilterEmpty) {
      const empty = multicaWorkspaceEl(
        "div",
        "ccp-multica-inline-message ccp-multica-assigned-empty",
        "当前没有分配给本地用户的任务，可切换“全部”查看任务。",
      );
      empty.dataset.multicaAssignedEmpty = "true";
      const viewAll = multicaWorkspaceEl("button", "ccp-multica-button", "查看全部任务");
      viewAll.type = "button";
      viewAll.title = "切换到全部任务";
      viewAll.addEventListener("click", () => {
        multicaWorkspaceState.issueFilter = "all";
        multicaWorkspaceRenderContent();
        multicaWorkspaceRefreshBoardSource(false);
      });
      empty.appendChild(document.createTextNode(" "));
      empty.appendChild(viewAll);
      page.appendChild(empty);
    }
    multicaWorkspaceRenderEditor(page, module);
    const scroll = multicaWorkspaceEl("div", "ccp-multica-board-scroll");
    if (multicaWorkspaceState.issueViewMode === "list" || multicaWorkspaceState.issueViewMode === "table") {
      const list = multicaWorkspaceEl("div", `ccp-multica-issue-${multicaWorkspaceState.issueViewMode}`);
      if (!source.items.length) list.appendChild(multicaWorkspaceEl("div", "ccp-multica-column-empty", "无任务"));
      source.items.forEach((issue) => {
        const id = multicaWorkspaceEntityId(issue);
        const status = multicaWorkspaceBoardColumns.find((column) => column.key === multicaWorkspaceIssueStatus(issue))?.label || multicaWorkspaceIssueStatus(issue);
        const priority = String(multicaWorkspaceObjectValue(issue, "priority") || "-");
        const assignee = String(multicaWorkspaceObjectValue(issue, "assigneeId", "assignee_id") || "未分配");
        const project = String(multicaWorkspaceObjectValue(issue, "projectId", "project_id") || "-");
        if (multicaWorkspaceState.issueViewMode === "table") {
          const row = multicaWorkspaceEl("div", "ccp-multica-table-row");
          [multicaWorkspaceItemTitle(issue), status, priority, assignee, project, multicaWorkspaceFormatUpdatedAt(issue) || "-"].forEach((value) => row.appendChild(multicaWorkspaceEl("span", "ccp-multica-table-cell", value)));
          row.title = id;
          row.addEventListener("dblclick", () => multicaWorkspaceOpenEditor(module, issue));
          list.appendChild(row);
        } else {
          const row = multicaWorkspaceEl("div", "ccp-multica-issue-list-row");
          row.appendChild(multicaWorkspaceEl("strong", "ccp-multica-issue-list-title", multicaWorkspaceItemTitle(issue)));
          row.appendChild(multicaWorkspaceEl("span", "ccp-multica-badge", status));
          row.appendChild(multicaWorkspaceEl("span", "ccp-multica-field", `${priority} · ${assignee} · ${project}`));
          const edit = multicaWorkspaceEl("button", "ccp-multica-button", "编辑");
          edit.type = "button";
          edit.addEventListener("click", () => multicaWorkspaceOpenEditor(module, issue));
          row.appendChild(edit);
          list.appendChild(row);
        }
      });
      if (multicaWorkspaceState.issueViewMode === "table" && source.items.length) {
        const header = multicaWorkspaceEl("div", "ccp-multica-table-row ccp-multica-table-header");
        ["标题", "状态", "优先级", "负责人", "项目", "更新时间"].forEach((value) => header.appendChild(multicaWorkspaceEl("span", "ccp-multica-table-cell", value)));
        list.insertBefore(header, list.firstChild);
      }
      scroll.appendChild(list);
      page.appendChild(scroll);
      content.appendChild(page);
      return;
    }
    if (multicaWorkspaceState.issueViewMode === "swimlane") {
      const swimlane = multicaWorkspaceEl("div", "ccp-multica-swimlane");
      multicaWorkspaceBoardColumns.forEach((column) => {
        const lane = multicaWorkspaceEl("section", "ccp-multica-swimlane-lane");
        const items = source.items.filter((item) => multicaWorkspaceIssueStatus(item) === column.key);
        lane.appendChild(multicaWorkspaceEl("h3", "ccp-multica-column-title", `${column.label} (${items.length})`));
        const list = multicaWorkspaceEl("div", "ccp-multica-column-list");
        items.forEach((item) => multicaWorkspaceAppendBoardCard(list, item, module));
        if (!items.length) list.appendChild(multicaWorkspaceEl("div", "ccp-multica-column-empty", "无任务"));
        lane.appendChild(list);
        swimlane.appendChild(lane);
      });
      scroll.appendChild(swimlane);
      page.appendChild(scroll);
      content.appendChild(page);
      return;
    }
    const board = multicaWorkspaceEl("div", "ccp-multica-board");
    board.dataset.multicaBoard = "true";
    multicaWorkspaceBoardColumns.forEach((column) => {
      const columnItems = source.items.filter((item) => multicaWorkspaceIssueStatus(item) === column.key);
      const lane = multicaWorkspaceEl("section", "ccp-multica-board-column");
      lane.dataset.multicaBoardStatus = column.key;
      lane.dataset.tone = column.tone;
      lane.addEventListener("dragover", (event) => {
        if (!multicaWorkspaceState.draggedIssue) return;
        event.preventDefault();
        if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
      });
      lane.addEventListener("drop", (event) => {
        event.preventDefault();
        const dragged = multicaWorkspaceState.draggedIssue;
        multicaWorkspaceState.draggedIssue = null;
        if (!dragged || multicaWorkspaceIssueStatus(dragged) === column.key) return;
        void multicaWorkspacePatchEntity(module, dragged, { status: column.key }, `已移至${column.label}`);
      });
      const laneHeader = multicaWorkspaceEl("div", "ccp-multica-column-header");
      laneHeader.appendChild(multicaWorkspaceEl("span", "ccp-multica-column-dot"));
      laneHeader.appendChild(multicaWorkspaceEl("h3", "ccp-multica-column-title", column.label));
      laneHeader.appendChild(multicaWorkspaceEl("span", "ccp-multica-column-count", String(columnItems.length)));
      const laneActions = multicaWorkspaceEl("div", "ccp-multica-column-actions");
      const more = multicaWorkspaceEl("button", "ccp-multica-icon-button", "…");
      more.type = "button";
      more.title = `刷新${column.label}`;
      more.setAttribute("aria-label", `刷新${column.label}`);
      more.addEventListener("click", () => multicaWorkspaceRefreshBoardSource(true));
      const create = multicaWorkspaceEl("button", "ccp-multica-icon-button", "+");
      create.type = "button";
      create.title = `新建${column.label}任务`;
      create.setAttribute("aria-label", `新建${column.label}任务`);
      create.addEventListener("click", () => multicaWorkspaceOpenEditor(module, null, { status: column.key }));
      laneActions.append(more, create);
      laneHeader.appendChild(laneActions);
      lane.appendChild(laneHeader);
      const list = multicaWorkspaceEl("div", "ccp-multica-column-list");
      if (!columnItems.length) {
        const emptyMessage = filterDependencies?.loading
          ? "正在读取智能体和小队…"
          : filterDependencies?.error
            ? "智能体和小队目录读取失败"
            : "无任务";
        list.appendChild(multicaWorkspaceEl("div", "ccp-multica-column-empty", emptyMessage));
      }
      else columnItems.forEach((item) => multicaWorkspaceAppendBoardCard(list, item, module));
      lane.appendChild(list);
      board.appendChild(lane);
    });
    scroll.appendChild(board);
    page.appendChild(scroll);
    content.appendChild(page);
  }

  function multicaWorkspaceSkillsInventoryReadOnly() {
    const runtime = multicaWorkspaceState.bootstrap?.runtime;
    const inventorySupported = runtime?.skillsInventorySupported ?? runtime?.skills_inventory_supported;
    const executionSupported = runtime?.skillsSupported ?? runtime?.skills_supported;
    return inventorySupported === true && executionSupported !== true;
  }

  function multicaWorkspaceSkillExecutionSupported(item) {
    if (item?.execution_supported === false) return false;
    return !multicaWorkspaceSkillsInventoryReadOnly();
  }

  function multicaWorkspaceAppendSkillItem(parent, item) {
    const article = multicaWorkspaceEl("article", "ccp-multica-item ccp-multica-skill-item");
    const id = typeof item?.id === "string" ? item.id : "";
    const executionSupported = multicaWorkspaceSkillExecutionSupported(item);
    const title = multicaWorkspaceEl("h3", "ccp-multica-item-title", multicaWorkspaceItemTitle(item));
    const bindings = multicaWorkspaceBindingsForSkill(id);
    const action = multicaWorkspaceEl(
      "button",
      "ccp-multica-button",
      executionSupported ? (multicaWorkspaceState.selectedSkillIds.has(id) ? "已选择" : "选择") : "仅查看",
    );
    action.type = "button";
    action.disabled = !id || !executionSupported;
    action.title = !executionSupported
      ? "当前 Codex 页面只提供只读 Skill 清单，不能选择或派发"
      : item?.dispatch_allowed === true
      ? "选择本次任务 Skill"
      : "可选择后进行解析；派发前仍需通过安装、信任和兼容性检查";
    action.addEventListener("click", () => {
      if (!id || action.disabled) return;
      if (multicaWorkspaceState.selectedSkillIds.has(id)) multicaWorkspaceState.selectedSkillIds.delete(id);
      else multicaWorkspaceState.selectedSkillIds.add(id);
      multicaWorkspaceRenderContent();
    });
    const trusted = item?.trust_state === "trusted";
    const review = multicaWorkspaceEl(
      "button",
      "ccp-multica-button",
      trusted ? "撤销信任" : "审查并信任",
    );
    review.type = "button";
    review.disabled = !id || item?.installed !== true || item?.compatible === false;
    review.title = trusted
      ? "撤销此 Skill 的 CCP 执行信任"
      : "记录本次 Skill 的明确审查决定；不会安装或执行 Skill";
    review.addEventListener("click", () => {
      if (!id || review.disabled) return;
      void multicaWorkspaceReviewSkill(id, !trusted, item?.manifest_digest);
    });
    const bind = multicaWorkspaceEl("button", "ccp-multica-button", "绑定");
    bind.type = "button";
    bind.disabled = !id || !executionSupported || item?.dispatch_allowed !== true;
    bind.title = !executionSupported
      ? "当前 Codex 页面只提供只读 Skill 清单，不能绑定或派发"
      : bind.disabled ? "请先安装并信任此 Skill" : "绑定到任务或智能体";
    bind.addEventListener("click", () => {
      if (!bind.disabled) multicaWorkspaceStartSkillBinding(id);
    });
    const actions = multicaWorkspaceEl("div", "ccp-multica-skill-actions");
    actions.append(action, review, bind);
    const heading = multicaWorkspaceEl("div", "ccp-multica-skill-heading");
    heading.append(title, actions);
    article.appendChild(heading);
    const fields = multicaWorkspaceEl("div", "ccp-multica-fields");
    [
      ["来源", item?.inventory_source],
      ["安装", item?.installed === true ? "已安装" : "未安装"],
      ["信任", item?.trust_state || "待审查"],
      ["兼容", item?.compatible === false ? "不兼容" : "待验证"],
      ["执行", executionSupported ? "可执行" : "仅只读清单，不可执行"],
      ["最近加载", item?.runtime_loaded || "无记录"],
    ].forEach(([key, value]) => {
      const text = multicaWorkspaceValue(value);
      if (!text) return;
      const field = multicaWorkspaceEl("span", "ccp-multica-field");
      field.append(multicaWorkspaceEl("span", "ccp-multica-field-key", `${key}:`), document.createTextNode(text));
      fields.appendChild(field);
    });
    if (fields.childNodes.length) article.appendChild(fields);
    if (bindings.length) {
      const bindingList = multicaWorkspaceEl("div", "ccp-multica-skill-bindings");
      bindings.forEach((binding) => {
        const scopeKind = multicaWorkspaceBindingValue(binding, "scopeKind", "scope_kind");
        const scopeId = multicaWorkspaceBindingValue(binding, "scopeId", "scope_id");
        const row = multicaWorkspaceEl("div", "ccp-multica-skill-binding");
        row.appendChild(multicaWorkspaceEl("span", "", `${multicaWorkspaceScopeLabel(scopeKind)}: ${scopeId || ""}`));
        const remove = multicaWorkspaceEl("button", "ccp-multica-button", "解绑");
        remove.type = "button";
        remove.title = "删除此 Skill 绑定";
        remove.addEventListener("click", () => void multicaWorkspaceRemoveSkillBinding(binding));
        row.appendChild(remove);
        bindingList.appendChild(row);
      });
      article.appendChild(bindingList);
    }
    const draft = multicaWorkspaceState.skillBindingDraft;
    if (draft?.skillId === id) {
      const form = multicaWorkspaceEl("div", "ccp-multica-skill-binding-form");
      const scope = document.createElement("select");
      scope.setAttribute("aria-label", "Skill 绑定范围");
      [
        ["task", "任务"],
        ["agent", "智能体"],
      ].forEach(([value, label]) => {
        const option = document.createElement("option");
        option.value = value;
        option.textContent = label;
        option.selected = value === draft.scopeKind;
        scope.appendChild(option);
      });
      scope.addEventListener("change", () => {
        const current = multicaWorkspaceState.skillBindingDraft;
        if (current?.skillId === id) {
          multicaWorkspaceState.skillBindingDraft = { ...current, scopeKind: scope.value, status: "idle", message: "" };
        }
      });
      const target = document.createElement("input");
      target.type = "text";
      target.value = draft.scopeId || "";
      target.placeholder = "目标 ID";
      target.setAttribute("aria-label", "Skill 绑定目标 ID");
      target.addEventListener("input", () => {
        const current = multicaWorkspaceState.skillBindingDraft;
        if (current?.skillId === id) multicaWorkspaceState.skillBindingDraft = { ...current, scopeId: target.value, status: "idle", message: "" };
      });
      const save = multicaWorkspaceEl("button", "ccp-multica-button", draft.status === "loading" ? "保存中…" : "保存绑定");
      save.type = "button";
      save.disabled = draft.status === "loading";
      save.addEventListener("click", () => void multicaWorkspaceSaveSkillBinding(item));
      const cancel = multicaWorkspaceEl("button", "ccp-multica-button", "取消");
      cancel.type = "button";
      cancel.disabled = draft.status === "loading";
      cancel.addEventListener("click", () => multicaWorkspaceCancelSkillBinding());
      form.append(scope, target, save, cancel);
      if (draft.message) form.appendChild(multicaWorkspaceEl("span", "ccp-multica-stale", draft.message));
      article.appendChild(form);
    }
    parent.appendChild(article);
  }

  function multicaWorkspaceBindingValue(binding, camelKey, snakeKey) {
    if (!binding || typeof binding !== "object") return undefined;
    return binding[camelKey] ?? binding[snakeKey];
  }

  // Keep the narrow route contract discoverable to the static bridge audit;
  // actual calls still pass through multicaWorkspaceCall for cancellation and
  // common error handling.
  const multicaWorkspaceSkillRouteContracts = Object.freeze([
    'postJson("/multica/skills/review"',
    'postJson("/multica/skills/bindings", {}',
    'postJson("/multica/skills/bind", payload',
    'postJson("/multica/skills/unbind", payload',
  ]);

  function multicaWorkspaceBindingsForSkill(id) {
    if (!id) return [];
    return multicaWorkspaceState.skillBindings.filter((binding) => {
      const reference = multicaWorkspaceBindingValue(binding, "skillRef", "skill_ref");
      return reference && reference.id === id;
    });
  }

  function multicaWorkspaceScopeLabel(scopeKind) {
    return ({ task: "任务", agent: "智能体" })[scopeKind] || scopeKind || "范围";
  }

  function multicaWorkspaceStartSkillBinding(id) {
    if (!id) return;
    multicaWorkspaceState.skillBindingDraft = {
      skillId: id,
      scopeKind: "task",
      scopeId: "",
      status: "idle",
      message: "",
    };
    multicaWorkspaceRenderContent();
  }

  function multicaWorkspaceCancelSkillBinding() {
    multicaWorkspaceState.skillBindingDraft = null;
    multicaWorkspaceRenderContent();
  }

  async function multicaWorkspaceLoadSkillBindings() {
    if (multicaWorkspaceState.skillBindingsLoading) return;
    multicaWorkspaceState.skillBindingsLoading = true;
    multicaWorkspaceState.skillBindingsError = "";
    try {
      const result = await multicaWorkspaceCall("/multica/skills/bindings", {});
      multicaWorkspaceState.skillBindings = Array.isArray(result.bindings) ? result.bindings : [];
    } catch (error) {
      multicaWorkspaceState.skillBindingsError = multicaWorkspaceErrorMessage(error);
    } finally {
      multicaWorkspaceState.skillBindingsLoading = false;
      if (multicaWorkspaceState.route === "skills") multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceSaveSkillBinding(item) {
    const draft = multicaWorkspaceState.skillBindingDraft;
    const id = typeof item?.id === "string" ? item.id : "";
    if (!draft || draft.skillId !== id) return;
    if (!multicaWorkspaceSkillExecutionSupported(item) || item?.dispatch_allowed !== true) {
      multicaWorkspaceState.skillBindingDraft = {
        ...draft,
        status: "failed",
        message: "当前 Codex 页面只提供只读 Skill 清单，不能绑定或派发",
      };
      multicaWorkspaceRenderContent();
      return;
    }
    const scopeId = String(draft.scopeId || "").trim();
    if (!scopeId) {
      multicaWorkspaceState.skillBindingDraft = { ...draft, status: "failed", message: "请输入绑定目标 ID" };
      multicaWorkspaceRenderContent();
      return;
    }
    const current = multicaWorkspaceBindingsForSkill(id).find((binding) =>
      multicaWorkspaceBindingValue(binding, "scopeKind", "scope_kind") === draft.scopeKind &&
      multicaWorkspaceBindingValue(binding, "scopeId", "scope_id") === scopeId,
    );
    multicaWorkspaceState.skillBindingDraft = { ...draft, status: "loading", message: "" };
    multicaWorkspaceRenderContent();
    try {
      const payload = {
        scopeKind: draft.scopeKind,
        scopeId,
        skillRef: { id },
        enabled: true,
      };
      const digest = item?.manifest_digest ?? item?.manifestDigest;
      if (typeof digest === "string" && digest) payload.skillRef.manifestDigest = digest;
      const revision = multicaWorkspaceBindingValue(current, "revision", "revision");
      if (Number.isSafeInteger(revision)) payload.expectedRevision = revision;
      await multicaWorkspaceCall("/multica/skills/bind", payload);
      multicaWorkspaceState.skillBindingDraft = null;
      await multicaWorkspaceLoadSkillBindings();
    } catch (error) {
      multicaWorkspaceState.skillBindingDraft = {
        ...draft,
        status: "failed",
        message: multicaWorkspaceErrorMessage(error),
      };
      multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceRemoveSkillBinding(binding) {
    const reference = multicaWorkspaceBindingValue(binding, "skillRef", "skill_ref");
    const scopeKind = multicaWorkspaceBindingValue(binding, "scopeKind", "scope_kind");
    const scopeId = multicaWorkspaceBindingValue(binding, "scopeId", "scope_id");
    if (!reference?.id || !scopeKind || !scopeId) return;
    try {
      const payload = { scopeKind, scopeId, skillId: reference.id };
      const revision = multicaWorkspaceBindingValue(binding, "revision", "revision");
      if (Number.isSafeInteger(revision)) payload.expectedRevision = revision;
      await multicaWorkspaceCall("/multica/skills/unbind", payload);
      await multicaWorkspaceLoadSkillBindings();
    } catch (error) {
      multicaWorkspaceState.skillBindingsError = multicaWorkspaceErrorMessage(error);
      multicaWorkspaceRenderContent();
    }
  }

  async function multicaWorkspaceReviewSkill(id, trusted, manifestDigest) {
    multicaWorkspaceState.skillReview = { status: "loading", id };
    multicaWorkspaceRenderContent();
    try {
      const payload = { id, trusted: trusted === true };
      if (typeof manifestDigest === "string" && manifestDigest) payload.manifestDigest = manifestDigest;
      const result = await multicaWorkspaceCall("/multica/skills/review", payload);
      multicaWorkspaceState.skillReview = {
        status: "ok",
        id,
        trusted: result.trusted === true,
      };
      await multicaWorkspaceQuery(moduleForMulticaWorkspace("skills"), true);
    } catch (error) {
      multicaWorkspaceState.skillReview = {
        status: "failed",
        id,
        message: multicaWorkspaceErrorMessage(error),
      };
    }
    multicaWorkspaceRenderContent();
  }

  async function multicaWorkspaceResolveSelectedSkills() {
    if (multicaWorkspaceSkillsInventoryReadOnly()) {
      multicaWorkspaceState.skillResolution = {
        status: "failed",
        message: "当前 Codex 页面只提供只读 Skill 清单，不能解析或派发",
      };
      multicaWorkspaceRenderContent();
      return;
    }
    const refs = Array.from(multicaWorkspaceState.selectedSkillIds).map((id) => ({ id }));
    if (!refs.length) {
      multicaWorkspaceState.skillResolution = { status: "failed", message: "请先选择至少一个已审查 Skill" };
      multicaWorkspaceRenderContent();
      return;
    }
    multicaWorkspaceState.skillResolution = { status: "loading" };
    multicaWorkspaceRenderContent();
    try {
      const result = await multicaWorkspaceCall("/multica/skills/resolve", { bindings: { task: refs } });
      multicaWorkspaceState.skillResolution = result;
    } catch (error) {
      multicaWorkspaceState.skillResolution = { status: "failed", message: multicaWorkspaceErrorMessage(error) };
    }
    multicaWorkspaceRenderContent();
  }

  function multicaWorkspaceErrorMessage(error) {
    const message = `${error?.code || error?.errorCode || ""} ${error?.message || error || ""}`;
    if (/revision[_ -]?conflict|expected[_ -]?revision|\b409\b|数据冲突/i.test(message)) return "数据已在其他位置更新，请刷新后重试";
    if (/active[_ -]?attempt[_ -]?conflict/i.test(message)) return "该任务已有运行中的 attempt";
    if (/orphan|thread[_ -]?(unknown|missing)|execution[_ -]?binding[_ -]?unknown/i.test(message)) return "绑定的 Codex 对话不存在，请刷新或创建新 attempt";
    if (/unsupported|capabilit|page[_ -]?host[_ -]?unavailable/i.test(message)) return "当前 Codex 页面不支持此操作";
    if (/cancelled|请求已取消/i.test(message)) return "工作区请求已取消";
    if (/unauthorized|forbidden|needs?_login|登录|401|403/i.test(message)) return "需要登录或当前账号无权访问";
    if (/disabled|unconfigured|connection/i.test(message)) return "本地工作区尚未就绪";
    if (/timeout|network|unreachable|bridge/i.test(message)) return "本地工作区暂不可达";
    if (/invalid|too[_ -]?large|unknown[_ -]?(resource|route)|not[_ -]?persisted/i.test(message)) return "提交的数据不符合本地工作区约束";
    return "工作区操作失败";
  }

  function multicaWorkspaceRenderSettings(content) {
    const enabled = multicaWorkspaceFeatureEnabled();
    const saving = multicaWorkspaceState.settingsSave?.status === "loading";
    const row = multicaWorkspaceEl("div", "ccp-multica-setting-row");
    const copy = multicaWorkspaceEl("div", "ccp-multica-setting-copy");
    copy.append(
      multicaWorkspaceEl("h3", "ccp-multica-setting-title", "启用本地 Multica 工作区"),
      multicaWorkspaceEl("div", "ccp-multica-setting-description", "控制 Codex 左侧入口和本地工作区页面，不影响供应商、代理、模型或 Codex/Claude 配置。"),
    );
    const toggle = multicaWorkspaceEl("button", "ccp-multica-toggle", saving ? "保存中…" : enabled ? "开启" : "关闭");
    toggle.type = "button";
    toggle.disabled = saving;
    toggle.dataset.enabled = String(enabled);
    toggle.setAttribute("role", "switch");
    toggle.setAttribute("aria-checked", String(enabled));
    toggle.setAttribute("aria-label", "启用本地 Multica 工作区");
    toggle.addEventListener("click", () => void multicaWorkspaceSetEnabled(!enabled));
    row.append(copy, toggle);
    content.appendChild(row);
    const result = multicaWorkspaceState.settingsSave;
    if (result?.message) {
      const status = multicaWorkspaceEl("div", `ccp-multica-state${result.status === "failed" ? " ccp-multica-stale" : ""}`, result.message);
      status.setAttribute("role", "status");
      content.appendChild(status);
    }
  }

  async function multicaWorkspaceSetEnabled(nextValue) {
    if (multicaWorkspaceState.settingsSave?.status === "loading") return;
    multicaWorkspaceState.settingsSave = { status: "loading", message: "" };
    multicaWorkspaceRenderContent();
    try {
      await setBackendSetting("multicaWorkspaceEnabled", nextValue);
      multicaWorkspaceState.settingsSave = { status: "ok", message: "设置已保存" };
      if (nextValue) {
        ensureMulticaWorkspaceRuntime();
        multicaWorkspaceRenderContent();
      } else {
        cleanupMulticaWorkspace();
      }
    } catch (error) {
      multicaWorkspaceState.settingsSave = { status: "failed", message: `保存失败：${multicaWorkspaceErrorMessage(error)}` };
      multicaWorkspaceRenderContent();
    }
  }

  function multicaWorkspaceRenderContent() {
    const content = multicaWorkspaceState.root?.content;
    if (!content) return;
    const module = moduleForMulticaWorkspace(multicaWorkspaceState.route);
    const error = multicaWorkspaceState.errors.get(module.key);
    const collection = multicaWorkspacePermissionError(error)
      ? null
      : multicaWorkspaceState.collections.get(module.key);
    multicaWorkspaceClear(content);
    content.dataset.route = module.key;
    if (module.key === "my-issues") {
      multicaWorkspaceRenderIssueBoard(content, module);
      return;
    }
    const header = multicaWorkspaceEl("div", "ccp-multica-content-header");
    header.appendChild(multicaWorkspaceEl("h2", "ccp-multica-content-title", module.label));
    if (module.key !== "settings" && collection && Number.isFinite(Number(collection.total))) {
      header.appendChild(multicaWorkspaceEl("span", "ccp-multica-count", `${collection.total} 条`));
    }
    if (collection?.stale) header.appendChild(multicaWorkspaceEl("span", "ccp-multica-stale", "过期"));
    if (module.key === "skills") {
      const resolve = multicaWorkspaceEl("button", "ccp-multica-button", "解析选择");
      resolve.type = "button";
      resolve.disabled = multicaWorkspaceSkillsInventoryReadOnly();
      resolve.title = resolve.disabled ? "当前 Codex 页面只提供只读 Skill 清单，不能解析或派发" : "解析已选择的 Skill";
      resolve.addEventListener("click", () => void multicaWorkspaceResolveSelectedSkills());
      header.appendChild(resolve);
    }
    const writableResource = multicaWorkspaceWritableResource(module);
    if (writableResource) {
      const create = multicaWorkspaceEl("button", "ccp-multica-button", "新建");
      create.type = "button";
      create.dataset.variant = "primary";
      create.disabled = multicaWorkspaceState.mutationBusy;
      create.addEventListener("click", () => multicaWorkspaceOpenEditor(module));
      header.appendChild(create);
    }
    if (module.key !== "settings") {
      const refresh = multicaWorkspaceEl("button", "ccp-multica-button", multicaWorkspaceState.loading.has(module.key) ? "取消" : "刷新");
      refresh.type = "button";
      refresh.addEventListener("click", () => {
        if (multicaWorkspaceState.loading.has(module.key)) {
          multicaWorkspaceState.querySeq += 1;
          multicaWorkspaceCancelQuery();
          multicaWorkspaceState.loading.delete(module.key);
          multicaWorkspaceRenderContent();
          return;
        }
        void multicaWorkspaceQuery(module, true);
      });
      header.appendChild(refresh);
    }
    multicaWorkspaceAppendModuleMenu(header);
    content.appendChild(header);
    if (module.key === "settings") {
      multicaWorkspaceRenderSettings(content);
      return;
    }
    if (multicaWorkspaceState.mutationNotice && writableResource) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", multicaWorkspaceState.mutationNotice.message || "");
      notice.setAttribute("role", "status");
      if (multicaWorkspaceState.mutationNotice.state === "error") notice.dataset.state = "error";
      content.appendChild(notice);
    }
    if ((module.key === "issues" || module.key === "my-issues") && multicaWorkspaceState.executionNotice?.message) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", multicaWorkspaceState.executionNotice.message);
      notice.setAttribute("role", "status");
      if (multicaWorkspaceState.executionNotice.state === "error") notice.dataset.state = "error";
      content.appendChild(notice);
    }
    if ((module.key === "issues" || module.key === "my-issues") && multicaWorkspaceState.executionsError) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-inline-message", `执行记录：${multicaWorkspaceState.executionsError}`);
      notice.dataset.state = "error";
      content.appendChild(notice);
    }
    multicaWorkspaceRenderEditor(content, module);
    if (module.key === "skills" && multicaWorkspaceState.skillBindingsError) {
      const bindingNotice = multicaWorkspaceEl("div", "ccp-multica-state ccp-multica-stale", `绑定状态：${multicaWorkspaceState.skillBindingsError}`);
      content.appendChild(bindingNotice);
    }
    if (module.key === "skills" && multicaWorkspaceSkillsInventoryReadOnly()) {
      const notice = multicaWorkspaceEl("div", "ccp-multica-state ccp-multica-stale", "当前 Codex 页面只提供只读 Skill 清单，不能解析、绑定或派发 Skill");
      content.appendChild(notice);
    }
    if (multicaWorkspaceState.loading.has(module.key) && !collection) {
      content.appendChild(multicaWorkspaceEl("div", "ccp-multica-state", "正在读取…"));
      return;
    }
    if (error && !collection) {
      const state = multicaWorkspaceEl("div", "ccp-multica-state");
      state.appendChild(multicaWorkspaceEl("strong", "", multicaWorkspaceErrorMessage(error)));
      const retry = multicaWorkspaceEl("button", "ccp-multica-button", "重试");
      retry.type = "button";
      retry.addEventListener("click", () => void multicaWorkspaceQuery(module, true));
      state.appendChild(retry);
      if (/unauthorized|forbidden|登录|401|403/i.test(String(error?.message || error))) {
        const manager = multicaWorkspaceEl("button", "ccp-multica-button", "打开管理器");
        manager.type = "button";
        manager.addEventListener("click", () => void postJson("/manager/open", {}));
        state.appendChild(manager);
      }
      content.appendChild(state);
      return;
    }
    if (!collection || !Array.isArray(collection.items) || collection.items.length === 0) {
      const state = multicaWorkspaceEl("div", "ccp-multica-state", error ? multicaWorkspaceErrorMessage(error) : "暂无数据");
      content.appendChild(state);
      return;
    }
    const list = multicaWorkspaceEl("div", "ccp-multica-list");
    collection.items.forEach((item) => {
      if (module.key === "skills") multicaWorkspaceAppendSkillItem(list, item);
      else if (writableResource) multicaWorkspaceAppendEntityItem(list, item, module);
      else multicaWorkspaceAppendItem(list, item);
    });
    if (module.key === "skills" && multicaWorkspaceState.skillResolution) {
      const resolution = multicaWorkspaceState.skillResolution;
      const state = multicaWorkspaceEl("div", "ccp-multica-state");
      state.textContent = resolution.status === "loading"
        ? "正在解析 Skill 清单…"
        : resolution.status === "ok"
          ? multicaWorkspaceSkillsInventoryReadOnly()
            ? "Skill 清单仅供查看，当前 Codex 页面不提供 Skill 执行能力"
            : "Skill 清单已解析，派发前仍需当前 Codex 页面返回实际加载结果"
          : String(resolution.message || "Skill 解析被阻止");
      content.appendChild(state);
    }
    if (module.key === "skills" && multicaWorkspaceState.skillReview) {
      const review = multicaWorkspaceState.skillReview;
      const state = multicaWorkspaceEl("div", "ccp-multica-state");
      state.textContent = review.status === "loading"
        ? "正在保存 Skill 审查…"
        : review.status === "ok"
          ? (review.trusted ? "Skill 已信任" : "Skill 信任已撤销")
          : String(review.message || "Skill 审查失败");
      content.appendChild(state);
    }
    content.appendChild(list);
  }

  async function multicaWorkspaceLoadBootstrap(force = false, timeoutMs = 15000) {
    if (window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return false;
    if (multicaWorkspaceState.bootstrapLoading && !force) {
      return !!multicaWorkspaceState.bootstrap && !multicaWorkspaceState.bootstrapError;
    }
    if (force) multicaWorkspaceCancelBootstrap();
    const sequence = ++multicaWorkspaceState.bootstrapSeq;
    multicaWorkspaceState.bootstrapLoading = true;
    multicaWorkspaceState.bootstrapError = "";
    multicaWorkspaceSetStatus("连接中", "warning");
    if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
    const request = multicaWorkspaceRequest("/multica/workspace/bootstrap", {}, timeoutMs);
    multicaWorkspaceState.bootstrapRequest = request;
    try {
      let result = await request.promise;
      if (sequence !== multicaWorkspaceState.bootstrapSeq ||
          window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return false;
      if (!result || result.status === "failed") {
        throw multicaWorkspaceErrorFromResult(result, "bootstrap failed");
      }
      const nextWorkspaceId = String(result.workspace?.id || result.workspaceId || "").trim();
      if (multicaWorkspaceState.workspaceId && nextWorkspaceId &&
          multicaWorkspaceState.workspaceId !== nextWorkspaceId) {
        // A workspace switch invalidates every cached collection. Never show
        // a successful response from the previous tenant in the new one.
        multicaWorkspaceState.querySeq += 1;
        multicaWorkspaceCancelQuery();
        multicaWorkspaceState.loading.clear();
        multicaWorkspaceState.collections.clear();
        multicaWorkspaceState.errors.clear();
        multicaWorkspaceState.skillBindings = [];
        multicaWorkspaceState.skillBindingsError = "";
      }
      multicaWorkspaceState.workspaceId = nextWorkspaceId;
      multicaWorkspaceState.bootstrap = result;
      multicaWorkspaceState.bootstrapError = "";
      const collections = result.collections && typeof result.collections === "object" ? result.collections : {};
      multicaWorkspaceModules.forEach((module) => {
        const collection = collections[module.resource] || collections[module.key];
        if (!collection || !Array.isArray(collection.items)) return;
        multicaWorkspaceState.collections.set(module.key, {
          ...collection,
          workspaceId: nextWorkspaceId,
        });
        multicaWorkspaceState.errors.delete(module.key);
      });
      const runtime = result.runtime;
      const pageReady = runtime?.available !== false;
      multicaWorkspaceSetStatus(pageReady ? "本地工作区就绪" : "当前 Codex 页面能力不可用", pageReady ? "ok" : "warning");
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
      if (nextWorkspaceId) void multicaWorkspaceLoadExecutions(true);
      if (nextWorkspaceId) void multicaWorkspaceLoadSavedIssueViewsFromControlPlane();
      return true;
    } catch (error) {
      if (sequence !== multicaWorkspaceState.bootstrapSeq) return false;
      if (multicaWorkspacePermissionError(error)) {
        multicaWorkspaceState.workspaceId = "";
        multicaWorkspaceState.collections.clear();
        multicaWorkspaceState.errors.clear();
        multicaWorkspaceState.skillBindings = [];
        multicaWorkspaceState.skillBindingsError = "";
        if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
      }
      multicaWorkspaceState.bootstrapError = multicaWorkspaceErrorMessage(error);
      multicaWorkspaceSetStatus(multicaWorkspaceState.bootstrapError, "error");
      if (multicaWorkspaceBridgeUnavailable(error) && !multicaWorkspaceState.workspaceId) {
        multicaWorkspaceSetEntryAvailability("启动器未连接，请通过 CCP 启动 Codex");
      }
      if (multicaWorkspaceState.opened) multicaWorkspaceRenderContent();
      return false;
    } finally {
      if (sequence === multicaWorkspaceState.bootstrapSeq) {
        multicaWorkspaceState.bootstrapLoading = false;
        if (multicaWorkspaceState.bootstrapRequest === request) multicaWorkspaceState.bootstrapRequest = null;
      }
    }
  }

  async function multicaWorkspaceQuery(module, force = false, timeoutMs = 15000) {
    if (window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return false;
    if (!module) return false;
    if (module.key === "settings") {
      if (multicaWorkspaceState.opened && module.key === multicaWorkspaceState.route) multicaWorkspaceRenderContent();
      return true;
    }
    if (multicaWorkspaceState.loading.has(module.key) && !force) {
      return multicaWorkspaceState.collections.has(module.key);
    }
    if (force) multicaWorkspaceCancelQuery();
    const sequence = ++multicaWorkspaceState.querySeq;
    multicaWorkspaceState.loading.clear();
    multicaWorkspaceState.loading.add(module.key);
    multicaWorkspaceState.errors.delete(module.key);
    if (multicaWorkspaceState.opened && module.key === multicaWorkspaceState.route) multicaWorkspaceRenderContent();
    const request = multicaWorkspaceRequest("/multica/workspace/query", { resource: module.resource, limit: 50, offset: 0 }, timeoutMs);
    multicaWorkspaceState.queryRequest = request;
    try {
      const result = await request.promise;
      if (sequence !== multicaWorkspaceState.querySeq ||
          window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return false;
      if (!result || result.status === "failed") {
        throw multicaWorkspaceErrorFromResult(result, "workspace query failed");
      }
      if (!Array.isArray(result.items)) throw new Error("workspace collection invalid");
      if (module.key === "autopilots") {
        const itemsWithRuns = await Promise.all(result.items.map(async (item) => {
          const autopilotId = multicaWorkspaceEntityId(item);
          if (!autopilotId) return item;
          try {
            const runsResult = await multicaWorkspaceCall("/multica/autopilots/runs", { autopilotId });
            return { ...item, runs: Array.isArray(runsResult.runs) ? runsResult.runs : [] };
          } catch (_) {
            return { ...item, runs: [] };
          }
        }));
        result = { ...result, items: itemsWithRuns };
      }
      const responseWorkspaceId = String(result.workspaceId || result.workspace?.id || "").trim();
      if (multicaWorkspaceState.workspaceId && responseWorkspaceId &&
          responseWorkspaceId !== multicaWorkspaceState.workspaceId) {
        throw new Error("workspace response scope changed");
      }
      multicaWorkspaceState.collections.set(module.key, {
        ...result,
        workspaceId: responseWorkspaceId || multicaWorkspaceState.workspaceId,
      });
      if (module.key === "skills") await multicaWorkspaceLoadSkillBindings();
      if (module.key === "issues" || module.key === "my-issues") void multicaWorkspaceLoadExecutions(true);
      multicaWorkspaceSetEntryAvailability("");
      return true;
    } catch (error) {
      if (sequence !== multicaWorkspaceState.querySeq) return false;
      multicaWorkspaceState.errors.set(module.key, error);
      const previous = multicaWorkspaceState.collections.get(module.key);
      if (multicaWorkspacePermissionError(error) ||
          (previous?.workspaceId && multicaWorkspaceState.workspaceId &&
            previous.workspaceId !== multicaWorkspaceState.workspaceId)) {
        multicaWorkspaceState.collections.delete(module.key);
      } else if (previous) {
        multicaWorkspaceState.collections.set(module.key, { ...previous, stale: true });
      }
      multicaWorkspaceSetStatus(multicaWorkspaceErrorMessage(error), "error");
      if (multicaWorkspaceBridgeUnavailable(error) && !multicaWorkspaceState.workspaceId) {
        multicaWorkspaceSetEntryAvailability("启动器未连接，请通过 CCP 启动 Codex");
      }
      return false;
    } finally {
      if (sequence === multicaWorkspaceState.querySeq) {
        multicaWorkspaceState.loading.delete(module.key);
        if (multicaWorkspaceState.queryRequest === request) multicaWorkspaceState.queryRequest = null;
        if (multicaWorkspaceState.opened && module.key === multicaWorkspaceState.route) multicaWorkspaceRenderContent();
      }
    }
  }

  async function multicaWorkspaceLoadCurrentRoute(force = true, timeoutMs = 15000, openSequence = multicaWorkspaceState.openSeq) {
    const bootstrapReady = await multicaWorkspaceLoadBootstrap(force, timeoutMs);
    if (openSequence !== multicaWorkspaceState.openSeq ||
        (!multicaWorkspaceState.opened && !multicaWorkspaceState.opening) ||
        window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return false;
    if (!bootstrapReady) return false;
    const module = moduleForMulticaWorkspace(multicaWorkspaceState.route);
    if (module.key === "settings") return true;
    return multicaWorkspaceQuery(module, force, timeoutMs);
  }

  async function multicaWorkspaceOpen() {
    if (!multicaWorkspaceFeatureEnabled()) {
      cleanupMulticaWorkspace();
      return;
    }
    if (multicaWorkspaceState.opened || multicaWorkspaceState.opening) {
      if (multicaWorkspaceState.opened) void multicaWorkspaceLoadCurrentRoute(true, 15000);
      return;
    }
    const plugin = pluginEntryButton();
    const main = multicaWorkspaceNativeMain(plugin);
    const host = multicaWorkspaceEnsureHost();
    if (!plugin || !main || !host) {
      const detail = "等待 Codex 内容区";
      multicaWorkspaceSetStatus(detail, "warning");
      multicaWorkspaceSetEntryAvailability(detail);
      return;
    }
    const openSequence = ++multicaWorkspaceState.openSeq;
    multicaWorkspaceState.opening = true;
    multicaWorkspaceEnsureEntry(plugin);
    multicaWorkspaceSetEntryAvailability("");
    multicaWorkspaceSetStatus("连接中", "warning");
    const currentPlugin = pluginEntryButton();
    const currentMain = main.isConnected ? main : multicaWorkspaceNativeMain(currentPlugin);
    if (!currentMain || !host.isConnected || !multicaWorkspaceBindMain(currentMain)) {
      multicaWorkspaceFailOpen("等待 Codex 内容区");
      return;
    }
    // Keep Codex's native main visible while the local control plane proves
    // both bootstrap and the first my-issues collection. Only then take over
    // the surface; a bridge timeout must never replace usable Codex content
    // with a dead workspace view.
    const ready = await multicaWorkspaceLoadCurrentRoute(true, 15000, openSequence);
    if (openSequence !== multicaWorkspaceState.openSeq ||
        !multicaWorkspaceState.opening ||
        window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return;
    if (!ready) {
      const module = moduleForMulticaWorkspace(multicaWorkspaceState.route);
      const routeError = multicaWorkspaceState.errors.get(module.key);
      multicaWorkspaceFailOpen(
        multicaWorkspaceState.bootstrapError ||
          (routeError ? multicaWorkspaceErrorMessage(routeError) : "本地任务暂不可用，请点击重试"),
      );
      return;
    }
    const readyPlugin = pluginEntryButton();
    const readyMain = currentMain.isConnected ? currentMain : multicaWorkspaceNativeMain(readyPlugin);
    if (!readyMain || !host.isConnected || !multicaWorkspaceBindMain(readyMain)) {
      multicaWorkspaceFailOpen("等待 Codex 内容区");
      return;
    }
    multicaWorkspaceState.opened = true;
    multicaWorkspaceState.opening = false;
    readyMain.style.visibility = "hidden";
    readyMain.style.pointerEvents = "none";
    readyMain.setAttribute?.("inert", "");
    readyMain.setAttribute?.("aria-hidden", "true");
    multicaWorkspaceUpdateGeometry();
    multicaWorkspaceRenderContent();
    multicaWorkspaceEnsureEntry(readyPlugin || currentPlugin || plugin);
    multicaWorkspaceStartBackgroundSync();
  }

  function multicaWorkspaceHide() {
    multicaWorkspaceState.openSeq += 1;
    multicaWorkspaceState.opening = false;
    multicaWorkspaceState.opened = false;
    multicaWorkspaceState.moduleMenuOpen = false;
    multicaWorkspaceRestoreMain();
    if (multicaWorkspaceState.mainResizeObserver) {
      try { multicaWorkspaceState.mainResizeObserver.disconnect(); } catch (_) {}
      multicaWorkspaceState.mainResizeObserver = null;
    }
    multicaWorkspaceState.main = null;
    multicaWorkspaceState.mainSnapshot = null;
    if (multicaWorkspaceState.host) multicaWorkspaceState.host.style.display = "none";
    if (multicaWorkspaceState.entry) {
      multicaWorkspaceState.entry.setAttribute("aria-current", "false");
      multicaWorkspaceState.entry.setAttribute("data-state", "inactive");
    }
    multicaWorkspaceSetEntryAvailability("");
  }

  async function multicaWorkspaceBackgroundSync() {
    if (multicaWorkspaceState.backgroundBusy || !multicaWorkspaceFeatureEnabled()) return;
    multicaWorkspaceState.backgroundBusy = true;
    try {
      if (!multicaWorkspaceState.workspaceId && !multicaWorkspaceState.bootstrapLoading) {
        await multicaWorkspaceLoadBootstrap(true, multicaWorkspaceBackgroundTimeoutMs);
      }
      if (!multicaWorkspaceState.workspaceId || multicaWorkspaceState.queryRequest) return;
      for (const route of ["issues", "my-issues"]) {
        if (multicaWorkspaceState.queryRequest) return;
        await multicaWorkspaceQuery(moduleForMulticaWorkspace(route), true, multicaWorkspaceBackgroundTimeoutMs);
      }
      if (!multicaWorkspaceState.executionsLoading) await multicaWorkspaceLoadExecutions(true);
    } finally {
      multicaWorkspaceState.backgroundBusy = false;
    }
  }

  function multicaWorkspaceStartBackgroundSync() {
    if (multicaWorkspaceState.backgroundStarted) return;
    multicaWorkspaceState.backgroundStarted = true;
    const tick = async () => {
      multicaWorkspaceState.backgroundTimer = null;
      await multicaWorkspaceBackgroundSync();
      if (!multicaWorkspaceState.backgroundStarted ||
          window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return;
      multicaWorkspaceState.backgroundTimer = setTimeout(tick, multicaWorkspaceBackgroundIntervalMs);
    };
    multicaWorkspaceState.backgroundTimer = setTimeout(tick, 0);
  }

  function multicaWorkspaceStopBackgroundSync() {
    multicaWorkspaceState.backgroundStarted = false;
    if (multicaWorkspaceState.backgroundTimer) clearTimeout(multicaWorkspaceState.backgroundTimer);
    multicaWorkspaceState.backgroundTimer = null;
  }

  function multicaWorkspaceScheduleAnchorRetry() {
    if (multicaWorkspaceState.anchorTimer || multicaWorkspaceState.anchorAttempts >= 8) {
      if (multicaWorkspaceState.anchorAttempts >= 8 && !multicaWorkspaceState.anchorDiagnosticSent) {
        multicaWorkspaceState.anchorDiagnosticSent = true;
        sendClaudeCodexProDiagnostic("multica_workspace_anchor_unavailable", { attempts: multicaWorkspaceState.anchorAttempts });
      }
      return;
    }
    multicaWorkspaceState.anchorAttempts += 1;
    multicaWorkspaceState.anchorTimer = setTimeout(() => {
      multicaWorkspaceState.anchorTimer = null;
      ensureMulticaWorkspaceRuntime();
    }, 250);
  }

  function multicaWorkspaceAnchorChanged(mutations) {
    if (!mutations) return false;
    return mutations.some((mutation) => {
      if (mutation.type === "attributes" || mutation.type === "characterData") {
        return multicaPluginAnchorMutationNode(mutation.target);
      }
      return [...Array.from(mutation.addedNodes), ...Array.from(mutation.removedNodes)]
        .some((node) => node.nodeType === 1 && (
          multicaPluginAnchorMutationNode(node) ||
          node.matches?.("main") || node.querySelector?.("main")
        ));
    });
  }

  function ensureMulticaWorkspaceRuntime() {
    if (window.__claudeCodexProMulticaWorkspaceGeneration !== claudeCodexProMulticaWorkspaceGeneration) return;
    if (!document?.querySelectorAll || !document?.createElement) return;
    window.__claudeCodexProMulticaWorkspaceCleanup = cleanupMulticaWorkspace;
    if (!multicaWorkspaceFeatureEnabled()) {
      if (multicaWorkspaceState.entry || multicaWorkspaceState.host || multicaWorkspaceState.opened) {
        cleanupMulticaWorkspace();
      }
      return;
    }
    const plugin = pluginEntryButton();
    if (!plugin) {
      multicaWorkspaceScheduleAnchorRetry();
      return;
    }
    multicaWorkspaceState.anchorAttempts = 0;
    multicaWorkspaceState.anchorDiagnosticSent = false;
    if (multicaWorkspaceState.anchorTimer) {
      clearTimeout(multicaWorkspaceState.anchorTimer);
      multicaWorkspaceState.anchorTimer = null;
    }
    multicaWorkspaceEnsureEntry(plugin);
    multicaWorkspaceEnsureHost();
    multicaWorkspaceStartBackgroundSync();
    const main = multicaWorkspaceNativeMain(plugin);
    if (!main) {
      // Keep the navigation entry visible while Codex mounts its main route.
      // A later DOM mutation/scan will bind the content surface.
      multicaWorkspaceScheduleAnchorRetry();
      return;
    }
    if (multicaWorkspaceState.opened) {
      multicaWorkspaceBindMain(main);
      main.style.visibility = "hidden";
      main.style.pointerEvents = "none";
      main.setAttribute?.("inert", "");
      main.setAttribute?.("aria-hidden", "true");
      multicaWorkspaceUpdateGeometry();
    }
    if (!multicaWorkspaceState.navHandler) {
      multicaWorkspaceState.navHandler = (event) => {
        const target = event.target;
        if (multicaWorkspaceState.entry?.contains?.(target)) return;
        if (multicaWorkspaceState.nativeThreadActivation) return;
        const nativeNavigation = target?.closest?.([
          "[data-app-action-sidebar-thread-id]",
          "[data-app-action-sidebar-project-row]",
          'nav a[href]',
          'aside a[href]',
          'nav [role="link"]',
          'aside [role="link"]',
          'nav [role="button"]',
          'aside [role="button"]',
          "nav button",
          "aside button",
        ].join(", "));
        if (nativeNavigation && !nativeNavigation?.dataset?.ccpMulticaNav) multicaWorkspaceHide();
      };
      document.addEventListener("click", multicaWorkspaceState.navHandler, true);
    }
  }

  function cleanupMulticaWorkspace() {
    if (multicaWorkspaceState.anchorTimer) clearTimeout(multicaWorkspaceState.anchorTimer);
    multicaWorkspaceState.anchorTimer = null;
    multicaWorkspaceStopBackgroundSync();
    multicaWorkspaceState.querySeq += 1;
    multicaWorkspaceState.bootstrapSeq += 1;
    multicaWorkspaceCancelQuery();
    multicaWorkspaceCancelBootstrap();
    multicaWorkspaceState.executionSeq += 1;
    multicaWorkspaceState.activeRequests.forEach((request) => request?.cancel?.());
    multicaWorkspaceState.activeRequests.clear();
    multicaWorkspaceState.loading.clear();
    multicaWorkspaceState.editor = null;
    multicaWorkspaceState.executionDraft = null;
    multicaWorkspaceState.executionBusy.clear();
    multicaWorkspaceState.executionsLoading = false;
    multicaWorkspaceHide();
    if (multicaWorkspaceState.navHandler) {
      document.removeEventListener("click", multicaWorkspaceState.navHandler, true);
      multicaWorkspaceState.navHandler = null;
    }
    if (multicaWorkspaceState.mainResizeObserver) {
      try { multicaWorkspaceState.mainResizeObserver.disconnect(); } catch (_) {}
      multicaWorkspaceState.mainResizeObserver = null;
    }
    multicaWorkspaceRestoreMain();
    multicaWorkspaceState.entry?.remove?.();
    multicaWorkspaceState.host?.remove?.();
    document.querySelectorAll?.('[data-ccp-multica-nav="true"], #ccp-multica-workspace-root')
      .forEach((node) => node.remove());
    multicaWorkspaceState.entry = null;
    multicaWorkspaceState.host = null;
    multicaWorkspaceState.shadow = null;
    multicaWorkspaceState.root = null;
    multicaWorkspaceState.main = null;
    multicaWorkspaceState.mainSnapshot = null;
    codexPageHostClientPromise = null;
    codexPageHostClient = null;
    codexPageHostInitializeResponse = null;
    if (window.__claudeCodexProMulticaWorkspaceCleanup === cleanupMulticaWorkspace) {
      window.__claudeCodexProMulticaWorkspaceCleanup = null;
    }
  }

  window.__claudeCodexProMulticaWorkspaceCleanup = cleanupMulticaWorkspace;

  function labelUnlockedPluginEntry(button) {
    const labelTextNode = Array.from(button.querySelectorAll("span, div")).reverse()
      .flatMap((node) => Array.from(node.childNodes))
      .find((node) => node.nodeType === 3 && /^(插件|Plugins)( - 已解锁| - Unlocked)?$/i.test((node.nodeValue || "").trim()));
    if (!labelTextNode) return;
    const current = (labelTextNode.nodeValue || "").trim();
    labelTextNode.nodeValue = /^Plugins/i.test(current) ? "Plugins - Unlocked" : "插件 - 已解锁";
  }

  function clearPluginEntryUnlockLabel(button) {
    const labelTextNode = Array.from(button.querySelectorAll("span, div")).reverse()
      .flatMap((node) => Array.from(node.childNodes))
      .find((node) => node.nodeType === 3 && /^(插件 - 已解锁|Plugins - Unlocked)$/i.test((node.nodeValue || "").trim()));
    if (!labelTextNode) return;
    labelTextNode.nodeValue = /^Plugins/i.test((labelTextNode.nodeValue || "").trim()) ? "Plugins" : "插件";
  }

  function enablePluginEntry() {
    if (pluginPatchDisabledInRelayMode()) return;
    if (!claudeCodexProSettings().pluginEntryUnlock) return;
    const pluginButton = pluginEntryButton();
    if (!pluginButton) return;
    const spoofed = spoofChatGPTAuthMethod(pluginButton);
    pluginButton.disabled = false;
    pluginButton.removeAttribute("disabled");
    pluginButton.style.display = "";
    pluginButton.querySelectorAll("*").forEach((node) => {
      node.style.display = "";
    });
    labelUnlockedPluginEntry(pluginButton);
    const reactPropsKey = Object.keys(pluginButton).find((key) => key.startsWith("__reactProps"));
    if (reactPropsKey) {
      pluginButton[reactPropsKey].disabled = false;
    }
    if (pluginButton.dataset.codexPluginEnabled !== "true") {
      pluginButton.dataset.codexPluginEnabled = "true";
      pluginButton.addEventListener("click", () => {
        spoofChatGPTAuthMethod(pluginButton);
      }, true);
    }
    sendClaudeCodexProDiagnostic("plugin_entry_unlock_applied", { spoofed });
  }

  function pluginPatchDisabledInRelayMode() {
    return !claudeCodexProBackendSettingsLoaded || claudeCodexProBackendSettings.launchMode === "relay";
  }

  function pluginInstallCandidates() {
    const nodes = Array.from(document.querySelectorAll(selectors.disabledInstallButton));
    return Array.from(new Set(nodes.map((node) => node.closest?.("button, [role='button']") || node)));
  }

  function installButtonLabel(element) {
    return (element.textContent || "").trim();
  }

  function isInstallButtonLabel(text) {
    return /^安装\s*/.test(text) || /^Install\s*/i.test(text) || text === "强制安装";
  }

  function patchReactDisabledProps(element) {
    Object.keys(element)
      .filter((key) => key.startsWith("__reactProps"))
      .forEach((key) => {
        const props = element[key];
        if (!props || typeof props !== "object") return;
        props.disabled = false;
        props["aria-disabled"] = false;
        props["data-disabled"] = undefined;
      });
  }

  function clearDisabledState(element) {
    if (!(element instanceof HTMLElement)) return;
    if ("disabled" in element) element.disabled = false;
    element.removeAttribute("disabled");
    element.removeAttribute("aria-disabled");
    element.removeAttribute("data-disabled");
    element.removeAttribute("inert");
    element.classList.remove("disabled", "opacity-50", "cursor-not-allowed", "pointer-events-none");
    element.classList.add("codex-force-install-unlocked");
    element.style.pointerEvents = "auto";
    element.style.opacity = "";
    element.style.cursor = "pointer";
    element.tabIndex = 0;
    patchReactDisabledProps(element);
  }

  function installButtonUnlockNodes(button) {
    const nodes = [button];
    button.querySelectorAll?.("button, [role='button'], [disabled], [aria-disabled], [data-disabled], .cursor-not-allowed, .pointer-events-none")
      .forEach((node) => nodes.push(node));
    let parent = button.parentElement;
    for (let depth = 0; parent && depth < 3; depth += 1, parent = parent.parentElement) {
      if (parent.matches?.("button, [role='button'], [disabled], [aria-disabled], [data-disabled], .cursor-not-allowed, .pointer-events-none")) {
        nodes.push(parent);
      }
    }
    return Array.from(new Set(nodes));
  }

  function installForcedInstallGuard(button) {
    if (button.dataset.codexForceInstallUnlocked === "true") return;
    button.dataset.codexForceInstallUnlocked = "true";
    const keepUnlocked = () => installButtonUnlockNodes(button).forEach(clearDisabledState);
    ["pointerdown", "mousedown", "mouseup", "click", "focus"].forEach((eventName) => {
      button.addEventListener(eventName, keepUnlocked, true);
    });
  }

  function unblockButtonElement(button) {
    installButtonUnlockNodes(button).forEach(clearDisabledState);
    installForcedInstallGuard(button);
  }

  function labelForcedInstallButton(button) {
    const walker = document.createTreeWalker(button, NodeFilter.SHOW_TEXT);
    let textNode = null;
    while (!textNode && walker.nextNode()) {
      const node = walker.currentNode;
      if (isInstallButtonLabel((node.nodeValue || "").trim())) textNode = node;
    }
    if (textNode) {
      textNode.nodeValue = "强制安装";
    }
  }

  function clearForcedInstallButtonLabel(button) {
    const walker = document.createTreeWalker(button, NodeFilter.SHOW_TEXT);
    let textNode = null;
    while (!textNode && walker.nextNode()) {
      const node = walker.currentNode;
      if ((node.nodeValue || "").trim() === "强制安装") textNode = node;
    }
    if (textNode) {
      textNode.nodeValue = "安装";
    }
  }

  function clearPluginPatchArtifacts() {
    const pluginButton = pluginEntryButton();
    if (pluginButton) {
      delete pluginButton.dataset.codexPluginEnabled;
      clearPluginEntryUnlockLabel(pluginButton);
    }
    pluginInstallCandidates().forEach(clearForcedInstallButtonLabel);
  }

  function unblockPluginInstallButtons() {
    if (pluginPatchDisabledInRelayMode()) return;
    if (!claudeCodexProSettings().forcePluginInstall) return;
    pluginInstallCandidates().forEach((button) => {
      const text = installButtonLabel(button);
      if (!isInstallButtonLabel(text)) return;
      unblockButtonElement(button);
      labelForcedInstallButton(button);
    });
  }

  function refreshForcePluginInstallUnlockLoop() {
    const shouldRun = !pluginPatchDisabledInRelayMode() && claudeCodexProSettings().forcePluginInstall;
    if (!shouldRun) {
      clearInterval(window.__codexForcePluginInstallRefreshTimer);
      window.__codexForcePluginInstallRefreshTimer = null;
      return;
    }
    if (window.__codexForcePluginInstallRefreshTimer) return;
    window.__codexForcePluginInstallRefreshTimer = setInterval(() => {
      if (!claudeCodexProSettings().forcePluginInstall || pluginPatchDisabledInRelayMode()) {
        clearInterval(window.__codexForcePluginInstallRefreshTimer);
        window.__codexForcePluginInstallRefreshTimer = null;
        return;
      }
      unblockPluginInstallButtons();
    }, codexForcePluginInstallRefreshIntervalMs);
  }

  let cachedSessionRows = [];
  let cachedSessionRowsAt = 0;

  function sessionRows(forceRefresh = false) {
    const now = Date.now();
    if (!forceRefresh && now - cachedSessionRowsAt < 150) {
      cachedSessionRows = cachedSessionRows.filter((row) => row.isConnected);
      if (cachedSessionRows.length > 0) return cachedSessionRows;
    }

    cachedSessionRows = Array.from(document.querySelectorAll(selectors.sidebarThread));
    cachedSessionRowsAt = now;
    return cachedSessionRows;
  }

  function archivePageHintVisible() {
    if (window.location.href.includes("archive")) return true;
    if (document.querySelector('[data-codex-archive-page-row="true"], [data-codex-archive-delete-all]')) return true;
    const archiveNav = document.querySelector(selectors.archiveNav);
    if (archiveNav?.className?.includes?.("bg-token-list-hover-background")) return true;
    return !!Array.from(document.querySelectorAll("h1, h2, h3")).find((element) => (element.textContent || "").trim() === "已归档对话");
  }

  function archiveRowFromUnarchiveButton(button) {
    return button.closest('[data-codex-archive-page-row="true"]')
      || button.closest('[role="listitem"], [role="row"]')
      || button.closest(".flex.w-full.items-center.justify-between")
      || button.parentElement;
  }

  function archivedPageRows() {
    if (!archivePageHintVisible()) return [];
    const rows = Array.from(document.querySelectorAll("button")).filter((button) => (button.textContent || "").trim() === "取消归档").map(archiveRowFromUnarchiveButton).filter(Boolean);
    rows.forEach((row) => {
      row.dataset.codexArchivePageRow = "true";
      row.setAttribute("data-codex-archive-page-row", "true");
    });
    return rows;
  }

  function archivedSessionRows() {
    if (!archivePageHintVisible()) return [];
    return sessionRows().filter((row) => row.querySelector('button[aria-label="取消归档对话"]') || row.outerHTML.includes("取消归档") || row.outerHTML.includes("unarchive"));
  }

  function archivedRows() {
    if (!archivePageHintVisible()) return [];
    return [...archivedSessionRows(), ...archivedPageRows()];
  }

  function archivedPageVisible() {
    return archivePageHintVisible() && archivedRows().length > 0;
  }

  function sessionRefFromRow(row) {
    const href = row.getAttribute("href") || row.querySelector("a")?.getAttribute("href") || "";
    const idMatch = href.match(/(?:session|conversation|thread)[=/:-]([A-Za-z0-9_.-]+)/i) || href.match(/([A-Za-z0-9_-]{8,})$/);
    const codexThreadId = row.getAttribute("data-app-action-sidebar-thread-id") || "";
    const fallbackId = row.getAttribute("data-session-id") || row.getAttribute("data-testid") || "";
    const sessionId = codexThreadId || (idMatch && idMatch[1]) || fallbackId;
    const titleNode = row.querySelector(`${selectors.threadTitle}, .truncate.select-none, .truncate.text-base`);
    const rawTitle = (titleNode?.textContent || (titleNode ? "" : (row.textContent || "Untitled session")));
    const title = (titleNode ? rawTitle : rawTitle.replace(/\s*(导出|删除|移动|移出项目)(\s*(导出|删除|移动|移出项目))*$/g, "")).trim().slice(0, 160);
    return { session_id: sessionId, title };
  }

  function claudeCodexProDiagnosticPayload(event, detail) {
    return {
      event,
      detail: detail || {},
      helperBase,
      hasBridge: !!window.__codexSessionDeleteBridge,
      location: window.location?.href || "",
      userAgent: navigator.userAgent || "",
      timestamp: new Date().toISOString(),
    };
  }

  function sendClaudeCodexProDiagnostic(event, detail) {
    const payload = claudeCodexProDiagnosticPayload(event, detail);
    if (window.__CLAUDE_CODEX_PRO_TEST_SERVICE_TIER__) {
      window.__claudeCodexProServiceTierTestDiagnostics = window.__claudeCodexProServiceTierTestDiagnostics || [];
      window.__claudeCodexProServiceTierTestDiagnostics.push(payload);
      return;
    }
    if (window.__codexSessionDeleteBridge) {
      window.__codexSessionDeleteBridge("/diagnostics/log", payload).catch(() => {});
    }
    const body = JSON.stringify(payload);
    try {
      if (navigator.sendBeacon) {
        const blob = new Blob([body], { type: "application/json" });
        if (navigator.sendBeacon(`${helperBase}/diagnostics/log`, blob)) return;
      }
    } catch (_) {}
    fetch(`${helperBase}/diagnostics/log`, {
      method: "POST",
      headers: withHelperToken({ "Content-Type": "application/json" }),
      body,
      keepalive: true,
    }).catch(() => {});
  }

  sendClaudeCodexProDiagnostic("script_loaded", {
    version: claudeCodexProVersion,
    build: claudeCodexProBuild,
  });

  function locationThreadId() {
    const source = `${window.location.pathname}${window.location.search}${window.location.hash}`;
    const match = source.match(/(?:session|conversation|thread)(?:\/|=|:|-)([A-Za-z0-9_.-]+)/i)
      || source.match(/\/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})(?:[/?#]|$)/)
      || source.match(/\/([A-Za-z0-9_-]{24,})(?:[/?#]|$)/);
    return match ? decodeURIComponent(match[1]) : "";
  }

  function finiteNonNegativeNumber(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) && numeric >= 0 ? numeric : 0;
  }

  function finiteScrollNumber(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : 0;
  }

  function validThreadScrollSessionKey(sessionId) {
    const key = projectMoveSessionKey(sessionId);
    if (!key || key === "__proto__" || key === "prototype" || key === "constructor") return "";
    return /^[A-Za-z0-9_.-]{8,128}$/.test(key) ? key : "";
  }

  function currentSessionRef() {
    const rows = sessionRows();
    for (const row of rows) {
      const ref = sessionRefFromRow(row);
      if (ref.session_id && isCurrentSessionRow(row, ref)) return ref;
    }
    return { session_id: locationThreadId(), title: "" };
  }

  function readThreadScrollEntries() {
    if (window.__codexThreadScrollEntries && typeof window.__codexThreadScrollEntries === "object") {
      return { ...window.__codexThreadScrollEntries };
    }
    try {
      const parsed = JSON.parse(localStorage.getItem(codexThreadScrollKey) || "{}");
      const rawEntries = parsed?.version === codexThreadScrollVersion && parsed?.entries && typeof parsed.entries === "object"
        ? parsed.entries
        : parsed && typeof parsed === "object"
          ? parsed
          : {};
      const entries = Object.create(null);
      Object.entries(rawEntries).forEach(([key, value]) => {
        const safeKey = validThreadScrollSessionKey(key);
        if (!safeKey || !value || typeof value !== "object") return;
        entries[safeKey] = {
          top: finiteScrollNumber(value.top),
          scrollHeight: finiteNonNegativeNumber(value.scrollHeight),
          clientHeight: finiteNonNegativeNumber(value.clientHeight),
          at: finiteNonNegativeNumber(value.at),
        };
      });
      window.__codexThreadScrollEntries = entries;
      return { ...entries };
    } catch {
      window.__codexThreadScrollEntries = Object.create(null);
      return {};
    }
  }

  function writeThreadScrollEntries(entries) {
    const pruned = Object.create(null);
    Object.entries(entries || {})
      .sort((left, right) => finiteNonNegativeNumber(right[1]?.at) - finiteNonNegativeNumber(left[1]?.at))
      .slice(0, codexThreadScrollMaxEntries)
      .forEach(([key, value]) => {
        const safeKey = validThreadScrollSessionKey(key);
        if (safeKey) pruned[safeKey] = value;
      });
    window.__codexThreadScrollEntries = pruned;
    localStorage.setItem(codexThreadScrollKey, JSON.stringify({ version: codexThreadScrollVersion, entries: pruned }));
  }

  function currentThreadScroller() {
    const explicit = document.querySelector(".thread-scroll-container");
    if (explicit?.isConnected) return explicit;
    const root = conversationTimelineRoot();
    if (!root?.isConnected) return document.scrollingElement || document.documentElement;
    const style = getComputedStyle(root);
    if (/(auto|scroll)/.test(style.overflowY) && root.scrollHeight > root.clientHeight) return root;
    return nearestTimelineScroller(root);
  }

  function threadScrollRuntime() {
    if (!window.__codexThreadScrollRuntime || typeof window.__codexThreadScrollRuntime !== "object") {
      window.__codexThreadScrollRuntime = {
        activeSessionId: "",
        activeScroller: null,
        scrollListener: null,
        scrollListenerUsesWindow: false,
        lastSavedTop: -1,
        lastSavedHeight: -1,
        lastSavedClientHeight: -1,
        restoreLock: null,
        applyingRestore: false,
        pendingNavigation: null,
        userScrollIntentUntil: 0,
        userCancelledRestoreSessionId: "",
      };
    }
    return window.__codexThreadScrollRuntime;
  }

  function clearThreadScrollRestoreTimers() {
    (window.__codexThreadScrollRestoreTimers || []).forEach((timer) => clearTimeout(timer));
    window.__codexThreadScrollRestoreTimers = [];
  }

  function clearThreadScrollSyncTimers() {
    (window.__codexThreadScrollSyncTimers || []).forEach((timer) => clearTimeout(timer));
    window.__codexThreadScrollSyncTimers = [];
  }

  function clearThreadScrollRestoreLock() {
    threadScrollRuntime().restoreLock = null;
  }

  function cancelThreadScrollRestoreForUserIntent() {
    const runtime = threadScrollRuntime();
    const cancelledSessionId = validThreadScrollSessionKey(runtime.restoreLock?.sessionId)
      || validThreadScrollSessionKey(currentSessionRef().session_id)
      || validThreadScrollSessionKey(runtime.activeSessionId);
    runtime.userScrollIntentUntil = Date.now() + codexThreadScrollUserIntentWindowMs;
    runtime.userCancelledRestoreSessionId = cancelledSessionId;
    window.__codexThreadScrollRestoreRevision = (window.__codexThreadScrollRestoreRevision || 0) + 1;
    window.__codexThreadScrollSyncRevision = (window.__codexThreadScrollSyncRevision || 0) + 1;
    clearThreadScrollRestoreTimers();
    clearThreadScrollSyncTimers();
    clearThreadScrollRestoreLock();
  }

  function userScrollIntentActive() {
    return finiteNonNegativeNumber(threadScrollRuntime().userScrollIntentUntil) > Date.now();
  }

  function threadScrollRestoreCancelledForSession(sessionId = threadScrollRuntime().activeSessionId) {
    const key = validThreadScrollSessionKey(sessionId);
    return !!key && threadScrollRuntime().userCancelledRestoreSessionId === key;
  }

  function activeThreadScrollRestoreLock(sessionId = threadScrollRuntime().activeSessionId) {
    const runtime = threadScrollRuntime();
    const key = validThreadScrollSessionKey(sessionId);
    const lock = runtime.restoreLock;
    if (!lock || !key || lock.sessionId !== key) return null;
    if (lock.expiresAt <= Date.now()) {
      clearThreadScrollRestoreLock();
      return null;
    }
    return lock;
  }

  function currentThreadScrollRestoreLock() {
    const sessionId = threadScrollRuntime().restoreLock?.sessionId;
    return sessionId ? activeThreadScrollRestoreLock(sessionId) : null;
  }

  function threadScrollIsReversed(scroller) {
    return getComputedStyle(scroller).flexDirection === "column-reverse";
  }

  function threadScrollRange(scroller) {
    const extent = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
    return threadScrollIsReversed(scroller)
      ? { min: -extent, max: 0, bottom: 0 }
      : { min: 0, max: extent, bottom: extent };
  }

  function startThreadScrollRestoreLock(sessionId, entry) {
    const key = validThreadScrollSessionKey(sessionId);
    if (!key || !entry) {
      clearThreadScrollRestoreLock();
      return null;
    }
    const runtime = threadScrollRuntime();
    runtime.restoreLock = {
      sessionId: key,
      targetTop: finiteScrollNumber(entry.top),
      expiresAt: Date.now() + codexThreadScrollRestoreWindowMs,
    };
    return runtime.restoreLock;
  }

  function prepareThreadScrollRestoreLock(sessionId) {
    const key = validThreadScrollSessionKey(sessionId);
    const entry = key ? readThreadScrollEntries()[key] : null;
    if (entry) startThreadScrollRestoreLock(key, entry);
  }

  function threadScrollTargetTop(scroller, targetTop) {
    const range = threadScrollRange(scroller);
    return Math.max(range.min, Math.min(range.max, finiteScrollNumber(targetTop)));
  }

  function threadScrollNearBottom(scroller, top) {
    const range = threadScrollRange(scroller);
    return Math.abs(range.bottom - finiteScrollNumber(top)) <= Math.max(24, scroller.clientHeight * 0.15);
  }

  function threadScrollGuardScroller(scroller) {
    if (!scroller) return null;
    const runtime = threadScrollRuntime();
    const rootScroller = document.scrollingElement || document.documentElement || document.body;
    const normalizedScroller = scroller === document.body || scroller === document.documentElement ? rootScroller : scroller;
    if (normalizedScroller === runtime.activeScroller) return normalizedScroller;
    const currentScroller = currentThreadScroller();
    if (normalizedScroller === currentScroller) return normalizedScroller;
    return null;
  }

  function shouldBlockThreadScrollAutobottom(scroller, top) {
    const runtime = threadScrollRuntime();
    const lock = currentThreadScrollRestoreLock();
    if (!lock || !claudeCodexProSettings().threadScrollRestore) return false;
    const guardScroller = threadScrollGuardScroller(scroller);
    if (runtime.applyingRestore || !guardScroller) return false;
    const targetTop = threadScrollTargetTop(guardScroller, lock.targetTop);
    return Math.abs(finiteScrollNumber(top) - targetTop) > 8 && threadScrollNearBottom(guardScroller, top);
  }

  function scrollToRequestedTop(args, scroller) {
    if (!args.length) return null;
    const first = args[0];
    if (typeof first === "object" && first !== null) return first.top == null ? null : finiteScrollNumber(first.top);
    if (args.length >= 2) return finiteScrollNumber(args[1]);
    return scroller?.scrollTop ?? null;
  }

  function scrollByRequestedTop(args, scroller) {
    if (!args.length || !scroller) return null;
    const first = args[0];
    let delta = null;
    if (typeof first === "object" && first !== null) {
      delta = first.top == null ? null : Number(first.top);
    } else if (args.length >= 2) {
      delta = Number(args[1]);
    }
    return Number.isFinite(delta) ? finiteScrollNumber(scroller.scrollTop + delta) : null;
  }

  function shouldBlockThreadScrollIntoView(element) {
    const runtime = threadScrollRuntime();
    const lock = currentThreadScrollRestoreLock();
    if (runtime.applyingRestore || !lock || !element) return false;
    const activeScroller = threadScrollGuardScroller(runtime.activeScroller) || threadScrollGuardScroller(currentThreadScroller());
    if (!activeScroller || element === activeScroller || !activeScroller.contains?.(element)) return false;
    if (threadScrollIsReversed(activeScroller) && shouldBlockThreadScrollAutobottom(activeScroller, 0)) return true;
    const elementRect = element.getBoundingClientRect?.();
    if (!elementRect) return false;
    const elementBottomTop = activeScroller.scrollTop + elementRect.bottom - timelineScrollerViewportTop(activeScroller) - activeScroller.clientHeight;
    return shouldBlockThreadScrollAutobottom(activeScroller, elementBottomTop);
  }

  function installThreadScrollProgrammaticScrollGuard() {
    if (window.__codexThreadScrollProgrammaticGuardInstalled === codexThreadScrollProgrammaticGuardVersion) return;
    window.__codexThreadScrollProgrammaticGuardInstalled = codexThreadScrollProgrammaticGuardVersion;
    window.__codexThreadScrollOriginals = window.__codexThreadScrollOriginals || {};
    const originals = window.__codexThreadScrollOriginals;
    originals.elementScrollTo = originals.elementScrollTo || Element.prototype.scrollTo;
    if (typeof originals.elementScrollTo === "function") {
      Element.prototype.scrollTo = function codexThreadScrollGuardedScrollTo(...args) {
        const top = scrollToRequestedTop(args, this);
        if (top != null && window.__codexThreadScrollHandlers?.shouldBlockAutobottom?.(this, top)) return;
        return originals.elementScrollTo.apply(this, args);
      };
    }
    originals.elementScroll = originals.elementScroll || Element.prototype.scroll;
    if (typeof originals.elementScroll === "function") {
      Element.prototype.scroll = function codexThreadScrollGuardedScroll(...args) {
        const top = scrollToRequestedTop(args, this);
        if (top != null && window.__codexThreadScrollHandlers?.shouldBlockAutobottom?.(this, top)) return;
        return originals.elementScroll.apply(this, args);
      };
    }
    originals.elementScrollBy = originals.elementScrollBy || Element.prototype.scrollBy;
    if (typeof originals.elementScrollBy === "function") {
      Element.prototype.scrollBy = function codexThreadScrollGuardedScrollBy(...args) {
        const top = scrollByRequestedTop(args, this);
        if (top != null && window.__codexThreadScrollHandlers?.shouldBlockAutobottom?.(this, top)) return;
        return originals.elementScrollBy.apply(this, args);
      };
    }
    originals.scrollIntoView = originals.scrollIntoView || Element.prototype.scrollIntoView;
    if (typeof originals.scrollIntoView === "function") {
      Element.prototype.scrollIntoView = function codexThreadScrollGuardedScrollIntoView(...args) {
        if (window.__codexThreadScrollHandlers?.shouldBlockIntoView?.(this)) return;
        return originals.scrollIntoView.apply(this, args);
      };
    }
    originals.windowScrollTo = originals.windowScrollTo || window.scrollTo;
    if (typeof originals.windowScrollTo === "function") {
      window.scrollTo = function codexThreadScrollGuardedWindowScrollTo(...args) {
        const scroller = document.scrollingElement || document.documentElement || document.body;
        const top = scrollToRequestedTop(args, scroller);
        if (top != null && window.__codexThreadScrollHandlers?.shouldBlockAutobottom?.(scroller, top)) return;
        return originals.windowScrollTo.apply(this, args);
      };
    }
    originals.windowScroll = originals.windowScroll || window.scroll;
    if (typeof originals.windowScroll === "function") {
      window.scroll = function codexThreadScrollGuardedWindowScroll(...args) {
        const scroller = document.scrollingElement || document.documentElement || document.body;
        const top = scrollToRequestedTop(args, scroller);
        if (top != null && window.__codexThreadScrollHandlers?.shouldBlockAutobottom?.(scroller, top)) return;
        return originals.windowScroll.apply(this, args);
      };
    }
    originals.windowScrollBy = originals.windowScrollBy || window.scrollBy;
    if (typeof originals.windowScrollBy === "function") {
      window.scrollBy = function codexThreadScrollGuardedWindowScrollBy(...args) {
        const scroller = document.scrollingElement || document.documentElement || document.body;
        const top = scrollByRequestedTop(args, scroller);
        if (top != null && window.__codexThreadScrollHandlers?.shouldBlockAutobottom?.(scroller, top)) return;
        return originals.windowScrollBy.apply(this, args);
      };
    }
  }

  function bindThreadScrollListener(scroller) {
    const runtime = threadScrollRuntime();
    const currentUsesWindow = !runtime.activeScroller || runtime.activeScroller === document.scrollingElement || runtime.activeScroller === document.documentElement || runtime.activeScroller === document.body;
    const nextUsesWindow = !scroller || scroller === document.scrollingElement || scroller === document.documentElement || scroller === document.body;
    let listenerReplaced = false;
    if (runtime.scrollListener && runtime.scrollListenerVersion !== codexThreadScrollListenerVersion) {
      const currentTarget = currentUsesWindow ? window : runtime.activeScroller;
      currentTarget?.removeEventListener?.("scroll", runtime.scrollListener, true);
      runtime.scrollListener = null;
      runtime.scrollListenerVersion = "";
      listenerReplaced = true;
    }
    runtime.scrollListener = runtime.scrollListener || (() => scheduleThreadScrollSave());
    runtime.scrollListenerVersion = codexThreadScrollListenerVersion;
    if (!listenerReplaced && runtime.activeScroller === scroller && runtime.scrollListenerUsesWindow === nextUsesWindow) return;
    if (runtime.activeScroller) {
      const target = currentUsesWindow ? window : runtime.activeScroller;
      target.removeEventListener("scroll", runtime.scrollListener, true);
    }
    runtime.activeScroller = scroller;
    runtime.scrollListenerUsesWindow = nextUsesWindow;
    if (!scroller || !claudeCodexProSettings().threadScrollRestore) return;
    const target = nextUsesWindow ? window : scroller;
    target.addEventListener("scroll", runtime.scrollListener, true);
  }

  function saveThreadScrollPositionNow(sessionId = threadScrollRuntime().activeSessionId, scroller = threadScrollRuntime().activeScroller) {
    if (!claudeCodexProSettings().threadScrollRestore) return;
    const runtime = threadScrollRuntime();
    const key = validThreadScrollSessionKey(sessionId);
    if (!key || !scroller) return;
    if (activeThreadScrollRestoreLock(key)) return;
    const snapshot = {
      top: finiteScrollNumber(scroller.scrollTop),
      scrollHeight: finiteNonNegativeNumber(scroller.scrollHeight),
      clientHeight: finiteNonNegativeNumber(scroller.clientHeight),
      at: Date.now(),
    };
    if (Math.abs(runtime.lastSavedTop - snapshot.top) < 2 && runtime.lastSavedHeight === snapshot.scrollHeight && runtime.lastSavedClientHeight === snapshot.clientHeight) return;
    const entries = readThreadScrollEntries();
    entries[key] = snapshot;
    writeThreadScrollEntries(entries);
    runtime.lastSavedTop = snapshot.top;
    runtime.lastSavedHeight = snapshot.scrollHeight;
    runtime.lastSavedClientHeight = snapshot.clientHeight;
  }

  function scheduleThreadScrollSave() {
    if (!claudeCodexProSettings().threadScrollRestore || window.__codexThreadScrollSaveTimer) return;
    window.__codexThreadScrollSaveTimer = setTimeout(() => {
      window.__codexThreadScrollSaveTimer = null;
      saveThreadScrollPositionNow();
    }, codexThreadScrollSaveThrottleMs);
  }

  function restoreThreadScrollPosition(sessionId) {
    const runtime = threadScrollRuntime();
    const key = validThreadScrollSessionKey(sessionId);
    if (!claudeCodexProSettings().threadScrollRestore || !key || runtime.activeSessionId !== key || userScrollIntentActive() || threadScrollRestoreCancelledForSession(key)) return;
    const lock = activeThreadScrollRestoreLock(key);
    const entry = lock || readThreadScrollEntries()[key];
    if (!entry) return;
    const scroller = currentThreadScroller();
    if (!scroller) return;
    bindThreadScrollListener(scroller);
    const targetTop = threadScrollTargetTop(scroller, lock ? lock.targetTop : entry.top);
    if (Math.abs(scroller.scrollTop - targetTop) <= 1) return;
    runtime.applyingRestore = true;
    try {
      if (typeof scroller.scrollTo === "function") {
        scroller.scrollTo({ top: targetTop, behavior: "auto" });
      } else {
        scroller.scrollTop = targetTop;
      }
    } finally {
      runtime.applyingRestore = false;
    }
    runtime.lastSavedTop = targetTop;
    runtime.lastSavedHeight = finiteNonNegativeNumber(scroller.scrollHeight);
    runtime.lastSavedClientHeight = finiteNonNegativeNumber(scroller.clientHeight);
  }

  function scheduleThreadScrollRestore(sessionId) {
    clearThreadScrollRestoreTimers();
    const key = validThreadScrollSessionKey(sessionId);
    if (!claudeCodexProSettings().threadScrollRestore || !key || userScrollIntentActive() || threadScrollRestoreCancelledForSession(key)) return;
    const entry = readThreadScrollEntries()[key];
    if (!entry) {
      clearThreadScrollRestoreLock();
      return;
    }
    startThreadScrollRestoreLock(key, entry);
    const restoreRevision = (window.__codexThreadScrollRestoreRevision || 0) + 1;
    window.__codexThreadScrollRestoreRevision = restoreRevision;
    window.__codexThreadScrollRestoreTimers = codexThreadScrollRestoreDelaysMs.map((delay) => setTimeout(() => {
      if (window.__codexThreadScrollRestoreRevision !== restoreRevision) return;
      restoreThreadScrollPosition(key);
    }, delay));
  }

  function syncThreadScrollState(forceRestore = false) {
    const runtime = threadScrollRuntime();
    const currentRef = currentSessionRef();
    const nextSessionId = validThreadScrollSessionKey(currentRef.session_id);
    if (!nextSessionId) return;
    if (!claudeCodexProSettings().threadScrollRestore) {
      bindThreadScrollListener(null);
      clearThreadScrollRestoreTimers();
      clearThreadScrollRestoreLock();
      runtime.activeSessionId = nextSessionId;
      return;
    }
    if (runtime.activeSessionId !== nextSessionId) prepareThreadScrollRestoreLock(nextSessionId);
    const nextScroller = currentThreadScroller();
    bindThreadScrollListener(nextScroller);
    if (runtime.activeSessionId !== nextSessionId) {
      runtime.lastSavedTop = -1;
      runtime.lastSavedHeight = -1;
      runtime.lastSavedClientHeight = -1;
      clearThreadScrollRestoreLock();
      runtime.activeSessionId = nextSessionId;
      runtime.pendingNavigation = null;
      runtime.userScrollIntentUntil = 0;
      if (runtime.userCancelledRestoreSessionId !== nextSessionId) runtime.userCancelledRestoreSessionId = "";
      scheduleThreadScrollRestore(nextSessionId);
      return;
    }
    runtime.activeSessionId = nextSessionId;
    if (forceRestore && !userScrollIntentActive() && !threadScrollRestoreCancelledForSession(nextSessionId)) scheduleThreadScrollRestore(nextSessionId);
  }

  function scheduleThreadScrollSyncAttempts(forceRestore = true) {
    const currentKey = validThreadScrollSessionKey(currentSessionRef().session_id) || validThreadScrollSessionKey(threadScrollRuntime().activeSessionId);
    if (userScrollIntentActive() || threadScrollRestoreCancelledForSession(currentKey)) return;
    clearThreadScrollSyncTimers();
    const syncRevision = (window.__codexThreadScrollSyncRevision || 0) + 1;
    window.__codexThreadScrollSyncRevision = syncRevision;
    window.__codexThreadScrollSyncTimers = codexThreadScrollRestoreDelaysMs.map((delay) => setTimeout(() => {
      if (window.__codexThreadScrollSyncRevision !== syncRevision) return;
      scheduleThreadScrollSync(forceRestore);
    }, delay));
  }

  function captureThreadScrollNavigation(targetSessionId) {
    if (!claudeCodexProSettings().threadScrollRestore) return;
    const runtime = threadScrollRuntime();
    const targetKey = validThreadScrollSessionKey(targetSessionId);
    const sessionChanged = !!targetKey && targetKey !== runtime.activeSessionId;
    if (sessionChanged) {
      runtime.userScrollIntentUntil = 0;
      runtime.userCancelledRestoreSessionId = "";
    }
    const pending = runtime.pendingNavigation;
    const duplicatePendingTarget = !!targetKey && pending?.targetSessionId === targetKey && Date.now() - finiteNonNegativeNumber(pending.at) < 5000;
    if (!duplicatePendingTarget) saveThreadScrollPositionNow();
    if (targetKey) {
      runtime.pendingNavigation = { fromSessionId: runtime.activeSessionId, targetSessionId: targetKey, at: Date.now() };
      prepareThreadScrollRestoreLock(targetKey);
    }
    scheduleThreadScrollSyncAttempts(true);
  }

  function editableThreadScrollTarget(element) {
    return !!element?.closest?.("input, textarea, select, [contenteditable='true'], [contenteditable='']");
  }

  function eventTargetsActiveThreadScroller(event) {
    const runtime = threadScrollRuntime();
    const scroller = threadScrollGuardScroller(runtime.activeScroller) || threadScrollGuardScroller(currentThreadScroller());
    if (!scroller) return false;
    const target = event?.target;
    if (!target || target === document || target === window) return true;
    return target === scroller || scroller.contains?.(target) || scroller.contains?.(document.activeElement);
  }

  function markThreadScrollUserIntent(event) {
    if (!claudeCodexProSettings().threadScrollRestore || !eventTargetsActiveThreadScroller(event)) return;
    cancelThreadScrollRestoreForUserIntent();
  }

  function markThreadScrollKeyboardIntent(event) {
    if (editableThreadScrollTarget(event.target)) return;
    if (!["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " ", "Spacebar"].includes(event.key)) return;
    markThreadScrollUserIntent(event);
  }

  function markThreadScrollPointerIntent(event) {
    const scroller = threadScrollGuardScroller(threadScrollRuntime().activeScroller) || threadScrollGuardScroller(currentThreadScroller());
    if (event.target === scroller) markThreadScrollUserIntent(event);
  }

  function updateThreadScrollHandlers() {
    window.__codexThreadScrollHandlers = {
      shouldBlockAutobottom: shouldBlockThreadScrollAutobottom,
      shouldBlockIntoView: shouldBlockThreadScrollIntoView,
      markUserIntent: markThreadScrollUserIntent,
      markKeyboardIntent: markThreadScrollKeyboardIntent,
      markPointerIntent: markThreadScrollPointerIntent,
      captureNavigation: captureThreadScrollNavigation,
      saveNow: saveThreadScrollPositionNow,
      prepareRestoreLock: prepareThreadScrollRestoreLock,
      scheduleSyncAttempts: scheduleThreadScrollSyncAttempts,
    };
  }

  function installThreadScrollUserIntentCapture() {
    if (window.__codexThreadScrollUserIntentInstalled === codexThreadScrollUserIntentVersion) return;
    document.removeEventListener("wheel", window.__codexThreadScrollWheelIntentHandler, true);
    document.removeEventListener("touchmove", window.__codexThreadScrollTouchIntentHandler, true);
    document.removeEventListener("keydown", window.__codexThreadScrollKeyIntentHandler, true);
    document.removeEventListener("pointerdown", window.__codexThreadScrollPointerIntentHandler, true);
    window.__codexThreadScrollWheelIntentHandler = (event) => window.__codexThreadScrollHandlers?.markUserIntent?.(event);
    window.__codexThreadScrollTouchIntentHandler = (event) => window.__codexThreadScrollHandlers?.markUserIntent?.(event);
    window.__codexThreadScrollKeyIntentHandler = (event) => window.__codexThreadScrollHandlers?.markKeyboardIntent?.(event);
    window.__codexThreadScrollPointerIntentHandler = (event) => window.__codexThreadScrollHandlers?.markPointerIntent?.(event);
    document.addEventListener("wheel", window.__codexThreadScrollWheelIntentHandler, { capture: true, passive: true });
    document.addEventListener("touchmove", window.__codexThreadScrollTouchIntentHandler, { capture: true, passive: true });
    document.addEventListener("keydown", window.__codexThreadScrollKeyIntentHandler, true);
    document.addEventListener("pointerdown", window.__codexThreadScrollPointerIntentHandler, true);
    window.__codexThreadScrollUserIntentInstalled = codexThreadScrollUserIntentVersion;
  }

  function installThreadScrollNavigationCapture() {
    document.removeEventListener("pointerdown", window.__codexThreadScrollNavigationHandler, true);
    document.removeEventListener("click", window.__codexThreadScrollClickNavigationHandler, true);
    document.removeEventListener("keydown", window.__codexThreadScrollKeyboardHandler, true);
    const navigationHandler = (event) => {
      if (!claudeCodexProSettings().threadScrollRestore) return;
      const row = event.target?.closest?.(selectors.sidebarThread);
      if (!row) return;
      window.__codexThreadScrollHandlers?.captureNavigation?.(sessionRefFromRow(row).session_id);
    };
    const clickHandler = (event) => {
      if (!claudeCodexProSettings().threadScrollRestore) return;
      const row = event.target?.closest?.(selectors.sidebarThread);
      if (!row) return;
      window.__codexThreadScrollHandlers?.captureNavigation?.(sessionRefFromRow(row).session_id);
    };
    const keyboardHandler = (event) => {
      if (!claudeCodexProSettings().threadScrollRestore) return;
      if (event.key !== "Enter" && event.key !== " ") return;
      const row = event.target?.closest?.(selectors.sidebarThread);
      if (!row) return;
      window.__codexThreadScrollHandlers?.captureNavigation?.(sessionRefFromRow(row).session_id);
    };
    window.__codexThreadScrollNavigationHandler = navigationHandler;
    window.__codexThreadScrollClickNavigationHandler = clickHandler;
    window.__codexThreadScrollKeyboardHandler = keyboardHandler;
    document.addEventListener("pointerdown", navigationHandler, true);
    document.addEventListener("click", clickHandler, true);
    document.addEventListener("keydown", keyboardHandler, true);
  }

  function scheduleThreadScrollSync(forceRestore = false) {
    if (window.__codexThreadScrollSyncPending) return;
    window.__codexThreadScrollSyncPending = true;
    setTimeout(() => {
      window.__codexThreadScrollSyncPending = false;
      syncThreadScrollState(forceRestore);
    }, 0);
  }

  function installThreadScrollRouteHooks() {
    if (window.__codexThreadScrollRouteHooksInstalled === codexThreadScrollRouteHooksVersion) return;
    window.__codexThreadScrollRouteHooksInstalled = codexThreadScrollRouteHooksVersion;
    window.__codexThreadScrollOriginals = window.__codexThreadScrollOriginals || {};
    const originals = window.__codexThreadScrollOriginals;
    ["pushState", "replaceState"].forEach((method) => {
      const currentMethod = history[method];
      const original = originals[`history_${method}`] || currentMethod;
      originals[`history_${method}`] = original;
      if (typeof original !== "function") return;
      history[method] = function codexThreadScrollPatchedHistory(...args) {
        window.__codexThreadScrollHandlers?.saveNow?.();
        const result = original.apply(this, args);
        window.__codexThreadScrollHandlers?.captureNavigation?.(locationThreadId());
        return result;
      };
    });
    window.removeEventListener("popstate", window.__codexThreadScrollPopStateHandler, true);
    window.removeEventListener("hashchange", window.__codexThreadScrollHashChangeHandler, true);
    document.removeEventListener("visibilitychange", window.__codexThreadScrollVisibilityHandler, true);
    window.__codexThreadScrollPopStateHandler = () => {
      window.__codexThreadScrollHandlers?.saveNow?.();
      window.__codexThreadScrollHandlers?.captureNavigation?.(locationThreadId());
    };
    window.__codexThreadScrollHashChangeHandler = () => {
      window.__codexThreadScrollHandlers?.saveNow?.();
      window.__codexThreadScrollHandlers?.captureNavigation?.(locationThreadId());
    };
    window.__codexThreadScrollVisibilityHandler = () => {
      if (document.visibilityState === "hidden") window.__codexThreadScrollHandlers?.saveNow?.();
    };
    window.addEventListener("popstate", window.__codexThreadScrollPopStateHandler, true);
    window.addEventListener("hashchange", window.__codexThreadScrollHashChangeHandler, true);
    document.addEventListener("visibilitychange", window.__codexThreadScrollVisibilityHandler, true);
  }

  async function postJson(path, payload) {
    if (!window.__codexSessionDeleteBridge) {
      if (path === "/backend/status" || path === "/backend/repair") {
        try {
          const response = await fetch(`${helperBase}${path}`, {
            method: "POST",
            headers: withHelperToken({ "Content-Type": "application/json" }),
            body: JSON.stringify(payload || {}),
          });
          return await response.json();
        } catch (error) {
          return { status: "failed", message: "未连接" };
        }
      }
      sendClaudeCodexProDiagnostic("bridge_missing_for_route", { path });
      return { status: "failed", message: "桥接不可用，请重启启动器" };
    }
    function bridgeWithBackendTimeout(path, payload) {
      return Promise.race([
        window.__codexSessionDeleteBridge(path, payload),
        new Promise((resolve) => setTimeout(() => resolve({ status: "failed", message: "后端检查超时", timeout: true }), 2000)),
      ]);
    }
    async function fetchBackendStatusFromHelper(path, payload) {
      try {
        const response = await fetch(`${helperBase}${path}`, {
          method: "POST",
          headers: withHelperToken({ "Content-Type": "application/json" }),
          body: JSON.stringify(payload || {}),
        });
        return await response.json();
      } catch (error) {
        return { status: "failed", message: "未连接" };
      }
    }
    try {
      if (path === "/backend/status") {
        const bridgeStatus = bridgeWithBackendTimeout(path, payload);
        const helperStatus = fetchBackendStatusFromHelper(path, payload);
        const first = await Promise.race([
          bridgeStatus.then((result) => ({ source: "bridge", result })),
          helperStatus.then((result) => ({ source: "helper", result })),
        ]);
        if (first.result?.status === "ok") return first.result;

        const second = first.source === "bridge" ? await helperStatus : await bridgeStatus;
        if (second?.status === "ok") {
          if (first.source === "bridge") {
            sendClaudeCodexProDiagnostic("backend_status_bridge_failed_http_fallback_ok", {
              path,
              httpStatus: 200,
              responseStatus: second.status || "",
            });
          }
          return second;
        }

        const bridgeResult = first.source === "bridge" ? first.result : second;
        const helperResult = first.source === "helper" ? first.result : second;
        if (bridgeResult?.timeout) sendClaudeCodexProDiagnostic("backend_bridge_timeout", { path });
        sendClaudeCodexProDiagnostic("backend_status_bridge_and_http_failed", {
          path,
          errorName: "",
          errorMessage: "",
        });
        return helperResult || bridgeResult;
      }
      if (path === "/backend/repair") {
        const result = await bridgeWithBackendTimeout(path, payload);
        if (result?.status === "ok") return result;
        if (result?.timeout) sendClaudeCodexProDiagnostic("backend_bridge_timeout", { path });
        const fallback = await fetchBackendStatusFromHelper(path, payload);
        if (fallback?.status === "ok") {
          sendClaudeCodexProDiagnostic("backend_status_bridge_failed_http_fallback_ok", {
            path,
            httpStatus: 200,
            responseStatus: fallback.status || "",
          });
          return fallback;
        }
        sendClaudeCodexProDiagnostic("backend_status_bridge_and_http_failed", {
          path,
          errorName: "",
          errorMessage: "",
        });
        return fallback;
      }
      return await window.__codexSessionDeleteBridge(path, payload);
    } catch (error) {
      sendClaudeCodexProDiagnostic("bridge_call_failed", {
        path,
        errorName: error?.name || "",
        errorMessage: error?.message || String(error),
      });
      if (path === "/backend/status" || path === "/backend/repair") {
        const fallback = await fetchBackendStatusFromHelper(path, payload);
        if (fallback?.status === "ok") {
          sendClaudeCodexProDiagnostic("backend_status_bridge_failed_http_fallback_ok", {
            path,
            httpStatus: 200,
            responseStatus: fallback.status || "",
          });
          return fallback;
        }
        sendClaudeCodexProDiagnostic("backend_status_bridge_and_http_failed", {
          path,
          errorName: error?.name || "",
          errorMessage: error?.message || String(error),
        });
        return fallback;
      }
      throw error;
    }
  }

  function downloadMarkdown(filename, markdown) {
    if (!filename || typeof markdown !== "string") {
      throw new Error("导出结果不完整");
    }
    const blob = new Blob([markdown], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  let codexStateApiPromise = null;
  let chatsSortInFlight = false;
  let chatsSortSignature = "";
  let chatsSortLastFetchAt = 0;

  async function codexStateApi() {
    codexStateApiPromise = codexStateApiPromise || import("./assets/vscode-api-Dc9pX2Bc.js");
    const api = await codexStateApiPromise;
    if (typeof api.n !== "function") throw new Error("Codex 状态 API 不可用");
    return api.n;
  }

  async function codexStateCall(method, params) {
    const call = await codexStateApi();
    return await call(method, params);
  }

  async function getCodexGlobalState(key) {
    const result = await codexStateCall("get-global-state", { params: { key } });
    return result && Object.prototype.hasOwnProperty.call(result, "value") ? result.value : result;
  }

  async function setCodexGlobalState(key, value) {
    return await codexStateCall("set-global-state", { params: { key, value } });
  }

  function objectGlobalState(value) {
    return value && typeof value === "object" && !Array.isArray(value) ? { ...value } : {};
  }

  function uniqueValues(values) {
    return Array.from(new Set(values.filter((value) => typeof value === "string" && value.trim().length > 0)));
  }

  let codexModelCatalog = { status: "loading", model: "", default_model: "", model_provider: "", provider_name: "", models: [], model_descriptors: [], sources: [], responses_api: { status: "unknown", message: "" } };
  let codexModelCatalogLoadedAt = 0;
  let codexModelCatalogPromise = null;

  if (window.__CLAUDE_CODEX_PRO_TEST_SERVICE_TIER__) {
    window.__claudeCodexProServiceTierTest = {
      applyServiceTierOverride: (method, params, threadIdHint = "") => applyCodexServiceTierRequestOverride(method, params, threadIdHint),
      requestOverride: (message) => codexServiceTierRequestOverride(message),
      diagnostics: () => [...(window.__claudeCodexProServiceTierTestDiagnostics || [])],
      setModelCatalog: (catalog = {}) => {
        codexModelCatalog = {
          status: "ok",
          model: "",
          default_model: "",
          model_provider: "",
          provider_name: "",
          models: [],
          model_descriptors: [],
          sources: [],
          responses_api: { status: "unknown", message: "" },
          ...catalog,
        };
        codexModelCatalogLoadedAt = Date.now();
        codexModelCatalogPromise = null;
      },
      setServiceTierState: (state = {}) => {
        codexServiceTierState = { ...codexServiceTierState, ...state };
      },
      setThreadState: (state = {}) => {
        localStorage.setItem(codexThreadServiceTierKey, JSON.stringify({
          version: codexThreadServiceTierVersion,
          mode: "inherit",
          defaultMode: "inherit",
          entries: {},
          ...state,
        }));
      },
    };
    return;
  }

  async function loadCodexModelCatalog(force = false) {
    if (!force && codexModelCatalogPromise) return codexModelCatalogPromise;
    if (!force && codexModelCatalogLoadedAt && Date.now() - codexModelCatalogLoadedAt < 10000) return codexModelCatalog;
    codexModelCatalogPromise = postJson("/codex-model-catalog", {})
      .then((result) => {
        codexModelCatalog = result && typeof result === "object" ? result : { status: "failed", model: "", default_model: "", model_provider: "", provider_name: "", models: [], model_descriptors: [], sources: [], responses_api: { status: "unknown", message: "" } };
        codexModelCatalogLoadedAt = Date.now();
        refreshCodexServiceTierControls();
        return codexModelCatalog;
      })
      .catch((error) => {
        codexModelCatalog = { status: "failed", message: String(error?.message || error), model: "", default_model: "", model_provider: "", provider_name: "", models: [], model_descriptors: [], sources: [], responses_api: { status: "unknown", message: "" } };
        codexModelCatalogLoadedAt = Date.now();
        return codexModelCatalog;
      })
      .finally(() => {
        codexModelCatalogPromise = null;
      });
    return codexModelCatalogPromise;
  }

  function applyCodexRequestOverrides(method, params, threadIdHint = "") {
    if (!claudeCodexProSettings().serviceTierControls) return params;
    return applyCodexServiceTierRequestOverride(method, params, threadIdHint);
  }

  function threadIdVariants(sessionId) {
    if (typeof sessionId !== "string" || !sessionId.trim()) return [];
    const id = sessionId.trim();
    const bareId = id.startsWith("local:") ? id.slice("local:".length) : id;
    return uniqueValues([id, bareId, `local:${bareId}`]);
  }

  function projectMoveSessionKey(sessionId) {
    const variants = threadIdVariants(sessionId);
    const bareId = variants.find((id) => !id.startsWith("local:"));
    return bareId || variants[0] || "";
  }

  function uuidV7TimestampMs(sessionId) {
    const id = projectMoveSessionKey(sessionId).replaceAll("-", "");
    if (!/^[0-9a-fA-F]{12}/.test(id)) return 0;
    const timestamp = Number.parseInt(id.slice(0, 12), 16);
    return Number.isFinite(timestamp) ? timestamp : 0;
  }

  function numericTimestamp(value) {
    const timestamp = Number(value);
    return Number.isFinite(timestamp) && timestamp > 0 ? timestamp : 0;
  }

  function timestampValueToMs(value) {
    const timestamp = numericTimestamp(value);
    if (!timestamp) return 0;
    return timestamp < 1000000000000 ? timestamp * 1000 : timestamp;
  }

  function sortMsForSession(sessionId, preferredValue) {
    return numericTimestamp(preferredValue) || uuidV7TimestampMs(sessionId);
  }

  function timestampMsFromPayload(payload) {
    return numericTimestamp(payload?.updated_at_ms) || timestampValueToMs(payload?.updated_at) || numericTimestamp(payload?.created_at_ms);
  }

  function relativeTimeLabel(timestampMs, nowMs = Date.now()) {
    const timestamp = numericTimestamp(timestampMs);
    if (!timestamp) return "";
    const elapsedSeconds = Math.max(0, Math.floor((nowMs - timestamp) / 1000));
    if (elapsedSeconds < 60) return "刚刚";
    const elapsedMinutes = Math.floor(elapsedSeconds / 60);
    if (elapsedMinutes < 60) return `${elapsedMinutes} 分`;
    const elapsedHours = Math.floor(elapsedMinutes / 60);
    if (elapsedHours < 24) return `${elapsedHours} 小时`;
    const elapsedDays = Math.floor(elapsedHours / 24);
    if (elapsedDays < 7) return `${elapsedDays} 天`;
    const elapsedWeeks = Math.floor(elapsedDays / 7);
    if (elapsedWeeks < 5) return `${elapsedWeeks} 周`;
    const elapsedMonths = Math.floor(elapsedDays / 30);
    if (elapsedMonths < 12) return `${Math.max(1, elapsedMonths)} 月`;
    return `${Math.floor(elapsedDays / 365)} 年`;
  }

  function normalizeWorkspacePath(path) {
    const normalized = String(path || "").trim().replace(/\\/g, "/").replace(/\/+$/, "");
    return normalized || String(path || "").trim();
  }

  function sameWorkspacePath(left, right) {
    const leftPath = normalizeWorkspacePath(left);
    const rightPath = normalizeWorkspacePath(right);
    return !!leftPath && !!rightPath && leftPath === rightPath;
  }

  function displayProjectName(path) {
    const trimmed = String(path || "").replace(/\/+$/, "");
    return trimmed.split(/[\\/]+/).filter(Boolean).pop() || trimmed || "未命名项目";
  }

  function normalizeProjectLabel(value) {
    return String(value || "").replace(/\s+/g, " ").trim();
  }

  function projectsSection() {
    return document.querySelector('[data-app-action-sidebar-section-heading="Projects"]');
  }

  function chatsSection() {
    return document.querySelector('[data-app-action-sidebar-section-heading="Chats"]');
  }

  function projectRowListItem(projectRow) {
    return projectRow.closest?.('[role="listitem"][aria-label]') || projectRow.closest?.('[role="listitem"]') || projectRow;
  }

  function nativeProjectTargets() {
    const section = projectsSection();
    const seen = new Set();
    const targets = [];
    Array.from(document.querySelectorAll('[data-app-action-sidebar-project-row]')).forEach((row) => {
      if (section && !section.contains(row)) return;
      const path = row.getAttribute("data-app-action-sidebar-project-id") || "";
      const normalizedPath = normalizeWorkspacePath(path);
      if (!normalizedPath || seen.has(normalizedPath)) return;
      const label = row.getAttribute("data-app-action-sidebar-project-label") || row.getAttribute("aria-label") || displayProjectName(path);
      seen.add(normalizedPath);
      targets.push({ kind: "project", label: String(label || displayProjectName(path)), description: path, path, normalizedPath, row, listItem: projectRowListItem(row) });
    });
    return targets;
  }

  function serializableProjectTarget(target) {
    return { kind: target.kind, label: target.label, description: target.description, path: target.path, normalizedPath: target.normalizedPath || normalizeWorkspacePath(target.path) };
  }

  function projectMoveTargets() {
    return [
      { kind: "projectless", label: "普通对话", description: "不属于任何项目", path: "", normalizedPath: "" },
      ...nativeProjectTargets().map(serializableProjectTarget),
    ];
  }

  function readLegacyProjectMoveProjection() {
    try {
      const parsed = JSON.parse(localStorage.getItem(legacyProjectMoveOverridesKey) || "{}");
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
      const now = Date.now();
      const next = {};
      for (const [key, value] of Object.entries(parsed)) {
        if (!value || typeof value !== "object" || !value.targetCwd) continue;
        const sessionId = projectMoveSessionKey(value.sessionId || key);
        if (!sessionId) continue;
        next[sessionId] = {
          sessionId,
          targetKind: "project",
          targetCwd: String(value.targetCwd),
          targetLabel: String(value.targetLabel || displayProjectName(value.targetCwd)),
          title: String(value.title || ""),
          sortMs: sortMsForSession(sessionId, value.sortMs || value.updatedAtMs || value.updated_at_ms),
          sortMsTrusted: false,
          at: typeof value.at === "number" ? value.at : now,
        };
      }
      return next;
    } catch {
      return {};
    }
  }

  function readProjectMoveProjection() {
    try {
      const parsed = JSON.parse(localStorage.getItem(projectMoveProjectionKey) || "{}");
      const raw = parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
      const merged = { ...readLegacyProjectMoveProjection(), ...raw };
      const now = Date.now();
      const projection = {};
      for (const [key, value] of Object.entries(merged)) {
        if (!value || typeof value !== "object") continue;
        const sessionId = projectMoveSessionKey(value.sessionId || key);
        if (!sessionId) continue;
        if (typeof value.at === "number" && now - value.at > projectMoveProjectionTtlMs) continue;
        const targetKind = value.targetKind === "projectless" ? "projectless" : "project";
        const targetCwd = String(value.targetCwd || value.path || "");
        if (targetKind === "project" && !targetCwd) continue;
        projection[sessionId] = {
          sessionId,
          targetKind,
          targetCwd,
          targetLabel: String(value.targetLabel || value.label || (targetKind === "projectless" ? "普通对话" : displayProjectName(targetCwd))),
          title: String(value.title || ""),
          sortMs: sortMsForSession(sessionId, value.sortMs || value.updatedAtMs || value.updated_at_ms),
          sortMsTrusted: value.sortMsTrusted === true,
          at: typeof value.at === "number" ? value.at : now,
        };
      }
      return projection;
    } catch {
      return readLegacyProjectMoveProjection();
    }
  }

  function writeProjectMoveProjection(projection) {
    try {
      localStorage.setItem(projectMoveProjectionKey, JSON.stringify(projection || {}));
      localStorage.removeItem(legacyProjectMoveOverridesKey);
    } catch (error) {
      window.__codexProjectMoveProjectionFailures = window.__codexProjectMoveProjectionFailures || [];
      window.__codexProjectMoveProjectionFailures.push(String(error?.stack || error));
    }
  }

  function saveProjectMoveProjection(ref, target, sortMs) {
    const id = projectMoveSessionKey(ref.session_id);
    if (!id || !target) return;
    const projection = readProjectMoveProjection();
    projection[id] = {
      sessionId: id,
      targetKind: target.kind === "projectless" ? "projectless" : "project",
      targetCwd: target.path || "",
      targetLabel: target.label || (target.kind === "projectless" ? "普通对话" : displayProjectName(target.path)),
      title: ref.title || "",
      sortMs: sortMsForSession(ref.session_id, sortMs || target.sortMs),
      sortMsTrusted: target.sortMsTrusted === true,
      at: Date.now(),
    };
    writeProjectMoveProjection(projection);
  }

  function clearProjectMoveProjection(ref) {
    const projection = readProjectMoveProjection();
    const keys = threadIdVariants(ref.session_id).map(projectMoveSessionKey).filter(Boolean);
    let changed = false;
    keys.forEach((key) => {
      if (Object.prototype.hasOwnProperty.call(projection, key)) {
        delete projection[key];
        changed = true;
      }
    });
    if (changed) writeProjectMoveProjection(projection);
  }

  function projectionForSessionId(sessionId, projection = readProjectMoveProjection()) {
    const key = projectMoveSessionKey(sessionId);
    return key ? projection[key] || null : null;
  }

  function projectRowFromListItem(projectItem) {
    if (!projectItem) return null;
    if (projectItem.matches?.("[data-app-action-sidebar-project-row]")) return projectItem;
    return projectItem.querySelector?.("[data-app-action-sidebar-project-row]") || null;
  }

  function targetPath(target) {
    return target?.path || target?.targetCwd || "";
  }

  function targetLabel(target) {
    return target?.label || target?.targetLabel || displayProjectName(targetPath(target));
  }

  function projectItemMatchesTarget(projectItem, target) {
    const projectRow = projectRowFromListItem(projectItem);
    const projectPath = projectRow?.getAttribute?.("data-app-action-sidebar-project-id") || "";
    if (projectPath && sameWorkspacePath(projectPath, targetPath(target))) return true;
    const actual = normalizeProjectLabel(projectRow?.getAttribute?.("data-app-action-sidebar-project-label") || projectItem?.getAttribute?.("aria-label"));
    const labels = uniqueValues([targetLabel(target), displayProjectName(targetPath(target))]).map(normalizeProjectLabel).filter(Boolean);
    return !!actual && labels.includes(actual);
  }

  function findProjectListItem(target) {
    const nativeTarget = nativeProjectTargets().find((project) => sameWorkspacePath(project.path, targetPath(target)));
    if (nativeTarget?.listItem) return nativeTarget.listItem;
    const section = projectsSection();
    if (!section) return null;
    return Array.from(section.querySelectorAll('[role="listitem"][aria-label]')).find((item) => projectItemMatchesTarget(item, target)) || null;
  }

  function closestProjectListItem(row) {
    const item = row.closest?.('[role="listitem"][aria-label]');
    return item?.closest?.('[data-app-action-sidebar-section-heading="Projects"]') ? item : null;
  }

  function rowIsInChats(row) {
    return !!row.closest?.('[data-app-action-sidebar-section-heading="Chats"]');
  }

  function chatsThreadList() {
    return chatsSection()?.querySelector?.('[role="list"][aria-label="对话"], [role="list"]') || null;
  }

  function rowIsUnderTargetProject(row, target) {
    const item = closestProjectListItem(row);
    return !!item && projectItemMatchesTarget(item, target);
  }

  function rowIsUnderTarget(row, target) {
    return target?.targetKind === "projectless" || target?.kind === "projectless" ? rowIsInChats(row) : rowIsUnderTargetProject(row, target);
  }

  function rowListItem(row) {
    return row.closest?.('[role="listitem"]') || row;
  }

  function rowContentRoot(row) {
    return Array.from(row?.children || []).find((child) => String(child.className || "").includes("h-full w-full items-center")) || null;
  }

  function normalizedText(node) {
    return String(node?.textContent || "").replace(/\s+/g, " ").trim();
  }

  function classNameText(node) {
    return String(node?.className || "");
  }

  function isRelativeTimeText(text) {
    const value = String(text || "").replace(/\s+/g, " ").trim();
    return /^(刚刚|just now|\d+\s*(秒|秒钟|分|分钟|小时|天|日|周|星期|个月|月|年|sec|secs|second|seconds|min|mins|minute|minutes|h|hr|hrs|hour|hours|d|day|days|w|wk|wks|week|weeks|mo|mos|month|months|y|yr|yrs|year|years))$/i.test(value);
  }

  function nodeIsThreadTitle(row, node) {
    return Array.from(row?.querySelectorAll?.('[data-thread-title], .truncate.select-none, .truncate.text-base') || [])
      .some((titleNode) => titleNode === node || titleNode.contains(node));
  }

  function closestTimeWrapper(row, node) {
    const root = rowContentRoot(row) || row;
    let current = node?.parentElement || null;
    while (current && current !== root && current !== row) {
      const className = classNameText(current);
      if (current.dataset?.codexProjectMoveTimeWrapper === "true" || (className.includes("ml-[3px]") && className.includes("min-w-[26px]"))) return current;
      current = current.parentElement;
    }
    return null;
  }

  function nodeInsideStatusIcon(row, node) {
    const stop = closestTimeWrapper(row, node) || rowContentRoot(row) || row;
    let current = node || null;
    while (current && current !== stop && current !== row) {
      const className = classNameText(current);
      if (className.includes("animate-spin")) return true;
      if (className.includes("size-5") && className.includes("shrink-0")) return true;
      if (className.includes("contain-paint") && className.includes("contain-layout")) return true;
      current = current.parentElement;
    }
    return false;
  }

  function cleanupManagedStatusIconTimeNodes(row) {
    Array.from(row?.querySelectorAll?.('[data-codex-project-move-time="true"]') || []).forEach((node) => {
      if (!nodeInsideStatusIcon(row, node)) return;
      const text = normalizedText(node);
      delete node.dataset.codexProjectMoveTime;
      delete node.dataset.codexProjectMoveTimeMs;
      if (node.children.length === 0 && isRelativeTimeText(text)) node.textContent = "";
    });
  }

  function nodeLooksLikeTimeLabel(row, node) {
    if (nodeInsideStatusIcon(row, node)) return false;
    if (node?.dataset?.codexProjectMoveTime === "true") return true;
    if (node.children.length > 0) return false;
    const text = normalizedText(node);
    const className = classNameText(node);
    if ((className.includes("tabular-nums") || className.includes("text-token-description-foreground")) && text.length <= 24) return true;
    if (!isRelativeTimeText(text)) return false;
    const rowRect = row?.getBoundingClientRect?.();
    const nodeRect = node?.getBoundingClientRect?.();
    if (!rowRect || !nodeRect || rowRect.width <= 0 || nodeRect.width <= 0) return false;
    return nodeRect.left >= rowRect.left + rowRect.width * 0.45 || nodeRect.right >= rowRect.right - 96;
  }

  function rowTimeLabelCandidates(row) {
    cleanupManagedStatusIconTimeNodes(row);
    const root = rowContentRoot(row) || row;
    const raw = Array.from(root?.querySelectorAll?.("div, span, time, small") || []).filter((node) => {
      if (nodeIsThreadTitle(row, node)) return false;
      return nodeLooksLikeTimeLabel(row, node);
    });
    return raw.filter((node) => !raw.some((other) => other !== node && node.contains(other)));
  }

  function rowTimeLabelNode(row) {
    const candidates = rowTimeLabelCandidates(row);
    return candidates.find((node) => node.dataset?.codexProjectMoveTime !== "true" && !node.closest?.('[data-codex-project-move-time-wrapper="true"]')) || candidates[0] || null;
  }

  function removeTimeLabelNode(row, node) {
    if (!node || !row?.contains?.(node)) return;
    const wrapper = node.closest?.('[data-codex-project-move-time-wrapper="true"]') || closestTimeWrapper(row, node);
    if (wrapper && wrapper !== row && row.contains(wrapper)) {
      wrapper.remove();
      return;
    }
    node.remove();
  }

  function cleanupRowTimeLabels(row, keepNode) {
    if (!keepNode) return;
    rowTimeLabelCandidates(row).forEach((node) => {
      if (node === keepNode) return;
      if (node.dataset?.codexProjectMoveTime === "true" || node.closest?.('[data-codex-project-move-time-wrapper="true"]')) removeTimeLabelNode(row, node);
    });
  }

  function ensureRowTimeLabelNode(row) {
    const existing = rowTimeLabelNode(row);
    if (existing) {
      cleanupRowTimeLabels(row, existing);
      return existing;
    }
    const root = rowContentRoot(row);
    if (!root) return null;
    const wrapper = document.createElement("div");
    wrapper.className = "ml-[3px] flex items-center justify-end gap-1 min-w-[26px]";
    wrapper.dataset.codexProjectMoveTimeWrapper = "true";
    const inner = document.createElement("div");
    const label = document.createElement("div");
    label.className = "text-token-description-foreground text-sm leading-4 empty:hidden tabular-nums overflow-visible truncate text-right group-focus-within:opacity-0 group-hover:opacity-0";
    label.dataset.codexProjectMoveTime = "true";
    inner.appendChild(label);
    wrapper.appendChild(inner);
    root.appendChild(wrapper);
    return label;
  }

  function updateRowTimeLabel(row, sortMs) {
    const label = ensureRowTimeLabelNode(row);
    if (!label) return;
    const timestamp = numericTimestamp(sortMs);
    const text = relativeTimeLabel(timestamp);
    label.dataset.codexProjectMoveTime = "true";
    label.dataset.codexProjectMoveTimeMs = String(timestamp || 0);
    if (text && label.textContent !== text) label.textContent = text;
    cleanupRowTimeLabels(row, label);
  }

  function rowProjectionKind(row) {
    return row?.dataset?.codexProjectMoveTargetKind || rowListItem(row)?.dataset?.codexProjectMoveTargetKind || "";
  }

  function rowSortMs(row, ref = sessionRefFromRow(row), target = null) {
    return sortMsForSession(ref.session_id, target?.sortMs || row?.dataset?.codexProjectMoveSortMs || rowListItem(row)?.dataset?.codexProjectMoveSortMs);
  }

  function threadRowFromListItem(item) {
    if (!item) return null;
    if (item.matches?.("[data-app-action-sidebar-thread-id]")) return item;
    return item.querySelector?.("[data-app-action-sidebar-thread-id]") || null;
  }

  function rowPinned(row) {
    return row?.getAttribute?.("data-app-action-sidebar-thread-pinned") === "true" || rowListItem(row)?.getAttribute?.("data-app-action-sidebar-thread-pinned") === "true";
  }

  function insertRowItemByTime(list, item, row, target) {
    const ref = sessionRefFromRow(row);
    const sortMs = rowSortMs(row, ref, target);
    item.dataset.codexProjectMoveSortMs = String(sortMs || 0);
    row.dataset.codexProjectMoveSortMs = String(sortMs || 0);
    if (target?.sortMsTrusted) updateRowTimeLabel(row, sortMs);
    const pinned = rowPinned(row);
    const sessionKey = projectMoveSessionKey(ref.session_id);
    const existingItems = Array.from(list.children).filter((child) => child !== item);
    let firstNonThreadItem = null;
    for (const child of existingItems) {
      const childRow = threadRowFromListItem(child);
      if (!childRow) {
        firstNonThreadItem = firstNonThreadItem || child;
        continue;
      }
      const childPinned = rowPinned(childRow);
      if (childPinned && !pinned) continue;
      if (!childPinned && pinned) {
        list.insertBefore(item, child);
        return;
      }
      const childRef = sessionRefFromRow(childRow);
      const childSortMs = rowSortMs(childRow, childRef);
      const childKey = projectMoveSessionKey(childRef.session_id);
      if (sortMs > childSortMs || (sortMs === childSortMs && sessionKey > childKey)) {
        list.insertBefore(item, child);
        return;
      }
    }
    if (firstNonThreadItem) {
      list.insertBefore(item, firstNonThreadItem);
      return;
    }
    list.appendChild(item);
  }

  function projectMoveInjectedList(projectItem) {
    let list = projectItem.querySelector('[data-codex-project-move-injected-list="true"]');
    if (!list) {
      const body = Array.from(projectItem.children).find((child) => child.classList?.contains("overflow-hidden")) || projectItem;
      list = document.createElement("div");
      list.setAttribute("role", "list");
      list.setAttribute("data-codex-project-move-injected-list", "true");
      list.className = "flex flex-col";
      body.appendChild(list);
    }
    return list;
  }

  function projectThreadList(projectItem, target) {
    const targetCwd = targetPath(target);
    const projectLists = Array.from(projectItem.querySelectorAll("[data-app-action-sidebar-project-list-id]"));
    return projectLists.find((list) => sameWorkspacePath(list.getAttribute("data-app-action-sidebar-project-list-id"), targetCwd))
      || projectLists[0]
      || projectMoveInjectedList(projectItem);
  }

  function projectEmptyStateNodes(projectItem) {
    const emptyLabels = new Set(["暂无对话", "No conversations"]);
    return Array.from(projectItem.querySelectorAll("div, span")).filter((node) => {
      if (node.classList?.contains("overflow-hidden")) return false;
      if (node.closest('[data-app-action-sidebar-thread-id], [data-codex-project-move-injected-list="true"]')) return false;
      return emptyLabels.has(normalizeProjectLabel(node.textContent));
    });
  }

  function setProjectEmptyStateHidden(projectItem, hidden) {
    projectEmptyStateNodes(projectItem).forEach((node) => {
      if (hidden) {
        node.dataset.codexProjectMoveEmptyHidden = "true";
        node.classList.add("codex-project-move-hidden");
      } else if (node.dataset.codexProjectMoveEmptyHidden === "true") {
        delete node.dataset.codexProjectMoveEmptyHidden;
        node.classList.remove("codex-project-move-hidden");
      }
    });
  }

  function updateProjectMoveEmptyStates() {
    document.querySelectorAll('[data-codex-project-move-injected-list="true"]').forEach((list) => {
      const projectItem = list.closest('[role="listitem"][aria-label]');
      const hasRows = Array.from(list.children).some((child) => child.querySelector?.("[data-app-action-sidebar-thread-id]") || child.matches?.("[data-app-action-sidebar-thread-id]"));
      if (!hasRows) list.remove();
      if (projectItem) setProjectEmptyStateHidden(projectItem, hasRows);
    });
    document.querySelectorAll('[data-codex-project-move-empty-hidden="true"]').forEach((node) => {
      const projectItem = node.closest('[role="listitem"][aria-label]');
      const list = projectItem?.querySelector?.('[data-codex-project-move-injected-list="true"]');
      if (!list || list.children.length === 0) {
        delete node.dataset.codexProjectMoveEmptyHidden;
        node.classList.remove("codex-project-move-hidden");
      }
    });
  }

  function moveRowToProjectList(row, target) {
    const projectItem = findProjectListItem(target);
    if (!projectItem) return false;
    const list = projectThreadList(projectItem, target);
    const item = rowListItem(row);
    if (!list) return false;
    insertRowItemByTime(list, item, row, target);
    cachedSessionRowsAt = 0;
    item.dataset.codexProjectMoveTargetKind = "project";
    item.dataset.codexProjectMoveTargetCwd = targetPath(target);
    row.dataset.codexProjectMoveTargetKind = "project";
    row.dataset.codexProjectMoveTargetCwd = targetPath(target);
    setProjectEmptyStateHidden(projectItem, true);
    return true;
  }

  function moveRowToChats(row, target = null) {
    const list = chatsThreadList();
    if (!list) return false;
    const item = rowListItem(row);
    insertRowItemByTime(list, item, row, target);
    cachedSessionRowsAt = 0;
    item.dataset.codexProjectMoveTargetKind = "projectless";
    row.dataset.codexProjectMoveTargetKind = "projectless";
    delete item.dataset.codexProjectMoveTargetCwd;
    delete row.dataset.codexProjectMoveTargetCwd;
    updateProjectMoveEmptyStates();
    return true;
  }

  function applyProjectMoveProjection() {
    if (!claudeCodexProSettings().projectMove) return;
    const projection = readProjectMoveProjection();
    const targetRowsById = new Map();
    const settledRefs = [];
    const now = Date.now();
    const rows = sessionRows(true);
    rows.forEach((row) => {
      const ref = sessionRefFromRow(row);
      const target = projectionForSessionId(ref.session_id, projection);
      if (target && rowIsUnderTarget(row, target)) {
        const rowId = projectMoveSessionKey(ref.session_id);
        const hadProjectionKind = !!rowProjectionKind(row);
        const existingRow = targetRowsById.get(rowId);
        if (existingRow && existingRow !== row) {
          const existingIsProjection = !!rowProjectionKind(existingRow);
          const currentIsProjection = !!rowProjectionKind(row);
          const rowToRemove = existingIsProjection && !currentIsProjection ? existingRow : row;
          rowListItem(rowToRemove).remove();
          if (rowToRemove === existingRow) targetRowsById.set(rowId, row);
          if (rowToRemove === row) return;
        } else {
          targetRowsById.set(rowId, row);
        }
        if (!hadProjectionKind && typeof target.at === "number" && now - target.at > projectMoveProjectionSettleMs) settledRefs.push(ref);
        const moved = target.targetKind === "projectless" ? moveRowToChats(row, target) : moveRowToProjectList(row, target);
        if (moved) targetRowsById.set(rowId, row);
        const projectItem = closestProjectListItem(row);
        if (projectItem) setProjectEmptyStateHidden(projectItem, true);
      }
    });
    rows.forEach((row) => {
      const ref = sessionRefFromRow(row);
      const rowId = projectMoveSessionKey(ref.session_id);
      const target = projectionForSessionId(ref.session_id, projection);
      if (!target) {
        const item = rowListItem(row);
        delete row.dataset.codexProjectMoveTargetKind;
        delete row.dataset.codexProjectMoveTargetCwd;
        delete item.dataset.codexProjectMoveTargetKind;
        delete item.dataset.codexProjectMoveTargetCwd;
        return;
      }
      if (rowIsUnderTarget(row, target)) return;
      if (targetRowsById.has(rowId)) {
        rowListItem(row).remove();
        return;
      }
      const moved = target.targetKind === "projectless" ? moveRowToChats(row, target) : moveRowToProjectList(row, target);
      if (moved) targetRowsById.set(rowId, row);
    });
    settledRefs.forEach(clearProjectMoveProjection);
    updateProjectMoveEmptyStates();
  }

  function scheduleProjectMoveProjection() {
    if (!claudeCodexProSettings().projectMove || window.__codexProjectMoveProjectionTimer) return;
    window.__codexProjectMoveProjectionTimer = setTimeout(() => {
      if (window.__codexProjectMoveRuntimeId !== codexProjectMoveRuntimeId) return;
      window.__codexProjectMoveProjectionTimer = null;
      applyProjectMoveProjection();
    }, 80);
  }

  async function refreshRecentConversationsForHost() {
    try {
      const signals = await import("./assets/app-server-manager-signals-C1h8B-R-.js");
      if (typeof signals.rn === "function") await signals.rn("refresh-recent-conversations-for-host", { hostId: "local", sortKey: "updated_at" });
    } catch (error) {
      window.__codexProjectMoveRefreshFailures = window.__codexProjectMoveRefreshFailures || [];
      window.__codexProjectMoveRefreshFailures.push(String(error?.stack || error));
    }
  }

  function refreshAfterProjectMove() {
    const refreshVisibleSidebar = () => {
      applyProjectMoveProjection();
      scheduleChatsSortCorrection(0);
    };
    refreshVisibleSidebar();
    refreshRecentConversationsForHost().finally(() => {
      projectMoveRefreshDelaysMs.forEach((delay) => setTimeout(refreshVisibleSidebar, delay));
    });
  }

  function visibleChatsRows() {
    const list = chatsThreadList();
    if (!list) return [];
    return Array.from(list.children).map(threadRowFromListItem).filter(Boolean).filter((row) => rowIsInChats(row));
  }

  function chatsSortNeedsCorrection(rows) {
    let previousPinned = true;
    let previousSortMs = Infinity;
    let previousKey = "\uffff";
    for (const row of rows) {
      const pinned = rowPinned(row);
      const ref = sessionRefFromRow(row);
      const sortMs = rowSortMs(row, ref);
      const key = projectMoveSessionKey(ref.session_id);
      if (previousPinned && !pinned) {
        previousPinned = false;
        previousSortMs = sortMs;
        previousKey = key;
        continue;
      }
      if (!previousPinned && pinned) return true;
      if (sortMs > previousSortMs || (sortMs === previousSortMs && key > previousKey)) return true;
      previousSortMs = sortMs;
      previousKey = key;
    }
    return false;
  }

  function reorderChatsRows(rows) {
    const list = chatsThreadList();
    if (!list || rows.length < 2) return;
    const rowItems = new Set(rows.map(rowListItem));
    const firstNonThreadItem = Array.from(list.children).find((child) => !rowItems.has(child) && !threadRowFromListItem(child));
    const orderedRows = [...rows].sort((left, right) => {
      const leftPinned = rowPinned(left);
      const rightPinned = rowPinned(right);
      if (leftPinned !== rightPinned) return leftPinned ? -1 : 1;
      const leftRef = sessionRefFromRow(left);
      const rightRef = sessionRefFromRow(right);
      const leftSortMs = rowSortMs(left, leftRef);
      const rightSortMs = rowSortMs(right, rightRef);
      if (leftSortMs !== rightSortMs) return rightSortMs - leftSortMs;
      return projectMoveSessionKey(rightRef.session_id).localeCompare(projectMoveSessionKey(leftRef.session_id));
    });
    orderedRows.forEach((row) => list.insertBefore(rowListItem(row), firstNonThreadItem || null));
    cachedSessionRowsAt = 0;
  }

  async function applyChatsSortCorrection() {
    if (!claudeCodexProSettings().projectMove || chatsSortInFlight) return;
    const rows = visibleChatsRows();
    if (rows.length < 2) return;
    const refs = rows.map(sessionRefFromRow).filter((ref) => ref.session_id);
    const signature = refs.map((ref) => projectMoveSessionKey(ref.session_id)).join("|");
    const allRowsHaveSortMs = rows.every((row) => numericTimestamp(row.dataset.codexProjectMoveSortMs || rowListItem(row).dataset.codexProjectMoveSortMs));
    const shouldRefreshSortKeys = signature !== chatsSortSignature || !allRowsHaveSortMs || Date.now() - chatsSortLastFetchAt > chatsSortDbRefreshIntervalMs;
    if (!shouldRefreshSortKeys && !chatsSortNeedsCorrection(rows)) return;
    chatsSortInFlight = true;
    try {
      if (shouldRefreshSortKeys) {
        const result = await postJson("/thread-sort-keys", { sessions: refs }).catch(() => ({ status: "failed", sort_keys: [] }));
        chatsSortLastFetchAt = Date.now();
        const byId = new Map();
        if (result?.status === "ok" && Array.isArray(result?.sort_keys)) {
          result.sort_keys.forEach((item) => {
            const key = projectMoveSessionKey(String(item?.session_id || ""));
            if (key) byId.set(key, item);
          });
        }
        rows.forEach((row) => {
          const ref = sessionRefFromRow(row);
          const payload = byId.get(projectMoveSessionKey(ref.session_id));
          const trustedSortMs = timestampMsFromPayload(payload);
          const sortMs = trustedSortMs || sortMsForSession(ref.session_id, row.dataset.codexProjectMoveSortMs || rowListItem(row).dataset.codexProjectMoveSortMs);
          row.dataset.codexProjectMoveSortMs = String(sortMs || 0);
          rowListItem(row).dataset.codexProjectMoveSortMs = String(sortMs || 0);
          if (trustedSortMs) updateRowTimeLabel(row, trustedSortMs);
        });
      }
      if (chatsSortNeedsCorrection(rows)) reorderChatsRows(rows);
      chatsSortSignature = visibleChatsRows().map((row) => projectMoveSessionKey(sessionRefFromRow(row).session_id)).join("|");
    } finally {
      chatsSortInFlight = false;
    }
  }

  function scheduleChatsSortCorrection(delay = chatsSortRefreshIntervalMs) {
    if (!claudeCodexProSettings().projectMove || window.__codexProjectMoveChatsSortTimer) return;
    window.__codexProjectMoveChatsSortTimer = setTimeout(() => {
      if (window.__codexProjectMoveRuntimeId !== codexProjectMoveRuntimeId) return;
      window.__codexProjectMoveChatsSortTimer = null;
      applyChatsSortCorrection().catch((error) => {
        window.__codexProjectMoveSortFailures = window.__codexProjectMoveSortFailures || [];
        window.__codexProjectMoveSortFailures.push(String(error?.stack || error));
      }).finally(() => {
        if (claudeCodexProSettings().projectMove) scheduleChatsSortCorrection();
      });
    }, delay);
  }

  async function setProjectlessThreadIds(ref, mode) {
    const variants = threadIdVariants(ref.session_id);
    if (variants.length === 0) throw new Error("未找到会话 ID");
    const existingIds = await getCodexGlobalState("projectless-thread-ids").catch(() => []);
    const ids = Array.isArray(existingIds) ? existingIds : [];
    const variantSet = new Set(variants);
    const nextIds = mode === "add" ? uniqueValues([...ids, ...variants]) : ids.filter((id) => !variantSet.has(id));
    if (nextIds.length !== ids.length || nextIds.some((id, index) => id !== ids[index])) await setCodexGlobalState("projectless-thread-ids", nextIds);
  }

  async function clearThreadWorkspaceHints(ref) {
    const variants = threadIdVariants(ref.session_id);
    if (variants.length === 0) return;
    const hints = objectGlobalState(await getCodexGlobalState("thread-workspace-root-hints").catch(() => ({})));
    const hintKeys = variants.filter((id) => Object.prototype.hasOwnProperty.call(hints, id));
    if (hintKeys.length > 0) {
      hintKeys.forEach((id) => delete hints[id]);
      await setCodexGlobalState("thread-workspace-root-hints", hints);
    }
  }

  async function moveSessionToProjectless(ref) {
    if (!ref.session_id) throw new Error("未找到会话 ID");
    await setProjectlessThreadIds(ref, "add");
    await clearThreadWorkspaceHints(ref);
    const sortKey = await postJson("/thread-sort-key", ref).catch(() => ({}));
    return { status: "moved", session_id: ref.session_id, updated_at: sortKey?.updated_at, updated_at_ms: sortKey?.updated_at_ms, created_at_ms: sortKey?.created_at_ms };
  }

  function isNativeProjectTarget(target) {
    return target?.kind === "project" && nativeProjectTargets().some((project) => sameWorkspacePath(project.path, target.path));
  }

  async function moveSessionToProject(ref, target) {
    if (!ref.session_id) throw new Error("未找到会话 ID");
    if (!target?.path) throw new Error("目标项目路径为空");
    if (!isNativeProjectTarget(target)) throw new Error("目标项目不在 Codex 项目列表中");
    const result = await postJson("/move-thread-workspace", { ...ref, target_cwd: target.path });
    if (result.status !== "moved") throw new Error(result.message || "移动项目失败");
    await setProjectlessThreadIds(ref, "remove");
    await clearThreadWorkspaceHints(ref);
    return result;
  }

  function showToast(message, undoToken) {
    document.querySelectorAll(".codex-delete-toast").forEach((node) => node.remove());
    const toast = document.createElement("div");
    toast.className = "codex-delete-toast";
    toast.textContent = message;
    if (undoToken) {
      const undo = document.createElement("button");
      undo.textContent = "撤销";
      undo.addEventListener("click", async () => {
        const result = await postJson("/undo", { undo_token: undoToken });
        toast.textContent = result.message || "撤销完成";
        setTimeout(() => toast.remove(), 5000);
      });
      toast.appendChild(undo);
    }
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 10000);
  }

  function upstreamWorktreeField(dialog, name) {
    return dialog.querySelector(`[data-codex-upstream-worktree-field="${name}"]`);
  }

  function upstreamWorktreePayload(dialog) {
    return {
      repoPath: upstreamWorktreeField(dialog, "repoPath")?.value || "",
      branchName: upstreamWorktreeField(dialog, "branchName")?.value || "",
      worktreePath: upstreamWorktreeField(dialog, "worktreePath")?.value || "",
      remote: upstreamWorktreeField(dialog, "remote")?.value || "upstream",
      baseBranch: upstreamWorktreeField(dialog, "baseBranch")?.value || "main",
      fetch: true,
    };
  }

  function readUpstreamBranchSelection() {
    try {
      return JSON.parse(sessionStorage.getItem(upstreamBranchSelectionKey) || "null");
    } catch {
      return null;
    }
  }

  function writeUpstreamBranchSelection(selection) {
    if (!selection) {
      sessionStorage.removeItem(upstreamBranchSelectionKey);
      return;
    }
    sessionStorage.setItem(upstreamBranchSelectionKey, JSON.stringify(selection));
  }

  function nativeBranchMenuCandidates() {
    return [...document.querySelectorAll('[role="menu"], [data-radix-menu-content], [cmdk-list]')];
  }

  function looksLikeBranchMenu(menu, trigger = branchMenuTriggerFromMenu(menu)) {
    const text = (menu.innerText || menu.textContent || "").toLowerCase();
    if (!branchMenuTriggerIsBranchControl(trigger)) return false;
    if (/^start in\b/.test(text) || /\bwork locally\b.*\bnew worktree\b.*\bcloud\b/s.test(text)) return false;
    return /\bbranches?\b|\bbranche\b|create and checkout new branch|create branch/.test(text);
  }

  function visibleElement(node) {
    if (!(node instanceof Element)) return false;
    const rect = node.getBoundingClientRect?.();
    return !!rect && rect.width > 0 && rect.height > 0;
  }

  function effectiveElementRect(node) {
    if (!(node instanceof Element)) return null;
    const rect = node.getBoundingClientRect?.();
    if (rect && rect.width > 0 && rect.height > 0) return rect;
    const controls = [...node.closest?.(".composer-footer")?.querySelectorAll?.("button, [role='button']") || []]
      .filter((candidate) => candidate !== node && visibleElement(candidate));
    const matching = controls.find((candidate) => normalizedElementText(candidate) === normalizedElementText(node));
    return matching?.getBoundingClientRect?.() || rect || null;
  }

  function sidebarProjectRows() {
    const section = projectsSection?.();
    return [...document.querySelectorAll('[data-app-action-sidebar-project-row][data-app-action-sidebar-project-id]')]
      .filter((row) => !section || section.contains(row));
  }

  function projectRowPath(row) {
    return row?.getAttribute?.("data-app-action-sidebar-project-id") || "";
  }

  function projectContextFromRow(row) {
    const path = projectRowPath(row);
    if (!path) return null;
    const label = row.getAttribute("data-app-action-sidebar-project-label")
      || row.getAttribute("aria-label")
      || displayProjectName(path);
    return {
      repoPath: path.startsWith("/") ? path : "",
      projectId: path.startsWith("/") ? "" : path,
      label: normalizeProjectLabel(label),
      at: Date.now(),
    };
  }

  function remoteProjectContextFromGlobalState(projectId) {
    const normalizedProjectId = String(projectId || "").trim();
    if (!normalizedProjectId) return null;
    return { projectId: normalizedProjectId, repoPath: "", label: "", at: Date.now() };
  }

  function readUpstreamProjectContext() {
    try {
      const context = JSON.parse(sessionStorage.getItem(upstreamProjectContextKey) || "null");
      if (!context || typeof context !== "object") return null;
      if (typeof context.at === "number" && Date.now() - context.at > upstreamProjectContextTtlMs) return null;
      if (!context.repoPath && !context.projectId) return null;
      return context;
    } catch {
      return null;
    }
  }

  function writeUpstreamProjectContext(context) {
    if (!context?.repoPath && !context?.projectId) return;
    try {
      sessionStorage.setItem(upstreamProjectContextKey, JSON.stringify({
        repoPath: context.repoPath || "",
        projectId: context.projectId || "",
        label: context.label || "",
        at: Date.now(),
      }));
    } catch {
    }
  }

  function readCodexMemoryProjectContext() {
    try {
      const context = JSON.parse(sessionStorage.getItem(codexMemoryProjectContextKey) || "null");
      if (!context || typeof context !== "object") return null;
      if (typeof context.at === "number" && Date.now() - context.at > upstreamProjectContextTtlMs) return null;
      if (!context.repoPath && !context.projectId) return null;
      return context;
    } catch {
      return null;
    }
  }

  function rememberCodexMemoryProjectContext(context) {
    if (!context?.repoPath && !context?.projectId) return context || null;
    const next = {
      repoPath: context.repoPath || "",
      projectId: context.projectId || "",
      label: context.label || "",
      at: Date.now(),
    };
    try {
      sessionStorage.setItem(codexMemoryProjectContextKey, JSON.stringify(next));
      writeUpstreamProjectContext(next);
    } catch {
    }
    return next;
  }

  function projectContextFromStartButton(button) {
    const row = button?.closest?.('[data-app-action-sidebar-project-row][data-app-action-sidebar-project-id]');
    return projectContextFromRow(row);
  }

  function rememberStartNewChatProjectContext(event) {
    const target = event.target instanceof Element ? event.target : event.target?.parentElement;
    const button = target?.closest?.('button[aria-label^="Start new chat in "]');
    const context = projectContextFromStartButton(button);
    if (context) writeUpstreamProjectContext(context);
  }

  function visibleProjectRows() {
    return sidebarProjectRows().filter((row) => visibleElement(row));
  }

  function currentProjectRepoPathFromStartButton() {
    const startButtons = [...document.querySelectorAll('button[aria-label^="Start new chat in "]')]
      .filter((button) => visibleElement(button));
    const bottomHalf = window.innerHeight * 0.5;
    startButtons.sort((left, right) => {
      const leftRect = left.getBoundingClientRect();
      const rightRect = right.getBoundingClientRect();
      const leftScore = Math.abs(leftRect.y - bottomHalf) + Math.max(0, bottomHalf - leftRect.y) * 0.5;
      const rightScore = Math.abs(rightRect.y - bottomHalf) + Math.max(0, bottomHalf - rightRect.y) * 0.5;
      return leftScore - rightScore;
    });
    for (const button of startButtons) {
      const row = button.closest('[data-app-action-sidebar-project-row][data-app-action-sidebar-project-id]');
      const path = projectRowPath(row);
      if (path?.startsWith?.("/")) return path;
    }
    return "";
  }

  function currentProjectContextFromStartButton() {
    const startButtons = [...document.querySelectorAll('button[aria-label^="Start new chat in "]')]
      .filter((button) => visibleElement(button));
    const bottomHalf = window.innerHeight * 0.5;
    startButtons.sort((left, right) => {
      const leftRect = left.getBoundingClientRect();
      const rightRect = right.getBoundingClientRect();
      const leftScore = Math.abs(leftRect.y - bottomHalf) + Math.max(0, bottomHalf - leftRect.y) * 0.5;
      const rightScore = Math.abs(rightRect.y - bottomHalf) + Math.max(0, bottomHalf - rightRect.y) * 0.5;
      return leftScore - rightScore;
    });
    for (const button of startButtons) {
      const context = projectContextFromStartButton(button);
      if (context) return context;
    }
    return null;
  }

  function currentProjectRepoPathFromSelectedProjectButton() {
    const projectButtons = [...document.querySelectorAll('button[aria-haspopup="menu"]')]
      .filter((button) => visibleElement(button))
      .filter((button) => button.getBoundingClientRect().x > 300)
      .map((button) => (button.innerText || button.textContent || "").trim())
      .filter(Boolean);
    for (const label of projectButtons) {
      const match = visibleProjectRows().find((row) => {
        const rowLabel = row.getAttribute("data-app-action-sidebar-project-label") || row.getAttribute("aria-label") || "";
        return rowLabel.trim() === label;
      });
      const path = projectRowPath(match);
      if (path?.startsWith?.("/")) return path;
    }
    return "";
  }

  function projectContextFromProjectLabel(label) {
    const normalizedLabel = normalizeProjectLabel(label);
    if (!normalizedLabel) return null;
    const row = visibleProjectRows().find((candidate) => {
      const rowPath = projectRowPath(candidate);
      const rowLabels = [
        candidate.getAttribute("data-app-action-sidebar-project-label"),
        candidate.getAttribute("aria-label"),
        displayProjectName(rowPath),
      ].map(normalizeProjectLabel).filter(Boolean);
      return rowLabels.includes(normalizedLabel);
    });
    const context = projectContextFromRow(row);
    if (!context) return null;
    return context.projectId ? { ...remoteProjectContextFromGlobalState(context.projectId), label: context.label } : context;
  }

  function repoPathFromProjectLabel(label) {
    return projectContextFromProjectLabel(label)?.repoPath || "";
  }

  function contextMatchesProjectLabel(context, label) {
    const expected = normalizeProjectLabel(label);
    if (!expected) return true;
    const actual = normalizeProjectLabel(context?.label);
    return !actual || actual === expected;
  }

  function currentProjectContextFromStoredSelection(label = "") {
    const context = readUpstreamProjectContext();
    return contextMatchesProjectLabel(context, label) ? context : null;
  }

  function currentProjectContextForBranchMenu(menu, trigger = branchMenuTriggerFromMenu(menu)) {
    const footer = trigger?.closest?.(".composer-footer");
    const projectButton = footer ? [...footer.querySelectorAll('button, [role="button"]')]
      .filter((node) => node !== trigger && visibleElement(node))
      .filter((node) => {
        const rect = effectiveElementRect(node);
        const triggerRect = effectiveElementRect(trigger);
        return rect && triggerRect && rect.x < triggerRect.x;
      })
      .sort((left, right) => effectiveElementRect(left).x - effectiveElementRect(right).x)
      .find((node) => projectContextFromProjectLabel(normalizedElementText(node))) : null;
    const projectLabel = normalizedElementText(projectButton);
    return currentProjectContextFromStoredSelection(projectLabel)
      || projectContextFromProjectLabel(projectLabel)
      || currentProjectContextFromStoredSelection()
      || currentProjectContext();
  }

  function currentProjectRepoPathForBranchMenu(menu, trigger = branchMenuTriggerFromMenu(menu)) {
    return currentProjectContextForBranchMenu(menu, trigger)?.repoPath || "";
  }

  function currentProjectRepoPathFromExpandedRows() {
    const expandedRows = visibleProjectRows().filter((row) => row.getAttribute("data-app-action-sidebar-project-collapsed") === "false");
    const pathRows = expandedRows.filter((row) => projectRowPath(row).startsWith("/"));
    if (pathRows.length === 1) return projectRowPath(pathRows[0]);
    return "";
  }

  function currentProjectRepoPath() {
    return currentProjectRepoPathFromSelectedProjectButton()
      || currentProjectRepoPathFromStartButton()
      || currentProjectRepoPathFromExpandedRows();
  }

  function currentProjectContext() {
    const stored = currentProjectContextFromStoredSelection();
    if (stored) return stored;
    const selectedPath = currentProjectRepoPathFromSelectedProjectButton();
    if (selectedPath) return { repoPath: selectedPath, projectId: "", label: displayProjectName(selectedPath), at: Date.now() };
    const startContext = currentProjectContextFromStartButton();
    if (startContext) return startContext;
    const expandedPath = currentProjectRepoPathFromExpandedRows();
    if (expandedPath) return { repoPath: expandedPath, projectId: "", label: displayProjectName(expandedPath), at: Date.now() };
    return null;
  }

  function newWorktreeModeActive() {
    return [...document.querySelectorAll('button, [role="button"]')]
      .filter((node) => visibleElement(node))
      .some((node) => {
        return normalizedElementText(node) === "New worktree";
      });
  }

  function normalizedElementText(node) {
    return (node?.innerText || node?.textContent || "").replace(/\s+/g, " ").trim();
  }

  async function loadUpstreamBranchDefaults(context) {
    const repoPath = typeof context === "string" ? context : context?.repoPath || "";
    const projectId = typeof context === "string" ? "" : context?.projectId || "";
    if (!repoPath && !projectId) return null;
    const cacheKey = projectId ? `project:${projectId}` : `repo:${repoPath}`;
    const cacheTtlMs = projectId ? upstreamRemoteBranchDefaultsCacheTtlMs : upstreamBranchDefaultsCacheTtlMs;
    const cached = upstreamBranchDefaultsCache.get(cacheKey);
    if (cached && Date.now() - cached.loadedAt < cacheTtlMs) return cached;
    const inflight = upstreamBranchDefaultsInflight.get(cacheKey);
    if (inflight) return inflight;
    const request = postJson("/upstream-worktree/defaults", { repoPath, projectId })
      .then((result) => {
        const entry = { repoPath, projectId, result, loadedAt: Date.now() };
        if (result?.status === "ok") upstreamBranchDefaultsCache.set(cacheKey, entry);
        return entry;
      })
      .finally(() => upstreamBranchDefaultsInflight.delete(cacheKey));
    upstreamBranchDefaultsInflight.set(cacheKey, request);
    return request;
  }

  function renderUpstreamBranchOption(menu, context, ref) {
    const repoPath = context?.repoPath || "";
    const label = ref.label || `${ref.remote || "upstream"}/${ref.branch || "main"}`;
    const item = document.createElement("div");
    item.setAttribute("role", "menuitem");
    item.setAttribute("aria-checked", "false");
    item.setAttribute(upstreamBranchOptionAttribute, "true");
    item.setAttribute("data-repo-path", repoPath);
    item.setAttribute("data-project-id", context?.projectId || "");
    item.setAttribute("data-remote", ref.remote || "upstream");
    item.setAttribute("data-base-branch", ref.branch || "main");
    item.setAttribute("data-label", label);
    item.className = "codex-upstream-branch-option cursor-interaction flex items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-token-foreground hover:bg-token-list-hover-background";
    item.innerHTML = `${branchIconSvg()}<span class="min-w-0 flex-1 truncate">${escapeHtml(label)}</span>${checkmarkSvg()}`;
    menu.appendChild(item);
  }

  function branchIconSvg() {
    return '<svg aria-hidden="true" data-codex-upstream-branch-icon="true" viewBox="0 0 24 24" class="h-4 w-4 shrink-0 text-token-text-tertiary" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" x2="6" y1="3" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>';
  }

  function checkmarkSvg() {
    return '<svg hidden aria-hidden="true" data-codex-upstream-branch-check="true" viewBox="0 0 24 24" class="h-4 w-4 shrink-0 text-token-text-secondary" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"></path></svg>';
  }

  function branchMenuItems(menu) {
    return [...menu.querySelectorAll('[role="menuitem"], [data-radix-collection-item]')]
      .filter((item) => !item.closest?.(`[${upstreamBranchOptionAttribute}]`));
  }

  function branchMenuItemLabel(menuItem) {
    return normalizedElementText(menuItem);
  }

  function upstreamBranchOptionLabel(option) {
    return option?.getAttribute?.("data-label") || normalizedElementText(option);
  }

  function worktreeBranchMap(defaultsResult) {
    const repoRoot = defaultsResult?.repoRoot || "";
    const entries = Array.isArray(defaultsResult?.worktreeBranches) ? defaultsResult.worktreeBranches : [];
    return new Map(entries
      .filter((entry) => entry?.branch && entry?.path && entry.path !== repoRoot)
      .map((entry) => [entry.branch, entry.path]));
  }

  function annotateBranchMenuWorktreeUsage(menu, defaultsResult) {
    const usedBranches = worktreeBranchMap(defaultsResult);
    for (const item of branchMenuItems(menu)) {
      item.removeAttribute(branchWorktreePathAttribute);
      item.removeAttribute("title");
      const worktreePath = usedBranches.get(branchMenuItemLabel(item));
      if (!worktreePath) continue;
      item.setAttribute(branchWorktreePathAttribute, worktreePath);
      item.setAttribute("title", `该分支已在另一个 worktree 使用：${worktreePath}`);
    }
  }

  function branchWorktreePathFromMenuItem(menuItem) {
    const annotatedPath = menuItem?.getAttribute?.(branchWorktreePathAttribute) || "";
    if (annotatedPath) return annotatedPath;
    const menu = menuItem?.closest?.('[role="menu"], [data-radix-menu-content]');
    const context = currentProjectContextForBranchMenu(menu);
    const cacheKey = context?.projectId ? `project:${context.projectId}` : `repo:${context?.repoPath || ""}`;
    const usedBranches = worktreeBranchMap(upstreamBranchDefaultsCache.get(cacheKey)?.result);
    return usedBranches.get(branchMenuItemLabel(menuItem)) || "";
  }

  function upstreamBranchOptionsMatchRefs(menu, context, refs) {
    const repoPath = context?.repoPath || "";
    const projectId = context?.projectId || "";
    const options = [...menu.querySelectorAll(`[${upstreamBranchOptionAttribute}]`)];
    if (options.length !== refs.length) return false;
    return options.every((option, index) => {
      const ref = refs[index];
      return option.getAttribute("data-repo-path") === repoPath
        && option.getAttribute("data-project-id") === projectId
        && option.getAttribute("data-remote") === (ref.remote || "upstream")
        && option.getAttribute("data-base-branch") === (ref.branch || "main")
        && upstreamBranchOptionLabel(option) === (ref.label || `${ref.remote || "upstream"}/${ref.branch || "main"}`);
    });
  }

  function syncUpstreamBranchMenuSelection(menu) {
    if (!menu) return;
    const selection = readUpstreamBranchSelection();
    for (const option of menu.querySelectorAll(`[${upstreamBranchOptionAttribute}]`)) {
      const selected = !!selection
        && option.getAttribute("data-repo-path") === (selection.repoPath || "")
        && option.getAttribute("data-project-id") === (selection.projectId || "")
        && option.getAttribute("data-remote") === (selection.remote || "upstream")
        && option.getAttribute("data-base-branch") === (selection.baseBranch || "main");
      option.setAttribute("aria-checked", selected ? "true" : "false");
      option.toggleAttribute("data-selected", selected);
      const check = option.querySelector('[data-codex-upstream-branch-check="true"]');
      if (check && selected) check.removeAttribute("hidden");
      if (check && !selected) check.setAttribute("hidden", "");
    }
  }

  function removeUpstreamBranchOptions(scope = document) {
    scope.querySelectorAll(`[${upstreamBranchOptionAttribute}], .codex-upstream-branch-group`)
      .forEach((node) => node.remove());
  }

  function cleanupInvalidUpstreamBranchOptions() {
    for (const menu of nativeBranchMenuCandidates()) {
      if (!menu.querySelector(`[${upstreamBranchOptionAttribute}], .codex-upstream-branch-group`)) continue;
      const trigger = branchMenuTriggerFromMenu(menu);
      if (!looksLikeBranchMenu(menu, trigger) || !branchMenuInNewWorktreeMode(trigger)) {
        removeUpstreamBranchOptions(menu);
      }
    }
  }

  function branchMenuTriggerFromMenu(menu) {
    const labelledBy = menu?.getAttribute?.("aria-labelledby") || "";
    if (labelledBy) {
      const trigger = document.getElementById(labelledBy);
      if (trigger instanceof Element) return trigger;
    }
    return [...document.querySelectorAll('button')]
      .filter((button) => (button.innerText || button.textContent || "").trim() === "main")
      .sort((left, right) => right.getBoundingClientRect().x - left.getBoundingClientRect().x)[0] || null;
  }

  function branchMenuTriggerIsBranchControl(trigger) {
    const text = normalizedElementText(trigger);
    if (!text || /^(work locally|new worktree|cloud|no environment)$/i.test(text)) return false;
    const rect = effectiveElementRect(trigger);
    const footer = trigger?.closest?.(".composer-footer");
    if (!rect || !footer) return /branch|main|create branch/i.test(text);
    const modeTrigger = [...footer.querySelectorAll('button, [role="button"]')]
      .filter((node) => node !== trigger && visibleElement(node))
      .filter((node) => node.getBoundingClientRect().x < rect.x)
      .sort((left, right) => right.getBoundingClientRect().x - left.getBoundingClientRect().x)
      .find((node) => /^(work locally|new worktree|cloud)$/i.test(normalizedElementText(node)));
    return !!modeTrigger;
  }

  function branchMenuInNewWorktreeMode(trigger) {
    if (!trigger) return newWorktreeModeActive();
    const footer = trigger.closest?.(".composer-footer");
    const scope = footer || trigger.parentElement || document;
    const triggerRect = effectiveElementRect(trigger);
    if (!triggerRect) return false;
    const modeTrigger = [...scope.querySelectorAll('button, [role="button"]')]
      .filter((node) => node !== trigger && visibleElement(node))
      .filter((node) => node.getBoundingClientRect().x < triggerRect.x)
      .sort((left, right) => right.getBoundingClientRect().x - left.getBoundingClientRect().x)
      .find((node) => /worktree|work locally/i.test(normalizedElementText(node)));
    return normalizedElementText(modeTrigger) === "New worktree";
  }

  function branchTriggerLabelNode(trigger) {
    if (!trigger) return null;
    const nodes = [...trigger.querySelectorAll("span, div")]
      .filter((node) => (node.innerText || node.textContent || "").trim());
    return nodes.find((node) => node.classList?.contains("composer-footer__label--sm")) || nodes[0] || trigger;
  }

  function ensureNativeBranchTriggerLabel(trigger) {
    if (!trigger || trigger.querySelector?.('[data-codex-upstream-branch-selection-label="true"]')) return;
    const labelNode = branchTriggerLabelNode(trigger);
    if (!labelNode) return;
    trigger.setAttribute("data-codex-upstream-branch-trigger", "true");
    labelNode.setAttribute("data-codex-native-branch-label", "true");
    const selectionLabel = document.createElement("span");
    selectionLabel.setAttribute("data-codex-upstream-branch-selection-label", "true");
    selectionLabel.className = labelNode.className || "composer-footer__label--sm composer-footer__secondary-label max-w-40 truncate";
    selectionLabel.hidden = true;
    labelNode.insertAdjacentElement("afterend", selectionLabel);
  }

  function clearUpstreamBranchTriggerLabel() {
    document.querySelectorAll('[data-codex-upstream-branch-trigger="true"]').forEach((trigger) => {
      const nativeLabel = trigger.querySelector('[data-codex-native-branch-label="true"]');
      const selectionLabel = trigger.querySelector('[data-codex-upstream-branch-selection-label="true"]');
      if (nativeLabel) nativeLabel.hidden = false;
      if (selectionLabel) selectionLabel.hidden = true;
      trigger.removeAttribute("aria-label");
      trigger.removeAttribute("title");
    });
  }

  function syncUpstreamBranchTriggerLabel() {
    const selection = readUpstreamBranchSelection();
    if (!selection?.label) {
      clearUpstreamBranchTriggerLabel();
      return;
    }
    document.querySelectorAll('[data-codex-upstream-branch-trigger="true"]').forEach((trigger) => {
      const nativeLabel = trigger.querySelector('[data-codex-native-branch-label="true"]');
      const selectionLabel = trigger.querySelector('[data-codex-upstream-branch-selection-label="true"]');
      if (!selectionLabel) return;
      if (nativeLabel) nativeLabel.hidden = true;
      selectionLabel.hidden = false;
      selectionLabel.textContent = selection.label;
      trigger.setAttribute("aria-label", selection.label);
      trigger.setAttribute("title", selection.label);
    });
  }

  function handleNativeBranchSelection(event) {
    const target = event.target instanceof Element ? event.target : event.target?.parentElement;
    const menuItem = target?.closest?.('[role="menuitem"], [data-radix-collection-item]');
    if (!menuItem || menuItem.closest?.(`[${upstreamBranchOptionAttribute}]`)) return;
    const menu = menuItem.closest?.('[role="menu"], [data-radix-menu-content]');
    if (!menu || !looksLikeBranchMenu(menu)) return;
    const text = (menuItem.innerText || menuItem.textContent || "").replace(/\s+/g, " ").trim();
    if (!text || /^branches$/i.test(text) || /^upstream$/i.test(text) || text === readUpstreamBranchSelection()?.label) return;
    const usedWorktreePath = branchWorktreePathFromMenuItem(menuItem);
    writeUpstreamBranchSelection(null);
    clearUpstreamBranchTriggerLabel();
    syncUpstreamBranchMenuSelection(menu);
    if (usedWorktreePath) {
      event.preventDefault();
      event.stopPropagation();
      event.stopImmediatePropagation?.();
      showToast(`该分支已在另一个 worktree 使用：${usedWorktreePath}`, null);
    }
  }

  async function injectUpstreamBranchOptions() {
    if (!claudeCodexProSettings().upstreamWorktreeCreate) {
      removeUpstreamBranchOptions();
      return;
    }
    cleanupInvalidUpstreamBranchOptions();
    for (const menu of nativeBranchMenuCandidates()) {
      const trigger = branchMenuTriggerFromMenu(menu);
      if (!looksLikeBranchMenu(menu, trigger)) continue;
      const context = currentProjectContextForBranchMenu(menu, trigger);
      if (!context?.repoPath && !context?.projectId) {
        removeUpstreamBranchOptions(menu);
        continue;
      }
      const defaults = await loadUpstreamBranchDefaults(context);
      const defaultsResult = defaults?.result;
      const refs = defaults?.result?.upstreamRefs || [];
      annotateBranchMenuWorktreeUsage(menu, defaultsResult);
      if (!branchMenuInNewWorktreeMode(trigger)) {
        removeUpstreamBranchOptions(menu);
        writeUpstreamBranchSelection(null);
        clearUpstreamBranchTriggerLabel();
        continue;
      }
      if (!refs.length) {
        removeUpstreamBranchOptions(menu);
        continue;
      }
      const resolvedContext = {
        repoPath: defaults?.repoPath || context.repoPath || defaultsResult?.repoRoot || "",
        projectId: defaults?.projectId || context.projectId || "",
      };
      if (upstreamBranchOptionsMatchRefs(menu, resolvedContext, refs)) {
        syncUpstreamBranchTriggerLabel();
        syncUpstreamBranchMenuSelection(menu);
        continue;
      }
      removeUpstreamBranchOptions(menu);
      ensureNativeBranchTriggerLabel(trigger);
      const group = document.createElement("div");
      group.className = "codex-upstream-branch-group px-2 py-1 text-xs text-token-text-tertiary";
      group.textContent = "Upstream";
      menu.appendChild(group);
      refs.forEach((ref) => renderUpstreamBranchOption(menu, resolvedContext, ref));
      syncUpstreamBranchTriggerLabel();
      syncUpstreamBranchMenuSelection(menu);
    }
  }

  function installUpstreamBranchDropdownAdapter() {
    const adapterVersion = "actual-upstream-refs-v16";
    window.__codexUpstreamBranchDropdownAdapterVersion = adapterVersion;
    if (window.__codexUpstreamBranchDropdownAdapterInstalled === adapterVersion) return;
    window.__codexUpstreamBranchDropdownAdapterInstalled = adapterVersion;
    document.addEventListener("click", (event) => {
      rememberStartNewChatProjectContext(event);
      const target = event.target instanceof Element ? event.target : event.target?.parentElement;
      const option = target?.closest?.(`[${upstreamBranchOptionAttribute}]`);
      if (!option) {
        handleNativeBranchSelection(event);
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      const selection = {
        repoPath: option.getAttribute("data-repo-path") || "",
        projectId: option.getAttribute("data-project-id") || "",
        remote: option.getAttribute("data-remote") || "upstream",
        baseBranch: option.getAttribute("data-base-branch") || "main",
        label: upstreamBranchOptionLabel(option) || "upstream/main",
      };
      writeUpstreamBranchSelection(selection);
      prepareUpstreamBranchSelection(selection);
      syncUpstreamBranchTriggerLabel();
      syncUpstreamBranchMenuSelection(option.closest?.('[role="menu"], [data-radix-menu-content], [cmdk-list]'));
      showToast(`将从 ${upstreamBranchOptionLabel(option) || "upstream/main"} 创建新 worktree`, null);
    }, true);
    let upstreamBranchInjectTimer = null;
    const schedule = () => {
      clearTimeout(upstreamBranchInjectTimer);
      upstreamBranchInjectTimer = setTimeout(() => {
        injectUpstreamBranchOptions().catch((error) => reportDiagnostic("upstream_branch_inject_failed", { error: error?.message || String(error) }));
      }, 80);
    };
    new MutationObserver(schedule).observe(document.body || document.documentElement, { childList: true, subtree: true });
    schedule();
  }

  function upstreamQualifiedSourceRef(selection) {
    if (selection?.qualifiedSourceRef) return selection.qualifiedSourceRef;
    const remote = (selection?.remote || "upstream").trim();
    const baseBranch = (selection?.baseBranch || "main").trim();
    return remote && baseBranch ? `refs/remotes/${remote}/${baseBranch}` : "";
  }

  function prepareUpstreamBranchSelection(selection) {
    if ((!selection?.repoPath && !selection?.projectId) || !selection.remote || !selection.baseBranch) return;
    void postJson("/upstream-worktree/prepare", {
      repoPath: selection.repoPath || "",
      projectId: selection.projectId || "",
      remote: selection.remote,
      baseBranch: selection.baseBranch,
      fetch: true,
    }).then((result) => {
      if (result?.status !== "ok") throw new Error(result?.message || "prepare failed");
      writePreparedUpstreamBranchSelection(selection, result);
    }).catch((error) => {
      sendClaudeCodexProDiagnostic("upstream_branch_prepare_failed", {
        label: selection.label || "",
        errorName: error?.name || "",
        errorMessage: error?.message || String(error),
      });
    });
  }

  function writePreparedUpstreamBranchSelection(selection, result) {
    const current = readUpstreamBranchSelection();
    if (!upstreamSelectionMatches(current, selection)) return;
    writeUpstreamBranchSelection({
      ...current,
      qualifiedSourceRef: result.qualifiedSourceRef || upstreamQualifiedSourceRef(selection),
      sourceHead: result.sourceHead || "",
      preparedAt: Date.now(),
    });
  }

  function upstreamSelectionMatches(left, right) {
    return !!left && !!right
      && (left.repoPath || "") === (right.repoPath || "")
      && (left.projectId || "") === (right.projectId || "")
      && (left.remote || "upstream") === (right.remote || "upstream")
      && (left.baseBranch || "main") === (right.baseBranch || "main");
  }

  function pendingWorktreeRequestMatchesSelection(request, selection) {
    if (!selection || !request || request.launchMode !== "start-conversation") return false;
    const sourceRoot = request.sourceWorkspaceRoot || "";
    if (selection.repoPath && sourceRoot) return sameWorkspacePath(sourceRoot, selection.repoPath);
    if (selection.projectId) return true;
    return !selection.repoPath || sameWorkspacePath(sourceRoot, selection.repoPath);
  }

  function applyUpstreamPendingWorktreeOverride(payload) {
    const selection = readUpstreamBranchSelection();
    const request = payload?.request;
    const sourceRef = upstreamQualifiedSourceRef(selection);
    if (!claudeCodexProSettings().upstreamWorktreeCreate || !sourceRef) return payload;
    if (!pendingWorktreeRequestMatchesSelection(request, selection)) return payload;
    if (request?.startingState?.type !== "branch") return payload;
    if (request.startingState.branchName === sourceRef) return payload;
    const nextRequest = {
      ...request,
      startingState: { ...request.startingState, branchName: sourceRef },
    };
    prepareUpstreamBranchSelection(selection);
    sendClaudeCodexProDiagnostic("upstream_pending_worktree_override_applied", {
      label: selection.label || "",
      sourceRef,
      sourceWorkspaceRoot: request.sourceWorkspaceRoot || "",
    });
    return { ...(payload || {}), request: nextRequest };
  }

  function installUpstreamPendingWorktreeDispatcherPatch() {
    const patchVersion = "1";
    if (window.__codexUpstreamPendingWorktreeDispatcherPatch === patchVersion) return;
    const patch = async () => {
      try {
        const module = await loadCodexAppModule("setting-storage-");
        const dispatcherClass = typeof module.v === "function" && String(module.v).includes("dispatchMessage") ? module.v : null;
        const dispatcher = dispatcherClass?.getInstance?.();
        if (!dispatcher || typeof dispatcher.dispatchMessage !== "function") throw new Error("Codex dispatcher unavailable");
        if (!dispatcher.__codexUpstreamWorktreeOriginalDispatchMessage) {
          dispatcher.__codexUpstreamWorktreeOriginalDispatchMessage = dispatcher.dispatchMessage.bind(dispatcher);
          dispatcher.dispatchMessage = (type, payload) => {
            const nextPayload = type === "pending-worktree-create"
              ? applyUpstreamPendingWorktreeOverride(payload)
              : payload;
            return dispatcher.__codexUpstreamWorktreeOriginalDispatchMessage(type, nextPayload);
          };
        }
        window.__codexUpstreamPendingWorktreeDispatcherPatch = patchVersion;
      } catch (error) {
        sendClaudeCodexProDiagnostic("upstream_pending_worktree_patch_failed", {
          errorName: error?.name || "",
          errorMessage: error?.message || String(error),
        });
      }
    };
    void patch();
  }

  function upstreamWorktreeNativePayloadFromElement(element) {
    const trigger = element?.closest?.("[data-codex-worktree-create], [data-worktree-create]") || element;
    const scopes = [
      trigger,
      trigger?.closest?.("form"),
      trigger?.closest?.("dialog, [role='dialog']"),
    ].filter((scope, index, all) => scope?.querySelector && all.indexOf(scope) === index);
    if (!scopes.length) return null;
    const valueFrom = (selectors) => {
      for (const scope of scopes) {
        for (const selector of selectors) {
          const node = scope.matches?.(selector) ? scope : scope.querySelector(selector);
          const dataAttribute = selector.match(/^\[([a-z0-9-]+)\]$/i)?.[1] || "";
          const value = node?.value || node?.getAttribute?.(dataAttribute) || node?.getAttribute?.("data-value") || node?.textContent || "";
          if (String(value).trim()) return String(value).trim();
        }
      }
      return "";
    };
    const repoPath = valueFrom(["[data-repo-path]", "[name='repoPath']", "[name='repo']"]);
    const branchName = valueFrom(["[data-branch-name]", "[name='branchName']", "[name='branch']"]);
    const worktreePath = valueFrom(["[data-worktree-path]", "[name='worktreePath']", "[name='path']"]);
    const remote = valueFrom(["[data-remote]", "[name='remote']"]) || "upstream";
    const baseBranch = valueFrom(["[data-base-branch]", "[name='baseBranch']", "[name='base']"]) || "main";
    if (!repoPath || !branchName || !worktreePath || !remote || !baseBranch) return null;
    return { repoPath, branchName, worktreePath, remote, baseBranch, fetch: true };
  }

  function upstreamWorktreePayloadFromSelection(trigger) {
    const selection = readUpstreamBranchSelection();
    if ((!selection?.repoPath && !selection?.projectId) || !selection?.remote || !selection?.baseBranch) return null;
    const nativePayload = upstreamWorktreeNativePayloadFromElement(trigger);
    if (!nativePayload?.branchName || !nativePayload?.worktreePath) return null;
    return {
      ...nativePayload,
      repoPath: selection.repoPath,
      projectId: selection.projectId || "",
      remote: selection.remote,
      baseBranch: selection.baseBranch,
      fetch: true,
    };
  }

  async function handleUpstreamWorktreeNativeCreate(event) {
    if (!claudeCodexProSettings().upstreamWorktreeCreate) return false;
    const target = event.target instanceof Element ? event.target : event.target?.parentElement;
    const trigger = target?.closest?.("[data-codex-worktree-create], [data-worktree-create]");
    if (!trigger) return false;
    const payload = upstreamWorktreePayloadFromSelection(trigger) || upstreamWorktreeNativePayloadFromElement(trigger);
    if (!payload) {
      showToast("无法安全识别 Codex 原生 worktree 表单，请使用 Claude Codex Pro 菜单创建。", null);
      return false;
    }
    event.preventDefault();
    event.stopPropagation();
    try {
      const result = await postJson("/upstream-worktree/create", payload);
      if (result?.status === "ok") {
        writeUpstreamBranchSelection(null);
        syncUpstreamBranchTriggerLabel();
        showToast(`已从 ${result.sourceRef} 创建 worktree`, null);
      } else {
        showToast(result?.message || "创建 upstream worktree 失败", null);
      }
    } catch (error) {
      showToast(error?.message || "创建 upstream worktree 失败", null);
    }
    return true;
  }

  function installUpstreamWorktreeNativeAdapter() {
    const adapterVersion = "2";
    installUpstreamPendingWorktreeDispatcherPatch();
    if (window.__codexUpstreamWorktreeNativeAdapterInstalled === adapterVersion) return;
    window.__codexUpstreamWorktreeNativeAdapterInstalled = adapterVersion;
    document.addEventListener("click", (event) => {
      handleUpstreamWorktreeNativeCreate(event);
    }, true);
  }

  function setUpstreamWorktreeMessage(dialog, message, status = "idle") {
    const messageNode = dialog.querySelector("[data-codex-upstream-worktree-message]");
    if (!messageNode) return;
    messageNode.dataset.status = status;
    messageNode.textContent = message || "";
  }

  async function loadUpstreamWorktreeDefaults(dialog) {
    const repoPath = upstreamWorktreeField(dialog, "repoPath")?.value?.trim() || "";
    if (!repoPath) {
      setUpstreamWorktreeMessage(dialog, "填写仓库路径后会自动读取 remote 和当前分支。", "idle");
      return;
    }
    setUpstreamWorktreeMessage(dialog, "正在读取仓库默认值…", "loading");
    try {
      const result = await postJson("/upstream-worktree/defaults", { repoPath });
      if (result?.status !== "ok") {
        setUpstreamWorktreeMessage(dialog, result?.message || "读取仓库默认值失败", "failed");
        return;
      }
      const remote = upstreamWorktreeField(dialog, "remote");
      const baseBranch = upstreamWorktreeField(dialog, "baseBranch");
      if (remote && !remote.value) remote.value = result.defaultRemote || "upstream";
      if (baseBranch && (!baseBranch.value || baseBranch.value === "main")) baseBranch.value = result.defaultBaseBranch || "main";
      setUpstreamWorktreeMessage(dialog, `将从 ${remote?.value || "upstream"}/${baseBranch?.value || "main"} 创建 worktree。`, "ok");
    } catch (error) {
      setUpstreamWorktreeMessage(dialog, error?.message || "读取仓库默认值失败", "failed");
    }
  }

  async function submitUpstreamWorktree(dialog) {
    const payload = upstreamWorktreePayload(dialog);
    if (!payload.repoPath || !payload.branchName || !payload.worktreePath || !payload.remote || !payload.baseBranch) {
      setUpstreamWorktreeMessage(dialog, "仓库路径、分支名、worktree 路径、remote 和 base branch 都必须填写。", "failed");
      return;
    }
    setUpstreamWorktreeMessage(dialog, "正在 fetch 并创建 worktree…", "loading");
    try {
      const result = await postJson("/upstream-worktree/create", payload);
      if (result?.status === "ok") {
        setUpstreamWorktreeMessage(dialog, `已从 ${result.sourceRef} 创建：${result.worktreePath}`, "ok");
        showToast(`已创建 upstream worktree：${result.branchName}`, null);
      } else {
        setUpstreamWorktreeMessage(dialog, result?.message || "创建 upstream worktree 失败", "failed");
      }
    } catch (error) {
      setUpstreamWorktreeMessage(dialog, error?.message || "创建 upstream worktree 失败", "failed");
    }
  }

  function openUpstreamWorktreeDialog() {
    document.querySelectorAll(`.${upstreamWorktreeDialogClass}`).forEach((node) => node.remove());
    const overlay = document.createElement("div");
    overlay.className = `codex-delete-confirm-overlay ${upstreamWorktreeDialogClass}`;
    overlay.innerHTML = `
      <div class="codex-delete-confirm-content" role="dialog" aria-modal="true" aria-label="Create upstream worktree">
        <div class="codex-delete-confirm-title">Create from upstream</div>
        <div class="codex-delete-confirm-message">等价于 git worktree add -b branch path upstream/base。创建前会先 fetch 远端分支。</div>
        <label class="claude-codex-pro-form-field">仓库路径<input data-codex-upstream-worktree-field="repoPath" type="text" placeholder="/path/to/repo"></label>
        <label class="claude-codex-pro-form-field">新分支名<input data-codex-upstream-worktree-field="branchName" type="text" placeholder="feature/my-task"></label>
        <label class="claude-codex-pro-form-field">Worktree 路径<input data-codex-upstream-worktree-field="worktreePath" type="text" placeholder="/path/to/worktrees/my-task"></label>
        <label class="claude-codex-pro-form-field">Remote<input data-codex-upstream-worktree-field="remote" type="text" value="upstream"></label>
        <label class="claude-codex-pro-form-field">Base branch<input data-codex-upstream-worktree-field="baseBranch" type="text" value="main"></label>
        <div class="claude-codex-pro-form-message" data-codex-upstream-worktree-message>填写仓库路径后会自动读取 remote 和当前分支。</div>
        <div class="codex-delete-confirm-actions">
          <button type="button" data-codex-upstream-worktree-cancel="true">取消</button>
          <button type="button" data-codex-upstream-worktree-defaults="true">读取默认值</button>
          <button type="button" data-codex-upstream-worktree-submit="true">Create from upstream</button>
        </div>
      </div>
    `;
    overlay.addEventListener("click", (event) => {
      const target = event.target instanceof Element ? event.target : event.target?.parentElement;
      if (event.target === overlay || target?.closest("[data-codex-upstream-worktree-cancel]")) {
        overlay.remove();
        return;
      }
      if (target?.closest("[data-codex-upstream-worktree-defaults]")) {
        loadUpstreamWorktreeDefaults(overlay);
        return;
      }
      if (target?.closest("[data-codex-upstream-worktree-submit]")) {
        submitUpstreamWorktree(overlay);
      }
    }, true);
    upstreamWorktreeField(overlay, "repoPath")?.addEventListener("change", () => loadUpstreamWorktreeDefaults(overlay));
    document.body.appendChild(overlay);
    upstreamWorktreeField(overlay, "repoPath")?.focus();
  }

  function escapeHtml(value) {
    return String(value)
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#39;");
  }

  function confirmDelete(title) {
    document.querySelectorAll(".codex-delete-confirm-overlay").forEach((node) => node.remove());
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "codex-delete-confirm-overlay";
      overlay.innerHTML = `
        <div class="codex-delete-confirm-content" role="dialog" aria-modal="true" aria-label="删除会话">
          <div class="codex-delete-confirm-title">删除会话</div>
          <div class="codex-delete-confirm-message">删除“${escapeHtml(title)}”？</div>
          <div class="codex-delete-confirm-actions">
            <button type="button" data-codex-delete-cancel="true">取消</button>
            <button type="button" data-codex-delete-confirm="true">删除</button>
          </div>
        </div>
      `;
      const finish = (value, event) => {
        event?.preventDefault();
        event?.stopPropagation();
        event?.target?.blur?.();
        overlay.remove();
        resolve(value);
      };
      overlay.addEventListener("click", (event) => {
        if (event.target === overlay || event.target.closest("[data-codex-delete-cancel]")) {
          finish(false, event);
          return;
        }
        if (event.target.closest("[data-codex-delete-confirm]")) {
          finish(true, event);
        }
      }, true);
      overlay.addEventListener("keydown", (event) => {
        if (event.key === "Escape") finish(false, event);
      }, true);
      document.body.appendChild(overlay);
      overlay.querySelector("[data-codex-delete-cancel]")?.focus();
    });
  }

  function rowHref(row) {
    return row.getAttribute("href") || row.querySelector("a")?.getAttribute("href") || "";
  }

  function isCurrentSessionRow(row, ref) {
    if (row.getAttribute("aria-current") === "page" || row.getAttribute("aria-current") === "true") return true;
    const href = rowHref(row);
    if (href) {
      try {
        const url = new URL(href, window.location.href);
        if (url.href === window.location.href || url.pathname === window.location.pathname) return true;
      } catch {
        if (window.location.href.includes(href)) return true;
      }
    }
    return !!ref.session_id && window.location.href.includes(ref.session_id);
  }

  function releaseDeleteFocus(row, button) {
    button.blur();
    if (row.contains(document.activeElement)) {
      document.activeElement.blur();
    }
  }

  function removeDeletedRow(row, button, ref) {
    releaseDeleteFocus(row, button);
    const shouldReload = isCurrentSessionRow(row, ref);
    row.remove();
    if (shouldReload) {
      window.location.reload();
    }
  }

  function updateDeleteButtonOffsets() {
    sessionRows().forEach((row) => {
      const hasArchiveConfirm = Array.from(row.querySelectorAll("button")).some((button) => {
        const rect = button.getBoundingClientRect();
        const label = button.getAttribute("aria-label") || "";
        const text = (button.textContent || "").trim();
        if (button.classList.contains(buttonClass) || button.classList.contains(exportButtonClass) || label === "归档对话" || label === "置顶对话") return false;
        return text === "确认" || (text.length > 0 && rect.width > 0 && rect.width <= 36 && rect.x > row.getBoundingClientRect().right - 50);
      });
      row.classList.toggle("codex-archive-confirm-visible", hasArchiveConfirm);
    });
  }

  function openDeleteConfirmForRow(row, button, ref, event) {
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation?.();
    releaseDeleteFocus(row, button);
    confirmDelete(ref.title).then(async (confirmed) => {
      if (!confirmed) return;
      releaseDeleteFocus(row, button);
      const result = await postJson("/delete", ref);
      if (result.status === "server_deleted" || result.status === "local_deleted") {
        removeDeletedRow(row, button, ref);
        showToast(result.message || "删除成功", result.undo_token);
      } else {
        showToast(result.message || "删除失败", null);
      }
    });
  }

  async function exportMarkdown(ref) {
    const result = await postJson("/export-markdown", ref);
    if (result.status === "exported" && result.filename && typeof result.markdown === "string") {
      downloadMarkdown(result.filename, result.markdown);
      showToast(result.message || "导出成功", null);
      return;
    }
    showToast(result.message || "导出失败", null);
  }

  function sortStateFromMoveResult(result, ref, row) {
    const trustedSortMs = timestampMsFromPayload(result);
    return { sortMs: trustedSortMs || rowSortMs(row, ref), sortMsTrusted: !!trustedSortMs };
  }

  function finishProjectMove(row, button, ref, target, message) {
    releaseDeleteFocus(row, button);
    button.disabled = false;
    button.textContent = "移动";
    saveProjectMoveProjection(ref, target, target.sortMs || rowSortMs(row, ref, target));
    if (target.kind === "projectless") moveRowToChats(row, target);
    refreshAfterProjectMove();
    showToast(message, null);
  }

  async function applyProjectMove(row, button, ref, target) {
    button.disabled = true;
    button.textContent = "移动中";
    try {
      if (target.kind === "projectless") {
        const result = await moveSessionToProjectless(ref);
        finishProjectMove(row, button, ref, { ...target, ...sortStateFromMoveResult(result, ref, row) }, `已移动到普通对话：“${ref.title || ref.session_id}”`);
      } else {
        const result = await moveSessionToProject(ref, target);
        finishProjectMove(row, button, ref, { ...target, ...sortStateFromMoveResult(result, ref, row) }, `已移动到“${target.label}”：“${ref.title || ref.session_id}”`);
      }
    } catch (error) {
      button.disabled = false;
      button.textContent = "移动";
      showToast(`移动失败：${error?.message || error}`, null);
    }
  }

  async function openProjectMoveMenuForRow(row, button, ref, event) {
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation?.();
    releaseDeleteFocus(row, button);
    document.querySelectorAll(`.${projectMoveOverlayClass}`).forEach((node) => node.remove());
    const overlay = document.createElement("div");
    overlay.className = projectMoveOverlayClass;
    overlay.innerHTML = `
      <div class="codex-project-move-panel" role="dialog" aria-modal="true" aria-label="移动对话">
        <div class="codex-project-move-header">
          <div class="codex-project-move-title">移动“${escapeHtml(ref.title || ref.session_id)}”</div>
        </div>
        <div class="codex-project-move-list"><div class="codex-project-move-empty">加载项目中...</div></div>
      </div>
    `;
    const panel = overlay.querySelector(".codex-project-move-panel");
    const rect = button.getBoundingClientRect();
    const panelWidth = Math.min(360, Math.max(240, window.innerWidth - 32));
    panel.style.left = `${Math.max(16, Math.min(window.innerWidth - panelWidth - 16, rect.right - panelWidth))}px`;
    panel.style.top = `${Math.max(16, Math.min(window.innerHeight - 120, rect.bottom + 6))}px`;
    const close = () => overlay.remove();
    overlay.addEventListener("click", (clickEvent) => {
      if (clickEvent.target === overlay) close();
    }, true);
    overlay.addEventListener("keydown", (keyEvent) => {
      if (keyEvent.key === "Escape") {
        keyEvent.preventDefault();
        close();
      }
    }, true);
    document.body.appendChild(overlay);
    try {
      const targets = projectMoveTargets();
      const list = overlay.querySelector(".codex-project-move-list");
      if (!list) return;
      list.innerHTML = "";
      if (targets.length === 0) {
        list.innerHTML = `<div class="codex-project-move-empty">没有可用目标</div>`;
        return;
      }
      for (const target of targets) {
        const item = document.createElement("button");
        item.type = "button";
        item.className = "codex-project-move-item";
        item.innerHTML = `
          <div class="codex-project-move-item-title">${escapeHtml(target.label)}</div>
          <div class="codex-project-move-item-path">${escapeHtml(target.description)}</div>
        `;
        item.addEventListener("click", async (selectEvent) => {
          selectEvent.preventDefault();
          selectEvent.stopPropagation();
          close();
          await applyProjectMove(row, button, ref, target);
        }, true);
        list.appendChild(item);
      }
      list.querySelector("button")?.focus();
    } catch (error) {
      close();
      showToast(`加载项目失败：${error?.message || error}`, null);
    }
  }

  function installDeleteButtonEventDelegation() {
    document.removeEventListener("pointerup", window.__codexSessionDeleteDocumentDeleteHandler, true);
    document.removeEventListener("click", window.__codexSessionDeleteDocumentDeleteHandler, true);
    const handler = (event) => {
      const button = event.target?.closest?.(`.${buttonClass}`);
      const row = button?.closest?.("[data-app-action-sidebar-thread-id]");
      if (!button || !row) return;
      const ref = sessionRefFromRow(row);
      if (!ref.session_id) return;
      openDeleteConfirmForRow(row, button, ref, event);
    };
    window.__codexSessionDeleteDocumentDeleteHandler = handler;
    document.addEventListener("pointerup", handler, true);
    document.addEventListener("click", handler, true);
  }

  function actionGroupFromRow(row) {
    return row.querySelector(`.${actionGroupClass}`);
  }

  function nativeActionButtonsFromRow(row) {
    return [...row.querySelectorAll('button,[role="button"],a')]
      .filter((node) => !node.closest(`.${actionGroupClass}`))
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        if (rect.width < 12 || rect.height < 12) return false;
        const label = [
          node.getAttribute("aria-label"),
          node.getAttribute("title"),
          node.dataset?.state,
          node.textContent,
        ]
          .filter(Boolean)
          .join(" ")
          .toLowerCase();
        if (/(pin|archive|置顶|归档)/i.test(label)) return true;
        const rowRect = row.getBoundingClientRect();
        return rect.left > rowRect.left + rowRect.width * 0.68;
      });
  }

  function syncActionGroupLayout(row, group) {
    if (!row || !group) return;
    const rowRect = row.getBoundingClientRect();
    const nativeButtons = nativeActionButtonsFromRow(row);
    const leftmostNative = nativeButtons
      .map((button) => button.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0)
      .sort((a, b) => a.left - b.left)[0];
    const gap = 8;
    const fallbackRight = 28;
    const right = leftmostNative
      ? Math.max(fallbackRight, Math.round(rowRect.right - leftmostNative.left + gap))
      : fallbackRight;
    const groupWidth = Math.ceil(group.getBoundingClientRect().width || 96);
    const titleNode = row.querySelector(selectors.threadTitle);
    const titleRect = titleNode?.getBoundingClientRect();
    const titleLeft = titleRect?.left || rowRect.left + 40;
    const maxTitleWidth = Math.max(24, Math.round(rowRect.width - (titleLeft - rowRect.left) - right - groupWidth - 14));
    group.style.setProperty("--codex-session-actions-right", `${right}px`);
    row.style.setProperty("--codex-session-title-mask", `${right + groupWidth + 12}px`);
    row.style.setProperty("--codex-session-title-max-width", `${maxTitleWidth}px`);
  }

  function syncActionGroupsLayout() {
    sessionRows().forEach((row) => {
      const group = actionGroupFromRow(row);
      if (group) syncActionGroupLayout(row, group);
    });
  }

  function removeActionGroups(row) {
    row.querySelectorAll(`.${actionGroupClass}`).forEach((group) => group.remove());
  }

  function stopActionButtonEvent(row, button, event) {
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation?.();
    releaseDeleteFocus(row, button);
  }

  function installActionButtonEvents(row, button, onActivate) {
    ["pointerdown", "mousedown", "mouseup", "touchstart"].forEach((eventName) => {
      button.addEventListener(eventName, (event) => stopActionButtonEvent(row, button, event), true);
    });
    button.addEventListener("pointerenter", () => showActionButtonTooltip(button));
    button.addEventListener("pointerleave", hideActionButtonTooltip);
    button.addEventListener("focus", () => showActionButtonTooltip(button));
    button.addEventListener("blur", hideActionButtonTooltip);
    button.addEventListener("pointerup", onActivate, true);
    button.addEventListener("click", (event) => {
      hideActionButtonTooltip();
      onActivate(event);
    }, true);
  }

  function installMoreButtonEvents(row, button, onActivate) {
    ["pointerdown", "mousedown", "mouseup", "touchstart"].forEach((eventName) => {
      button.addEventListener(eventName, (event) => stopActionButtonEvent(row, button, event), true);
    });
    button.addEventListener("pointerenter", () => showActionButtonTooltip(button));
    button.addEventListener("pointerleave", hideActionButtonTooltip);
    button.addEventListener("focus", () => showActionButtonTooltip(button));
    button.addEventListener("blur", hideActionButtonTooltip);
    button.addEventListener("pointerup", onActivate, true);
    button.addEventListener("click", (event) => {
      hideActionButtonTooltip();
      stopActionButtonEvent(row, button, event);
    }, true);
  }

  function hideActionButtonTooltip() {
    document.querySelectorAll(`.${actionTooltipClass}`).forEach((node) => node.remove());
  }

  function closeSessionMoreMenus(exceptMenu = null) {
    document.querySelectorAll(`.${moreMenuClass}`).forEach((menu) => {
      if (menu !== exceptMenu) {
        menu.hidden = true;
        menu.closest?.("[data-codex-delete-row]")?.classList.remove("codex-session-more-open");
        menu.__codexSessionMoreRow?.classList?.remove("codex-session-more-open");
      }
    });
  }

  function toggleSessionMoreMenu(row, button, menu) {
    const nextHidden = !menu.hidden;
    closeSessionMoreMenus(menu);
    menu.hidden = nextHidden;
    row.classList.toggle("codex-session-more-open", !menu.hidden);
    button.setAttribute("aria-expanded", String(!menu.hidden));
  }

  function installSessionMoreMenuAutoClose(row, menu) {
    const group = menu.__codexSessionMoreGroup || menu.closest?.(`.${actionGroupClass}`);
    const closeIfOutside = () => {
      window.setTimeout(() => {
        if (menu.hidden) return;
        const active = document.activeElement;
        if (group?.matches?.(":hover") || menu.matches?.(":hover") || menu.contains(active)) return;
        menu.hidden = true;
        row.classList.remove("codex-session-more-open");
        group?.querySelector?.(`.${moreButtonClass}`)?.setAttribute("aria-expanded", "false");
      }, 80);
    };
    group?.addEventListener("pointerleave", closeIfOutside, true);
    menu.addEventListener("pointerleave", closeIfOutside, true);
    menu.addEventListener("focusout", closeIfOutside, true);
  }

  function updateSessionMoreMenuDirection(button, menu) {
    menu.classList.remove("codex-session-more-menu-open-up");
    const buttonRect = button.getBoundingClientRect();
    const estimatedMenuHeight = Math.max(80, menu.getBoundingClientRect().height || 76);
    if (buttonRect.bottom + 30 + estimatedMenuHeight > window.innerHeight - 8) {
      menu.classList.add("codex-session-more-menu-open-up");
    }
  }

  function positionSessionMoreMenu(button, menu) {
    const rect = button.getBoundingClientRect();
    const menuWidth = Math.max(104, menu.getBoundingClientRect().width || 104);
    const left = Math.min(window.innerWidth - menuWidth - 8, Math.max(8, rect.right - menuWidth));
    menu.style.left = `${left}px`;
    menu.style.top = `${Math.max(8, rect.bottom + 4)}px`;
  }

  function createSessionMoreMenuItem(label, icon, onActivate) {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "codex-session-more-menu-item";
    item.innerHTML = `<span class="codex-session-more-menu-icon">${icon}</span><span>${label}</span>`;
    item.addEventListener("click", onActivate, true);
    return item;
  }

  function showActionButtonTooltip(button) {
    const label = button.dataset.codexActionLabel || button.getAttribute("aria-label") || "";
    if (!label) return;
    hideActionButtonTooltip();
    const tooltip = document.createElement("div");
    tooltip.className = actionTooltipClass;
    tooltip.textContent = label;
    document.body.appendChild(tooltip);
    const buttonRect = button.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    const gap = 8;
    const left = Math.min(
      window.innerWidth - tooltipRect.width - 8,
      Math.max(8, buttonRect.left + buttonRect.width / 2 - tooltipRect.width / 2),
    );
    const top = Math.min(
      window.innerHeight - tooltipRect.height - 8,
      buttonRect.bottom + gap,
    );
    tooltip.style.left = `${left}px`;
    tooltip.style.top = `${Math.max(8, top)}px`;
  }

  function refreshActionButton(originalButton, row, onActivate) {
    if (!originalButton.isConnected) return;
    const replacement = originalButton.cloneNode(true);
    installActionButtonEvents(row, replacement, onActivate);
    originalButton.replaceWith(replacement);
    return replacement;
  }

  function configureActionButton(button, label, icon) {
    button.setAttribute("aria-label", label);
    button.dataset.codexActionLabel = label;
    button.removeAttribute("title");
    button.textContent = icon;
  }

  function trashIconSvg() {
    return `
      <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 6h18"></path>
        <path d="M8 6V4h8v2"></path>
        <path d="M19 6l-1 14H6L5 6"></path>
        <path d="M10 11v5"></path>
        <path d="M14 11v5"></path>
      </svg>
    `;
  }

  function configureSvgActionButton(button, label, svg) {
    button.setAttribute("aria-label", label);
    button.dataset.codexActionLabel = label;
    button.removeAttribute("title");
    button.innerHTML = svg;
  }

  function attachButton(row) {
    const settings = claudeCodexProSettings();
    if (!settings.sessionDelete && !settings.markdownExport && !settings.projectMove) {
      removeActionGroups(row);
      row.dataset.codexDeleteRow = "false";
      row.dataset.codexProjectMoveRow = "false";
      return;
    }
    const existingGroup = actionGroupFromRow(row);
    const existingDeleteButton = existingGroup?.querySelector(`.${buttonClass}`);
    const existingMoreButton = existingGroup?.querySelector(`.${moreButtonClass}`);
    const existingExportButton = existingGroup?.querySelector(`.${exportButtonClass}`);
    const existingMoveButton = existingGroup?.querySelector(`.${projectMoveButtonClass}`);
    const needsMoreMenu = settings.markdownExport || settings.projectMove;
    const hasUnexpectedDelete = !settings.sessionDelete && !!existingDeleteButton;
    const hasUnexpectedMore = !needsMoreMenu && !!existingMoreButton;
    const hasUnexpectedExport = !!existingExportButton;
    const hasUnexpectedMove = !!existingMoveButton;
    const missingDelete = settings.sessionDelete && !existingDeleteButton;
    const missingMore = needsMoreMenu && !existingMoreButton;
    const deleteReady = !settings.sessionDelete || existingDeleteButton?.dataset.codexDeleteVersion === codexDeleteVersion;
    const groupReady = existingGroup?.dataset.codexActionGroupVersion === codexActionGroupVersion;
    if (groupReady && deleteReady && !hasUnexpectedDelete && !hasUnexpectedMore && !hasUnexpectedExport && !hasUnexpectedMove && !missingDelete && !missingMore) {
      syncActionGroupLayout(row, existingGroup);
      return;
    }
    removeActionGroups(row);
    row.dataset.codexDeleteRow = "false";
    row.dataset.codexProjectMoveRow = "false";
    const ref = sessionRefFromRow(row);
    if (!ref.session_id) return;
    row.dataset.codexDeleteRow = "true";
    row.dataset.codexProjectMoveRow = String(!!settings.projectMove);
    const group = document.createElement("div");
    group.className = actionGroupClass;
    group.dataset.codexActionGroupVersion = codexActionGroupVersion;
    if (settings.markdownExport || settings.projectMove) {
      const moreButton = document.createElement("button");
      moreButton.type = "button";
      moreButton.className = `${actionButtonClass} ${moreButtonClass}`;
      moreButton.setAttribute("aria-haspopup", "menu");
      moreButton.setAttribute("aria-expanded", "false");
      configureActionButton(moreButton, "更多操作", "…");
      const moreMenu = document.createElement("div");
      moreMenu.className = moreMenuClass;
      moreMenu.setAttribute("role", "menu");
      moreMenu.hidden = true;
      if (settings.markdownExport) {
        moreMenu.appendChild(createSessionMoreMenuItem("导出", "⇩", (event) => {
          stopActionButtonEvent(row, moreButton, event);
          closeSessionMoreMenus();
          exportMarkdown(ref);
        }));
      }
      if (settings.projectMove) {
        moreMenu.appendChild(createSessionMoreMenuItem("移动", "↗", (event) => {
          stopActionButtonEvent(row, moreButton, event);
          closeSessionMoreMenus();
          openProjectMoveMenuForRow(row, moreButton, ref, event);
        }));
      }
      const openMoreMenu = (event) => {
        stopActionButtonEvent(row, moreButton, event);
        hideActionButtonTooltip();
        toggleSessionMoreMenu(row, moreButton, moreMenu);
        if (!moreMenu.hidden) {
          positionSessionMoreMenu(moreButton, moreMenu);
          updateSessionMoreMenuDirection(moreButton, moreMenu);
        }
      };
      installMoreButtonEvents(row, moreButton, openMoreMenu);
      group.appendChild(moreButton);
      moreMenu.__codexSessionMoreRow = row;
      moreMenu.__codexSessionMoreGroup = group;
      document.body.appendChild(moreMenu);
      installSessionMoreMenuAutoClose(row, moreMenu);
    }
    if (settings.sessionDelete) {
      const deleteButton = document.createElement("button");
      deleteButton.type = "button";
      deleteButton.className = `${actionButtonClass} ${buttonClass}`;
      deleteButton.dataset.codexDeleteVersion = codexDeleteVersion;
      configureSvgActionButton(deleteButton, "删除", trashIconSvg());
      const openDeleteConfirm = (event) => openDeleteConfirmForRow(row, deleteButton, ref, event);
      installActionButtonEvents(row, deleteButton, openDeleteConfirm);
      group.appendChild(deleteButton);
      setTimeout(() => refreshActionButton(deleteButton, row, openDeleteConfirm), 0);
    }
    row.appendChild(group);
    syncActionGroupLayout(row, group);
  }

  function tryAttachButton(row) {
    try {
      attachButton(row);
    } catch (error) {
      window.__codexSessionDeleteAttachButtonFailures = window.__codexSessionDeleteAttachButtonFailures || [];
      window.__codexSessionDeleteAttachButtonFailures.push(String(error?.stack || error));
    }
  }

  function reactArchivedThreadFromNode(node) {
    const reactKey = Object.keys(node).find((key) => key.startsWith("__reactFiber$") || key.startsWith("__reactInternalInstance$"));
    let fiber = reactKey ? node[reactKey] : null;
    for (let depth = 0; fiber && depth < 20; depth += 1, fiber = fiber.return) {
      const props = fiber.memoizedProps || fiber.pendingProps || {};
      if (props.archivedThread?.id) return props.archivedThread;
      const childThread = props.children?.props?.archivedThread;
      if (childThread?.id) return childThread;
    }
    return null;
  }

  function archivedThreadFromRow(row) {
    for (const node of [row, ...row.querySelectorAll("*")]) {
      const thread = reactArchivedThreadFromNode(node);
      if (thread?.id || thread?.sessionId) return thread;
    }
    return null;
  }

  function archivedRefFromRow(row) {
    const archivedThread = archivedThreadFromRow(row);
    if (archivedThread?.id || archivedThread?.sessionId) {
      return { session_id: archivedThread.id || archivedThread.sessionId, title: archivedThread.title || row.querySelector(".truncate.text-base")?.textContent?.trim() || "Untitled session" };
    }
    const sidebarRef = sessionRefFromRow(row);
    if (sidebarRef.session_id) return sidebarRef;
    const titleNode = row.querySelector(".truncate.text-base, [data-thread-title], a, div");
    const title = ((titleNode || row).textContent || "Untitled session")
      .replace("取消归档", "")
      .replace("删除", "")
      .replace(/\d{4}年\d{1,2}月\d{1,2}日.*$/, "")
      .replace(/\s+·\s+.*$/, "")
      .trim()
      .slice(0, 160);
    return { session_id: "", title };
  }

  async function resolveArchivedThread(row) {
    const ref = archivedRefFromRow(row);
    if (ref.session_id) return ref;
    const resolved = await postJson("/archived-thread", { title: ref.title });
    return resolved?.session_id ? resolved : ref;
  }

  function stopArchivedButtonEvent(event) {
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation?.();
  }

  function isArchiveTitleText(value) {
    return value === "已归档对话" || value === "Archived conversations";
  }

  function archiveTitleContainer() {
    const heading = Array.from(document.querySelectorAll("h1, h2, h3"))
      .find((element) => isArchiveTitleText((element.textContent || "").trim()));
    if (heading) return heading;
    return Array.from(document.querySelectorAll("h1, h2, h3, div, span"))
      .find((element) => isArchiveTitleText((element.textContent || "").trim()) && element.getBoundingClientRect().x > 350);
  }

  function attachArchivedPageDeleteButton(row) {
    const settings = claudeCodexProSettings();
    row.querySelectorAll("[data-codex-archive-row-action]").forEach((button) => button.remove());
    row.dataset.codexArchiveDeleteRow = "false";
    if (!settings.sessionDelete && !settings.markdownExport) return;
    const unarchiveButton = Array.from(row.querySelectorAll("button")).find((button) => (button.textContent || "").trim() === "取消归档");
    if (!unarchiveButton) return;
    row.dataset.codexArchiveDeleteRow = "true";
    row.dataset.codexArchiveRowActionsVersion = codexArchiveRowActionsVersion;
    let insertionPoint = unarchiveButton;
    if (settings.markdownExport) {
      const exportButton = document.createElement("button");
      exportButton.type = "button";
      exportButton.className = `codex-archive-delete-all codex-archive-row-button ${exportButtonClass}`;
      exportButton.dataset.codexArchiveRowAction = "export";
      exportButton.textContent = "导出";
      ["pointerdown", "mousedown", "mouseup", "touchstart"].forEach((eventName) => {
        exportButton.addEventListener(eventName, stopArchivedButtonEvent, true);
      });
      exportButton.addEventListener("click", async (event) => {
        stopArchivedButtonEvent(event);
        const ref = await resolveArchivedThread(row);
        if (!ref.session_id) {
          showToast("导出失败：未找到归档会话 ID", null);
          return;
        }
        await exportMarkdown(ref);
      }, true);
      insertionPoint.insertAdjacentElement("afterend", exportButton);
      insertionPoint = exportButton;
    }
  }

  function truncateTimelineQuestion(text) {
    const normalized = String(text || "").replace(/\s+/g, " ").trim();
    const chars = Array.from(normalized);
    if (chars.length <= timelineQuestionLimit) return normalized;
    return `${chars.slice(0, timelineQuestionLimit).join("")}…`;
  }

  function conversationTimelineRoot() {
    return document.querySelector(".thread-scroll-container") || document.querySelector("main") || document.querySelector('[role="main"]');
  }

  function timelineQuestionSelector() {
    return [
      '[data-message-author-role="user"]',
      '[data-testid="conversation-turn"][data-message-author-role="user"]',
      '[data-testid="conversation-turn"] [data-message-author-role="user"]',
      '[class*="user-message"]',
      '[class*="UserMessage"]',
    ].join(", ");
  }

  function nodeOrAncestorLooksLikeCodexUserBubble(node) {
    if (node.nodeType !== 1) return false;
    const className = String(node.className || "");
    if (className.includes("bg-token-foreground/5") && node.parentElement?.classList?.contains("items-end")) return true;
    const bubble = node.closest?.("[class*='bg-token-foreground/5']");
    return !!bubble?.parentElement?.classList?.contains("items-end");
  }

  function nodeLooksLikeCodexUserBubble(node) {
    if (nodeOrAncestorLooksLikeCodexUserBubble(node)) return true;
    return !!node.querySelector?.(".group.flex.w-full.flex-col.items-end.justify-end.gap-1 > [class*='bg-token-foreground/5']");
  }

  function nodeLooksLikeTimelineQuestion(node) {
    if (node.nodeType !== 1 || isExtensionUiNode(node)) return false;
    const questionSelector = timelineQuestionSelector();
    return !!node.matches?.(questionSelector) || !!node.closest?.(questionSelector) || !!node.querySelector?.(questionSelector) || nodeLooksLikeCodexUserBubble(node);
  }

  function conversationTimelineQuestionCandidates(root) {
    const explicitCandidates = Array.from(root.querySelectorAll([
      '[data-message-author-role="user"]',
      '[data-testid="conversation-turn"][data-message-author-role="user"]',
      '[data-testid="conversation-turn"] [data-message-author-role="user"]',
      '[class*="user-message"]',
      '[class*="UserMessage"]',
    ].join(", ")));
    const codexUserBubbles = Array.from(root.querySelectorAll(".group.flex.w-full.flex-col.items-end.justify-end.gap-1")).flatMap((group) => {
      return Array.from(group.children).filter((child) => String(child.className || "").includes("bg-token-foreground/5"));
    });
    return [...explicitCandidates, ...codexUserBubbles];
  }

  function extractTimelineQuestionText(node) {
    const clone = node.cloneNode(true);
    clone.querySelectorAll("button, svg, [aria-hidden='true'], .sr-only").forEach((child) => child.remove());
    return clone.textContent.replace(/\s+/g, " ").trim();
  }

  function timelineNodeId(node) {
    if (!node.__codexConversationTimelineNodeId) {
      window.__codexConversationTimelineNodeCounter += 1;
      node.__codexConversationTimelineNodeId = String(window.__codexConversationTimelineNodeCounter);
    }
    return node.__codexConversationTimelineNodeId;
  }

  function visibleTimelineNode(node) {
    if (!node.isConnected) return false;
    const style = getComputedStyle(node);
    if (style.display === "none" || style.visibility === "hidden") return false;
    const rect = node.getBoundingClientRect();
    return rect.width > 0 || rect.height > 0 || !!node.textContent?.trim();
  }

  function conversationTimelineQuestions() {
    const root = conversationTimelineRoot();
    if (!root?.matches?.('.thread-scroll-container, main, [role="main"]')) return [];
    const seen = new Set();
    return conversationTimelineQuestionCandidates(root).flatMap((node) => {
      if (node.closest('[data-app-action-sidebar-thread-id]')) return [];
      if (isExtensionUiNode(node)) return [];
      const target = node.closest('[data-testid="conversation-turn"]') || node;
      if (seen.has(target)) return [];
      seen.add(target);
      if (!visibleTimelineNode(target)) return [];
      const text = extractTimelineQuestionText(node);
      if (!text) return [];
      return [{ node: target, text, nodeId: timelineNodeId(target) }];
    });
  }

  function timelineScrollerViewportTop(scroller) {
    if (scroller === document.scrollingElement || scroller === document.documentElement || scroller === document.body) return 0;
    return scroller.getBoundingClientRect().top;
  }

  function timelineScrollableHeight(scroller) {
    return Math.max(1, scroller.scrollHeight - scroller.clientHeight);
  }

  function timelineRawMarkerTop(question, scroller) {
    const scrollOffset = scroller.scrollTop + question.node.getBoundingClientRect().top - timelineScrollerViewportTop(scroller);
    const percent = (scrollOffset / timelineScrollableHeight(scroller)) * 100;
    return Math.max(timelineMinTopPercent, Math.min(timelineMaxTopPercent, percent));
  }

  function timelineMarkerTops(questions, scroller) {
    if (questions.length <= 1) return [50];
    const minGap = Math.min(timelineMaxMarkerGapPercent, (timelineMaxTopPercent - timelineMinTopPercent) / Math.max(questions.length - 1, 1));
    const tops = questions.map((question) => timelineRawMarkerTop(question, scroller));
    for (let index = 1; index < tops.length; index += 1) {
      tops[index] = Math.max(tops[index], tops[index - 1] + minGap);
    }
    for (let index = tops.length - 1; index >= 0; index -= 1) {
      const maxForIndex = timelineMaxTopPercent - ((tops.length - 1 - index) * minGap);
      tops[index] = Math.min(tops[index], maxForIndex);
    }
    return tops.map((top) => Math.max(timelineMinTopPercent, Math.min(timelineMaxTopPercent, top)));
  }

  function removeConversationTimeline() {
    document.querySelectorAll(`.${timelineClass}`).forEach((node) => node.remove());
  }

  function nearestTimelineScroller(node) {
    for (let current = node?.parentElement; current; current = current.parentElement) {
      const style = getComputedStyle(current);
      if (/(auto|scroll)/.test(style.overflowY) && current.scrollHeight > current.clientHeight) return current;
    }
    return document.querySelector(".thread-scroll-container") || document.scrollingElement || document.documentElement;
  }

  function scrollTimelineTarget(node) {
    const scroller = nearestTimelineScroller(node);
    const nodeRect = node.getBoundingClientRect();
    const nextTop = scroller.scrollTop + nodeRect.top - timelineScrollerViewportTop(scroller) - (scroller.clientHeight / 2) + (nodeRect.height / 2);
    scroller.scrollTo({ top: nextTop, behavior: "smooth" });
  }

  function highlightTimelineTarget(node) {
    node.classList.remove(timelineTargetClass);
    void node.offsetWidth;
    node.classList.add(timelineTargetClass);
    clearTimeout(node.__codexConversationTimelineHighlightTimer);
    node.__codexConversationTimelineHighlightTimer = setTimeout(() => {
      node.classList.remove(timelineTargetClass);
    }, 1300);
  }

  function createConversationTimelineMarker(question) {
    const marker = document.createElement("button");
    marker.type = "button";
    marker.className = timelineMarkerClass;
    marker.style.top = `${question.markerTop}%`;
    marker.setAttribute("aria-label", `跳转到：${truncateTimelineQuestion(question.text)}`);
    const tooltip = document.createElement("span");
    tooltip.className = timelineTooltipClass;
    tooltip.id = `codex-conversation-timeline-tooltip-${question.nodeId}`;
    tooltip.setAttribute("role", "tooltip");
    tooltip.textContent = truncateTimelineQuestion(question.text);
    marker.setAttribute("aria-describedby", tooltip.id);
    marker.appendChild(tooltip);
    const activateMarker = (event) => {
      event.preventDefault();
      event.stopPropagation();
      event.stopImmediatePropagation?.();
      document.querySelectorAll(`.${timelineMarkerClass}.codex-conversation-timeline-marker-active`).forEach((node) => {
        node.classList.remove("codex-conversation-timeline-marker-active");
      });
      marker.classList.add("codex-conversation-timeline-marker-active");
      scrollTimelineTarget(question.node);
      highlightTimelineTarget(question.node);
    };
    marker.addEventListener("pointerup", activateMarker, true);
    marker.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") activateMarker(event);
    }, true);
    return marker;
  }

  function prepareTimelineQuestions(questions) {
    if (questions.length === 0) return [];
    const scroller = nearestTimelineScroller(questions[0].node);
    const tops = timelineMarkerTops(questions, scroller);
    return questions.map((question, index) => ({ ...question, markerTop: Number(tops[index].toFixed(3)) }));
  }

  function timelineSignature(questions) {
    return questions.map((question) => `${question.nodeId}:${Math.round(question.markerTop * 10)}:${truncateTimelineQuestion(question.text)}`).join("|");
  }

  function refreshConversationTimeline() {
    if (!claudeCodexProSettings().conversationTimeline) {
      removeConversationTimeline();
      return;
    }
    const questions = prepareTimelineQuestions(conversationTimelineQuestions());
    if (questions.length === 0) {
      removeConversationTimeline();
      return;
    }
    const signature = timelineSignature(questions);
    const existing = document.querySelector(`.${timelineClass}`);
    if (
      existing?.dataset.codexConversationTimelineVersion === codexConversationTimelineVersion &&
      existing?.dataset.codexConversationTimelineSignature === signature
    ) {
      return;
    }
    removeConversationTimeline();
    const container = document.createElement("div");
    container.className = timelineClass;
    container.dataset.codexConversationTimelineVersion = codexConversationTimelineVersion;
    container.dataset.codexConversationTimelineSignature = signature;
    const track = document.createElement("div");
    track.className = timelineTrackClass;
    container.appendChild(track);
    questions.forEach((question) => {
      container.appendChild(createConversationTimelineMarker(question));
    });
    document.body.appendChild(container);
  }

  const conversationViewContentClasses = [
    "mx-auto",
    "w-full",
    "max-w-(--thread-content-max-width)",
    "px-toolbar",
    "relative",
    "flex",
    "shrink-0",
    "flex-col",
    "pb-8",
  ];
  const conversationViewComposerClasses = [
    "relative",
    "z-10",
    "flex",
    "flex-col",
    "mx-auto",
    "w-full",
    "max-w-(--thread-content-max-width)",
    "px-toolbar",
  ];
  const conversationViewState = {
    contentEl: null,
    composerEl: null,
    rafId: 0,
    settleFramesLeft: 0,
    mo: null,
    ro: null,
    pollId: 0,
    moObserved: false,
    observed: new WeakSet(),
    elements: new Set(),
  };

  function conversationViewTokenSet(el) {
    return new Set(String(el?.className || "").split(/\s+/).filter(Boolean));
  }

  function conversationViewHasAllClasses(el, classes) {
    const set = conversationViewTokenSet(el);
    return classes.every((cls) => set.has(cls));
  }

  function conversationViewFindByClasses(classes) {
    return Array.from(document.querySelectorAll("div")).find((el) => conversationViewHasAllClasses(el, classes)) || null;
  }

  function conversationViewFindContentEl() {
    return conversationViewFindByClasses(conversationViewContentClasses);
  }

  function conversationViewFindComposerEl() {
    return conversationViewFindByClasses(conversationViewComposerClasses);
  }

  function codexServiceTierBadgeVisibleElement(element) {
    if (!(element instanceof HTMLElement) || !element.isConnected) return false;
    const style = getComputedStyle(element);
    if (style.display === "none" || style.visibility === "hidden") return false;
    const rect = element.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  }

  function codexServiceTierBadgeText(element) {
    return String(element?.textContent || "").replace(/\s+/g, " ").trim();
  }

  function codexServiceTierKnownProviderNames() {
    return uniqueValues([
      codexModelCatalog.provider_name,
      codexModelCatalog.model_provider,
    ]).map((value) => value.toLowerCase());
  }

  function codexServiceTierLooksLikeProviderButton(button, providerNames) {
    const text = codexServiceTierBadgeText(button);
    if (!text || text.length > 32) return false;
    const lower = text.toLowerCase();
    if (providerNames.includes(lower)) return true;
    if (/\s/.test(text)) return false;
    if (!/[a-z]/i.test(text)) return false;
    if (!/^[a-z0-9][a-z0-9._-]{1,31}$/i.test(text)) return false;
    if (/^(local|remote|cloud|standard|default|fast|worktree|new|send|stop|codex)$/i.test(text)) return false;
    if (/^(gpt|o[1-9]|claude|gemini|deepseek|qwen|kimi|moonshot|mistral|llama|sonnet|opus|haiku)[a-z0-9._-]*$/i.test(text)) return false;
    return true;
  }

  function codexServiceTierBadgeButtonCandidates(composer) {
    const composerRect = composer.getBoundingClientRect();
    return Array.from(composer.querySelectorAll("button, [role='button']"))
      .filter((button) => !button.closest?.(`[data-codex-service-tier-badge="true"]`))
      .filter(codexServiceTierBadgeVisibleElement)
      .filter((button) => {
        const rect = button.getBoundingClientRect();
        return rect.bottom >= composerRect.top + composerRect.height * 0.35;
      })
      .sort((left, right) => {
        const leftRect = left.getBoundingClientRect();
        const rightRect = right.getBoundingClientRect();
        return (rightRect.bottom - leftRect.bottom) || (leftRect.left - rightRect.left);
      });
  }

  function codexServiceTierVisibleComposerFooters(root = document) {
    const footers = [
      ...(root?.matches?.(".composer-footer") ? [root] : []),
      ...Array.from(root?.querySelectorAll?.(".composer-footer") || []),
    ];
    return footers
      .filter(codexServiceTierBadgeVisibleElement)
      .sort((left, right) => {
        const leftRect = left.getBoundingClientRect();
        const rightRect = right.getBoundingClientRect();
        return (rightRect.bottom - leftRect.bottom) || (rightRect.width - leftRect.width);
      });
  }

  function codexServiceTierComposerScore(composer) {
    const text = codexServiceTierBadgeText(composer).toLowerCase();
    const providerNames = codexServiceTierKnownProviderNames();
    let score = 0;
    if (providerNames.some((name) => name && text.includes(name))) score += 40;
    if (/完全访问权限|full access|model|超高|high|sub2api|provider/i.test(text)) score += 20;
    if (/本地模式|local mode|worktree|branch|codex\//i.test(text)) score -= 30;
    if (composer.matches?.(".composer-footer")) score += 4;
    if (composer.querySelector?.(".composer-footer")) score += 8;
    const buttons = Array.from(composer.querySelectorAll?.("button, [role='button']") || []).filter(codexServiceTierBadgeVisibleElement);
    if (buttons.some((button) => codexServiceTierLooksLikeProviderButton(button, providerNames))) score += 30;
    score += Math.min(10, buttons.length);
    return score;
  }

  function codexServiceTierComposerCandidates() {
    const candidates = new Set();
    const threadComposer = conversationViewFindComposerEl();
    if (threadComposer && codexServiceTierBadgeVisibleElement(threadComposer)) candidates.add(threadComposer);
    codexServiceTierVisibleComposerFooters().forEach((footer) => {
      candidates.add(footer);
      let node = footer.parentElement;
      for (let depth = 0; node instanceof HTMLElement && depth < 6; depth += 1, node = node.parentElement) {
        if (codexServiceTierBadgeVisibleElement(node)) candidates.add(node);
      }
    });
    return Array.from(candidates);
  }

  function codexServiceTierBestComposerFooter(root = document) {
    return codexServiceTierVisibleComposerFooters(root)
      .map((footer, index) => ({ footer, index, score: codexServiceTierComposerScore(footer) }))
      .sort((left, right) => (right.score - left.score) || (left.index - right.index))[0]?.footer || null;
  }

  function codexServiceTierFindComposerEl() {
    return codexServiceTierComposerCandidates()
      .map((composer, index) => ({ composer, index, score: codexServiceTierComposerScore(composer) }))
      .sort((left, right) => (right.score - left.score) || (left.index - right.index))[0]?.composer || null;
  }

  function codexServiceTierBadgeAnchor(composer) {
    const providerNames = codexServiceTierKnownProviderNames();
    const buttons = codexServiceTierBadgeButtonCandidates(composer);
    const exact = buttons.find((button) => providerNames.includes(codexServiceTierBadgeText(button).toLowerCase()));
    if (exact) return exact;
    const composerRect = composer.getBoundingClientRect();
    return buttons.find((button) => {
      const rect = button.getBoundingClientRect();
      return rect.left >= composerRect.left + composerRect.width * 0.42 && codexServiceTierLooksLikeProviderButton(button, providerNames);
    }) || null;
  }

  function codexServiceTierComposerFooter(composer) {
    if (composer?.matches?.(".composer-footer")) return composer;
    return codexServiceTierBestComposerFooter(composer) || codexServiceTierBestComposerFooter() || null;
  }

  function codexServiceTierBadgeFooterGroup(composer) {
    const footer = codexServiceTierComposerFooter(composer);
    if (!footer) return null;
    const children = Array.from(footer.children).filter(codexServiceTierBadgeVisibleElement);
    if (!children.length) return footer;
    const providerNames = codexServiceTierKnownProviderNames();
    const providerGroup = children.find((child) => {
      const text = codexServiceTierBadgeText(child).toLowerCase();
      return providerNames.some((name) => name && text.includes(name));
    });
    return providerGroup || children[children.length - 1] || footer;
  }

  function codexServiceTierBadgePlacement(composer) {
    const anchor = composer ? codexServiceTierBadgeAnchor(composer) : null;
    if (anchor?.parentElement) return { parent: anchor.parentElement, before: anchor };
    const group = composer ? codexServiceTierBadgeFooterGroup(composer) : null;
    if (group) return { parent: group, before: group.firstChild };
    return null;
  }

  function wireCodexServiceTierBadge(badge) {
    if (!badge || badge.dataset.codexServiceTierBadgeWired === codexServiceTierBadgeVersion) return;
    badge.dataset.codexServiceTierBadgeWired = codexServiceTierBadgeVersion;
    badge.setAttribute("role", "button");
    badge.setAttribute("tabindex", "0");
    badge.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (codexServiceTierState.status === "loading") return;
      toggleCodexServiceTierFromBadge();
    });
    badge.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      event.stopPropagation();
      if (codexServiceTierState.status === "loading") return;
      toggleCodexServiceTierFromBadge();
    });
  }

  function installCodexServiceTierBadge() {
    if (!claudeCodexProSettings().serviceTierControls) {
      removeCodexServiceTierBadges();
      return;
    }
    const composer = codexServiceTierFindComposerEl();
    const placement = composer ? codexServiceTierBadgePlacement(composer) : null;
    const existingBadges = Array.from(document.querySelectorAll(`[data-codex-service-tier-badge="true"]`));
    if (!composer || !placement?.parent) {
      existingBadges.forEach((badge) => badge.remove());
      return;
    }
    let badge = existingBadges.find((node) => node.closest?.(".composer-footer") || node.closest?.("button") == null) || existingBadges[0];
    existingBadges.forEach((node) => {
      if (node !== badge) node.remove();
    });
    if (!badge || badge.dataset.codexServiceTierBadgeVersion !== codexServiceTierBadgeVersion) {
      badge?.remove();
      badge = document.createElement("span");
      badge.className = codexServiceTierBadgeClass;
      badge.dataset.codexServiceTierBadge = "true";
      badge.dataset.codexServiceTierBadgeVersion = codexServiceTierBadgeVersion;
    }
    wireCodexServiceTierBadge(badge);
    const before = placement.before?.parentElement === placement.parent ? placement.before : null;
    if (badge.parentElement !== placement.parent || badge.nextSibling !== before) {
      placement.parent.insertBefore(badge, before);
    }
    refreshCodexServiceTierBadges();
  }

  function removeCodexServiceTierBadges() {
    document.querySelectorAll(`[data-codex-service-tier-badge="true"]`).forEach((badge) => badge.remove());
  }

  function conversationViewRememberOriginals(el) {
    if (!el) return;
    conversationViewState.elements.add(el);
    const original = {
      width: el.style.width || "",
      maxWidth: el.style.maxWidth || "",
      marginLeft: el.style.marginLeft || "",
      marginRight: el.style.marginRight || "",
      left: el.style.left || "",
      transform: el.style.transform || "",
      boxSizing: el.style.boxSizing || "",
    };
    if (!("claudeCodexProConversationViewOriginalWidth" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalWidth = original.width;
    if (!("claudeCodexProConversationViewOriginalMaxWidth" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalMaxWidth = original.maxWidth;
    if (!("claudeCodexProConversationViewOriginalMarginLeft" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalMarginLeft = original.marginLeft;
    if (!("claudeCodexProConversationViewOriginalMarginRight" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalMarginRight = original.marginRight;
    if (!("claudeCodexProConversationViewOriginalLeft" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalLeft = original.left;
    if (!("claudeCodexProConversationViewOriginalTransform" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalTransform = original.transform;
    if (!("claudeCodexProConversationViewOriginalBoxSizing" in el.dataset)) el.dataset.claudeCodexProConversationViewOriginalBoxSizing = original.boxSizing;
  }

  function conversationViewRestoreElement(el) {
    if (!el) return;
    if ("claudeCodexProConversationViewOriginalWidth" in el.dataset) {
      el.style.width = el.dataset.claudeCodexProConversationViewOriginalWidth;
      delete el.dataset.claudeCodexProConversationViewOriginalWidth;
    }
    if ("claudeCodexProConversationViewOriginalMaxWidth" in el.dataset) {
      el.style.maxWidth = el.dataset.claudeCodexProConversationViewOriginalMaxWidth;
      delete el.dataset.claudeCodexProConversationViewOriginalMaxWidth;
    }
    if ("claudeCodexProConversationViewOriginalMarginLeft" in el.dataset) {
      el.style.marginLeft = el.dataset.claudeCodexProConversationViewOriginalMarginLeft;
      delete el.dataset.claudeCodexProConversationViewOriginalMarginLeft;
    }
    if ("claudeCodexProConversationViewOriginalMarginRight" in el.dataset) {
      el.style.marginRight = el.dataset.claudeCodexProConversationViewOriginalMarginRight;
      delete el.dataset.claudeCodexProConversationViewOriginalMarginRight;
    }
    if ("claudeCodexProConversationViewOriginalLeft" in el.dataset) {
      el.style.left = el.dataset.claudeCodexProConversationViewOriginalLeft;
      delete el.dataset.claudeCodexProConversationViewOriginalLeft;
    }
    if ("claudeCodexProConversationViewOriginalTransform" in el.dataset) {
      el.style.transform = el.dataset.claudeCodexProConversationViewOriginalTransform;
      delete el.dataset.claudeCodexProConversationViewOriginalTransform;
    }
    if ("claudeCodexProConversationViewOriginalBoxSizing" in el.dataset) {
      el.style.boxSizing = el.dataset.claudeCodexProConversationViewOriginalBoxSizing;
      delete el.dataset.claudeCodexProConversationViewOriginalBoxSizing;
    }
  }

  function conversationViewResetOwnOffset(el) {
    if (!el) return;
    const originalTransform = el.dataset.claudeCodexProConversationViewOriginalTransform || "";
    const originalLeft = el.dataset.claudeCodexProConversationViewOriginalLeft || "";
    if (el.style.left !== originalLeft) el.style.left = originalLeft;
    if (el.style.transform !== originalTransform) el.style.transform = originalTransform;
    const transform = String(el.style.transform || "").trim();
    if (/^(translateX\([^)]*\)\s*)+$/i.test(transform)) {
      el.style.transform = "";
    }
  }

  function conversationViewApplyNativeWidth(el) {
    conversationViewRememberOriginals(el);
    const maxWidth = `${conversationViewWidth()}px`;
    if (el.style.boxSizing !== "border-box") el.style.boxSizing = "border-box";
    if (el.style.width !== "100%") el.style.width = "100%";
    if (el.style.maxWidth !== maxWidth) el.style.maxWidth = maxWidth;
    if (el.style.marginLeft !== "auto") el.style.marginLeft = "auto";
    if (el.style.marginRight !== "auto") el.style.marginRight = "auto";
  }

  function conversationViewSessionRectFor(el) {
    return el?.parentElement?.getBoundingClientRect() || null;
  }

  function conversationViewHtmlCenter() {
    const rect = document.documentElement.getBoundingClientRect();
    return rect.left + rect.width / 2;
  }

  function conversationViewHasRoomForHtmlCenter(nativeRect, bounds) {
    if (!nativeRect || !bounds) return false;
    const targetLeft = conversationViewHtmlCenter() - nativeRect.width / 2;
    const targetRight = targetLeft + nativeRect.width;
    return targetLeft >= bounds.left - 0.5 && targetRight <= bounds.right + 0.5;
  }

  function conversationViewAlignElement(el) {
    if (!el?.isConnected) return;
    conversationViewApplyNativeWidth(el);
    conversationViewResetOwnOffset(el);
    const nativeRect = el.getBoundingClientRect();
    const bounds = conversationViewSessionRectFor(el);
    if (!conversationViewHasRoomForHtmlCenter(nativeRect, bounds)) return;
    const targetLeft = conversationViewHtmlCenter() - nativeRect.width / 2;
    const delta = targetLeft - nativeRect.left;
    if (Math.abs(delta) > 0.5) {
      const nextLeft = `${delta.toFixed(2)}px`;
      if (el.style.left !== nextLeft) el.style.left = nextLeft;
    }
  }

  function conversationViewObserveIfNeeded(el) {
    if (!el || !conversationViewState.ro || conversationViewState.observed.has(el)) return;
    conversationViewState.observed.add(el);
    conversationViewState.ro.observe(el);
  }

  function conversationViewResolveTargets() {
    if (!conversationViewState.contentEl?.isConnected) conversationViewState.contentEl = conversationViewFindContentEl();
    if (!conversationViewState.composerEl?.isConnected) conversationViewState.composerEl = conversationViewFindComposerEl();
    [
      document.documentElement,
      document.body,
      conversationViewState.contentEl,
      conversationViewState.contentEl?.parentElement,
      conversationViewState.contentEl?.parentElement?.parentElement,
      conversationViewState.composerEl,
      conversationViewState.composerEl?.parentElement,
      conversationViewState.composerEl?.parentElement?.parentElement,
    ].forEach(conversationViewObserveIfNeeded);
  }

  function conversationViewAlignNow() {
    if (!claudeCodexProSettings().conversationView) return;
    conversationViewResolveTargets();
    conversationViewAlignElement(conversationViewState.contentEl);
    conversationViewAlignElement(conversationViewState.composerEl);
  }

  function scheduleConversationViewAlign(frames = 16) {
    conversationViewState.settleFramesLeft = Math.max(conversationViewState.settleFramesLeft, frames);
    if (conversationViewState.rafId) return;
    const tick = () => {
      conversationViewState.rafId = 0;
      conversationViewAlignNow();
      conversationViewState.settleFramesLeft -= 1;
      if (conversationViewState.settleFramesLeft > 0) {
        conversationViewState.rafId = requestAnimationFrame(tick);
      }
    };
    conversationViewState.rafId = requestAnimationFrame(tick);
  }

  function cleanupConversationView() {
    if (conversationViewState.rafId) cancelAnimationFrame(conversationViewState.rafId);
    if (conversationViewState.pollId) clearInterval(conversationViewState.pollId);
    conversationViewState.rafId = 0;
    conversationViewState.pollId = 0;
    conversationViewState.mo?.disconnect();
    conversationViewState.ro?.disconnect();
    conversationViewState.mo = null;
    conversationViewState.ro = null;
    conversationViewState.moObserved = false;
    conversationViewState.observed = new WeakSet();
    conversationViewState.elements.forEach(conversationViewRestoreElement);
    conversationViewState.elements.clear();
    conversationViewState.contentEl = null;
    conversationViewState.composerEl = null;
  }

  window.__claudeCodexProConversationViewCleanup = cleanupConversationView;

  function ensureConversationViewRuntime() {
    if (conversationViewState.ro && conversationViewState.mo && conversationViewState.pollId) return;
    conversationViewState.ro = conversationViewState.ro || new ResizeObserver(() => scheduleConversationViewAlign());
    conversationViewState.mo = conversationViewState.mo || new MutationObserver(() => scheduleConversationViewAlign());
    if (document.body && !conversationViewState.moObserved) {
      conversationViewState.mo.observe(document.body, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["class", "hidden", "data-state", "aria-hidden"],
      });
      conversationViewState.moObserved = true;
    }
    conversationViewState.pollId = conversationViewState.pollId || window.setInterval(() => scheduleConversationViewAlign(2), 350);
  }

  function refreshConversationView() {
    if (!claudeCodexProSettings().conversationView) {
      cleanupConversationView();
      return;
    }
    ensureConversationViewRuntime();
    scheduleConversationViewAlign();
  }

  function scanLightweight() {
    installStyle();
    ensureMulticaWorkspaceRuntime();
    installCodexServiceTierDispatcherPatch();
    installClaudeCodexProMenu();
    scheduleBackendHeartbeat();
    installDeleteButtonEventDelegation();
    updateThreadScrollHandlers();
    installThreadScrollProgrammaticScrollGuard();
    installThreadScrollNavigationCapture();
    installThreadScrollUserIntentCapture();
    installThreadScrollRouteHooks();
    scheduleThreadScrollSync(true);
    refreshCodexServiceTierControls();
  }

  let zedRemoteStatusPromise = null;
  const zedRemoteMissingHostMessage = "Cannot determine remote SSH host for this file";

  function showZedRemoteToast(message) {
    document.querySelectorAll(`.${zedRemoteToastClass}`).forEach((node) => node.remove());
    const toast = document.createElement("div");
    toast.className = zedRemoteToastClass;
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 3200);
  }

  async function loadZedRemoteStatus() {
    zedRemoteStatusPromise = zedRemoteStatusPromise || postJson("/zed-remote/status", {});
    return zedRemoteStatusPromise;
  }

  async function resolveZedRemoteHost(hostId) {
    const result = await postJson("/zed-remote/resolve-host", { hostId });
    return result?.status === "ok" && result.ssh ? result.ssh : null;
  }

  function zedRemoteIsRemoteHostId(hostId) {
    return zedRemoteString(hostId).startsWith("remote-ssh-");
  }

  function zedRemoteProjectIdFromRow(row) {
    const projectList = row?.closest?.("[data-app-action-sidebar-project-list-id]");
    const projectId = zedRemoteString(projectList?.getAttribute?.("data-app-action-sidebar-project-list-id"));
    if (projectId) return projectId;
    const projectRow = row?.closest?.("[data-app-action-sidebar-project-id]");
    return zedRemoteString(projectRow?.getAttribute?.("data-app-action-sidebar-project-id"));
  }

  function zedRemoteWorkspaceRootFromObject(source) {
    if (!source || typeof source !== "object") return "";
    for (const key of ["remoteWorkspaceRoot", "workspaceRoot", "displayCwd", "cwd", "rootPath", "workingDirectory", "workingDir"]) {
      const workspaceRoot = zedRemoteString(source[key]);
      if (workspaceRoot.startsWith("/") && !/\/\.codex$/.test(workspaceRoot)) return workspaceRoot;
    }
    const hostConfig = source.hostConfig || source.sshHostConfig || source.remoteHostConfig || source.ssh || {};
    for (const key of ["remoteWorkspaceRoot", "workspaceRoot", "rootPath", "cwd"]) {
      const workspaceRoot = zedRemoteString(hostConfig[key]);
      if (workspaceRoot.startsWith("/") && !/\/\.codex$/.test(workspaceRoot)) return workspaceRoot;
    }
    return "";
  }

  function zedRemoteWorkspaceRootFromElement(element) {
    for (const key of zedRemoteReactKeys(element)) {
      const workspaceRoot = zedRemoteWalkObject(element[key], zedRemoteWorkspaceRootFromObject, { maxDepth: 10, maxNodes: 320 });
      if (workspaceRoot) return workspaceRoot;
    }
    return "";
  }

  function zedRemoteWorkspaceRootFromRow(row) {
    for (let node = row; node && node !== document.body; node = node.parentElement) {
      const workspaceRoot = zedRemoteWorkspaceRootFromElement(node);
      if (workspaceRoot) return workspaceRoot;
    }
    return "";
  }

  function zedRemoteActiveThreadRow() {
    const rows = sessionRows(true).filter((row) => row instanceof HTMLElement);
    return rows.find((row) => row.getAttribute("data-app-action-sidebar-thread-active") === "true")
      || rows.find((row) => row.getAttribute("aria-current") === "page" || row.getAttribute("aria-current") === "true")
      || null;
  }

  function zedRemoteCurrentFallbackPayload() {
    const row = zedRemoteActiveThreadRow();
    const ref = row ? sessionRefFromRow(row) : currentSessionRef();
    const threadId = ref.session_id || locationThreadId();
    const hostId = zedRemoteString(row?.getAttribute?.("data-app-action-sidebar-thread-host-id"));
    const isRemoteHost = zedRemoteIsRemoteHostId(hostId);
    const payload = {};
    if (threadId) payload.threadId = threadId;
    if (hostId && hostId !== "local") payload.hostId = hostId;
    if (!isRemoteHost) return payload;
    const remoteWorkspaceRoot = zedRemoteWorkspaceRootFromRow(row);
    const remoteProjectId = zedRemoteProjectIdFromRow(row);
    if (remoteWorkspaceRoot) payload.remoteWorkspaceRoot = remoteWorkspaceRoot;
    if (remoteProjectId) payload.remoteProjectId = remoteProjectId;
    return payload;
  }

  function zedRemoteCurrentThreadId() {
    return zedRemoteCurrentFallbackPayload().threadId || "";
  }

  async function resolveZedRemoteFallbackRequest() {
    const payload = zedRemoteCurrentFallbackPayload();
    if (!zedRemoteIsRemoteHostId(payload.hostId)) return null;
    const result = await postJson("/zed-remote/fallback-request", payload);
    return result?.status === "ok" && result.request ? result.request : null;
  }

  function zedRemoteOpenStrategy() {
    const strategy = zedRemoteString(claudeCodexProBackendSettings.zedRemoteOpenStrategy);
    return ["addToFocusedWorkspace", "reuseWindow", "newWindow", "default"].includes(strategy)
      ? strategy
      : "addToFocusedWorkspace";
  }

  function zedRemoteString(value) {
    return typeof value === "string" || typeof value === "number" ? String(value).trim() : "";
  }

  function zedRemoteTruthy(value) {
    if (value === true) return true;
    if (typeof value === "string") return /^(true|1|yes|enabled|ssh)$/i.test(value.trim());
    return false;
  }

  function zedRemoteHasTrustedSshSignal(source, hostConfig) {
    return zedRemoteTruthy(source?.supportsSsh) || zedRemoteTruthy(hostConfig?.supportsSsh);
  }

  function zedRemoteContextFromObject(source) {
    if (!source || typeof source !== "object") return null;
    const hostConfig = source.hostConfig || source.sshHostConfig || source.remoteHostConfig || source.ssh || {};
    const host = zedRemoteString(source.remoteHost || source.sshHost || source.host || source.hostname || source.hostName || hostConfig.host || hostConfig.hostname || hostConfig.hostName || hostConfig.sshHost);
    const hostId = zedRemoteString(source.hostId);
    const cwd = zedRemoteString(source.cwd || source.workspaceRoot || source.rootPath || source.remoteWorkspaceRoot || hostConfig.remoteWorkspaceRoot || hostConfig.workspaceRoot || hostConfig.rootPath);
    if ((!host || !zedRemoteHasTrustedSshSignal(source, hostConfig)) && !(hostId.startsWith("remote-ssh-") && cwd.startsWith("/"))) return null;
    const user = zedRemoteString(source.remoteUser || source.sshUser || source.user || source.username || hostConfig.user || hostConfig.username || hostConfig.sshUser);
    const port = zedRemoteString(source.remotePort || source.sshPort || source.port || hostConfig.port || hostConfig.sshPort);
    const workspaceRoot = cwd;
    return { hostId, ssh: { user, host, port }, workspaceRoot };
  }

  function zedRemoteWalkObject(root, visitor, options = {}) {
    const maxDepth = options.maxDepth || 6;
    const maxNodes = options.maxNodes || 180;
    const visited = new WeakSet();
    const stack = [{ value: root, depth: 0 }];
    let scanned = 0;
    while (stack.length && scanned < maxNodes) {
      const { value, depth } = stack.pop();
      if (!value || typeof value !== "object" || visited.has(value) || depth > maxDepth) continue;
      visited.add(value);
      scanned += 1;
      const result = visitor(value);
      if (result) return result;
      if (value instanceof Element || value === window || value === document || value === document.body || value === document.documentElement) continue;
      for (const key of Object.keys(value).slice(0, 80)) {
        if (key === "ownerDocument" || key === "parentElement" || key === "parentNode" || key === "children" || key === "childNodes") continue;
        let child;
        try {
          child = value[key];
        } catch {
          continue;
        }
        if (child && typeof child === "object") stack.push({ value: child, depth: depth + 1 });
      }
    }
    return null;
  }

  function zedRemoteReactKeys(element) {
    return Object.keys(element).filter((key) => key.startsWith("__reactFiber") || key.startsWith("__reactInternalInstance") || key.startsWith("__reactProps"));
  }

  function zedRemoteContextFromElement(element) {
    for (const key of zedRemoteReactKeys(element)) {
      const context = zedRemoteWalkObject(element[key], zedRemoteContextFromObject);
      if (context) return context;
    }
    return null;
  }

  function zedRemoteContextForElement(element) {
    for (let node = element; node && node !== document.body; node = node.parentElement) {
      const context = zedRemoteContextFromElement(node);
      if (context) return context;
    }
    return null;
  }

  function zedRemoteHostIdFromText(text) {
    const source = String(text || "");
    const match = source.match(/\bremote-ssh-[A-Za-z0-9:_-]+\b/);
    return match ? match[0] : "";
  }

  function zedRemoteWorkspaceRootForPath(path) {
    const source = String(path || "").trim();
    const projects = Array.from(document.querySelectorAll(selectors.sidebarThread))
      .map((row) => ({
        label: (row.textContent || "").replace(/\s+/g, " ").trim(),
        selected: row.getAttribute("aria-current") === "page" || row.getAttribute("data-selected") === "true" || row.getAttribute("data-active") === "true" || row.className.includes("selected"),
      }))
      .filter((row) => row.label);
    const selected = projects.find((row) => row.selected)?.label || "";
    for (const label of [selected, ...projects.map((row) => row.label)]) {
      const name = label.match(/^([A-Za-z0-9._-]+)/)?.[1];
      if (name && source.includes(`/repo/${name}/`)) return source.slice(0, source.indexOf(`/repo/${name}/`) + `/repo/${name}`.length);
    }
    const repoIndex = source.indexOf("/bin/repo/");
    if (repoIndex >= 0) {
      const afterRepo = source.slice(repoIndex + "/bin/repo/".length);
      const project = afterRepo.split("/")[0];
      if (project) return source.slice(0, repoIndex + "/bin/repo/".length + project.length);
    }
    return source;
  }

  function zedRemoteFallbackContextForElement(element) {
    const pathText = (element.textContent || "").trim();
    if (!pathText.startsWith("/")) return null;
    const root = element.closest("main") || document.body;
    const hostId = zedRemoteHostIdFromText(root?.textContent || "") || "remote-ssh-codex-managed:remote";
    return { hostId, ssh: { user: "", host: "", port: "" }, workspaceRoot: zedRemoteWorkspaceRootForPath(pathText) };
  }

  function zedRemoteContextFromSerializedState(text) {
    const source = String(text || "");
    if (!source.includes("hostConfig") || !source.includes("supportsSsh") || !source.includes("remoteWorkspaceRoot")) return null;
    const trimmed = source.trim();
    if (/^[{[]/.test(trimmed)) {
      try {
        const parsed = JSON.parse(trimmed);
        const context = zedRemoteWalkObject(parsed, zedRemoteContextFromObject, { maxDepth: 10, maxNodes: 300 });
        if (context) return context;
      } catch {
      }
    }
    if (!/['"]supportsSsh['"]\s*:\s*true/.test(source)) return null;
    const fieldValue = (name) => {
      const match = source.match(new RegExp(`["']${name}["']\\s*:\\s*["']([^"']+)["']`));
      return match ? match[1] : "";
    };
    const host = fieldValue("host") || fieldValue("hostname") || fieldValue("hostName") || fieldValue("sshHost") || fieldValue("remoteHost");
    if (!host) return null;
    return {
      ssh: {
        user: fieldValue("user") || fieldValue("username") || fieldValue("sshUser") || fieldValue("remoteUser"),
        host,
        port: fieldValue("port") || fieldValue("sshPort") || fieldValue("remotePort"),
      },
      workspaceRoot: fieldValue("remoteWorkspaceRoot") || fieldValue("workspaceRoot") || fieldValue("rootPath"),
    };
  }

  const zedRemoteContextCacheTtlMs = 1200;
  let zedRemoteContextCache = { scope: null, at: 0, value: null };

  function zedRemoteScopedElements(scope, selector) {
    const root = scope?.querySelectorAll ? scope : document;
    const nodes = [];
    if (scope instanceof HTMLElement && scope.matches?.(selector)) nodes.push(scope);
    root.querySelectorAll?.(selector).forEach((node) => nodes.push(node));
    return Array.from(new Set(nodes));
  }

  function zedRemoteContextFromDataset(node) {
    if (!(node instanceof HTMLElement)) return null;
    const data = node.dataset;
    return zedRemoteContextFromObject({
      hostConfig: data.hostConfig ? { host: data.hostConfig, supportsSsh: true } : {},
      supportsSsh: data.supportsSsh || data.supportsSshRemote,
      sshHost: data.sshHost,
      remoteHost: data.remoteHost,
      host: data.host,
      sshUser: data.sshUser,
      remoteUser: data.remoteUser,
      user: data.user,
      sshPort: data.sshPort,
      remotePort: data.remotePort,
      port: data.port,
      remoteWorkspaceRoot: data.remoteWorkspaceRoot,
      workspaceRoot: data.workspaceRoot,
    });
  }

  function zedRemoteContextUncached(scope = document) {
    const explicitSelector = "[data-host-config], [data-ssh-host], [data-remote-host], [data-remote-workspace-root], [data-supports-ssh]";
    for (const node of zedRemoteScopedElements(scope, explicitSelector)) {
      if (isExtensionUiNode(node)) continue;
      const context = zedRemoteContextFromDataset(node);
      if (context) return context;
    }
    const reactSelector = "[data-remote-path], [data-file-path], [data-path], [data-open-in-targets], [data-open-file], [data-codex-open-file], [role='menuitem']";
    const reactNodes = zedRemoteScopedElements(scope, reactSelector);
    if (scope instanceof HTMLElement && !isExtensionUiNode(scope)) reactNodes.unshift(scope);
    for (const node of Array.from(new Set(reactNodes)).slice(0, 60)) {
      if (!(node instanceof HTMLElement) || isExtensionUiNode(node)) continue;
      const context = zedRemoteContextFromElement(node);
      if (context) return context;
    }
    if (scope !== document) return null;
    const scripts = Array.from(document.querySelectorAll("script[type='application/json'], script[data-state], script#__NEXT_DATA__, script:not([src])"));
    for (const script of scripts.slice(0, 20)) {
      const context = zedRemoteContextFromSerializedState(script.textContent || "");
      if (context) return context;
    }
    return null;
  }

  function zedRemoteContext(scope = document) {
    const settings = claudeCodexProSettings();
    if (!settings.zedRemoteOpen) return null;
    const now = Date.now();
    if (zedRemoteContextCache.scope === scope && now - zedRemoteContextCache.at < zedRemoteContextCacheTtlMs) {
      return zedRemoteContextCache.value;
    }
    const value = zedRemoteContextUncached(scope);
    zedRemoteContextCache = { scope, at: now, value };
    return value;
  }

  function zedRemoteAbsolutePath(value, workspaceRoot) {
    const text = String(value || "").trim();
    if (!text) return "";
    if (text.startsWith("/")) return text;
    if (workspaceRoot && !text.includes("://") && !text.startsWith("~")) {
      return `${workspaceRoot.replace(/\/+$/, "")}/${text.replace(/^\.\//, "")}`;
    }
    return "";
  }

  function zedRemoteMetadataRemotePath(source) {
    if (!source || typeof source !== "object") return "";
    return zedRemoteString(source.remotePath || source.remote_path || source.path || source.filePath || source.file_path || source.openFile?.remotePath || source.openFile?.path);
  }

  function zedRemotePathFromElementMetadata(element) {
    const dataPath = element.dataset.remotePath || element.dataset.filePath || element.dataset.path || "";
    if (dataPath) return dataPath;
    for (const key of zedRemoteReactKeys(element)) {
      const path = zedRemoteWalkObject(element[key], zedRemoteMetadataRemotePath, { maxDepth: 6, maxNodes: 120 });
      if (path) return path;
    }
    return "";
  }

  function zedRemoteInlinePathFromElement(element, context) {
    if (!context?.hostId && !context?.ssh?.host) return "";
    const text = (element.textContent || "").trim();
    if (!text || text.length > 600 || !text.startsWith("/")) return "";
    const path = zedRemoteAbsolutePath(text, context.workspaceRoot || "");
    if (!path) return "";
    if (context.workspaceRoot && !path.startsWith(`${context.workspaceRoot.replace(/\/+$/, "")}/`) && path !== context.workspaceRoot) return "";
    return path;
  }

  function zedRemoteAnchorHasOpenFileMetadata(anchor) {
    if (!(anchor instanceof HTMLAnchorElement)) return false;
    if (anchor.dataset.remotePath || anchor.dataset.filePath || anchor.dataset.path || anchor.dataset.openInTargets || anchor.dataset.openFile || anchor.dataset.codexOpenFile) return true;
    const label = `${anchor.getAttribute("aria-label") || ""} ${anchor.getAttribute("data-testid") || ""} ${anchor.getAttribute("rel") || ""}`;
    return /open[-_\s]?file|open-in-targets|remote/i.test(label) && !!zedRemotePathFromElementMetadata(anchor);
  }

  function zedRemoteFileCandidates(context, scope = document) {
    const candidates = [];
    const seen = new Set();
    const addCandidate = (node, candidateContext, rawPath) => {
      if (!candidateContext?.ssh?.host && !candidateContext?.hostId) return;
      const path = zedRemoteAbsolutePath(rawPath, candidateContext.workspaceRoot || "");
      if (!path || seen.has(path)) return;
      seen.add(path);
      candidates.push({ node, request: { ssh: candidateContext.ssh, hostId: candidateContext.hostId || "", path } });
    };
    const selectors = "[data-remote-path], [data-file-path], [data-path], [data-open-in-targets], [data-open-file], [data-codex-open-file], a[data-remote-path], a[data-file-path], a[data-path]";
    zedRemoteScopedElements(scope, selectors).forEach((node) => {
      if (!(node instanceof HTMLElement) || isExtensionUiNode(node)) return;
      if (node instanceof HTMLAnchorElement && !zedRemoteAnchorHasOpenFileMetadata(node)) return;
      addCandidate(node, zedRemoteContextForElement(node) || context, zedRemotePathFromElementMetadata(node));
    });
    if (scope !== document) {
      zedRemoteScopedElements(scope, "span.inline-markdown, code, [class*='inlineMarkdown']").forEach((node) => {
        if (!(node instanceof HTMLElement) || isExtensionUiNode(node)) return;
        const candidateContext = zedRemoteContextForElement(node) || context || zedRemoteFallbackContextForElement(node);
        if (!candidateContext?.hostId && !candidateContext?.ssh?.host) return;
        const path = zedRemoteInlinePathFromElement(node, candidateContext);
        if (path) addCandidate(node, candidateContext, path);
      });
    }
    return candidates;
  }

  function zedRemoteBestOpenRequest(scope = document, context = zedRemoteContext(scope) || zedRemoteContext(document) || {}) {
    const candidates = zedRemoteFileCandidates(context, scope);
    if (candidates.length) return candidates[0].request;
    return null;
  }

  async function openZedRemote(request) {
    let nextRequest = request;
    if (!nextRequest?.ssh?.host && nextRequest?.hostId) {
      const ssh = await resolveZedRemoteHost(nextRequest.hostId);
      nextRequest = ssh ? { ...nextRequest, ssh } : nextRequest;
    }
    if (!nextRequest?.ssh?.host) {
      showZedRemoteToast(zedRemoteMissingHostMessage);
      return;
    }
    nextRequest = {
      ...nextRequest,
      strategy: nextRequest.strategy || zedRemoteOpenStrategy(),
      remember: claudeCodexProBackendSettings.zedRemoteProjectRegistryEnabled !== false,
    };
    try {
      const result = await postJson("/zed-remote/open", nextRequest);
      if (result?.status === "ok") {
        showZedRemoteToast("Opened in Zed Remote");
        return;
      }
      showZedRemoteToast(result?.message || "Cannot open this file in Zed Remote");
    } catch (error) {
      showZedRemoteToast(error?.message || "Cannot open this file in Zed Remote");
    }
  }

  async function openBestZedRemoteTarget() {
    const request = zedRemoteBestOpenRequest(document) || await resolveZedRemoteFallbackRequest();
    if (!request) {
      showZedRemoteToast("Cannot find a remote workspace or file for Zed");
      return;
    }
    openZedRemote(request);
  }

  function attachZedRemoteButton(candidate) {
    const anchor = candidate.node;
    if (anchor.dataset.codexZedRemoteVersion === zedRemoteOpenVersion) return;
    anchor.dataset.codexZedRemoteVersion = zedRemoteOpenVersion;
    const button = document.createElement("button");
    button.type = "button";
    button.className = zedRemoteButtonClass;
    button.textContent = "Open in Zed Remote";
    button.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      openZedRemote(candidate.request);
    }, true);
    anchor.insertAdjacentElement("afterend", button);
  }

  function removeZedRemoteButtons() {
    document.querySelectorAll(`[data-codex-zed-remote-version]`).forEach((node) => {
      delete node.dataset.codexZedRemoteVersion;
    });
    document.querySelectorAll(`.${zedRemoteButtonClass}`).forEach((node) => node.remove());
  }

  function createZedRemoteOpenInMenuItem(referenceItem) {
    const item = document.createElement("div");
    item.className = referenceItem?.className || "no-drag text-token-foreground outline-hidden rounded-lg px-[var(--padding-row-x)] py-[var(--padding-row-y)] text-sm group hover:bg-token-list-hover-background focus:bg-token-list-hover-background cursor-interaction flex flex-col";
    item.classList.add(zedRemoteOpenInMenuItemClass);
    item.setAttribute("role", referenceItem?.getAttribute("role") || "menuitem");
    item.setAttribute("tabindex", referenceItem?.getAttribute("tabindex") || "-1");
    item.setAttribute("data-orientation", referenceItem?.getAttribute("data-orientation") || "vertical");
    item.innerHTML = `
      <div class="flex w-full items-center gap-1.5">
        <span class="inline-flex size-[18px] items-center justify-center leading-none shrink-0 opacity-75 group-focus:opacity-100 group-hover:opacity-100">
          <img alt="" class="codex-zed-open-in-menu-icon icon-sm" src="apps/zed.png">
        </span>
        <span class="flex-1 min-w-0 truncate">Zed</span>
      </div>
    `;
    bindZedRemoteOpenInMenuItem(item, "injected");
    return item;
  }

  function zedRemoteOpenInMenuActivationIsDuplicate(target) {
    if (!(target instanceof HTMLElement)) return false;
    const now = Date.now();
    const activatedAt = Number(target.dataset.codexZedOpenInMenuActivatedAt || 0);
    if (activatedAt && now - activatedAt < zedRemoteOpenInMenuActivationWindowMs) return true;
    target.dataset.codexZedOpenInMenuActivatedAt = String(now);
    return false;
  }

  async function activateZedRemoteOpenInMenuItem(event) {
    if (!claudeCodexProSettings().zedRemoteOpen) return;
    if (event?.type === "keydown" && !["Enter", " "].includes(event.key)) return;
    const scope = event?.currentTarget?.closest?.('[role="menu"], [data-radix-popper-content-wrapper]') || event?.currentTarget || document;
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation?.();
    if (zedRemoteOpenInMenuActivationIsDuplicate(event?.currentTarget)) return;
    const request = zedRemoteBestOpenRequest(scope) || await resolveZedRemoteFallbackRequest();
    if (!request) {
      showZedRemoteToast("Cannot find a remote workspace or file for Zed");
      return;
    }
    openZedRemote(request);
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
  }

  function bindZedRemoteOpenInMenuItem(item, source) {
    item.setAttribute("data-codex-zed-open-in-menu", source);
    if (item.dataset.codexZedOpenInMenuBound === zedRemoteOpenInMenuVersion) return;
    item.dataset.codexZedOpenInMenuBound = zedRemoteOpenInMenuVersion;
    item.dataset.codexZedOpenInMenuVersion = zedRemoteOpenInMenuVersion;
    item.addEventListener("pointerup", activateZedRemoteOpenInMenuItem, true);
    item.addEventListener("click", activateZedRemoteOpenInMenuItem, true);
    item.addEventListener("keydown", activateZedRemoteOpenInMenuItem, true);
  }

  function removeZedRemoteOpenInMenuItems(scope = document) {
    const root = scope?.querySelectorAll ? scope : document;
    root.querySelectorAll(`.${zedRemoteOpenInMenuItemClass}, [data-codex-zed-open-in-menu="injected"]`).forEach((node) => node.remove());
  }

  function zedRemoteOpenInMenuScopes(scope = document) {
    const root = scope?.querySelectorAll ? scope : document;
    const menus = [];
    if (scope instanceof HTMLElement && scope.matches?.('[role="menu"]')) menus.push(scope);
    root.querySelectorAll?.('[role="menu"]').forEach((menu) => menus.push(menu));
    return Array.from(new Set(menus));
  }

  function refreshZedRemoteOpenInMenus(scope = document) {
    removeZedRemoteOpenInMenuItems(scope);
    if (!claudeCodexProSettings().zedRemoteOpen) return;
    const fallbackPayload = zedRemoteCurrentFallbackPayload();
    zedRemoteOpenInMenuScopes(scope).forEach((menu) => {
      if (!(menu instanceof HTMLElement) || isExtensionUiNode(menu)) return;
      const items = Array.from(menu.querySelectorAll('[role="menuitem"]')).filter((item) => !isExtensionUiNode(item));
      const menuText = items.map((item) => (item.textContent || "").trim()).join(" ");
      if (!/\b(VS Code|Cursor|Antigravity)\b/.test(menuText)) return;
      if (!zedRemoteBestOpenRequest(menu) && !zedRemoteIsRemoteHostId(fallbackPayload.hostId)) return;
      const existingZedItem = items.find((item) => (item.textContent || "").trim() === "Zed");
      if (existingZedItem) {
        bindZedRemoteOpenInMenuItem(existingZedItem, "native");
        return;
      }
      const referenceItem = items.find((item) => /^(VS Code|Cursor|Antigravity)$/.test((item.textContent || "").trim()));
      if (!referenceItem) return;
      referenceItem.parentElement?.appendChild(createZedRemoteOpenInMenuItem(referenceItem));
    });
  }

  async function refreshZedRemoteOpenControls(scope = document) {
    if (!claudeCodexProSettings().zedRemoteOpen) {
      removeZedRemoteButtons();
      removeZedRemoteOpenInMenuItems();
      return;
    }
    try {
      const status = await loadZedRemoteStatus();
      if (!status?.platformSupported || (!status.zedAppFound && !status.zedCliFound)) {
        removeZedRemoteButtons();
        removeZedRemoteOpenInMenuItems();
        return;
      }
    } catch (_) {
      removeZedRemoteButtons();
      removeZedRemoteOpenInMenuItems();
      return;
    }
    refreshZedRemoteOpenInMenus(scope);
  }

  function runScheduledZedRemoteMenuRefresh() {
    window.__codexZedRemoteMenuRefreshPending = false;
    clearTimeout(window.__codexZedRemoteMenuRefreshTimer);
    window.__codexZedRemoteMenuRefreshTimer = null;
    refreshZedRemoteOpenControls().catch(() => {
      removeZedRemoteOpenInMenuItems();
    });
  }

  function shouldRefreshZedRemoteMenus(mutations) {
    if (!claudeCodexProSettings().zedRemoteOpen) return false;
    if (!mutations) return true;
    return mutations.some((mutation) => {
      const target = mutation.target;
      if (isExtensionUiNode(target)) return false;
      if (target?.nodeType === 1 && target.matches?.('[role="menu"], [data-radix-popper-content-wrapper]')) return true;
      return [...Array.from(mutation.addedNodes), ...Array.from(mutation.removedNodes)].some((node) => node.nodeType === 1 && (
        node.matches?.('[role="menu"], [data-radix-popper-content-wrapper]') ||
        node.querySelector?.('[role="menu"], [data-radix-popper-content-wrapper]')
      ));
    });
  }

  function scheduleZedRemoteMenuRefresh(mutations) {
    if (!shouldRefreshZedRemoteMenus(mutations)) return;
    if (window.__codexZedRemoteMenuRefreshPending) return;
    window.__codexZedRemoteMenuRefreshPending = true;
    window.__codexZedRemoteMenuRefreshTimer = setTimeout(runScheduledZedRemoteMenuRefresh, 50);
  }

  function scanDeferred() {
    if (pluginPatchDisabledInRelayMode()) {
      clearPluginPatchArtifacts();
      refreshForcePluginInstallUnlockLoop();
    } else {
      const pluginUnlockStrategy = codexPluginUnlockStrategy();
      const settings = claudeCodexProSettings();
      logCodexPluginUnlockStrategy(pluginUnlockStrategy);
      if ((pluginUnlockStrategy === "legacy" || pluginUnlockStrategy === "unknown") && settings.pluginEntryUnlock) {
        enablePluginEntry();
      }
      if ((pluginUnlockStrategy === "modern" || pluginUnlockStrategy === "unknown") && settings.pluginMarketplaceUnlock) {
        const marketplaceRequestPatchStrategy = codexPluginMarketplaceRequestPatchStrategy();
        installPluginBuildFlavorFilterPatch();
        if (marketplaceRequestPatchStrategy === "bridge") {
          installPluginMarketplaceBridgePatch();
        } else if (marketplaceRequestPatchStrategy === "client") {
          installPluginMarketplaceRequestPatch();
        } else {
          installPluginMarketplaceWindowEventPatchOnly();
          installPluginMarketplaceBridgePatch();
          installPluginMarketplaceRequestPatch();
        }
      }
      unblockPluginInstallButtons();
      refreshForcePluginInstallUnlockLoop();
    }
    sessionRows().forEach(tryAttachButton);
    syncActionGroupsLayout();
    updateDeleteButtonOffsets();
    scheduleProjectMoveProjection();
    scheduleChatsSortCorrection();
    archivedPageRows().forEach(attachArchivedPageDeleteButton);
    refreshConversationTimeline();
    refreshConversationView();
    installCodexServiceTierBadge();
    codexMemoryUpdateBadge();
    void codexMemoryLoadSession();
    void codexMemoryMaybeSuggestCandidate();
    scheduleThreadScrollSync();
  }

  function runScanStep(step) {
    try {
      step();
    } catch (error) {
      window.__codexSessionDeleteScanFailures = window.__codexSessionDeleteScanFailures || [];
      window.__codexSessionDeleteScanFailures.push(String(error?.stack || error));
    }
  }

  function scan() {
    runScanStep(scanLightweight);
    requestAnimationFrame(() => runScanStep(scanDeferred));
  }

  const codexMemoryState = {
    status: "loading",
    workspace: "codex",
    totalItems: 0,
    pendingCandidates: 0,
    injectedItems: [],
    injectSummaryCachePath: "",
    summary: "正在读取盘古记忆…",
    lastLoadedAt: 0,
    lastSuggestionHash: "",
    lastSuggestionAt: 0,
    lastAutoSuggestDiagnosticHash: "",
    lastAutoSuggestDiagnosticAt: 0,
    activeUntil: 0,
    activeSource: "idle",
  };
  let codexMemoryLastHeartbeatAt = 0;

  function codexMemoryHeartbeat(force = false) {
    const now = Date.now();
    if (!force && now - codexMemoryLastHeartbeatAt < 10000) return;
    codexMemoryLastHeartbeatAt = now;
    sendClaudeCodexProDiagnostic("memory_runtime", {
      runtime: window.__claudeCodexProMemoryAssistRuntime || null,
    });
  }

  if (window.__claudeCodexProMemoryHeartbeatTimer) {
    clearInterval(window.__claudeCodexProMemoryHeartbeatTimer);
  }
  window.__claudeCodexProMemoryHeartbeatTimer = window.setInterval(() => {
    try {
      codexMemoryExposeRuntime();
    } catch (_) {}
  }, 10000);

  function codexMemoryPulseActivity(source = "stream", durationMs = 3200) {
    codexMemoryState.activeUntil = Date.now() + durationMs;
    codexMemoryState.activeSource = source || "stream";
    codexMemoryExposeRuntime();
    codexMemoryHeartbeat(true);
    codexMemoryUpdateBadge();
  }

  function codexMemoryExposeRuntime() {
    const settings = claudeCodexProSettings();
    const active = Date.now() < Number(codexMemoryState.activeUntil || 0);
    window.__claudeCodexProMemoryAssistRuntime = {
      enabled: !!settings.memoryAssistEnabled,
      injected: !!settings.memoryAssistEnabled && !!settings.memoryAssistInjectEnabled,
      status: codexMemoryState.status,
      active,
      workspace: codexMemoryState.workspace,
      totalItems: Number(codexMemoryState.totalItems || 0),
      pendingCandidates: Number(codexMemoryState.pendingCandidates || 0),
      injectSummaryCachePath: codexMemoryState.injectSummaryCachePath || "",
      summary: codexMemoryState.summary || "",
      source: active ? (codexMemoryState.activeSource || "stream") : "idle",
    };
    codexMemoryHeartbeat();
  }

  function codexMemoryWorkspace() {
    const project = currentProjectContext?.();
    if (project?.repoPath || project?.projectId) {
      const remembered = rememberCodexMemoryProjectContext(project);
      if (remembered?.repoPath) return `codex:repo:${remembered.repoPath}`;
      if (remembered?.projectId) return `codex:project:${remembered.projectId}`;
    }
    const cachedProject = readCodexMemoryProjectContext();
    if (cachedProject?.repoPath) return `codex:repo:${cachedProject.repoPath}`;
    if (cachedProject?.projectId) return `codex:project:${cachedProject.projectId}`;
    const pathParts = String(location.pathname || "").split("/").filter(Boolean);
    const thread = pathParts.find((part) => /^[a-z0-9][a-z0-9_-]{7,}$/i.test(part)) || "";
    if (thread) return `codex:thread:${thread.slice(0, 80)}`;
    const pathKey = `${location.origin || ""}${location.pathname || ""}`;
    if (pathKey.trim()) return `codex:path:${codexMemoryHash(pathKey).slice(0, 16)}`;
    return "codex";
  }

  function codexMemoryWorkspaceIsPathFallback(workspace) {
    return /^codex:path:/i.test(String(workspace || ""));
  }

  function codexMemoryVisibleThreadTitle() {
    const candidates = [
      document.querySelector('[data-testid="thread-title"]'),
      document.querySelector('[data-thread-title]'),
      document.querySelector("main h1"),
      document.querySelector("header h1"),
      document.querySelector('[role="main"] h1'),
    ].filter(Boolean);
    for (const node of candidates) {
      const text = normalizedElementText(node);
      if (text) return text;
    }
    return String(document.title || "").replace(/\s+-\s+Codex.*$/i, "").trim();
  }

  function codexMemoryVisibleProjectLabel() {
    const project = currentProjectContext?.();
    if (project?.label) return project.label;
    const rows = [...document.querySelectorAll("aside *, nav *")]
      .filter((node) => node instanceof HTMLElement && visibleElement(node))
      .map((node) => normalizedElementText(node))
      .filter((text) => text && text.length <= 80);
    const title = codexMemoryVisibleThreadTitle();
    const titleIndex = rows.findIndex((text) => title && (text === title || title.includes(text) || text.includes(title)));
    if (titleIndex > 0) {
      for (let index = titleIndex - 1; index >= 0; index -= 1) {
        const text = rows[index];
        if (/^(项目|搜索|新对话|已安排|插件|设置|账户)$/i.test(text)) continue;
        if (/^\d+\s*(分钟|小时|天|周|月|年|min|hour|day|week)/i.test(text)) continue;
        if (text !== title) return text;
      }
    }
    const expanded = visibleProjectRows?.().find((row) => row.getAttribute("data-app-action-sidebar-project-collapsed") === "false");
    return normalizeProjectLabel(expanded?.getAttribute?.("data-app-action-sidebar-project-label") || normalizedElementText(expanded));
  }

  async function codexMemoryResolvedWorkspace() {
    const workspace = codexMemoryWorkspace();
    if (!codexMemoryWorkspaceIsPathFallback(workspace)) {
      codexMemoryState.workspace = workspace;
      return workspace;
    }
    try {
      const result = await postJson("/memory/resolve-workspace", {
        workspace,
        url: location.href,
        title: document.title || "",
        threadTitle: codexMemoryVisibleThreadTitle(),
        projectLabel: codexMemoryVisibleProjectLabel(),
      });
      const resolved = String(result?.workspace || "").trim();
      if (result?.status === "ok" && result?.resolved && resolved && !codexMemoryWorkspaceIsPathFallback(resolved)) {
        const context = resolved.startsWith("codex:repo:")
          ? { repoPath: resolved.slice("codex:repo:".length), projectId: "", label: "", at: Date.now() }
          : { repoPath: resolved, projectId: "", label: "", at: Date.now() };
        rememberCodexMemoryProjectContext(context);
        codexMemoryState.workspace = resolved.startsWith("codex:") ? resolved : `codex:repo:${resolved}`;
        return codexMemoryState.workspace;
      }
    } catch (error) {
      codexMemoryAutoSuggestDiagnostic("workspace_resolve_failed", {
        message: String(error?.message || error).slice(0, 240),
      });
    }
    codexMemoryState.workspace = workspace;
    return workspace;
  }

  function codexMemoryConversationRoot() {
    return document.querySelector(".thread-scroll-container")
      || document.querySelector('[data-testid="conversation"]')
      || document.querySelector('main [data-testid="conversation-turn"]')?.closest("main")
      || document.querySelector('[role="main"] [data-message-author-role]')?.closest('[role="main"]')
      || document.querySelector("main");
  }

  function codexMemoryNodeIsInsideConversation(node) {
    if (!node || isExtensionUiNode(node)) return false;
    if (node.closest?.('[data-app-action-sidebar-thread-id], [data-app-action-sidebar-section-heading], nav, aside, header, [role="navigation"], [aria-label*="sidebar" i], [aria-label*="侧边" i]')) return false;
    const root = codexMemoryConversationRoot();
    return !!root && root.contains(node);
  }

  function codexMemoryMessageTarget(node) {
    return node?.closest?.('[data-testid="conversation-turn"]') || node;
  }

  function codexMemoryUserMessageCandidates(root) {
    if (!root) return [];
    const explicitCandidates = Array.from(root.querySelectorAll([
      '[data-message-author-role="user"]',
      '[data-testid="conversation-turn"][data-message-author-role="user"]',
      '[data-testid="conversation-turn"] [data-message-author-role="user"]',
      '[class*="user-message"]',
      '[class*="UserMessage"]',
    ].join(", ")));
    const codexUserBubbles = Array.from(root.querySelectorAll(".group.flex.w-full.flex-col.items-end.justify-end.gap-1")).flatMap((group) => {
      return Array.from(group.children).filter((child) => nodeOrAncestorLooksLikeCodexUserBubble(child));
    });
    return [...explicitCandidates, ...codexUserBubbles];
  }

  function codexMemoryAssistantMessageCandidates(root) {
    if (!root) return [];
    return Array.from(root.querySelectorAll([
      '[data-message-author-role="assistant"]',
      '[data-testid="conversation-turn"][data-message-author-role="assistant"]',
      '[data-testid="conversation-turn"] [data-message-author-role="assistant"]',
    ].join(", ")));
  }

  function codexMemoryOrderedMessageCandidates(root, role = "") {
    const candidates = role === "user"
      ? codexMemoryUserMessageCandidates(root)
      : role === "assistant"
        ? codexMemoryAssistantMessageCandidates(root)
        : [...codexMemoryUserMessageCandidates(root), ...codexMemoryAssistantMessageCandidates(root)];
    return candidates.sort((left, right) => {
      if (left === right) return 0;
      const position = left.compareDocumentPosition?.(right) || 0;
      if (position & Node.DOCUMENT_POSITION_PRECEDING) return 1;
      if (position & Node.DOCUMENT_POSITION_FOLLOWING) return -1;
      return 0;
    });
  }

  function codexMemoryMessageText(node) {
    const textNode = node.querySelector?.(".prose, [data-message-content], [data-testid='message-content']") || node;
    const clone = textNode.cloneNode?.(true);
    if (clone?.querySelectorAll) {
      clone.querySelectorAll("button, svg, [aria-hidden='true'], .sr-only, textarea, input").forEach((child) => child.remove());
    }
    return codexMemoryNormalizeMessageText((clone?.textContent || textNode.textContent || ""));
  }

  function codexMemoryNormalizeMessageText(text) {
    return String(text || "")
      .replace(/\s+/g, " ")
      .replace(/^(user|assistant|codex|你|我|用户|助手)\s*[:：]\s*/i, "")
      .trim();
  }

  function codexMemoryConversationMessages(role = "") {
    const root = codexMemoryConversationRoot();
    if (!root) return [];
    const seen = new Set();
    return codexMemoryOrderedMessageCandidates(root, role)
      .filter((node) => codexMemoryNodeIsInsideConversation(node))
      .map((node) => {
        const target = codexMemoryMessageTarget(node);
        const text = codexMemoryMessageText(node);
        const key = `${role || target.getAttribute?.("data-message-author-role") || ""}:${text}`;
        if (!text || seen.has(key)) return "";
        seen.add(key);
        return text;
      })
      .filter((text) => text.length >= 2 && text.length <= 4000);
  }

  function codexMemoryCurrentText() {
    const selection = String(window.getSelection?.() || "").trim();
    if (selection && document.activeElement?.closest?.("main, [role='main'], .thread-scroll-container")) return selection;
    const composer = document.querySelector("textarea, [contenteditable='true']");
    const composerText = composer?.value || composer?.textContent || "";
    if (String(composerText).trim()) return String(composerText).trim();
    const messages = codexMemoryConversationMessages();
    return messages.slice(-2).join("\n\n").trim();
  }

  function codexMemoryLatestUserText() {
    const texts = codexMemoryConversationMessages("user")
      .filter((text) => text.length >= 8 && text.length <= 2400);
    return texts[texts.length - 1] || "";
  }

  function codexMemorySuggestionFromText(rawText) {
    const text = String(rawText || "").replace(/\s+/g, " ").trim();
    if (!text || text.length < 8) return null;
    if (codexMemoryLooksLikeChatter(text) || codexMemoryLooksLikeTitleOnly(text)) return null;
    const patterns = [
      { re: /(?:\u76d8\u53e4\u8bb0\u5fc6|\u8bb0\u5fc6).*(?:\u662f\u5426|\u6709\u6ca1\u6709|\u6ca1\u6709|\u539f\u56e0|\u4fee\u590d|\u8bb0\u5f55|\u5019\u9009|\u76d1\u542c|\u5bf9\u8bdd|\u4f1a\u8bdd)|(?:\u8fd9\u6761\u5bf9\u8bdd|\u5f53\u524d\u5bf9\u8bdd|\u672c\u6761\u5bf9\u8bdd).*(?:\u8bb0\u5fc6|\u8bb0\u5f55|\u5019\u9009|\u76d8\u53e4)|(?:pangu|memory).*(?:candidate|record|remember|debug|fix|session)/i, reason: "memory self-check phrase" },
      { re: /(?:帮我|请|以后)?记住[:：]?\s*(.+)$/i, reason: "explicit remember phrase" },
      { re: /(?:以后都这样|以后按这个|以后统一|以后默认)[:：]?\s*(.+)$/i, reason: "future preference phrase" },
      { re: /(?:这个项目约定|项目约定|仓库约定|本项目约定)[:：]?\s*(.+)$/i, reason: "project convention phrase" },
      { re: /(?:以后.*(?:先|必须|不要|不能|需要).*)$/i, reason: "future rule phrase" },
      { re: /(?:我(?:喜欢|偏好|习惯)|我的(?:偏好|习惯)|默认用|统一用|优先用)[:：]?\s*(.+)$/i, reason: "user preference phrase" },
      { re: /(?:这个项目|本项目|当前项目|这个仓库|本仓库).*(?:必须|不要|不能|需要|保持|禁止|默认|统一|优先|遵守|保留|删除|改成|修复).*/i, reason: "project requirement phrase" },
      { re: /(?:注意|记得|以后注意)[:：]?\s*(?:要|必须|不要|不能|需要|保持|保留|避免|先|优先).*/i, reason: "attention rule phrase" },
      { re: /(?:UI|界面|前端|布局|样式|主题|按钮|开关|卡片|页面).*(?:改成|保持|删除|不要|不能|需要|对齐|一致|修复).*/i, reason: "ui workflow requirement" },
      { re: /(?:构建|测试|验证|提交|仓库|插件|skill|mcp|codex|claude).*(?:必须|不要|不能|需要|保持|默认|自动|修复|删除|改成).*/i, reason: "workflow requirement" },
    ];
    for (const pattern of patterns) {
      const match = text.match(pattern.re);
      const candidate = (match?.[1] || match?.[0] || "").trim();
      if (candidate.length >= 6) {
        return {
          text: candidate.slice(0, 2000),
          reason: pattern.reason,
        };
      }
    }
    if (codexMemoryLooksLearnableText(text)) {
      return {
        text: text.slice(0, 2000),
        reason: "learnable user instruction",
      };
    }
    return null;
  }

  function codexMemoryLooksLearnableText(text) {
    const normalized = String(text || "").replace(/\s+/g, " ").trim();
    if (normalized.length < 16 || normalized.length > 2400) return false;
    if (codexMemoryLooksMemorySelfCheckText(normalized)) return true;
    const ruleWords = /(?:必须|不要|不能|需要|保持|保留|删除|改成|修复|默认|统一|优先|禁止|避免|先|always|never|must|should|prefer|default|keep|remove|fix)/i;
    if (!ruleWords.test(normalized)) return false;
    const contextWords = /(?:这个项目|本项目|当前项目|这个仓库|本仓库|UI|界面|前端|布局|样式|主题|按钮|开关|卡片|页面|构建|测试|验证|提交|仓库|插件|skill|mcp|codex|claude|manager|workflow)/i;
    const userPreference = /(?:我(?:喜欢|偏好|习惯)|我的(?:偏好|习惯)|按我|给我|以后|注意|记得)/i;
    return contextWords.test(normalized) || userPreference.test(normalized);
  }

  function codexMemoryLooksMemorySelfCheckText(text) {
    const normalized = String(text || "").replace(/\s+/g, " ").trim();
    return /(?:\u76d8\u53e4\u8bb0\u5fc6|\u8bb0\u5fc6).*(?:\u662f\u5426|\u6709\u6ca1\u6709|\u6ca1\u6709|\u539f\u56e0|\u4fee\u590d|\u8bb0\u5f55|\u5019\u9009|\u76d1\u542c|\u5bf9\u8bdd|\u4f1a\u8bdd)|(?:\u8fd9\u6761\u5bf9\u8bdd|\u5f53\u524d\u5bf9\u8bdd|\u672c\u6761\u5bf9\u8bdd).*(?:\u8bb0\u5fc6|\u8bb0\u5f55|\u5019\u9009|\u76d8\u53e4)|(?:pangu|memory).*(?:candidate|record|remember|debug|fix|session)/i.test(normalized);
  }

  function codexMemoryLooksLikeChatter(text) {
    const normalized = String(text || "").replace(/[。！？!?.,，\s]/g, "").toLowerCase();
    if (!normalized) return true;
    const chatter = new Set([
      "你好",
      "您好",
      "嗨",
      "hi",
      "hello",
      "hey",
      "谢谢",
      "感谢",
      "好的",
      "好",
      "可以",
      "继续",
      "再来",
    ]);
    return chatter.has(normalized);
  }

  function codexMemoryLooksLikeTitleOnly(text) {
    const normalized = String(text || "").replace(/\s+/g, " ").trim();
    if (/^codex[:：].{1,40}$/i.test(normalized)) return true;
    if (/^(new chat|new conversation|untitled|无标题|新建对话)$/i.test(normalized)) return true;
    return normalized.length < 10 && !/[，。；：,.!?！？]/.test(normalized);
  }

  function codexMemoryHash(text) {
    let hash = 2166136261;
    for (let index = 0; index < text.length; index += 1) {
      hash ^= text.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
    return String(hash >>> 0);
  }

  function codexMemoryAutoSuggestDiagnostic(reason, detail = {}) {
    const payload = {
      reason,
      workspace: codexMemoryState.workspace || codexMemoryWorkspace(),
      ...detail,
    };
    const hash = codexMemoryHash(JSON.stringify(payload));
    const now = Date.now();
    if (hash === codexMemoryState.lastAutoSuggestDiagnosticHash && now - codexMemoryState.lastAutoSuggestDiagnosticAt < 60000) return;
    codexMemoryState.lastAutoSuggestDiagnosticHash = hash;
    codexMemoryState.lastAutoSuggestDiagnosticAt = now;
    sendClaudeCodexProDiagnostic("memory_auto_suggest", payload);
  }

  const codexMemoryCaptureTtlMs = 30 * 60 * 1000;
  const codexMemoryCaptureMaxEntries = 128;
  const codexMemoryCaptureRecent = new Map();
  const codexMemoryCaptureInFlight = new Map();

  function codexMemoryCaptureFingerprint(payload) {
    return JSON.stringify({
      workspace: payload.workspace,
      text: payload.text,
      candidateTriggered: payload.candidateTriggered,
      candidateReason: payload.candidateReason,
      skipReason: payload.skipReason,
    });
  }

  function codexMemoryPruneCaptureHistory(now = Date.now()) {
    for (const [fingerprint, entry] of codexMemoryCaptureRecent) {
      if (now - Number(entry?.completedAt || 0) >= codexMemoryCaptureTtlMs) {
        codexMemoryCaptureRecent.delete(fingerprint);
      }
    }
    while (codexMemoryCaptureRecent.size > codexMemoryCaptureMaxEntries) {
      const oldest = codexMemoryCaptureRecent.keys().next().value;
      if (oldest === undefined) break;
      codexMemoryCaptureRecent.delete(oldest);
    }
  }

  function codexMemoryRememberCapture(fingerprint, result) {
    codexMemoryPruneCaptureHistory();
    codexMemoryCaptureRecent.delete(fingerprint);
    codexMemoryCaptureRecent.set(fingerprint, {
      completedAt: Date.now(),
      result,
    });
    codexMemoryPruneCaptureHistory();
  }

  async function codexMemoryRecordCapture(text, detail = {}) {
    const normalized = String(text || "").replace(/\s+/g, " ").trim();
    if (!normalized) return null;
    try {
      const workspace = await codexMemoryResolvedWorkspace();
      const payload = {
        workspace,
        text: normalized.slice(0, 4000),
        source: "codex-dom-capture",
        sourceSessionId: location.href,
        candidateTriggered: !!detail.candidateTriggered,
        candidateReason: detail.candidateReason || "",
        skipReason: detail.skipReason || "",
      };
      const fingerprint = codexMemoryCaptureFingerprint(payload);
      codexMemoryPruneCaptureHistory();
      const recent = codexMemoryCaptureRecent.get(fingerprint);
      if (recent) return recent.result;
      const inFlight = codexMemoryCaptureInFlight.get(fingerprint);
      if (inFlight) return inFlight;

      let request;
      request = Promise.resolve()
        .then(async () => {
          const result = await postJson("/memory/capture", payload);
          if (result?.status !== "ok") throw new Error(result?.message || "capture failed");
          codexMemoryRememberCapture(fingerprint, result);
          return result;
        })
        .catch((error) => {
          codexMemoryCaptureRecent.delete(fingerprint);
          codexMemoryAutoSuggestDiagnostic("database_failed", {
            operation: "capture",
            message: String(error?.message || error).slice(0, 240),
            textLength: normalized.length,
          });
          return null;
        })
        .finally(() => {
          if (codexMemoryCaptureInFlight.get(fingerprint) === request) {
            codexMemoryCaptureInFlight.delete(fingerprint);
          }
        });
      codexMemoryCaptureInFlight.set(fingerprint, request);
      return request;
    } catch (error) {
      codexMemoryAutoSuggestDiagnostic("database_failed", {
        operation: "capture",
        message: String(error?.message || error).slice(0, 240),
        textLength: normalized.length,
      });
      return null;
    }
  }

  function codexMemorySetMessage(message, status = "") {
    const panel = document.getElementById(codexMemoryPanelId);
    const node = panel?.querySelector("[data-codex-memory-message]");
    if (!node) return;
    node.textContent = message || "";
    node.dataset.status = status;
  }

  function codexMemoryRenderList(items, emptyText = "暂无匹配记忆。") {
    const panel = document.getElementById(codexMemoryPanelId);
    const list = panel?.querySelector("[data-codex-memory-list]");
    if (!list) return;
    const rows = (items || []).map((entry) => entry.item || entry).filter(Boolean);
    list.innerHTML = rows.length ? rows.slice(0, 12).map((item) => `
      <div class="codex-memory-card">
        <strong>${escapeHtml(item.category || "general")} · ${escapeHtml(item.workspace || "")}</strong>
        <p>${escapeHtml(item.text || "")}</p>
        <small>${escapeHtml((item.tags || []).join(", "))}</small>
      </div>
    `).join("") : `<div class="codex-memory-card"><p>${escapeHtml(emptyText)}</p></div>`;
  }

  function codexMemoryUpdateBadge() {
    const settings = claudeCodexProSettings();
    const badge = document.getElementById(codexMemoryBadgeId);
    if (!settings.memoryAssistEnabled || !settings.memoryAssistInjectEnabled) {
      badge?.remove();
      document.getElementById(codexMemoryPanelId)?.remove();
      window.__claudeCodexProMemoryAssistRuntime = {
        enabled: !!settings.memoryAssistEnabled,
        injected: false,
        status: "disabled",
        active: false,
        workspace: codexMemoryState.workspace,
        totalItems: Number(codexMemoryState.totalItems || 0),
        pendingCandidates: Number(codexMemoryState.pendingCandidates || 0),
        injectSummaryCachePath: codexMemoryState.injectSummaryCachePath || "",
        summary: "盘古记忆当前未注入。",
        source: "idle",
      };
      codexMemoryHeartbeat(true);
      return;
    }
    const node = badge || document.createElement("button");
    node.id = codexMemoryBadgeId;
    node.type = "button";
    node.dataset.codexMemoryAssistVersion = codexMemoryAssistVersion;
    node.dataset.status = codexMemoryState.status;
    node.dataset.active = Date.now() < Number(codexMemoryState.activeUntil || 0) ? "true" : "false";
    node.innerHTML = `
      <span class="codex-memory-dot"></span>
      <span>盘古记忆</span>
      <span class="codex-memory-count">${codexMemoryState.totalItems || 0}</span>
      ${codexMemoryState.pendingCandidates ? `<span>待确认 ${codexMemoryState.pendingCandidates}</span>` : ""}
    `;
    node.title = codexMemoryState.summary || "盘古记忆";
    if (!badge) {
      node.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
        codexMemoryTogglePanel();
      });
      document.documentElement.appendChild(node);
    }
    codexMemoryExposeRuntime();
    codexMemoryHeartbeat();
    updateCodexMemoryBadgePosition();
  }

  async function codexMemoryLoadSession(force = false) {
    const settings = claudeCodexProSettings();
    if (!settings.memoryAssistEnabled || !settings.memoryAssistInjectEnabled) {
      codexMemoryUpdateBadge();
      return;
    }
    const now = Date.now();
    if (!force && now - codexMemoryState.lastLoadedAt < 5000) {
      codexMemoryUpdateBadge();
      return;
    }
    codexMemoryPulseActivity("session");
    codexMemoryState.lastLoadedAt = now;
    codexMemoryState.workspace = await codexMemoryResolvedWorkspace();
    const query = codexMemoryCurrentText();
    try {
      const result = await postJson("/memory/session", {
        workspace: codexMemoryState.workspace,
        query: query.slice(0, 1600),
        maxItems: settings.memoryAssistMaxInjectedItems || 5,
      });
      if (result?.status !== "ok") throw new Error(result?.message || "memory session failed");
      codexMemoryState.status = "ok";
      codexMemoryState.totalItems = Number(result.totalItems || 0);
      codexMemoryState.pendingCandidates = Number(result.pendingCandidates || 0);
      codexMemoryState.injectSummaryCachePath = String(result.injectSummaryCachePath || "");
      codexMemoryState.injectedItems = Array.isArray(result.injectedItems) ? result.injectedItems : [];
      codexMemoryState.summary = result.summary || "盘古记忆已启用。";
      codexMemoryRenderList(codexMemoryState.injectedItems);
    } catch (error) {
      codexMemoryState.status = "failed";
      codexMemoryState.summary = `盘古记忆不可用：${error?.message || error}`;
    }
    codexMemoryExposeRuntime();
    codexMemoryUpdateBadge();
  }

  async function codexMemoryMaybeSuggestCandidate(force = false) {
    const settings = claudeCodexProSettings();
    if (!settings.memoryAssistEnabled || !settings.memoryAssistInjectEnabled || !settings.memoryAssistAutoSuggestEnabled) return;
    const latestUserText = codexMemoryLatestUserText();
    if (!latestUserText) {
      codexMemoryAutoSuggestDiagnostic("no_latest_user_text", { force: !!force });
      return;
    }
    const suggestion = codexMemorySuggestionFromText(latestUserText);
    if (!suggestion) {
      await codexMemoryRecordCapture(latestUserText, {
        candidateTriggered: false,
        skipReason: "not_learnable",
      });
      codexMemoryAutoSuggestDiagnostic("not_learnable", {
        textLength: latestUserText.length,
        memorySelfCheck: codexMemoryLooksMemorySelfCheckText(latestUserText),
        force: !!force,
      });
      return;
    }
    const workspace = await codexMemoryResolvedWorkspace();
    const hash = codexMemoryHash(`${workspace}\n${suggestion.text}`);
    const now = Date.now();
    if (!force && hash === codexMemoryState.lastSuggestionHash && now - codexMemoryState.lastSuggestionAt < 120000) {
      await codexMemoryRecordCapture(latestUserText, {
        candidateTriggered: false,
        candidateReason: suggestion.reason,
        skipReason: "duplicate_recent_memory",
      });
      codexMemoryAutoSuggestDiagnostic("duplicate_recent_memory", {
        reason: suggestion.reason,
        textLength: suggestion.text.length,
      });
      return;
    }
    codexMemoryState.lastSuggestionHash = hash;
    codexMemoryState.lastSuggestionAt = now;
    codexMemoryPulseActivity("candidate");
    try {
      const result = await postJson("/memory/learn", {
        workspace,
        text: suggestion.text,
        category: "preference",
        source: "codex-dom-auto",
        sourceSessionId: location.href,
      });
      if (result?.status === "ok") {
        await codexMemoryRecordCapture(latestUserText, {
          candidateTriggered: true,
          candidateReason: `auto_learned: ${suggestion.reason}`,
          skipReason: "",
        });
        codexMemoryAutoSuggestDiagnostic("memory_auto_learned", {
          reason: suggestion.reason,
          textLength: suggestion.text.length,
          itemId: result.id || "",
        });
        await codexMemoryLoadSession(true);
        codexMemoryState.summary = "已自动写入长期记忆。";
        codexMemoryExposeRuntime();
        codexMemoryUpdateBadge();
      } else {
        throw new Error(result?.message || "learn failed");
      }
    } catch (error) {
      await codexMemoryRecordCapture(latestUserText, {
        candidateTriggered: false,
        candidateReason: suggestion.reason,
        skipReason: "learn_failed",
      });
      codexMemoryAutoSuggestDiagnostic("learn_failed", {
        reason: suggestion.reason,
        message: String(error?.message || error).slice(0, 240),
      });
      // Auto learning is opportunistic; the visible badge/session loader reports hard failures.
    }
  }

  function codexMemoryEnsurePanel() {
    let panel = document.getElementById(codexMemoryPanelId);
    if (panel) return panel;
    panel = document.createElement("div");
    panel.id = codexMemoryPanelId;
    panel.hidden = true;
    panel.innerHTML = `
      <div class="codex-memory-panel-header">
        <div>
          <strong>盘古记忆</strong>
          <span data-codex-memory-summary>${escapeHtml(codexMemoryState.summary || "")}</span>
        </div>
        <button type="button" class="codex-memory-panel-close" data-codex-memory-close="true">×</button>
      </div>
      <div class="codex-memory-panel-body">
        <textarea data-codex-memory-input placeholder="选中文本或填写要长期记住的内容"></textarea>
        <div class="codex-memory-actions">
          <button type="button" data-primary="true" data-codex-memory-learn="true">记住</button>
          <button type="button" data-codex-memory-search="true">搜索</button>
          <button type="button" data-codex-memory-candidates="true">待确认</button>
          <button type="button" data-codex-memory-refresh="true">刷新</button>
          <button type="button" data-codex-memory-manager="true">管理工具</button>
        </div>
        <div class="codex-memory-message" data-codex-memory-message></div>
        <div class="codex-memory-list" data-codex-memory-list></div>
      </div>
    `;
    panel.addEventListener("click", (event) => {
      const target = event.target;
      if (target?.closest?.("[data-codex-memory-close]")) {
        panel.hidden = true;
        return;
      }
      if (target?.closest?.("[data-codex-memory-refresh]")) {
        void codexMemoryLoadSession(true);
        return;
      }
      if (target?.closest?.("[data-codex-memory-manager]")) {
        void postJson("/manager/open", {});
        return;
      }
      if (target?.closest?.("[data-codex-memory-learn]")) {
        void codexMemoryLearnFromPanel();
        return;
      }
      if (target?.closest?.("[data-codex-memory-search]")) {
        void codexMemorySearchFromPanel();
        return;
      }
      if (target?.closest?.("[data-codex-memory-candidates]")) {
        void codexMemoryLoadCandidates();
      }
    });
    document.documentElement.appendChild(panel);
    return panel;
  }

  function codexMemoryTogglePanel() {
    const panel = codexMemoryEnsurePanel();
    const input = panel.querySelector("[data-codex-memory-input]");
    const selected = codexMemoryCurrentText();
    if (input && selected && !input.value) input.value = selected.slice(0, 4000);
    panel.querySelector("[data-codex-memory-summary]").textContent = codexMemoryState.summary || "";
    panel.hidden = !panel.hidden;
    codexMemoryPulseActivity("panel");
    codexMemoryRenderList(codexMemoryState.injectedItems);
  }

  async function codexMemoryLearnFromPanel() {
    const panel = codexMemoryEnsurePanel();
    const input = panel.querySelector("[data-codex-memory-input]");
    const text = String(input?.value || codexMemoryCurrentText()).trim();
    if (!text) {
      codexMemorySetMessage("没有可保存的内容。", "failed");
      return;
    }
    codexMemorySetMessage("正在保存记忆…", "");
    codexMemoryPulseActivity("learn");
    try {
      const workspace = await codexMemoryResolvedWorkspace();
      const result = await postJson("/memory/learn", {
        workspace,
        text,
        category: "codex",
        source: "codex-dom",
        sourceSessionId: location.href,
      });
      if (result?.status !== "ok") throw new Error(result?.message || "learn failed");
      codexMemorySetMessage("记忆已保存。", "ok");
      if (input) input.value = "";
      await codexMemoryLoadSession(true);
    } catch (error) {
      codexMemorySetMessage(`保存失败：${error?.message || error}`, "failed");
    }
  }

  async function codexMemorySearchFromPanel() {
    const panel = codexMemoryEnsurePanel();
    const input = panel.querySelector("[data-codex-memory-input]");
    const query = String(input?.value || codexMemoryCurrentText()).trim();
    codexMemorySetMessage("正在检索记忆…", "");
    codexMemoryPulseActivity("search");
    try {
      const workspace = await codexMemoryResolvedWorkspace();
      const result = await postJson("/memory/search", {
        workspace,
        query,
        includeGlobal: true,
        limit: 12,
      });
      if (result?.status !== "ok") throw new Error(result?.message || "search failed");
      codexMemoryRenderList(result.results || []);
      codexMemorySetMessage(`检索完成：${(result.results || []).length} 条。`, "ok");
    } catch (error) {
      codexMemorySetMessage(`检索失败：${error?.message || error}`, "failed");
    }
  }

  async function codexMemoryLoadCandidates() {
    codexMemorySetMessage("正在读取待确认记忆…", "");
    codexMemoryPulseActivity("candidate-list");
    try {
      const workspace = await codexMemoryResolvedWorkspace();
      const result = await postJson("/memory/candidates", {
        workspace,
        includeGlobal: true,
      });
      if (result?.status !== "ok") throw new Error(result?.message || "candidates failed");
      const candidates = result.candidates || [];
      const panel = codexMemoryEnsurePanel();
      const list = panel.querySelector("[data-codex-memory-list]");
      list.innerHTML = candidates.length ? candidates.map((candidate) => `
        <div class="codex-memory-card" data-codex-memory-candidate="${escapeHtml(candidate.id)}">
          <strong>${escapeHtml(candidate.category || "general")} · ${escapeHtml(candidate.workspace || "")}</strong>
          <p>${escapeHtml(candidate.text || "")}</p>
          <small>${escapeHtml(candidate.reason || "待确认")}</small>
          <div class="codex-memory-actions">
            <button type="button" data-codex-memory-approve="${escapeHtml(candidate.id)}">确认</button>
            <button type="button" data-codex-memory-reject="${escapeHtml(candidate.id)}">忽略</button>
          </div>
        </div>
      `).join("") : `<div class="codex-memory-card"><p>暂无待确认记忆。</p></div>`;
      list.querySelectorAll("[data-codex-memory-approve]").forEach((button) => {
        button.addEventListener("click", () => void codexMemoryReviewCandidate(button.getAttribute("data-codex-memory-approve"), true));
      });
      list.querySelectorAll("[data-codex-memory-reject]").forEach((button) => {
        button.addEventListener("click", () => void codexMemoryReviewCandidate(button.getAttribute("data-codex-memory-reject"), false));
      });
      codexMemorySetMessage(`待确认：${candidates.length} 条。`, "ok");
    } catch (error) {
      codexMemorySetMessage(`读取失败：${error?.message || error}`, "failed");
    }
  }

  async function codexMemoryReviewCandidate(id, approve) {
    if (!id) return;
    codexMemoryPulseActivity(approve ? "approve" : "reject");
    try {
      const result = await postJson(approve ? "/memory/approve" : "/memory/reject", { id });
      if (result?.status !== "ok") throw new Error(result?.message || "review failed");
      codexMemorySetMessage(approve ? "已确认写入长期记忆。" : "已忽略。", "ok");
      await codexMemoryLoadCandidates();
      await codexMemoryLoadSession(true);
    } catch (error) {
      codexMemorySetMessage(`操作失败：${error?.message || error}`, "failed");
    }
  }

  function isExtensionUiNode(node) {
    return !!node?.closest?.(`.codex-delete-toast, .codex-delete-confirm-overlay, .claude-codex-pro-modal-overlay, .${projectMoveOverlayClass}, .${timelineClass}, .codex-conversation-timeline, .${codexServiceTierBadgeClass}, .codex-zed-remote-button, .codex-zed-remote-toast, #claude-codex-pro-menu, #${codexMemoryBadgeId}, #${codexMemoryPanelId}, #ccp-multica-workspace-root, [data-ccp-multica-nav="true"]`);
  }

  function scanRelevantSelector() {
    return [
      selectors.sidebarThread,
      '[data-app-action-sidebar-section-heading="Chats"]',
      '[data-app-action-sidebar-section-heading="Projects"]',
      '[data-codex-project-move-row="true"]',
      '[data-codex-archive-page-row="true"]',
      "[data-codex-archive-delete-all]",
      '[data-message-author-role]',
      '[data-testid="conversation-turn"]',
      '[class*="user-message"]',
      '[class*="UserMessage"]',
      ".composer-footer",
      selectors.appHeader,
      selectors.archiveNav,
      selectors.pluginNavButton,
      ...(pluginPatchDisabledInRelayMode() ? [] : [selectors.disabledInstallButton]),
    ].join(", ");
  }

  function nodeSelfOrAncestorMatchesScanRelevance(node) {
    if (node.nodeType !== 1) return false;
    if (isExtensionUiNode(node)) return false;
    if (multicaPluginAnchorMutationNode(node)) return true;
    const questionSelector = timelineQuestionSelector();
    const relevantSelector = scanRelevantSelector();
    return !!node.matches?.(relevantSelector) ||
      !!node.closest?.(relevantSelector) ||
      !!node.matches?.(questionSelector) ||
      !!node.closest?.(questionSelector) ||
      nodeOrAncestorLooksLikeCodexUserBubble(node);
  }

  function isScanRelevantNode(node) {
    if (node.nodeType !== 1) return false;
    if (isExtensionUiNode(node)) return false;
    return nodeSelfOrAncestorMatchesScanRelevance(node) || !!node.querySelector?.(scanRelevantSelector()) || nodeLooksLikeTimelineQuestion(node);
  }

  function isChatContentMutation(mutation) {
    const target = mutation.target;
    if (!target?.closest?.('[data-message-author-role], [data-testid="conversation-turn"], main .prose')) return false;
    return !Array.from(mutation.addedNodes).some((node) => node.nodeType === 1 && isScanRelevantNode(node)) &&
      !Array.from(mutation.removedNodes).some((node) => node.nodeType === 1 && isScanRelevantNode(node));
  }

  function shouldScheduleScan(mutations) {
    if (!mutations) return true;
    return mutations.some((mutation) => {
      if (isChatContentMutation(mutation)) return false;
      const target = mutation.target;
      if (isExtensionUiNode(target)) return false;
      if (target?.nodeType === 1 && nodeSelfOrAncestorMatchesScanRelevance(target)) return true;
      const changedNodes = [...Array.from(mutation.addedNodes), ...Array.from(mutation.removedNodes)];
      return changedNodes.some((node) => node.nodeType === 1 && isScanRelevantNode(node));
    });
  }

  function runScheduledScan() {
    window.__codexSessionDeleteScanPending = false;
    clearTimeout(window.__codexSessionDeleteScanTimer);
    window.__codexSessionDeleteScanTimer = null;
    scan();
  }

  function scheduleScan(mutations) {
    scheduleZedRemoteMenuRefresh(mutations);
    if (multicaWorkspaceAnchorChanged(mutations)) {
      multicaWorkspaceState.anchorAttempts = 0;
      multicaWorkspaceState.anchorDiagnosticSent = false;
    }
    if (!shouldScheduleScan(mutations)) return;
    if (window.__codexSessionDeleteScanPending) return;
    window.__codexSessionDeleteScanPending = true;
    window.__codexSessionDeleteScanTimer = setTimeout(runScheduledScan, 200);
  }

  void loadBackendSettingsForStartup();
  void loadCodexServiceTierState();
  installUpstreamBranchDropdownAdapter();
  installUpstreamWorktreeNativeAdapter();
  scan();
  window.__codexProjectMoveApplyProjection = applyProjectMoveProjection;
  window.__codexProjectMoveReadProjection = readProjectMoveProjection;
  window.__codexProjectMoveTargets = projectMoveTargets;
  window.__codexProjectMoveSortChats = applyChatsSortCorrection;
  window.removeEventListener("resize", window.__claudeCodexProResizeHandler);
  let claudeCodexProResizeRafId = 0;
  window.__claudeCodexProResizeHandler = () => {
    cancelAnimationFrame(claudeCodexProResizeRafId);
    claudeCodexProResizeRafId = requestAnimationFrame(() => {
      updateFloatingClaudeCodexProMenuPosition(document.getElementById(claudeCodexProMenuId));
      runScanStep(refreshConversationTimeline);
      runScanStep(refreshConversationView);
      runScanStep(multicaWorkspaceUpdateGeometry);
    });
  };
  window.addEventListener("resize", window.__claudeCodexProResizeHandler);
  const windowControlsOverlay = navigator.windowControlsOverlay;
  if (windowControlsOverlay?.removeEventListener && window.__claudeCodexProWindowControlsOverlayHandler) {
    windowControlsOverlay.removeEventListener("geometrychange", window.__claudeCodexProWindowControlsOverlayHandler);
  }
  window.__claudeCodexProWindowControlsOverlayHandler = () => window.__claudeCodexProResizeHandler?.();
  if (windowControlsOverlay?.addEventListener) {
    windowControlsOverlay.addEventListener("geometrychange", window.__claudeCodexProWindowControlsOverlayHandler);
  }
  window.__codexSessionDeleteObserver?.disconnect();
  window.__codexSessionDeleteObserver = new MutationObserver(scheduleScan);
  window.__codexSessionDeleteObserver.observe(document.body || document.documentElement, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["class", "aria-label", "title", "hidden"],
    characterData: true,
  });
})();
