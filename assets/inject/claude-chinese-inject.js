(function () {
  "use strict";

  window.__CLAUDE_CODEX_PRO_CHINESE_INJECTED = true;
  document.documentElement.dataset.claudeCodexProChineseInjected = "true";

  const TEXT = new Map([
    ["New chat", "新建对话"],
    ["New session", "新建会话"],
    ["Chats", "对话"],
    ["Projects", "项目"],
    ["Artifacts", "产物"],
    ["Recents", "最近"],
    ["Settings", "设置"],
    ["Customize", "自定义"],
    ["Inference configuration", "推理配置"],
    ["Profile", "个人资料"],
    ["Account", "账号"],
    ["Billing", "账单"],
    ["Plans", "套餐"],
    ["Appearance", "外观"],
    ["Language", "语言"],
    ["Connectors", "连接器"],
    ["Skills", "技能"],
    ["Plugins", "插件"],
    ["Personal plugins", "个人插件"],
    ["Browse plugins", "浏览插件"],
    ["Organization plugins", "组织插件"],
    ["MCP servers", "MCP 服务器"],
    ["MCP server", "MCP 服务器"],
    ["Developer", "开发者"],
    ["Advanced", "高级"],
    ["General", "通用"],
    ["Notifications", "通知"],
    ["Privacy", "隐私"],
    ["Help", "帮助"],
    ["Log out", "退出登录"],
    ["Sign in", "登录"],
    ["Continue", "继续"],
    ["Send", "发送"],
    ["Stop", "停止"],
    ["Retry", "重试"],
    ["Cancel", "取消"],
    ["Save", "保存"],
    ["Done", "完成"],
    ["Edit", "编辑"],
    ["Delete", "删除"],
    ["Remove", "移除"],
    ["Install", "安装"],
    ["Installed", "已安装"],
    ["Enable", "启用"],
    ["Enabled", "已启用"],
    ["Disable", "禁用"],
    ["Disabled", "已禁用"],
    ["Search", "搜索"],
    ["Search plugins", "搜索插件"],
    ["No plugins found", "未找到插件"],
    ["Choose where Claude sends inference requests.", "选择 Claude 将推理请求发送到哪里。"],
    ["Local stdio servers", "本地 stdio 服务器"],
    ["Remote servers", "远程服务器"],
    ["Allow user-added MCP servers", "允许用户添加 MCP 服务器"],
    ["Message Claude", "给 Claude 发消息"],
    ["What can I help you with today?", "今天我能帮你什么？"],
  ]);

  const ATTRS = ["aria-label", "title", "placeholder", "value"];
  const queue = [];
  let scheduled = false;

  function translateText(value) {
    const trimmed = value.trim();
    if (!trimmed) return value;
    const direct = TEXT.get(trimmed);
    if (direct) return value.replace(trimmed, direct);
    let next = value;
    for (const [from, to] of TEXT.entries()) {
      if (next.includes(from)) next = next.split(from).join(to);
    }
    return next;
  }

  function translateNode(node) {
    if (!node) return;
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
    if (!root) return;
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

  function ensureStatusPill() {
    if (document.getElementById("ccp-claude-status-pill")) return;
    const pill = document.createElement("div");
    pill.id = "ccp-claude-status-pill";
    pill.dataset.claudeCodexProInjected = "true";
    pill.innerHTML = [
      '<span class="ccp-dot"></span>',
      '<strong>Claude 中文窗口</strong>',
      '<span>后端：包装 WebView</span>',
      '<button type="button" id="ccp-plugin-hub-button">插件中心</button>',
    ].join("");
    document.documentElement.appendChild(pill);
    const button = pill.querySelector("#ccp-plugin-hub-button");
    if (button) {
      button.addEventListener("click", () => {
        window.dispatchEvent(new CustomEvent("claude-codex-pro-plugin-hub-open"));
        if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === "function") {
          window.__TAURI_INTERNALS__.invoke("open_plugin_hub_window").catch(() => {
            window.location.href = "claude-codex-pro://plugin-hub";
          });
          return;
        }
        window.location.href = "claude-codex-pro://plugin-hub";
      });
    }
  }

  function installStyles() {
    if (document.getElementById("ccp-claude-chinese-style")) return;
    const style = document.createElement("style");
    style.id = "ccp-claude-chinese-style";
    style.textContent = `
      #ccp-claude-status-pill {
        position: fixed;
        top: 10px;
        left: 50%;
        transform: translateX(-50%);
        z-index: 2147483647;
        display: inline-flex;
        align-items: center;
        gap: 8px;
        height: 34px;
        padding: 0 10px;
        border: 1px solid rgba(22, 163, 74, 0.32);
        border-radius: 999px;
        background: rgba(255, 255, 255, 0.94);
        color: #102016;
        box-shadow: 0 10px 30px rgba(15, 23, 42, 0.14);
        font: 12px/1.2 "Segoe UI", "Microsoft YaHei", sans-serif;
        backdrop-filter: blur(12px);
      }
      #ccp-claude-status-pill .ccp-dot {
        width: 8px;
        height: 8px;
        border-radius: 999px;
        background: #16a34a;
        box-shadow: 0 0 0 4px rgba(22, 163, 74, 0.16);
      }
      #ccp-plugin-hub-button {
        height: 24px;
        border: 0;
        border-radius: 999px;
        padding: 0 10px;
        background: #0f766e;
        color: #fff;
        font: inherit;
        cursor: pointer;
      }
      #ccp-plugin-hub-button:hover {
        background: #115e59;
      }
    `;
    document.documentElement.appendChild(style);
  }

  function bootstrap() {
    installStyles();
    ensureStatusPill();
    enqueue(document.documentElement);
    installObserver();
    window.setInterval(() => {
      ensureStatusPill();
      enqueue(document.body || document.documentElement);
    }, 1500);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", bootstrap, { once: true });
  } else {
    bootstrap();
  }
})();
