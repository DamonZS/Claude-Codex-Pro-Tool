import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

/* ── Bundled fonts (offline, no Google Fonts request) ──
     Fontsource packages ship woff2 files that Vite bundles into dist/.
     CSS @font-face declarations are injected at build time.              */
import "@fontsource/inter";
import "@fontsource/jetbrains-mono";

const app = document.getElementById("app");

function renderBootError(error: unknown) {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  document.body.innerHTML = `
    <main style="box-sizing:border-box;min-height:100vh;padding:28px;background:#fafafa;color:#171717;font-family:Inter,system-ui,sans-serif;">
      <section style="max-width:880px;margin:0 auto;border:1px solid #ddd;border-radius:8px;background:#fff;padding:20px;box-shadow:0 8px 28px rgba(0,0,0,.08);">
        <h1 style="margin:0 0 10px;font-size:20px;">管理工具前端启动失败</h1>
        <p style="margin:0 0 14px;color:#555;">页面没有继续显示为空黑屏，下面是启动错误。</p>
        <pre style="white-space:pre-wrap;overflow:auto;border-radius:6px;background:#111;color:#f4f4f5;padding:14px;font-size:12px;line-height:1.5;">${escapeHtml(message)}</pre>
      </section>
    </main>
  `;
}

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

if (app instanceof HTMLElement) {
  try {
    createRoot(app).render(<App />);
  } catch (error) {
    renderBootError(error);
  }
} else {
  renderBootError("index.html 中缺少 #app 根节点。");
}
