(() => {
  "use strict";

  // Injected by the launcher through CDP Page.addScriptToEvaluateOnNewDocument,
  // right after renderer-inject.js. Adds one entry button under the sidebar's
  // "more" (…) control that switches the app to a read-only codex-bot view built
  // from the local helper's /codex-bot/* routes. No writes, no new ports, no
  // Codex files touched, and no native node is ever removed: the bot view is our
  // own container that gets hidden/shown while Codex's own DOM stays untouched.
  const panelVersion = "2";
  if (window.__codexBotPanelVersion === panelVersion) return;
  window.__codexBotPanelVersion = panelVersion;

  // Same helper base + capability token contract as renderer-inject.js. The
  // token stays in this closure and is only ever sent as a request header.
  const helperBase = window.__CODEX_SESSION_DELETE_HELPER__ || "http://127.0.0.1:57321";
  const helperToken = window.__CLAUDE_CODEX_PRO_HELPER_TOKEN__ || "";
  const helperTokenHeader = "x-claude-codex-pro-token";
  const withHelperToken = (headers) => {
    const merged = Object.assign({}, headers || {});
    if (helperToken) merged[helperTokenHeader] = helperToken;
    return merged;
  };

  const viewId = "codex-bot-view";
  const panelId = viewId;
  const navItemId = "codex-bot-nav-item";
  const styleId = "codex-bot-panel-style";
  // "false" = bot view visible, "true" = back to Codex's own chat view.
  const collapsedKey = "codex-bot.panel.collapsed";
  const svgNamespace = "http://www.w3.org/2000/svg";
  // OpenAI mark, inlined from apps/claude-codex-pro-manager/src/assets/openai.svg.
  // Single colour via currentColor, so it follows the native icon colour/hover.
  const brandIconPath =
    "M21.55 10.004a5.416 5.416 0 00-.478-4.501c-1.217-2.09-3.662-3.166-6.05-2.66A5.59 5.59 0 0010.831 1C8.39.995 6.224 2.546 5.473 4.838A5.553 5.553 0 001.76 7.496a5.487 5.487 0 00.691 6.5 5.416 5.416 0 00.477 4.502c1.217 2.09 3.662 3.165 6.05 2.66A5.586 5.586 0 0013.168 23c2.443.006 4.61-1.546 5.361-3.84a5.553 5.553 0 003.715-2.66 5.488 5.488 0 00-.693-6.497zm-8.381 11.558a4.199 4.199 0 01-2.675-.954c.034-.018.093-.05.132-.074l4.44-2.53a.71.71 0 00.364-.623v-6.176l1.877 1.069c.02.01.033.029.036.05v5.115c-.003 2.274-1.87 4.118-4.174 4.123zM4.192 17.78a4.059 4.059 0 01-.498-2.763c.032.02.09.055.131.078l4.44 2.53c.225.13.504.13.73 0l5.42-3.088v2.138a.068.068 0 01-.027.057L9.9 19.288c-1.999 1.136-4.552.46-5.707-1.51zm-1.169-9.564A4.15 4.15 0 015.198 6.41l-.002.151v5.06a.711.711 0 00.364.624l5.42 3.087-1.876 1.07a.067.067 0 01-.063.005l-4.489-2.559c-1.995-1.14-2.679-3.658-1.53-5.63zm15.417 3.54l-5.42-3.088L14.896 7.6a.067.067 0 01.063-.006l4.489 2.557c1.998 1.14 2.683 3.662 1.529 5.633a4.163 4.163 0 01-2.174 1.807V12.38a.71.71 0 00-.363-.623zm1.867-2.773a6.04 6.04 0 00-.132-.078l-4.44-2.53a.731.731 0 00-.729 0l-5.42 3.088V7.325a.068.068 0 01.027-.057L14.1 4.713c2-1.137 4.555-.46 5.707 1.513.487.833.664 1.809.499 2.757zM8.566 12.793l-1.877-1.068a.065.065 0 01-.036-.051V6.559c.001-2.277 1.873-4.122 4.181-4.12.976 0 1.92.338 2.671.954-.034.018-.092.05-.131.073l-4.44 2.53a.71.71 0 00-.365.623zm1.02-2.168L12 9.25l2.414 1.375v2.75L12 14.75l-2.415-1.375z";
  const listLimit = 20;
  const messageLimit = 20;
  const previewLimit = 60;
  const messagePreviewLimit = 200;
  // Primary anchor: the "more" (…) control of Codex's left icon rail. The three
  // legacy sidebar selectors stay as a fallback for other shell revisions.
  const legacyNavSelectors = [
    'nav[role="navigation"] button.sidebar-item',
    'aside.app-shell-left-panel button.sidebar-item',
    'nav[role="navigation"] button.h-token-nav-row.w-full',
  ];
  const railScopes = [
    'nav[role="navigation"]',
    'aside.app-shell-left-panel',
    '[class*="left-panel"]',
    '[class*="sidebar"]',
    "aside",
  ];
  const moreLabels = ["更多", "more", "⋯", "…", "...", "more options", "more actions"];
  const navRetryDelaysMs = [0, 300, 900, 2000, 4000, 7000, 11000];
  const sectionKeys = ["usage", "bots", "rooms", "messages"];
  const sectionTitles = {
    usage: "用量摘要",
    bots: "Bot 列表",
    rooms: "房间列表",
    messages: "最近消息",
  };
  const reasonTexts = {
    "missing-home": "数据源未就绪：未找到 DSH_HOME（需运行带 DS-Bot 的 DSH）",
    "missing-state": "数据源未就绪：未找到 bot/state.json（需运行带 DS-Bot 的 DSH）",
    "missing-database": "数据源未就绪：未找到 bot 数据库（需运行带 DS-Bot 的 DSH）",
    "parse-error": "数据源解析失败，请检查 DSH 版本后重试",
    busy: "数据源正忙，请稍后重试",
    "unsupported-schema": "数据源版本不受支持，请升级 CCP 或 DSH",
  };

  const state = {
    collapsed: readCollapsed(),
    mounted: false,
    loadedOnce: false,
    phase: "idle",
    reason: "",
    errorText: "",
    summary: null,
    bots: [],
    rooms: [],
    botsError: false,
    roomsError: false,
    selection: null,
    messages: [],
    messagePhase: "idle",
    requestSeq: 0,
    messageSeq: 0,
    timers: [],
    observer: null,
  };
  const refs = {
    navItem: null,
    view: null,
    meta: null,
    status: null,
    grid: null,
    usageBody: null,
    botsBody: null,
    roomsBody: null,
    messagesBody: null,
    botsCount: null,
    roomsCount: null,
    messagesTitle: null,
    backButton: null,
  };

  const panelCss = `
#codex-bot-nav-item { display: flex; align-items: center; justify-content: center; cursor: pointer; }
#codex-bot-nav-item .codex-bot-nav-icon {
  width: 18px; height: 18px; flex: 0 0 auto;
  color: inherit; opacity: .75;
}
#codex-bot-nav-item:hover .codex-bot-nav-icon,
#codex-bot-nav-item[data-active="true"] .codex-bot-nav-icon { opacity: 1; }
#codex-bot-view {
  position: fixed; inset: 0; z-index: 2147483000;
  display: flex; flex-direction: column; overflow: hidden;
  background: var(--color-token-bg-primary, var(--color-token-bg-secondary, #ffffff));
  color: var(--color-token-text-primary, var(--color-token-foreground, #1f2328));
  font: 400 13px/1.5 ui-sans-serif, system-ui, "Segoe UI", sans-serif;
}
#codex-bot-view[hidden] { display: none; }
#codex-bot-view .codex-bot-header {
  display: flex; align-items: center; gap: 8px; padding: 10px 14px;
  border-bottom: 1px solid var(--color-token-border-default, rgba(127, 127, 127, .25));
}
#codex-bot-view .codex-bot-title { font-size: 14px; font-weight: 650; letter-spacing: .01em; }
#codex-bot-view .codex-bot-meta {
  color: var(--color-token-text-secondary, #6b7280); font-size: 11px;
  max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
#codex-bot-view .codex-bot-header-actions { margin-left: auto; display: flex; gap: 6px; }
#codex-bot-view .codex-bot-button {
  border: 1px solid var(--color-token-border-default, rgba(127, 127, 127, .3));
  border-radius: 6px; padding: 4px 10px; cursor: pointer;
  background: var(--color-token-input-background, transparent);
  color: inherit; font: inherit;
}
#codex-bot-view .codex-bot-button:hover { background: var(--color-token-list-hover-background, rgba(127, 127, 127, .14)); }
#codex-bot-view .codex-bot-status { padding: 18px 14px; display: flex; flex-direction: column; gap: 8px; }
#codex-bot-view .codex-bot-status-title { font-weight: 600; }
#codex-bot-view .codex-bot-status-detail { color: var(--color-token-text-secondary, #6b7280); }
#codex-bot-view .codex-bot-retry { align-self: flex-start; }
#codex-bot-view .codex-bot-grid {
  flex: 1 1 auto; min-height: 0; overflow-y: auto; overscroll-behavior: contain;
  display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); align-content: start;
}
#codex-bot-view .codex-bot-section {
  min-width: 0; padding: 10px 0 14px;
  border-top: 1px solid var(--color-token-border-default, rgba(127, 127, 127, .18));
}
#codex-bot-view .codex-bot-section-head {
  display: flex; align-items: baseline; gap: 6px; padding: 2px 14px 6px;
  font-size: 11px; text-transform: uppercase; letter-spacing: .08em;
  color: var(--color-token-text-secondary, #6b7280);
}
#codex-bot-view .codex-bot-section-count { margin-left: auto; font-size: 10px; }
#codex-bot-view .codex-bot-row {
  display: flex; align-items: center; gap: 8px; width: 100%;
  padding: 7px 14px; border: 0; background: transparent; color: inherit;
  font: inherit; text-align: left; cursor: pointer;
}
#codex-bot-view .codex-bot-row:hover { background: var(--color-token-list-hover-background, rgba(127, 127, 127, .14)); }
#codex-bot-view .codex-bot-row[data-selected="true"] { background: var(--color-token-list-hover-background, rgba(127, 127, 127, .18)); }
#codex-bot-view .codex-bot-row-main { display: flex; flex-direction: column; gap: 1px; min-width: 0; flex: 1 1 auto; }
#codex-bot-view .codex-bot-row-name { font-weight: 600; }
#codex-bot-view .codex-bot-row-sub { color: var(--color-token-text-secondary, #6b7280); font-size: 11px; }
#codex-bot-view .codex-bot-row-usage { flex: 0 0 auto; color: var(--color-token-text-secondary, #6b7280); font-size: 11px; }
#codex-bot-view .codex-bot-ellipsis { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; display: block; }
#codex-bot-view .codex-bot-metric { display: flex; gap: 8px; padding: 2px 14px; }
#codex-bot-view .codex-bot-metric-label { min-width: 44px; color: var(--color-token-text-secondary, #6b7280); }
#codex-bot-view .codex-bot-bar-row { display: flex; align-items: center; gap: 8px; padding: 3px 14px; }
#codex-bot-view .codex-bot-bar-name { min-width: 72px; max-width: 96px; }
#codex-bot-view .codex-bot-bar { flex: 1 1 auto; height: 6px; border-radius: 3px; background: rgba(127, 127, 127, .2); overflow: hidden; }
#codex-bot-view .codex-bot-bar-fill { height: 100%; background: var(--ccp-theme-accent, #4b8bf5); }
#codex-bot-view .codex-bot-bar-value { flex: 0 0 auto; font-size: 11px; color: var(--color-token-text-secondary, #6b7280); }
#codex-bot-view .codex-bot-message { padding: 7px 14px; border-top: 1px solid var(--color-token-border-default, rgba(127, 127, 127, .12)); }
#codex-bot-view .codex-bot-message-head { display: flex; gap: 8px; font-size: 11px; color: var(--color-token-text-secondary, #6b7280); }
#codex-bot-view .codex-bot-message-who { font-weight: 600; color: inherit; }
#codex-bot-view .codex-bot-message-text { margin-top: 2px; white-space: pre-wrap; overflow-wrap: anywhere; }
#codex-bot-view .codex-bot-empty, #codex-bot-view .codex-bot-error {
  padding: 6px 14px; color: var(--color-token-text-secondary, #6b7280);
}
#codex-bot-view .codex-bot-error { color: var(--color-token-text-primary, #1f2328); }
#codex-bot-view .codex-bot-skeleton {
  height: 10px; margin: 6px 14px; border-radius: 5px;
  background: var(--color-token-list-hover-background, rgba(127, 127, 127, .18));
}
@media (prefers-color-scheme: dark) {
  html:not(.light):not([data-theme="light"]) #codex-bot-view {
    background: var(--color-token-bg-primary, #17191d);
    color: var(--color-token-text-primary, #e6e6e6);
  }
  html:not(.light):not([data-theme="light"]) #codex-bot-view .codex-bot-meta,
  html:not(.light):not([data-theme="light"]) #codex-bot-view .codex-bot-row-sub,
  html:not(.light):not([data-theme="light"]) #codex-bot-view .codex-bot-empty { color: var(--color-token-text-secondary, #9aa0a6); }
}
html.dark #codex-bot-view,
html[data-theme="dark"] #codex-bot-view,
:root[data-theme="dark"] #codex-bot-view {
  background: var(--color-token-bg-primary, #17191d);
  color: var(--color-token-text-primary, #e6e6e6);
}
`;

  // ---------------------------------------------------------------- helpers

  function safeText(value) {
    return value === undefined || value === null ? "" : String(value);
  }

  function truncate(value, max) {
    const text = safeText(value);
    if (text.length <= max) return text;
    return `${text.slice(0, Math.max(0, max - 1))}…`;
  }

  function toNumber(value) {
    const number = Number(value);
    return Number.isFinite(number) && number > 0 ? Math.round(number) : 0;
  }

  function formatTokens(value) {
    return String(toNumber(value)).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  }

  function normalizeUsage(usage) {
    const input = toNumber(usage && usage.input);
    const output = toNumber(usage && usage.output);
    const total = toNumber(usage && usage.total) || input + output;
    return { input, output, total };
  }

  function formatTime(value) {
    try {
      if (value === undefined || value === null || value === "") return "";
      const numeric = typeof value === "number" ? value : Number(value);
      if (Number.isFinite(numeric) && numeric > 0) {
        return new Date(numeric).toISOString().replace("T", " ").slice(0, 16);
      }
      return truncate(value, 24);
    } catch (error) {
      return "";
    }
  }

  function createEl(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined && text !== null) node.textContent = safeText(text);
    return node;
  }

  function clearNode(node) {
    if (!node) return;
    Array.from(node.children || []).forEach((child) => {
      try {
        child.remove();
      } catch (error) {
        try {
          node.removeChild(child);
        } catch (innerError) {
          /* ignore */
        }
      }
    });
    try {
      node.textContent = "";
    } catch (error) {
      /* ignore */
    }
  }

  function appendSkeletons(node, count) {
    for (let index = 0; index < count; index += 1) {
      node.appendChild(createEl("div", "codex-bot-skeleton"));
    }
  }

  function readCollapsed() {
    try {
      return window.localStorage.getItem(collapsedKey) !== "false";
    } catch (error) {
      return true;
    }
  }

  function writeCollapsed(collapsed) {
    try {
      window.localStorage.setItem(collapsedKey, collapsed ? "true" : "false");
    } catch (error) {
      /* storage may be unavailable; the entry button still works this session */
    }
  }

  // Inline OpenAI mark: currentColor, native icon sizing, never an image request.
  function buildBrandIcon() {
    try {
      if (typeof document.createElementNS !== "function") return createEl("span", "codex-bot-nav-icon");
      const svg = document.createElementNS(svgNamespace, "svg");
      svg.setAttribute("class", "codex-bot-nav-icon");
      svg.setAttribute("viewBox", "0 0 24 24");
      svg.setAttribute("aria-hidden", "true");
      svg.setAttribute("focusable", "false");
      svg.setAttribute("fill", "currentColor");
      svg.setAttribute("fill-rule", "evenodd");
      const path = document.createElementNS(svgNamespace, "path");
      path.setAttribute("d", brandIconPath);
      svg.appendChild(path);
      return svg;
    } catch (error) {
      return createEl("span", "codex-bot-nav-icon");
    }
  }

  // ------------------------------------------------------------- transport

  function helperGet(path) {
    // Codex 页面的 CSP 不允许直接 fetch helper（实测 Failed to fetch），
    // 注入后优先走 CCP 自带的 CDP 桥；桥不可用时才退回直连（手工注入/测试环境）。
    const bridge = window.__codexSessionDeleteBridge;
    if (typeof bridge === "function") {
      return Promise.resolve(bridge(path, {})).then((result) => {
        if (!result || result.status === "failed") throw new Error("helper request failed");
        return result;
      });
    }
    return fetch(`${helperBase}${path}`, {
      method: "GET",
      headers: withHelperToken({ Accept: "application/json" }),
      credentials: "omit",
    }).then((response) => {
      if (!response || !response.ok) throw new Error("helper request failed");
      return response.json();
    });
  }

  function unavailableTitle(reason) {
    return reasonTexts[reason] || "数据源未就绪（需运行带 DS-Bot 的 DSH）";
  }

  // -------------------------------------------------------------- mounting

  function moreButtonLabels(node) {
    return [
      node.getAttribute("aria-label"),
      node.getAttribute("title"),
      node.getAttribute("data-tooltip"),
      node.textContent,
    ]
      .filter(Boolean)
      .map((value) => String(value).trim().toLowerCase());
  }

  function isMoreButton(node) {
    if (!node || node.id === navItemId) return false;
    return moreButtonLabels(node).some(
      (label) => moreLabels.includes(label) || label.includes("更多") || label.startsWith("more "),
    );
  }

  function findMoreButton() {
    const seen = new Set();
    const scopes = railScopes.concat([""]);
    for (const scope of scopes) {
      let buttons = [];
      try {
        buttons = Array.from(document.querySelectorAll(`${scope} button`));
      } catch (error) {
        buttons = [];
      }
      for (const node of buttons) {
        if (seen.has(node) || node.nodeType !== 1) continue;
        seen.add(node);
        if (isMoreButton(node)) return node;
      }
    }
    return null;
  }

  function findNavAnchor() {
    const candidates = [findMoreButton()];
    for (const selector of legacyNavSelectors) {
      try {
        Array.from(document.querySelectorAll(selector)).forEach((node) => candidates.push(node));
      } catch (error) {
        /* ignore this selector */
      }
    }
    for (const node of candidates) {
      if (node && node.nodeType === 1 && node.parentElement) return node;
    }
    return null;
  }

  function installStyle() {
    if (!document.head || typeof document.createElement !== "function") return;
    const existing = typeof document.getElementById === "function" ? document.getElementById(styleId) : null;
    if (existing) return;
    const style = document.createElement("style");
    style.setAttribute("id", styleId);
    style.textContent = panelCss;
    document.head.appendChild(style);
  }

  function buildNavItem(anchor) {
    const item = document.createElement("button");
    item.setAttribute("id", navItemId);
    item.setAttribute("type", "button");
    item.setAttribute("title", "codex-bot");
    item.setAttribute("aria-label", "codex-bot");
    // Reuse the neighbouring rail button's own classes so it inherits Codex
    // hover / active tokens instead of restyling native controls.
    const anchorClass = safeText(anchor && anchor.className).trim();
    item.className = anchorClass ? `codex-bot-nav-item ${anchorClass}` : "codex-bot-nav-item";
    item.appendChild(buildBrandIcon());
    item.setAttribute("aria-expanded", "false");
    item.addEventListener("click", () => {
      try {
        toggleView();
      } catch (error) {
        /* panel-local failure only */
      }
    });
    return item;
  }

  function buildSection(key) {
    const section = createEl("section", "codex-bot-section");
    section.setAttribute("data-codex-bot-section", key);
    const head = createEl("div", "codex-bot-section-head");
    const title = createEl("span", "codex-bot-section-title", sectionTitles[key]);
    head.appendChild(title);
    const count = createEl("span", "codex-bot-section-count");
    head.appendChild(count);
    section.appendChild(head);
    const body = createEl("div", "codex-bot-section-body");
    body.setAttribute("data-codex-bot-body", key);
    section.appendChild(body);
    return { section, title, count, body };
  }

  function actionButton(label, action, handler) {
    const button = createEl("button", "codex-bot-button", label);
    button.setAttribute("type", "button");
    button.setAttribute("data-codex-bot-action", action);
    button.addEventListener("click", () => {
      try {
        handler();
      } catch (error) {
        /* panel-local failure only */
      }
    });
    return button;
  }

  function buildView() {
    const view = document.createElement("div");
    view.setAttribute("id", viewId);
    view.setAttribute("role", "region");
    view.setAttribute("aria-label", "codex-bot");

    const header = createEl("div", "codex-bot-header");
    header.appendChild(createEl("span", "codex-bot-title", "codex-bot"));
    const meta = createEl("span", "codex-bot-meta");
    header.appendChild(meta);
    const actions = createEl("div", "codex-bot-header-actions");
    actions.appendChild(actionButton("刷新", "refresh", () => void refresh()));
    const backButton = actionButton("返回聊天", "back", () => toggleView(true));
    actions.appendChild(backButton);
    header.appendChild(actions);
    view.appendChild(header);

    const status = createEl("div", "codex-bot-status");
    status.setAttribute("data-codex-bot-status", "idle");
    view.appendChild(status);

    const grid = createEl("div", "codex-bot-grid");
    const built = {};
    sectionKeys.forEach((key) => {
      const part = buildSection(key);
      built[key] = part;
      grid.appendChild(part.section);
    });
    view.appendChild(grid);

    refs.view = view;
    refs.meta = meta;
    refs.status = status;
    refs.grid = grid;
    refs.usageBody = built.usage.body;
    refs.botsBody = built.bots.body;
    refs.roomsBody = built.rooms.body;
    refs.messagesBody = built.messages.body;
    refs.botsCount = built.bots.count;
    refs.roomsCount = built.rooms.count;
    refs.messagesTitle = built.messages.title;
    refs.backButton = backButton;
    return view;
  }

  function mountIfPossible() {
    if (state.mounted) return true;
    try {
      const anchor = findNavAnchor();
      if (!anchor) return false;
      const container = anchor.parentElement;
      if (!container) return false;
      // Resolve the host before mutating anything, so a missing body can never
      // leave a half-mounted entry row behind for the retry loop.
      const host = document.body || document.documentElement;
      if (!host) return false;
      installStyle();
      const item = buildNavItem(anchor);
      if (typeof anchor.insertAdjacentElement === "function") {
        anchor.insertAdjacentElement("afterend", item);
      } else {
        container.appendChild(item);
      }
      refs.navItem = item;
      host.appendChild(buildView());

      state.mounted = true;
      if (state.observer) {
        try {
          state.observer.disconnect();
        } catch (error) {
          /* ignore */
        }
        state.observer = null;
      }
      render();
      if (!state.collapsed) void refresh();
      return true;
    } catch (error) {
      state.mounted = false;
      return false;
    }
  }

  function scheduleMount() {
    navRetryDelaysMs.forEach((delay) => {
      const timer = setTimeout(() => {
        try {
          mountIfPossible();
        } catch (error) {
          /* give up quietly after the last attempt */
        }
      }, delay);
      state.timers.push(timer);
    });
    try {
      if (typeof MutationObserver !== "function" || !document.body) return;
      state.observer = new MutationObserver(() => {
        try {
          if (state.mounted) return;
          mountIfPossible();
        } catch (error) {
          /* ignore */
        }
      });
      state.observer.observe(document.body, { childList: true, subtree: true });
    } catch (error) {
      state.observer = null;
    }
  }

  function dispose() {
    state.timers.forEach((timer) => {
      try {
        clearTimeout(timer);
      } catch (error) {
        /* ignore */
      }
    });
    state.timers = [];
    if (state.observer) {
      try {
        state.observer.disconnect();
      } catch (error) {
        /* ignore */
      }
      state.observer = null;
    }
    [refs.view, refs.navItem].forEach((node) => {
      try {
        node?.remove?.();
      } catch (error) {
        /* ignore */
      }
    });
    refs.view = null;
    refs.navItem = null;
    state.mounted = false;
  }

  // ---------------------------------------------------------------- render

  // The only place the view is switched: our own container is shown/hidden and
  // Codex's chat DOM is never touched, so returning is instant and lossless.
  function renderViewState() {
    try {
      if (refs.view) {
        refs.view.hidden = state.collapsed;
        refs.view.setAttribute("data-collapsed", state.collapsed ? "true" : "false");
        refs.view.setAttribute("aria-hidden", state.collapsed ? "true" : "false");
      }
      if (refs.navItem) {
        refs.navItem.setAttribute("data-active", state.collapsed ? "false" : "true");
        refs.navItem.setAttribute("aria-expanded", state.collapsed ? "false" : "true");
      }
    } catch (error) {
      /* ignore */
    }
  }

  function retryButton() {
    const button = actionButton("重试", "retry", () => void refresh());
    button.className = "codex-bot-button codex-bot-retry";
    return button;
  }

  function renderStatus() {
    const status = refs.status;
    if (!status) return;
    clearNode(status);
    const failed = state.phase === "unavailable" || state.phase === "error";
    if (state.phase === "unavailable") {
      status.setAttribute("data-codex-bot-status", "unavailable");
      status.appendChild(createEl("div", "codex-bot-status-title", unavailableTitle(state.reason)));
      if (state.reason) status.appendChild(createEl("div", "codex-bot-status-detail", `原因：${state.reason}`));
      status.appendChild(retryButton());
    } else if (state.phase === "error") {
      status.setAttribute("data-codex-bot-status", "error");
      status.appendChild(createEl("div", "codex-bot-status-title", state.errorText || "面板请求失败"));
      status.appendChild(createEl("div", "codex-bot-status-detail", "本地 helper 未响应；Codex 其余功能不受影响。"));
      status.appendChild(retryButton());
    } else {
      status.setAttribute("data-codex-bot-status", state.phase);
    }
    status.style.display = failed ? "" : "none";
    if (refs.grid) refs.grid.style.display = failed ? "none" : "";
    if (refs.view) refs.view.setAttribute("data-phase", state.phase);
    if (refs.meta) {
      const parts = [];
      if (state.summary && state.summary.revision !== undefined) parts.push(`rev ${safeText(state.summary.revision)}`);
      if (state.summary && state.summary.stale) parts.push("缓存");
      refs.meta.textContent = parts.join(" · ");
    }
  }

  function metricRow(label, usage) {
    const row = createEl("div", "codex-bot-metric");
    row.appendChild(createEl("span", "codex-bot-metric-label", label));
    row.appendChild(
      createEl(
        "span",
        "codex-bot-metric-value",
        `输入 ${formatTokens(usage.input)} · 输出 ${formatTokens(usage.output)} · 合计 ${formatTokens(usage.total)}`,
      ),
    );
    return row;
  }

  function rankedBots() {
    return state.bots
      .map((bot) => ({ bot, usage: normalizeUsage(bot && bot.usage) }))
      .sort((left, right) => right.usage.total - left.usage.total)
      .slice(0, 5);
  }

  function barRow(name, usage, max) {
    const row = createEl("div", "codex-bot-bar-row");
    const label = createEl("span", "codex-bot-bar-name codex-bot-ellipsis", name);
    label.setAttribute("title", name);
    row.appendChild(label);
    const bar = createEl("div", "codex-bot-bar");
    const fill = createEl("div", "codex-bot-bar-fill");
    fill.style.width = max > 0 && usage.total > 0 ? `${Math.max(4, Math.round((usage.total / max) * 100))}%` : "0%";
    bar.appendChild(fill);
    row.appendChild(bar);
    row.appendChild(createEl("span", "codex-bot-bar-value", formatTokens(usage.total)));
    return row;
  }

  function renderUsage() {
    const body = refs.usageBody;
    if (!body) return;
    clearNode(body);
    if (state.phase === "loading") {
      appendSkeletons(body, 2);
      return;
    }
    if (state.phase !== "ready") return;
    const summaryUsage = (state.summary && state.summary.usage) || {};
    body.appendChild(metricRow("今日", normalizeUsage(summaryUsage.today)));
    body.appendChild(metricRow("7 天", normalizeUsage(summaryUsage.week)));
    if (!state.bots.length) {
      body.appendChild(createEl("div", "codex-bot-empty", "暂无用量数据"));
      return;
    }
    const ranked = rankedBots();
    const max = ranked.length ? ranked[0].usage.total : 0;
    ranked.forEach((entry) => {
      body.appendChild(barRow(safeText(entry.bot && entry.bot.name) || "(未命名 Bot)", entry.usage, max));
    });
  }

  function botSubtitle(bot) {
    const parts = [];
    if (bot && bot.model) parts.push(safeText(bot.model));
    if (bot && bot.role) parts.push(safeText(bot.role));
    if (bot && bot.currentSessionId) parts.push(truncate(bot.currentSessionId, 18));
    if (bot && bot.messageCount !== undefined) parts.push(`${formatTokens(bot.messageCount)} 条消息`);
    if (bot && bot.lastActivityAt) parts.push(truncate(formatTime(bot.lastActivityAt), 16));
    else if (bot && bot.createdAt) parts.push(`创建于 ${truncate(formatTime(bot.createdAt), 16)}`);
    const mainBotId = state.summary && state.summary.mainBotId;
    if (mainBotId && bot && safeText(bot.id) === safeText(mainBotId)) parts.unshift("Main");
    return parts.filter(Boolean).join(" · ");
  }

  function renderBots() {
    const body = refs.botsBody;
    if (!body) return;
    clearNode(body);
    if (refs.botsCount) refs.botsCount.textContent = `${state.bots.length}`;
    if (state.phase === "loading") {
      appendSkeletons(body, 3);
      return;
    }
    if (state.phase !== "ready") return;
    if (state.botsError) {
      body.appendChild(createEl("div", "codex-bot-error", "Bot 列表加载失败"));
      return;
    }
    if (!state.bots.length) {
      body.appendChild(createEl("div", "codex-bot-empty", "暂无 Bot（DSH 尚未创建 Bot 团队）"));
      return;
    }
    state.bots.forEach((bot) => {
      const id = safeText(bot && bot.id);
      const name = safeText(bot && bot.name) || "(未命名 Bot)";
      const row = createEl("button", "codex-bot-row");
      row.setAttribute("type", "button");
      row.setAttribute("data-codex-bot-id", id);
      if (state.selection && state.selection.kind === "bot" && state.selection.id === id) {
        row.setAttribute("data-selected", "true");
      }
      const main = createEl("span", "codex-bot-row-main");
      const nameEl = createEl("span", "codex-bot-row-name codex-bot-ellipsis", name);
      nameEl.setAttribute("title", name);
      main.appendChild(nameEl);
      const subtitle = botSubtitle(bot);
      if (subtitle) {
        const subEl = createEl("span", "codex-bot-row-sub codex-bot-ellipsis", subtitle);
        subEl.setAttribute("title", subtitle);
        main.appendChild(subEl);
      }
      row.appendChild(main);
      row.appendChild(createEl("span", "codex-bot-row-usage", formatTokens(normalizeUsage(bot && bot.usage).total)));
      row.addEventListener("click", () => {
        try {
          void selectSession("bot", id, name);
        } catch (error) {
          /* panel-local failure only */
        }
      });
      body.appendChild(row);
    });
  }

  function roomPreview(room) {
    const parts = [];
    if (room && room.memberCount !== undefined) parts.push(`成员 ${formatTokens(room.memberCount)}`);
    if (room && room.lastMessageAt) parts.push(truncate(formatTime(room.lastMessageAt), 16));
    let line = parts.filter(Boolean).join(" · ");
    if (room && room.preview) {
      line = line ? `${line} · ${truncate(room.preview, previewLimit)}` : truncate(room.preview, previewLimit);
    }
    return line;
  }

  function renderRooms() {
    const body = refs.roomsBody;
    if (!body) return;
    clearNode(body);
    if (refs.roomsCount) refs.roomsCount.textContent = `${state.rooms.length}`;
    if (state.phase === "loading") {
      appendSkeletons(body, 2);
      return;
    }
    if (state.phase !== "ready") return;
    if (state.roomsError) {
      body.appendChild(createEl("div", "codex-bot-error", "房间列表加载失败"));
      return;
    }
    if (!state.rooms.length) {
      body.appendChild(createEl("div", "codex-bot-empty", "暂无房间"));
      return;
    }
    state.rooms.forEach((room) => {
      const id = safeText(room && room.id);
      const name = safeText(room && room.name) || "(未命名房间)";
      const row = createEl("button", "codex-bot-row");
      row.setAttribute("type", "button");
      row.setAttribute("data-codex-bot-room-id", id);
      if (state.selection && state.selection.kind === "room" && state.selection.id === id) {
        row.setAttribute("data-selected", "true");
      }
      const main = createEl("span", "codex-bot-row-main");
      const nameEl = createEl("span", "codex-bot-row-name codex-bot-ellipsis", name);
      nameEl.setAttribute("title", name);
      main.appendChild(nameEl);
      const subtitle = roomPreview(room);
      if (subtitle) {
        const subEl = createEl("span", "codex-bot-row-sub codex-bot-ellipsis", subtitle);
        subEl.setAttribute("title", subtitle);
        main.appendChild(subEl);
      }
      row.appendChild(main);
      row.addEventListener("click", () => {
        try {
          void selectSession("room", id, name);
        } catch (error) {
          /* panel-local failure only */
        }
      });
      body.appendChild(row);
    });
  }

  function messageRow(message) {
    const row = createEl("div", "codex-bot-message");
    const head = createEl("div", "codex-bot-message-head");
    head.appendChild(createEl("span", "codex-bot-message-who", safeText(message && message.who) || "?"));
    head.appendChild(createEl("span", "codex-bot-message-time", truncate(formatTime(message && message.time), 16)));
    if (message && message.seq !== undefined) {
      head.appendChild(createEl("span", "codex-bot-message-seq", `#${safeText(message.seq)}`));
    }
    row.appendChild(head);
    const text = safeText(message && message.text);
    const textEl = createEl("div", "codex-bot-message-text", truncate(text, messagePreviewLimit));
    textEl.setAttribute("title", truncate(text, messagePreviewLimit));
    row.appendChild(textEl);
    return row;
  }

  function renderMessages() {
    const body = refs.messagesBody;
    if (!body) return;
    clearNode(body);
    if (!state.selection) {
      if (refs.messagesTitle) refs.messagesTitle.textContent = sectionTitles.messages;
      body.appendChild(createEl("div", "codex-bot-empty", "选择上方的 Bot 或房间查看最近消息"));
      return;
    }
    if (refs.messagesTitle) {
      refs.messagesTitle.textContent = `${sectionTitles.messages} · ${truncate(state.selection.label, 24)}`;
    }
    if (state.phase === "loading" || state.messagePhase === "loading") {
      appendSkeletons(body, 3);
      return;
    }
    if (state.messagePhase === "error") {
      body.appendChild(createEl("div", "codex-bot-error", "消息加载失败或数据源未就绪（chat-index.sqlite）"));
      body.appendChild(retryButton());
      return;
    }
    if (!state.messages.length) {
      body.appendChild(createEl("div", "codex-bot-empty", "该会话暂无消息"));
      return;
    }
    state.messages.forEach((message) => body.appendChild(messageRow(message)));
  }

  function render() {
    try {
      renderViewState();
      renderStatus();
      renderUsage();
      renderBots();
      renderRooms();
      renderMessages();
    } catch (error) {
      /* a render failure must never escape into Codex's own UI */
    }
  }

  // ----------------------------------------------------------------- flow

  function toggleView(next) {
    try {
      const collapsed = typeof next === "boolean" ? next : !state.collapsed;
      state.collapsed = collapsed;
      writeCollapsed(collapsed);
      renderViewState();
      if (!collapsed && !state.loadedOnce) void refresh();
      return state.collapsed;
    } catch (error) {
      return state.collapsed;
    }
  }

  async function refresh() {
    const seq = state.requestSeq + 1;
    state.requestSeq = seq;
    state.loadedOnce = true;
    state.phase = "loading";
    state.errorText = "";
    render();
    let summary = null;
    try {
      summary = await helperGet("/codex-bot/summary");
    } catch (error) {
      if (seq !== state.requestSeq) return;
      state.phase = "error";
      state.errorText = "无法连接本地 helper，面板数据不可用";
      state.summary = null;
      state.bots = [];
      state.rooms = [];
      render();
      return;
    }
    if (seq !== state.requestSeq) return;
    state.summary = summary && typeof summary === "object" ? summary : {};
    if (state.summary.available === false) {
      state.phase = "unavailable";
      state.reason = safeText(state.summary.reason);
      state.bots = [];
      state.rooms = [];
      state.botsError = false;
      state.roomsError = false;
      render();
      return;
    }
    state.reason = "";
    state.phase = "ready";
    render();
    const [bots, rooms] = await Promise.all([
      helperGet(`/codex-bot/bots?limit=${listLimit}`).catch(() => null),
      helperGet(`/codex-bot/rooms?limit=${listLimit}`).catch(() => null),
    ]);
    if (seq !== state.requestSeq) return;
    state.botsError = bots === null;
    state.roomsError = rooms === null;
    state.bots = Array.isArray(bots) ? bots : [];
    state.rooms = Array.isArray(rooms) ? rooms : [];
    render();
  }

  async function selectSession(kind, id, label) {
    const sessionId = safeText(id);
    if (!sessionId) return;
    state.selection = { kind: safeText(kind) || "bot", id: sessionId, label: safeText(label) || sessionId };
    state.messagePhase = "loading";
    state.messages = [];
    render();
    const seq = state.messageSeq + 1;
    state.messageSeq = seq;
    let rows = [];
    try {
      rows = await helperGet(`/codex-bot/messages?session=${encodeURIComponent(sessionId)}&limit=${messageLimit}`);
    } catch (error) {
      if (seq !== state.messageSeq) return;
      state.messagePhase = "error";
      state.messages = [];
      render();
      return;
    }
    if (seq !== state.messageSeq) return;
    state.messages = Array.isArray(rows) ? rows : [];
    state.messagePhase = "ready";
    render();
  }

  function snapshot() {
    return {
      version: panelVersion,
      collapsed: state.collapsed,
      viewVisible: !state.collapsed,
      mounted: state.mounted,
      loadedOnce: state.loadedOnce,
      phase: state.phase,
      reason: state.reason,
      errorText: state.errorText,
      botCount: state.bots.length,
      roomCount: state.rooms.length,
      messageCount: state.messages.length,
      messagePhase: state.messagePhase,
      selection: state.selection ? { ...state.selection } : null,
      revision: state.summary && state.summary.revision !== undefined ? state.summary.revision : null,
      stale: Boolean(state.summary && state.summary.stale),
    };
  }

  window.__codexBotPanel = {
    version: panelVersion,
    viewId,
    panelId,
    navItemId,
    collapsedKey,
    mount: () => mountIfPossible(),
    show: () => toggleView(false),
    hide: () => toggleView(true),
    toggle: (next) => toggleView(next),
    refresh: () => refresh(),
    select: (kind, id, label) => selectSession(kind, id, label),
    readCollapsed: () => readCollapsed(),
    writeCollapsed: (value) => writeCollapsed(Boolean(value)),
    snapshot: () => snapshot(),
    dispose: () => dispose(),
  };

  function bootstrap() {
    try {
      installStyle();
      scheduleMount();
    } catch (error) {
      /* never let the panel break Codex's own renderer */
    }
  }

  if (document.readyState === "loading") {
    try {
      document.addEventListener("DOMContentLoaded", bootstrap, { once: true });
    } catch (error) {
      bootstrap();
    }
  } else {
    bootstrap();
  }
})();