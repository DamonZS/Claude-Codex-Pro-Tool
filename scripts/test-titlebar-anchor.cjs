const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(path.join(__dirname, "../assets/inject/renderer-inject.js"), "utf8");
const start = source.indexOf("  function visibleRectForCodexStatusAnchor(");
const end = source.indexOf("  function installClaudeCodexProMenu(", start);
assert.ok(start >= 0 && end > start);

// Execute the production positioning functions against CSS-pixel geometry.
// This tests the layout calculation, not a browser's font or WCO implementation.
function fixture({ width = 1280, zoom = 1, version = "dev-0.12.0", overlay = true, dom = false, windows = true } = {}) {
  const geometry = { width, zoom, version, overlay, dom };
  const style = new Map();
  const rect = (left, top, width, height) => ({ left, top, width, height, right: left + width, bottom: top + height });
  const titleWidth = () => Array.from(`CCP ${geometry.version}`).reduce((width, char) => width + (char === "." ? 3 : char === " " ? 4 : 7), 0);
  const menuLeft = () => parseFloat(style.get("--claude-codex-pro-menu-left")) || 0;
  const controlsRect = () => rect((geometry.width - 138) / geometry.zoom, 0, 138 / geometry.zoom, 36 / geometry.zoom);
  class Element {
    closest() { return null; }
    getAttribute(name) { return name === "aria-label" ? "Minimize" : null; }
    getBoundingClientRect() { return controlsRect(); }
    querySelector() { return null; }
  }
  const minimize = new Element();
  const title = {};
  const menu = {
    classList: { contains: () => true },
    style: { getPropertyValue: (key) => style.get(key) || "", setProperty: (key, value) => style.set(key, value) },
    querySelector: () => title,
    getBoundingClientRect: () => rect(menuLeft(), 0, titleWidth() + 24, 36 / geometry.zoom),
  };
  const rangeRect = () => rect(menuLeft() + 18, 0, titleWidth(), 13);
  const header = {
    getBoundingClientRect: () => rect(0, 36, geometry.width / geometry.zoom, 44),
    querySelectorAll: () => [],
  };
  const context = vm.createContext({
    Element,
    isExtensionUiNode: () => false,
    claudeCodexProMenuId: "ccp-test-menu",
    claudeCodexProMenuFloatingClass: "ccp-floating",
    selectors: { appHeader: "header" },
    window: { get innerWidth() { return geometry.width / geometry.zoom; } },
    navigator: {
      userAgent: windows ? "Windows" : "Macintosh",
      windowControlsOverlay: {
        get visible() { return geometry.overlay; },
        getTitlebarAreaRect: () => rect(0, 0, controlsRect().left, controlsRect().height),
      },
    },
    document: {
      querySelector: () => header,
      querySelectorAll: () => geometry.dom ? [minimize] : [],
      createRange: () => ({
        selectNodeContents: (node) => assert.equal(node, title),
        getBoundingClientRect: rangeRect,
      }),
    },
  });
  vm.runInContext(source.slice(start, end), context);
  return {
    geometry, style, menu, rangeRect, controlsRect,
    update: () => context.updateFloatingClaudeCodexProMenuPosition(menu),
  };
}

function assertAnchored(state) {
  state.update();
  assert.equal(state.style.get("visibility"), "visible");
  assert.ok(Math.abs(state.controlsRect().left - state.rangeRect().right - 8) < 0.001);
  assert.ok(state.menu.getBoundingClientRect().right < state.controlsRect().left);
  assert.equal(parseFloat(state.style.get("--claude-codex-pro-menu-top")), state.controlsRect().top);
  assert.equal(parseFloat(state.style.get("--claude-codex-pro-menu-height")), state.controlsRect().height);
}

test("version end stays eight CSS pixels before minimize across lengths, resize and zoom", () => {
  const state = fixture();
  for (const version of ["0.1", "dev-0.12.0", "2026.09.18.123456789"]) {
    for (const width of [800, 1280, 1920]) {
      for (const zoom of [0.75, 1, 1.25, 1.5, 2]) {
        Object.assign(state.geometry, { version, width, zoom });
        assertAnchored(state);
        const left = state.style.get("--claude-codex-pro-menu-left");
        assertAnchored(state);
        assert.equal(state.style.get("--claude-codex-pro-menu-left"), left, "repeated layout must not drift");
      }
    }
  }
});

test("DOM minimize is used when WCO is hidden, and missing anchor hides rather than overlaps", () => {
  const state = fixture({ overlay: false, dom: true });
  assertAnchored(state);
  state.geometry.dom = false;
  state.update();
  assert.equal(state.style.get("visibility"), "hidden");
  state.geometry.overlay = true;
  assertAnchored(state);
});

test("insufficient titlebar space hides the whole version and resizing restores it", () => {
  const state = fixture({ width: 400, zoom: 2, version: "2026.09.18.123456789" });
  state.update();
  assert.equal(state.style.get("visibility"), "hidden");
  state.geometry.width = 1280;
  assertAnchored(state);
});

test("macOS retains the existing header-row fallback", () => {
  const state = fixture({ windows: false, overlay: false });
  state.update();
  assert.equal(state.style.get("--claude-codex-pro-menu-top"), "36px");
  assert.equal(state.style.get("--claude-codex-pro-menu-height"), "44px");
  assert.equal(state.menu.getBoundingClientRect().right, 1280 - 16);
  assert.equal(state.style.has("visibility"), false);
});
