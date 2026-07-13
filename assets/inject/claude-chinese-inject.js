(function () {
  "use strict";

  const existingRuntime = window.__CLAUDE_CODEX_PRO_CHINESE_RUNTIME__;
  if (existingRuntime?.ready && typeof existingRuntime.refresh === "function") {
    existingRuntime.refresh();
    return;
  }
  const runtime = { ready: false, refresh: null };
  window.__CLAUDE_CODEX_PRO_CHINESE_RUNTIME__ = runtime;
  window.__CLAUDE_CODEX_PRO_CHINESE_INJECTED = true;
  document.documentElement.dataset.claudeCodexProChineseInjected = "true";

  const TEXT = new Map([
    ["New chat", "\u65b0\u5efa\u5bf9\u8bdd"],
    ["New session", "\u65b0\u5efa\u4f1a\u8bdd"],
    ["Chats", "\u5bf9\u8bdd"],
    ["Projects", "\u9879\u76ee"],
    ["Artifacts", "\u4ea7\u7269"],
    ["Recents", "\u6700\u8fd1"],
    ["Settings", "\u8bbe\u7f6e"],
    ["Customize", "\u81ea\u5b9a\u4e49"],
    ["Inference configuration", "\u63a8\u7406\u914d\u7f6e"],
    ["Profile", "\u4e2a\u4eba\u8d44\u6599"],
    ["Account", "\u8d26\u53f7"],
    ["Billing", "\u8d26\u5355"],
    ["Plans", "\u5957\u9910"],
    ["Appearance", "\u5916\u89c2"],
    ["Language", "\u8bed\u8a00"],
    ["Connectors", "\u8fde\u63a5\u5668"],
    ["Skills", "\u6280\u80fd"],
    ["Plugins", "\u63d2\u4ef6"],
    ["Personal plugins", "\u4e2a\u4eba\u63d2\u4ef6"],
    ["Browse plugins", "\u6d4f\u89c8\u63d2\u4ef6"],
    ["Organization plugins", "\u7ec4\u7ec7\u63d2\u4ef6"],
    ["MCP servers", "MCP \u670d\u52a1\u5668"],
    ["MCP server", "MCP \u670d\u52a1\u5668"],
    ["Developer", "\u5f00\u53d1\u8005"],
    ["Advanced", "\u9ad8\u7ea7"],
    ["General", "\u901a\u7528"],
    ["Notifications", "\u901a\u77e5"],
    ["Privacy", "\u9690\u79c1"],
    ["Help", "\u5e2e\u52a9"],
    ["Log out", "\u9000\u51fa\u767b\u5f55"],
    ["Sign in", "\u767b\u5f55"],
    ["Continue", "\u7ee7\u7eed"],
    ["Send", "\u53d1\u9001"],
    ["Stop", "\u505c\u6b62"],
    ["Retry", "\u91cd\u8bd5"],
    ["Cancel", "\u53d6\u6d88"],
    ["Save", "\u4fdd\u5b58"],
    ["Done", "\u5b8c\u6210"],
    ["Edit", "\u7f16\u8f91"],
    ["Delete", "\u5220\u9664"],
    ["Remove", "\u79fb\u9664"],
    ["Install", "\u5b89\u88c5"],
    ["Installed", "\u5df2\u5b89\u88c5"],
    ["Enable", "\u542f\u7528"],
    ["Enabled", "\u5df2\u542f\u7528"],
    ["Disable", "\u7981\u7528"],
    ["Disabled", "\u5df2\u7981\u7528"],
    ["Search", "\u641c\u7d22"],
    ["Search plugins", "\u641c\u7d22\u63d2\u4ef6"],
    ["No plugins found", "\u672a\u627e\u5230\u63d2\u4ef6"],
    ["Choose where Claude sends inference requests.", "\u9009\u62e9 Claude \u5c06\u63a8\u7406\u8bf7\u6c42\u53d1\u9001\u5230\u54ea\u91cc\u3002"],
    ["Local stdio servers", "\u672c\u5730 stdio \u670d\u52a1\u5668"],
    ["Remote servers", "\u8fdc\u7a0b\u670d\u52a1\u5668"],
    ["Allow user-added MCP servers", "\u5141\u8bb8\u7528\u6237\u6dfb\u52a0 MCP \u670d\u52a1\u5668"],
    ["Message Claude", "\u7ed9 Claude \u53d1\u6d88\u606f"],
    ["What can I help you with today?", "\u4eca\u5929\u6211\u80fd\u5e2e\u4f60\u4ec0\u4e48\uff1f"],
  ]);
  const TEXT_ENTRIES = Array.from(TEXT.entries()).sort(([left], [right]) => right.length - left.length);

  const ATTRS = ["aria-label", "title", "placeholder", "value"];
  const queue = [];
  let scheduled = false;
  let positionRaf = 0;
  let chineseEnabled = true;
  let backendProbeTimer = 0;
  const ccpDisplayVersion = window.__CLAUDE_CODEX_PRO_VERSION__ || "unknown";
  const backendState = { status: "checking", message: "\u6b63\u5728\u68c0\u67e5" };

  function translateText(value) {
    if (!chineseEnabled) return value;
    const trimmed = value.trim();
    if (!trimmed) return value;
    if (/\bCodex\b/.test(trimmed)) return value;
    const direct = TEXT.get(trimmed);
    if (direct) return value.replace(trimmed, direct);
    let next = value;
    for (const [from, to] of TEXT_ENTRIES) {
      if (/\bCodex\b/.test(next)) continue;
      if (next.includes(from)) next = next.split(from).join(to);
    }
    return next;
  }

  function isInjectedUi(node) {
    return !!node?.closest?.("#ccp-claude-status-pill, #ccp-claude-status-panel");
  }

  function isEditableUi(node) {
    const element = node?.nodeType === Node.ELEMENT_NODE ? node : node?.parentElement;
    return !!element?.closest?.([
      "input",
      "textarea",
      "select",
      "[contenteditable]",
      '[role="textbox"]',
      ".ProseMirror",
      ".cm-editor",
      ".cm-content",
      '[data-testid*="composer"]',
      '[class*="composer"]',
      '[class*="Editor"]',
      '[class*="editor"]',
    ].join(","));
  }

  function translateNode(node) {
    if (!node || isInjectedUi(node) || isEditableUi(node)) return;
    if (node.nodeType === Node.TEXT_NODE) {
      const next = translateText(node.nodeValue || "");
      if (next !== node.nodeValue) node.nodeValue = next;
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;
    for (const attr of ATTRS) {
      if (!node.hasAttribute(attr)) continue;
      const value = node.getAttribute(attr) || "";
      const next = translateText(value);
      if (next !== value) node.setAttribute(attr, next);
    }
  }

  function enqueue(root) {
    if (!chineseEnabled || !root || isInjectedUi(root) || isEditableUi(root)) return;
    queue.push(root);
    if (!scheduled) {
      scheduled = true;
      window.requestAnimationFrame(flush);
    }
  }

  function flush() {
    scheduled = false;
    const batch = queue.splice(0, 160);
    for (const root of batch) {
      translateNode(root);
      if (root.nodeType === Node.ELEMENT_NODE) {
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
        let node;
        while ((node = walker.nextNode())) translateNode(node);
      }
    }
    if (queue.length) enqueue(document.body);
  }

  function installObserver() {
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        enqueue(mutation.target);
        mutation.addedNodes.forEach(enqueue);
        if (mutation.target instanceof Element && !isInjectedUi(mutation.target)) scheduleStatusPosition();
      }
    });
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ATTRS,
    });
  }

  function openPluginHub() {
    window.dispatchEvent(new CustomEvent("claude-codex-pro-plugin-hub-open"));
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === "function") {
      window.__TAURI_INTERNALS__.invoke("open_plugin_hub_window").catch(() => {
        window.location.href = "claude-codex-pro://plugin-hub";
      });
      return;
    }
    window.location.href = "claude-codex-pro://plugin-hub";
  }

  function tauriInvoke(command, payload) {
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === "function") {
      return window.__TAURI_INTERNALS__.invoke(command, payload);
    }
    return Promise.reject(new Error("tauri bridge unavailable"));
  }

  function closeStatusPanel() {
    document.getElementById("ccp-claude-status-panel")?.remove();
  }

  function statusText() {
    const backendOnline = backendState.status === "ok";
    return {
      backend: backendOnline ? "\u5728\u7ebf" : (backendState.status === "checking" ? "\u68c0\u67e5\u4e2d" : "\u79bb\u7ebf"),
      backendStatus: backendOnline ? "ok" : "failed",
    };
  }

  function renderStatusPanel() {
    const panel = document.getElementById("ccp-claude-status-panel");
    if (!panel) return;
    const state = statusText();
    const backend = panel.querySelector("[data-ccp-panel-status]");
    const backendMessage = panel.querySelector("[data-ccp-panel-status-message]");
    const chinese = panel.querySelector("[data-ccp-panel-chinese]");
    const toggle = panel.querySelector("[data-ccp-toggle-chinese]");
    if (backend) {
      backend.dataset.status = state.backendStatus;
      backend.textContent = state.backend;
    }
    if (backendMessage) backendMessage.textContent = backendState.message || "";
    if (chinese) {
      chinese.dataset.status = chineseEnabled ? "ok" : "failed";
      chinese.textContent = chineseEnabled ? "\u5df2\u5f00\u542f" : "\u5df2\u5173\u95ed";
    }
    if (toggle) toggle.dataset.enabled = String(chineseEnabled);
  }

  function updateStatusPill() {
    const pill = document.getElementById("ccp-claude-status-pill");
    if (!pill) return;
    const state = statusText();
    const backend = pill.querySelector("[data-ccp-backend-status]");
    if (backend) backend.dataset.status = state.backendStatus;
    pill.title = `CCP ${ccpDisplayVersion}\uff1a${state.backend}`;
    renderStatusPanel();
  }

  async function checkBackendStatus() {
    backendState.status = "checking";
    backendState.message = "\u6b63\u5728\u68c0\u67e5";
    updateStatusPill();
    try {
      const result = await Promise.race([
        tauriInvoke("backend_version", {}),
        new Promise((_, reject) => setTimeout(() => reject(new Error("timeout")), 2000)),
      ]);
      backendState.status = result?.status === "ok" ? "ok" : "failed";
      backendState.message = result?.message || (backendState.status === "ok" ? "\u540e\u7aef\u5728\u7ebf" : "\u540e\u7aef\u79bb\u7ebf");
    } catch (error) {
      backendState.status = "failed";
      backendState.message = error?.message || "\u540e\u7aef\u79bb\u7ebf";
    }
    updateStatusPill();
  }

  function scheduleBackendHeartbeat() {
    if (backendProbeTimer) return;
    void checkBackendStatus();
    backendProbeTimer = window.setInterval(checkBackendStatus, 5000);
  }

  function openStatusPanel() {
    closeStatusPanel();
    const state = statusText();
    const panel = document.createElement("div");
    panel.id = "ccp-claude-status-panel";
    panel.dataset.claudeCodexProInjected = "true";
    panel.innerHTML = [
      '<div class="ccp-panel-head"><strong>Claude Codex Pro ' + ccpDisplayVersion + '</strong><button type="button" data-ccp-close aria-label="\u5173\u95ed">\u00d7</button></div>',
      '<div class="ccp-panel-row"><span>\u8fde\u63a5\u72b6\u6001</span><b data-ccp-panel-status data-status="' + state.backendStatus + '">' + state.backend + '</b></div>',
      '<div class="ccp-panel-note" data-ccp-panel-status-message>' + (backendState.message || "") + '</div>',
      '<div class="ccp-panel-row"><span>\u6c49\u5316\u72b6\u6001</span><b data-ccp-panel-chinese data-status="ok">\u5df2\u5f00\u542f</b></div>',
      '<button type="button" class="ccp-switch" data-ccp-toggle-chinese data-enabled="true"><span></span>\u6c49\u5316\u5f00\u5173</button>',
      '<button type="button" id="ccp-plugin-hub-button">\u63d2\u4ef6\u4e2d\u5fc3</button>',
    ].join("");
    document.documentElement.appendChild(panel);
    panel.querySelector("[data-ccp-close]")?.addEventListener("click", closeStatusPanel);
    panel.querySelector("#ccp-plugin-hub-button")?.addEventListener("click", openPluginHub);
    panel.querySelector("[data-ccp-toggle-chinese]")?.addEventListener("click", () => {
      chineseEnabled = !chineseEnabled;
      if (chineseEnabled) enqueue(document.body || document.documentElement);
      updateStatusPill();
    });
    renderStatusPanel();
    positionStatusPanel();
  }

  function visibleRectForLeftAnchor(node, headerRect) {
    if (!(node instanceof Element) || isInjectedUi(node)) return null;
    const rect = node.getBoundingClientRect();
    if (!(rect.width > 0 && rect.height > 0)) return null;
    if (rect.right < headerRect.left || rect.left > Math.min(window.innerWidth * 0.55, headerRect.right)) return null;
    if (!String(node.textContent || node.getAttribute("aria-label") || node.getAttribute("title") || "").trim() && !node.querySelector?.("svg")) return null;
    return rect;
  }

  function findLeftAnchor(header, headerRect) {
    const selector = [
      "button",
      "a",
      '[role="button"]',
      '[aria-label]',
      '[title]',
      '[class*="truncate"]',
      '[class*="text-sm"]',
      '[class*="text-base"]',
      '[data-testid]',
      "h1",
      "h2",
      "svg",
    ].join(",");
    return Array.from(header?.querySelectorAll?.(selector) || [])
      .map((node) => visibleRectForLeftAnchor(node, headerRect))
      .filter(Boolean)
      .sort((a, b) => a.left - b.left || a.top - b.top)[0] || null;
  }

  function findWindowLeftAnchor(header, headerRect) {
    const leftRoots = [
      document.querySelector("aside"),
      document.querySelector("nav"),
      document.querySelector('[role="navigation"]'),
      document.querySelector('[class*="sidebar"]'),
    ].filter(Boolean);
    for (const root of leftRoots) {
      const rootRect = root.getBoundingClientRect?.();
      if (!rootRect || !(rootRect.width > 0 && rootRect.height > 0)) continue;
      const anchor = findLeftAnchor(root, {
        left: Math.max(0, rootRect.left),
        right: Math.min(window.innerWidth * 0.55, rootRect.right),
      });
      if (anchor) return anchor;
    }
    return findLeftAnchor(header, headerRect);
  }

  function positionStatusPanel() {
    const pill = document.getElementById("ccp-claude-status-pill");
    const panel = document.getElementById("ccp-claude-status-panel");
    if (!pill || !panel) return;
    const rect = pill.getBoundingClientRect();
    const panelWidth = panel.getBoundingClientRect().width || 260;
    panel.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - panelWidth - 8))}px`;
    panel.style.top = `${Math.max(8, rect.bottom + 8)}px`;
  }

  function positionStatusPill() {
    const pill = document.getElementById("ccp-claude-status-pill");
    if (!pill) return;
    const header = document.querySelector("header") || document.querySelector('[class*="titlebar"], [class*="toolbar"], nav');
    const headerRect = header?.getBoundingClientRect?.();
    if (!header || !headerRect?.height) {
      pill.style.left = "44px";
      pill.style.top = "8px";
      positionStatusPanel();
      return;
    }
    const anchorRect = findWindowLeftAnchor(header, headerRect);
    const pillWidth = pill.getBoundingClientRect().width || 150;
    const minLeft = Math.max(8, headerRect.left + 8);
    const maxLeft = Math.max(minLeft, Math.min(window.innerWidth - pillWidth - 8, headerRect.right - pillWidth - 8));
    const left = Math.max(minLeft, Math.min(anchorRect ? anchorRect.right + 8 : headerRect.left + 44, maxLeft));
    pill.style.left = `${left}px`;
    pill.style.top = `${headerRect.top + Math.max(0, (headerRect.height - 32) / 2)}px`;
    positionStatusPanel();
  }

  function scheduleStatusPosition() {
    window.cancelAnimationFrame(positionRaf);
    positionRaf = window.requestAnimationFrame(positionStatusPill);
  }

  function ensureStatusPill() {
    if (document.getElementById("ccp-claude-status-pill")) {
      updateStatusPill();
      scheduleStatusPosition();
      return;
    }
    const pill = document.createElement("div");
    pill.id = "ccp-claude-status-pill";
    pill.dataset.claudeCodexProInjected = "true";
    pill.setAttribute("role", "button");
    pill.setAttribute("tabindex", "0");
    pill.setAttribute("aria-label", `CCP ${ccpDisplayVersion}`);
    pill.innerHTML = [
      '<span class="ccp-dot" data-ccp-backend-status data-status="failed"></span>',
      '<strong>CCP ' + ccpDisplayVersion + '</strong>',
    ].join("");
    document.documentElement.appendChild(pill);
    pill.addEventListener("click", openStatusPanel);
    pill.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      openStatusPanel();
    });
    updateStatusPill();
    scheduleStatusPosition();
  }

  function installStyles() {
    if (document.getElementById("ccp-claude-chinese-style")) return;
    const style = document.createElement("style");
    style.id = "ccp-claude-chinese-style";
    style.textContent = `
      #ccp-claude-status-pill {
        position: fixed;
        top: 8px;
        left: 44px;
        z-index: 2147483647;
        display: inline-flex;
        align-items: center;
        gap: 8px;
        min-height: 32px;
        padding: 0 4px;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: #102016;
        box-shadow: none;
        font: 12px/1.2 "Segoe UI", "Microsoft YaHei", sans-serif;
        backdrop-filter: none;
        cursor: pointer;
        user-select: none;
      }
      #ccp-claude-status-pill .ccp-state {
        display: inline-flex;
        align-items: center;
        gap: 4px;
      }
      #ccp-claude-status-pill .ccp-dot {
        width: 8px;
        height: 8px;
        border-radius: 999px;
        background: #ef4444;
        box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.14);
      }
      #ccp-claude-status-pill .ccp-dot[data-status="ok"] {
        background: #16a34a;
        box-shadow: 0 0 0 4px rgba(22, 163, 74, 0.16);
      }
      #ccp-claude-status-panel {
        position: fixed;
        z-index: 2147483647;
        width: 260px;
        padding: 12px;
        border: 1px solid rgba(15, 23, 42, 0.12);
        border-radius: 12px;
        background: rgba(255,255,255,.98);
        color: #111827;
        box-shadow: 0 18px 55px rgba(15, 23, 42, 0.18);
        font: 13px/1.35 "Segoe UI", "Microsoft YaHei", sans-serif;
      }
      #ccp-claude-status-panel .ccp-panel-head,
      #ccp-claude-status-panel .ccp-panel-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
      }
      #ccp-claude-status-panel .ccp-panel-head {
        margin-bottom: 10px;
      }
      #ccp-claude-status-panel .ccp-panel-row {
        padding: 7px 0;
        border-top: 1px solid rgba(15, 23, 42, 0.08);
      }
      #ccp-claude-status-panel .ccp-panel-note {
        margin: -2px 0 4px;
        color: #64748b;
        font-size: 12px;
        word-break: break-word;
      }
      #ccp-claude-status-panel [data-ccp-close] {
        border: 0;
        background: transparent;
        color: #475569;
        font: 18px/1 "Segoe UI", sans-serif;
        cursor: pointer;
      }
      #ccp-claude-status-panel [data-status="ok"] {
        color: #166534;
      }
      #ccp-claude-status-panel [data-status="failed"] {
        color: #b91c1c;
      }
      #ccp-plugin-hub-button {
        width: 100%;
        height: 30px;
        margin-top: 10px;
        border: 0;
        border-radius: 8px;
        padding: 0 10px;
        background: #0f766e;
        color: #fff;
        font: inherit;
        cursor: pointer;
      }
      #ccp-plugin-hub-button:hover {
        background: #115e59;
      }
      #ccp-claude-status-panel .ccp-switch {
        width: 100%;
        height: 30px;
        margin-top: 8px;
        border: 1px solid rgba(15, 23, 42, 0.12);
        border-radius: 8px;
        background: #f8fafc;
        color: #111827;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        font: inherit;
        cursor: pointer;
      }
      #ccp-claude-status-panel .ccp-switch span {
        width: 28px;
        height: 16px;
        border-radius: 999px;
        background: #ef4444;
        position: relative;
      }
      #ccp-claude-status-panel .ccp-switch span::after {
        content: "";
        position: absolute;
        top: 3px;
        left: 3px;
        width: 10px;
        height: 10px;
        border-radius: 999px;
        background: #fff;
        transition: transform .16s ease;
      }
      #ccp-claude-status-panel .ccp-switch[data-enabled="true"] span {
        background: #16a34a;
      }
      #ccp-claude-status-panel .ccp-switch[data-enabled="true"] span::after {
        transform: translateX(12px);
      }
    `;
    document.documentElement.appendChild(style);
  }

  let bootstrapped = false;

  function bootstrap() {
    if (bootstrapped) return;
    bootstrapped = true;
    installStyles();
    ensureStatusPill();
    scheduleBackendHeartbeat();
    enqueue(document.documentElement);
    installObserver();
    window.addEventListener("resize", scheduleStatusPosition);
    window.setInterval(() => {
      ensureStatusPill();
      enqueue(document.body || document.documentElement);
    }, 1500);
  }

  runtime.refresh = () => {
    if (!bootstrapped && document.readyState !== "loading") {
      bootstrap();
      return;
    }
    if (!bootstrapped) return;
    ensureStatusPill();
    enqueue(document.body || document.documentElement);
    scheduleStatusPosition();
  };
  runtime.ready = true;

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", bootstrap, { once: true });
  } else {
    bootstrap();
  }
})();
