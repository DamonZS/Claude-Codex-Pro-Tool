// Unit tests for assets/inject/codex-bot-panel.js.
//
// The panel is a CDP-injected IIFE, so it is evaluated inside a node:vm context
// with hand-built window / document / localStorage / fetch stubs. Same style as
// codex-theme-loader.test.mjs: no jsdom, no new dependencies.
//
// Covers: mounting with and without the left-rail "more" (…) anchor, the entry
// button's position under that anchor, switching into the bot view without
// touching Codex's own chat DOM, returning to the chat view, the localStorage
// view key, the loading / empty / error / unavailable states, the locally
// observed single-bot + zero-room + empty-message data shape, zero usage, and
// hostile bot names that must only ever appear as text.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const SOURCE = readFileSync(fileURLToPath(new URL("./codex-bot-panel.js", import.meta.url)), "utf8");
const HELPER_BASE = "http://127.0.0.1:57321";
const TOKEN = "test-helper-token";
const TOKEN_HEADER = "x-claude-codex-pro-token";
const COLLAPSED_KEY = "codex-bot.panel.collapsed";
const NAV_ITEM_ID = "codex-bot-nav-item";
const VIEW_ID = "codex-bot-view";

const flush = () => new Promise((resolve) => setImmediate(resolve));

// --------------------------------------------------------------- DOM stubs

class ClassListStub {
  constructor(names = "") {
    this.values = new Set(String(names).split(/\s+/).filter(Boolean));
  }
  add(...names) {
    names.forEach((name) => this.values.add(String(name)));
  }
  remove(...names) {
    names.forEach((name) => this.values.delete(String(name)));
  }
  contains(name) {
    return this.values.has(String(name));
  }
  toggle(name, force) {
    const has = this.contains(name);
    const next = typeof force === "boolean" ? force : !has;
    if (next) this.add(name);
    else this.remove(name);
    return next;
  }
}

class StyleStub {
  constructor() {
    this.properties = {};
  }
  setProperty(name, value) {
    this.properties[String(name)] = String(value);
  }
  getPropertyValue(name) {
    return this.properties[String(name)] ?? "";
  }
  removeProperty(name) {
    delete this.properties[String(name)];
  }
}

function parseCompound(selector) {
  const attrs = [];
  let rest = String(selector).replace(/\[([^\]]+)\]/g, (_, body) => {
    const match = /^([^=]+?)(?:="?([^"\]]*)"?)?$/.exec(body.trim());
    if (match) attrs.push([match[1].trim(), match[2] === undefined ? null : match[2]]);
    return "";
  });
  const classes = [];
  rest = rest.replace(/\.([A-Za-z0-9_-]+)/g, (_, name) => {
    classes.push(name);
    return "";
  });
  let id = "";
  rest = rest.replace(/#([A-Za-z0-9_-]+)/g, (_, name) => {
    id = name;
    return "";
  });
  return { tag: rest.trim(), id, classes, attrs };
}

function matchesCompound(element, selector) {
  const { tag, id, classes, attrs } = parseCompound(selector);
  if (tag && element.tagName !== tag.toUpperCase()) return false;
  if (id && element.getAttribute("id") !== id) return false;
  if (classes.some((name) => !element.classList.contains(name))) return false;
  return attrs.every(([name, value]) =>
    value === null ? element.hasAttribute(name) : element.getAttribute(name) === value,
  );
}

function matchesSelector(element, selector) {
  if (!element || element.nodeType !== 1) return false;
  const parts = String(selector).trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return false;
  if (!matchesCompound(element, parts[parts.length - 1])) return false;
  let index = parts.length - 2;
  let node = element.parentNode;
  while (index >= 0 && node) {
    if (matchesCompound(node, parts[index])) index -= 1;
    node = node.parentNode;
  }
  return index < 0;
}

class ElementStub {
  constructor(tagName, ownerDocument) {
    this.nodeType = 1;
    this.tagName = String(tagName).toUpperCase();
    this.ownerDocument = ownerDocument;
    this.parentNode = null;
    this.children = [];
    this.childNodes = this.children;
    this.attributes = new Map();
    this.listeners = new Map();
    this.classList = new ClassListStub();
    this.style = new StyleStub();
    this.textContent = "";
    this._className = "";
    this._innerHTML = "";
    this._innerHTMLWrites = [];
  }
  get parentElement() {
    return this.parentNode && this.parentNode.nodeType === 1 ? this.parentNode : null;
  }
  get isConnected() {
    if (this === this.ownerDocument?.documentElement) return true;
    return Boolean(this.parentNode && this.parentNode.isConnected);
  }
  get firstChild() {
    return this.children[0] ?? null;
  }
  get nextSibling() {
    const parent = this.parentNode;
    if (!parent) return null;
    return parent.children[parent.children.indexOf(this) + 1] ?? null;
  }
  get previousSibling() {
    const parent = this.parentNode;
    if (!parent) return null;
    const index = parent.children.indexOf(this);
    return index > 0 ? parent.children[index - 1] : null;
  }
  get innerHTML() {
    return this._innerHTML;
  }
  set innerHTML(value) {
    this._innerHTML = String(value);
    this._innerHTMLWrites.push(String(value));
  }
  get className() {
    return this._className;
  }
  set className(value) {
    const next = String(value);
    this._className = next;
    this.classList = new ClassListStub(next);
    this.attributes.set("class", next);
  }
  get id() {
    return this.getAttribute("id") ?? "";
  }
  set id(value) {
    this.setAttribute("id", value);
  }
  appendChild(child) {
    if (child.parentNode) child.remove();
    child.parentNode = this;
    child.ownerDocument = this.ownerDocument;
    this.children.push(child);
    return child;
  }
  removeChild(child) {
    const index = this.children.indexOf(child);
    if (index >= 0) this.children.splice(index, 1);
    child.parentNode = null;
    return child;
  }
  remove() {
    if (this.parentNode) this.parentNode.removeChild(this);
  }
  insertAdjacentElement(position, element) {
    const parent = this.parentNode;
    if (!parent) throw new Error("insertAdjacentElement: no parent");
    const index = parent.children.indexOf(this);
    if (position === "afterend") {
      element.remove();
      element.parentNode = parent;
      parent.children.splice(index + 1, 0, element);
      return element;
    }
    if (position === "beforebegin") {
      element.remove();
      element.parentNode = parent;
      parent.children.splice(index, 0, element);
      return element;
    }
    throw new Error(`insertAdjacentElement: unsupported position ${position}`);
  }
  addEventListener(type, handler) {
    const list = this.listeners.get(type) ?? [];
    list.push(handler);
    this.listeners.set(type, list);
  }
  removeEventListener(type, handler) {
    const list = this.listeners.get(type) ?? [];
    this.listeners.set(type, list.filter((entry) => entry !== handler));
  }
  dispatchEvent(type, event = {}) {
    const list = this.listeners.get(type) ?? [];
    list.slice().forEach((handler) => handler({ type, target: this, ...event }));
  }
  setAttribute(name, value) {
    const key = String(name);
    this.attributes.set(key, String(value));
    if (key === "class") this.className = String(value);
  }
  getAttribute(name) {
    const key = String(name);
    return this.attributes.has(key) ? this.attributes.get(key) : null;
  }
  hasAttribute(name) {
    return this.attributes.has(String(name));
  }
  removeAttribute(name) {
    this.attributes.delete(String(name));
  }
  matches(selector) {
    return matchesSelector(this, selector);
  }
  closest(selector) {
    let node = this;
    while (node && node.nodeType === 1) {
      if (matchesSelector(node, selector)) return node;
      node = node.parentNode;
    }
    return null;
  }
  querySelector(selector) {
    return this.querySelectorAll(selector)[0] ?? null;
  }
  querySelectorAll(selector) {
    const out = [];
    const walk = (node) => {
      node.children.forEach((child) => {
        out.push(child);
        walk(child);
      });
    };
    walk(this);
    return out.filter((element) => matchesSelector(element, selector));
  }
}

class DocumentStub {
  constructor() {
    this.readyState = "complete";
    this.listeners = new Map();
    this.createdElements = [];
    this.documentElement = new ElementStub("html", this);
    this.head = new ElementStub("head", this);
    this.body = new ElementStub("body", this);
    this.documentElement.appendChild(this.head);
    this.documentElement.appendChild(this.body);
  }
  createElement(tagName) {
    const element = new ElementStub(tagName, this);
    this.createdElements.push(element);
    return element;
  }
  createElementNS(namespace, tagName) {
    const element = new ElementStub(tagName, this);
    element.namespaceURI = String(namespace);
    this.createdElements.push(element);
    return element;
  }
  createTextNode(text) {
    return { nodeType: 3, textContent: String(text) };
  }
  getElementById(id) {
    return this.querySelector(`#${id}`);
  }
  querySelector(selector) {
    return this.querySelectorAll(selector)[0] ?? null;
  }
  querySelectorAll(selector) {
    const self = matchesSelector(this.documentElement, selector) ? [this.documentElement] : [];
    return self.concat(this.documentElement.querySelectorAll(selector));
  }
  addEventListener(type, handler) {
    const list = this.listeners.get(type) ?? [];
    list.push(handler);
    this.listeners.set(type, list);
  }
  removeEventListener(type, handler) {
    const list = this.listeners.get(type) ?? [];
    this.listeners.set(type, list.filter((entry) => entry !== handler));
  }
}

function createLocalStorage() {
  const map = new Map();
  return {
    _map: map,
    getItem(key) {
      return map.has(String(key)) ? map.get(String(key)) : null;
    },
    setItem(key, value) {
      map.set(String(key), String(value));
    },
    removeItem(key) {
      map.delete(String(key));
    },
  };
}

// ----------------------------------------------------------- query helpers

function walk(root) {
  const out = [];
  const visit = (node) => {
    (node.children ?? []).forEach((child) => {
      out.push(child);
      visit(child);
    });
  };
  visit(root);
  return out;
}

function textOf(root) {
  return walk(root)
    .map((node) => node.textContent || "")
    .join(" ");
}

function byClass(root, className) {
  return walk(root).filter((node) => node.classList.contains(className));
}

function byAttribute(root, name, value) {
  return walk(root).filter((node) => node.getAttribute(name) === value);
}

// ------------------------------------------------------------- test harness

function summaryPayload(overrides = {}) {
  return {
    available: true,
    revision: 7,
    botCount: 1,
    roomCount: 0,
    mainBotId: "",
    stale: false,
    usage: {
      today: { input: 0, output: 0, total: 0 },
      week: { input: 0, output: 0, total: 0 },
    },
    ...overrides,
  };
}

// Mirrors the real local DSH state: one bot, no rooms, no chat database.
function localBot(overrides = {}) {
  return {
    id: "session-a589bf57-7a3c-4b6a-96dc-2abac9b444ab",
    name: "Chief",
    role: "MAIN",
    color: "deepseek",
    capabilities: ["chat"],
    createdAt: 1750000000000,
    usage: { input: 0, output: 0, total: 0 },
    ...overrides,
  };
}

function createHarness(options = {}) {
  const document = new DocumentStub();
  let anchor = null;
  if (options.anchor !== false) {
    const rail = document.createElement("nav");
    rail.setAttribute("role", "navigation");
    // The icon rail's "more" (…) control: the codex-bot entry goes right below it.
    anchor = document.createElement("button");
    anchor.className = "rail-icon-button no-drag";
    anchor.setAttribute("aria-label", "More");
    anchor.textContent = "…";
    rail.appendChild(anchor);
    // A later rail row, so "inserted directly after" is distinguishable from "appended".
    rail.appendChild(document.createElement("button"));
    document.body.appendChild(rail);
  }
  // Codex's own chat surface: it must survive every view switch untouched.
  const chat = document.createElement("main");
  chat.className = "native-chat";
  chat.textContent = "native chat transcript";
  document.body.appendChild(chat);

  const storage = options.storage ?? createLocalStorage();
  if (options.collapsed !== undefined) {
    storage.setItem(COLLAPSED_KEY, options.collapsed ? "true" : "false");
  }

  const calls = [];
  const flags = { fail: Boolean(options.fetchThrows) };
  const routes = { "/codex-bot/summary": summaryPayload(), ...(options.routes || {}) };

  const fetchStub = async (input, init = {}) => {
    const url = String(input);
    calls.push({ url, method: init.method || "GET", headers: init.headers || {} });
    if (flags.fail) throw new Error("helper unreachable");
    const path = url.startsWith(HELPER_BASE) ? url.slice(HELPER_BASE.length) : url;
    const key = path.split("?")[0];
    if (!Object.prototype.hasOwnProperty.call(routes, key)) {
      throw new Error(`unexpected request ${path}`);
    }
    const route = routes[key];
    const payload = typeof route === "function" ? route(path) : route;
    if (payload === undefined) throw new Error(`no payload for ${path}`);
    return { ok: true, status: 200, json: async () => payload };
  };

  const timers = [];
  const windowStub = {
    localStorage: storage,
    __CODEX_SESSION_DELETE_HELPER__: HELPER_BASE,
    __CLAUDE_CODEX_PRO_HELPER_TOKEN__: TOKEN,
  };

  const bridgeCalls = [];
  if (options.bridge) {
    windowStub.__codexSessionDeleteBridge = (path, payload) => {
      bridgeCalls.push({ path, payload });
      return Promise.resolve().then(() => options.bridge(path, payload));
    };
  }

  const context = vm.createContext({
    window: windowStub,
    document,
    localStorage: storage,
    fetch: fetchStub,
    setTimeout: (fn, delay) => {
      const timer = { fn, delay };
      timers.push(timer);
      return timer;
    },
    clearTimeout: (timer) => {
      const index = timers.indexOf(timer);
      if (index >= 0) timers.splice(index, 1);
    },
    console: { log() {}, warn() {}, error() {} },
  });
  vm.runInContext(SOURCE, context, { filename: "codex-bot-panel.js" });

  const api = windowStub.__codexBotPanel;
  assert.ok(api, "the script must expose window.__codexBotPanel");

  return {
    api,
    document,
    storage,
    calls,
    bridgeCalls,
    flags,
    routes,
    anchor,
    chat,
    window: windowStub,
    timers,
    runTimers() {
      const pending = timers.splice(0, timers.length);
      pending.forEach((timer) => timer.fn());
    },
    panel() {
      // The codex-bot view container (older assertions call it the panel).
      return document.getElementById(VIEW_ID);
    },
    moreButton() {
      return anchor;
    },
    viewHidden() {
      const view = document.getElementById(VIEW_ID);
      return view ? view.hidden : null;
    },
    actionButton(action) {
      const view = document.getElementById(VIEW_ID);
      if (!view) return null;
      return (
        byClass(view, "codex-bot-button").find(
          (button) => button.getAttribute("data-codex-bot-action") === action,
        ) ?? null
      );
    },
    navItem() {
      return document.getElementById(NAV_ITEM_ID);
    },
    body(key) {
      const panel = document.getElementById(VIEW_ID);
      return panel ? byAttribute(panel, "data-codex-bot-body", key)[0] ?? null : null;
    },
    statusNode() {
      const panel = document.getElementById(VIEW_ID);
      return panel ? panel.querySelector(".codex-bot-status") : null;
    },
    retryButtons() {
      const panel = document.getElementById(VIEW_ID);
      return panel ? byClass(panel, "codex-bot-retry") : [];
    },
    createdNodeNames() {
      return document.createdElements.map((element) => element.tagName);
    },
    innerHTMLWrites() {
      return [...document.createdElements, document.documentElement, document.head, document.body]
        .flatMap((element) => element._innerHTMLWrites || []);
    },
    text() {
      const panel = document.getElementById(VIEW_ID);
      return panel ? textOf(panel) : "";
    },
  };
}

// ------------------------------------------------------------------- tests

test("mounts the rail entry right under the more (…) button and the bot view", () => {
  const harness = createHarness();

  assert.equal(harness.api.mount(), true);
  const navItem = harness.navItem();
  assert.ok(navItem, "the codex-bot entry button must be inserted next to the more control");
  assert.ok(harness.panel(), "the codex-bot view container must be mounted");

  assert.equal(navItem.tagName, "BUTTON");
  assert.equal(navItem.previousSibling, harness.moreButton(), "the entry sits directly below the more (…) button");
  assert.equal(
    navItem.parentNode,
    harness.moreButton().parentNode,
    "the entry lives in the same icon rail as the more button",
  );
  assert.notEqual(navItem.nextSibling, null, "the entry is not merely appended to the end");
  assert.ok(
    navItem.classList.contains("rail-icon-button"),
    "the entry reuses the rail button classes instead of restyling native controls",
  );
  assert.equal(navItem.getAttribute("aria-label"), "codex-bot");
  assert.ok(navItem.querySelector("svg"), "the entry carries an inline SVG icon");
  assert.equal(navItem.querySelector("path").getAttribute("d").length > 100, true, "the icon is a real path");
  assert.equal(
    navItem.querySelector("img"),
    null,
    "the icon never loads an external image",
  );

  const view = harness.panel();
  assert.equal(view.tagName, "DIV");
  assert.equal(view.getAttribute("role"), "region");
  assert.ok(view.querySelector(".codex-bot-header"), "the view has a header row");
  ["usage", "bots", "rooms", "messages"].forEach((key) => {
    assert.ok(harness.body(key), `section ${key} must be present`);
  });
  const style = harness.document.getElementById("codex-bot-panel-style");
  assert.ok(style, "the view stylesheet must be installed in head");
  assert.ok(String(style.textContent).includes("--color-token-text-primary"), "the view reads Codex theme tokens");
  assert.match(
    String(style.textContent),
    /background-color: var\(--bt-main, #fcfcfc\);\s*background-image: linear-gradient\(/,
    "the view paints an opaque base before layering the host token, so a translucent token cannot show the native UI through",
  );
  assert.match(
    String(style.textContent),
    /:root\[data-theme="dark"\] #codex-bot-view \{\s*background-color: #17191d;/,
    "the dark theme paints the same opaque base",
  );
  assert.equal(harness.api.snapshot().mounted, true);
});

test("inserts the rail entry directly after the more (…) anchor and stays idempotent", () => {
  const harness = createHarness();
  harness.api.mount();

  const more = harness.moreButton();
  assert.ok(more, "the harness provides the rail's more (…) control");
  assert.equal(more.getAttribute("aria-label"), "More");

  const item = harness.navItem();
  assert.equal(more.nextSibling, item, "the entry is the more button's very next sibling");
  assert.equal(item.previousSibling, more);
  const railChildren = more.parentNode.children;
  assert.equal(railChildren[0], more, "the more button is still the first rail row");
  assert.equal(railChildren[1], item, "the entry takes the slot right below it");

  // Idempotent: a second mount and every scheduled retry add nothing.
  assert.equal(harness.api.mount(), true);
  harness.runTimers();
  assert.equal(harness.document.querySelectorAll(`#${NAV_ITEM_ID}`).length, 1, "one entry row in total");
  assert.equal(harness.document.querySelectorAll(`#${VIEW_ID}`).length, 1, "one view container in total");
  assert.equal(more.nextSibling, item, "the entry stays put after the retries");
});

test("clicking the entry switches to the bot view while Codex's chat DOM stays intact", async () => {
  const harness = createHarness({ collapsed: true, routes: { "/codex-bot/bots": [], "/codex-bot/rooms": [] } });
  harness.api.mount();
  assert.equal(harness.viewHidden(), true, "the rail entry starts on Codex's own chat view");

  const chat = harness.chat;
  const chatParent = chat.parentNode;
  harness.navItem().dispatchEvent("click");
  await flush();
  await flush();

  assert.equal(harness.viewHidden(), false, "clicking the entry shows the bot view");
  assert.equal(harness.api.snapshot().viewVisible, true);
  assert.equal(harness.storage.getItem(COLLAPSED_KEY), "false", "the view choice is persisted");
  assert.equal(harness.panel().getAttribute("data-phase"), "ready", "entering the view loads the data");
  ["usage", "bots", "rooms", "messages"].forEach((key) => {
    assert.ok(harness.body(key), `the bot view keeps its ${key} section`);
  });

  assert.equal(chat.parentNode, chatParent, "the native chat node keeps its parent");
  assert.equal(chat.isConnected, true, "the native chat node is still in the DOM");
  assert.equal(chat.textContent, "native chat transcript", "the native chat content is untouched");
  assert.equal(harness.document.querySelectorAll(".native-chat").length, 1, "no chat node was re-created");
});

test("the 返回聊天 control hides the bot view and leaves the chat DOM alone", async () => {
  const harness = createHarness({ collapsed: false, routes: { "/codex-bot/bots": [], "/codex-bot/rooms": [] } });
  harness.api.mount();
  await harness.api.refresh();
  assert.equal(harness.viewHidden(), false);

  const chat = harness.chat;
  const backButton = harness.actionButton("back");
  assert.ok(backButton, "the view header carries the 返回聊天 control");
  assert.equal(backButton.textContent, "返回聊天");
  backButton.dispatchEvent("click");
  await flush();

  assert.equal(harness.viewHidden(), true, "返回聊天 hides the bot view");
  assert.equal(harness.storage.getItem(COLLAPSED_KEY), "true", "the switch is persisted");
  assert.equal(harness.api.snapshot().viewVisible, false);
  assert.equal(harness.panel().getAttribute("data-collapsed"), "true");
  assert.equal(harness.navItem().getAttribute("data-active"), "false", "the rail entry drops its active state");
  assert.equal(chat.isConnected, true, "the native chat node survives the round trip");
  assert.equal(chat.textContent, "native chat transcript");
});

test("does not throw and stays unmounted when the sidebar anchor is missing", () => {
  const harness = createHarness({ anchor: false });

  assert.equal(harness.api.mount(), false);
  assert.equal(harness.navItem(), null);
  assert.equal(harness.panel(), null);
  assert.equal(harness.api.snapshot().mounted, false);

  // The scheduled retries must also stay quiet instead of throwing.
  assert.doesNotThrow(() => harness.runTimers());
  assert.equal(harness.panel(), null);
  assert.equal(harness.calls.length, 0, "no helper request without a mounted panel");
});

test("persists the view state in localStorage and restores it on the next run", () => {
  const storage = createLocalStorage();

  const first = createHarness({ storage, collapsed: false });
  first.api.mount();
  assert.equal(first.viewHidden(), false, "the stored false shows the bot view");
  assert.equal(first.panel().getAttribute("data-collapsed"), "false");
  assert.equal(first.navItem().getAttribute("aria-expanded"), "true");

  const backButton = first.actionButton("back");
  assert.ok(backButton, "the view must expose a 返回聊天 control");
  assert.equal(backButton.textContent, "返回聊天");
  backButton.dispatchEvent("click");

  assert.equal(storage.getItem(COLLAPSED_KEY), "true", "returning writes the documented key");
  assert.equal(first.api.readCollapsed(), true);
  assert.equal(first.viewHidden(), true);
  assert.equal(first.panel().getAttribute("data-collapsed"), "true");
  assert.equal(first.panel().getAttribute("aria-hidden"), "true");

  const second = createHarness({ storage });
  second.api.mount();
  assert.equal(second.api.readCollapsed(), true, "the stored value is read back");
  assert.equal(second.viewHidden(), true);

  // Entering the bot view again flips the same key back to "false".
  second.api.toggle(false);
  assert.equal(storage.getItem(COLLAPSED_KEY), "false");
  assert.equal(second.viewHidden(), false);
  assert.equal(second.panel().getAttribute("data-collapsed"), "false");
});

test("renders the empty state when a ready summary reports zero bots and zero rooms", async () => {
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 0, roomCount: 0 }),
      "/codex-bot/bots": [],
      "/codex-bot/rooms": [],
    },
  });
  harness.api.mount();
  assert.equal(harness.panel().getAttribute("data-phase"), "loading");
  const skeletons = byClass(harness.panel(), "codex-bot-skeleton");
  assert.ok(skeletons.length > 0, "the first load shows skeletons");

  await harness.api.refresh();
  const snapshot = harness.api.snapshot();
  assert.equal(snapshot.phase, "ready");
  assert.equal(snapshot.botCount, 0);
  assert.equal(snapshot.roomCount, 0);

  const botsText = textOf(harness.body("bots"));
  const roomsText = textOf(harness.body("rooms"));
  assert.ok(botsText.includes("暂无 Bot"), `bot empty state, got: ${botsText}`);
  assert.ok(roomsText.includes("暂无房间"), `room empty state, got: ${roomsText}`);
  assert.equal(byClass(harness.panel(), "codex-bot-row").length, 0, "empty state adds no rows");
  assert.equal(byClass(harness.panel(), "codex-bot-skeleton").length, 0, "skeletons are replaced");
  assert.equal(harness.statusNode().style.display, "none", "a ready panel hides the status block");
});

test("shows the unavailable state with a retry control when available is false", async () => {
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ available: false, reason: "missing-database", revision: 0 }),
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const snapshot = harness.api.snapshot();
  assert.equal(snapshot.phase, "unavailable");
  assert.equal(snapshot.reason, "missing-database");

  const status = harness.statusNode();
  assert.equal(status.getAttribute("data-codex-bot-status"), "unavailable");
  const statusText = textOf(status);
  assert.ok(statusText.includes("未找到 bot 数据库"), `not-ready copy, got: ${statusText}`);
  assert.ok(statusText.includes("missing-database"), "the raw reason is shown for support");
  assert.equal(harness.retryButtons().length, 1, "exactly one retry button");

  const before = harness.calls.length;
  harness.retryButtons()[0].dispatchEvent("click");
  await flush();
  assert.ok(harness.calls.length > before, "retry re-requests /codex-bot/summary");
  assert.equal(harness.calls[harness.calls.length - 1].url, `${HELPER_BASE}/codex-bot/summary`);
});

test("shows the error state when the helper is unreachable and recovers on retry", async () => {
  const harness = createHarness({
    collapsed: false,
    fetchThrows: true,
    routes: { "/codex-bot/bots": [], "/codex-bot/rooms": [] },
  });
  harness.api.mount();
  await harness.api.refresh();

  let snapshot = harness.api.snapshot();
  assert.equal(snapshot.phase, "error");
  const status = harness.statusNode();
  assert.equal(status.getAttribute("data-codex-bot-status"), "error");
  const statusText = textOf(status);
  assert.ok(statusText.includes("无法连接本地 helper"), `error copy, got: ${statusText}`);
  assert.ok(statusText.includes("Codex 其余功能不受影响"), "the failure is scoped to the panel");
  assert.equal(harness.retryButtons().length, 1);
  assert.equal(harness.panel().getAttribute("data-collapsed"), "false", "the panel itself still works");

  harness.flags.fail = false;
  harness.retryButtons()[0].dispatchEvent("click");
  await flush();
  await flush();
  snapshot = harness.api.snapshot();
  assert.equal(snapshot.phase, "ready", "retrying after the helper comes back reaches ready");
  assert.equal(snapshot.botCount, 0);
});

test("renders the locally observed single bot shape without undefined text", async () => {
  const bot = localBot();
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 1, roomCount: 0, mainBotId: bot.id }),
      "/codex-bot/bots": [bot],
      "/codex-bot/rooms": [],
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const botsBody = harness.body("bots");
  const rows = byClass(botsBody, "codex-bot-row");
  assert.equal(rows.length, 1, "one bot renders one row");
  assert.equal(rows[0].getAttribute("data-codex-bot-id"), bot.id);

  const botsText = textOf(botsBody);
  assert.ok(botsText.includes("Chief"));
  assert.ok(botsText.includes("MAIN"), "role is used as the subtitle");
  assert.ok(botsText.includes("Main"), "the main bot is marked via summary.mainBotId");
  assert.ok(!botsText.includes("undefined"), `no undefined leaks into the row: ${botsText}`);
  assert.ok(!botsText.includes("workDir"), "the bot row does not depend on a workDir field");

  const counts = byClass(harness.panel(), "codex-bot-section-count").map((node) => node.textContent);
  assert.deepEqual(counts, ["", "1", "0", ""], "section counters show 1 bot and 0 rooms");

  // Rooms are empty but that is an empty state, not an error.
  const roomsBody = harness.body("rooms");
  assert.ok(textOf(roomsBody).includes("暂无房间"));
  assert.equal(byClass(roomsBody, "codex-bot-error").length, 0, "zero rooms must not be an error");
  const requests = harness.calls.map((call) => call.url);
  assert.ok(requests.includes(`${HELPER_BASE}/codex-bot/bots?limit=20`));
  assert.ok(requests.includes(`${HELPER_BASE}/codex-bot/rooms?limit=20`));
  assert.equal(harness.calls[0].headers[TOKEN_HEADER], TOKEN, "every helper call carries the token header");
  assert.equal(harness.calls[0].method, "GET");
});

test("clicking a bot loads its recent messages and renders the empty-message state", async () => {
  const bot = localBot({ name: "Chief", currentSessionId: "session-a589bf57" });
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ mainBotId: bot.id }),
      "/codex-bot/bots": [bot],
      "/codex-bot/rooms": [],
      "/codex-bot/messages": (path) => {
        assert.ok(path.includes(`session=${encodeURIComponent(bot.id)}`), `session id must be encoded: ${path}`);
        assert.ok(path.includes("limit=20"), `message limit must be 20: ${path}`);
        return [];
      },
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const row = byClass(harness.body("bots"), "codex-bot-row")[0];
  row.dispatchEvent("click");
  await flush();
  await flush();

  const snapshot = harness.api.snapshot();
  assert.equal(snapshot.messagePhase, "ready");
  assert.equal(snapshot.selection.kind, "bot");
  assert.equal(snapshot.selection.id, bot.id);
  assert.equal(snapshot.messageCount, 0);

  const messagesBody = harness.body("messages");
  const messagesText = textOf(messagesBody);
  assert.ok(messagesText.includes("该会话暂无消息"), `empty message copy, got: ${messagesText}`);
  assert.equal(byClass(messagesBody, "codex-bot-message").length, 0);
  assert.equal(byClass(messagesBody, "codex-bot-error").length, 0, "an empty message list is not an error");
  assert.ok(textOf(harness.panel()).includes("Chief"), "the messages section is labelled with the selection");
  assert.equal(snapshot.phase, "ready", "the summary phase is untouched by message loading");
});

test("renders zero usage as 0 instead of NaN or blank", async () => {
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({
        usage: { today: { input: 0, output: 0, total: 0 }, week: { input: 0, output: 0, total: 0 } },
      }),
      "/codex-bot/bots": [],
      "/codex-bot/rooms": [],
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const usageText = textOf(harness.body("usage"));
  assert.ok(usageText.includes("输入 0 · 输出 0 · 合计 0"), `zero usage copy, got: ${usageText}`);
  assert.ok(usageText.includes("今日"));
  assert.ok(usageText.includes("7 天"));
  assert.ok(!usageText.includes("NaN"), "no NaN in the usage block");
  assert.equal(usageText.replace(/[^A-Za-z]/g, "").includes("undefined"), false, "no undefined in the usage block");
});

test("renders hostile bot and room names as text only, never as nodes or innerHTML", async () => {
  const hostileName = '<img src=x onerror=alert(1)>';
  const hostileText = '<script>alert("x")</script>';
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 1, roomCount: 1 }),
      "/codex-bot/bots": [localBot({ name: hostileName })],
      "/codex-bot/rooms": [{ id: "room-1", name: hostileName, memberCount: 2, preview: hostileText }],
      "/codex-bot/messages": [{ seq: 1, time: 1750000000000, who: hostileName, text: hostileText }],
    },
  });
  harness.api.mount();
  await harness.api.refresh();
  await harness.api.select("bot", "session-a589bf57-7a3c-4b6a-96dc-2abac9b444ab", hostileName);
  await flush();

  const panel = harness.panel();
  const rendered = textOf(panel);
  assert.ok(rendered.includes(hostileName), "the raw name is preserved as text");
  assert.ok(rendered.includes(hostileText), "the raw message text is preserved as text");

  const nodes = walk(panel);
  assert.equal(
    nodes.filter((node) => ["IMG", "SCRIPT", "IFRAME", "SVG"].includes(node.tagName)).length,
    0,
    "no element may be created from injected markup",
  );
  assert.equal(
    nodes.filter((node) => node.hasAttribute("onerror") || node.hasAttribute("onload")).length,
    0,
    "no event-handler attribute may come from data",
  );
  assert.ok(
    !harness.createdNodeNames().includes("IMG"),
    "the script never even creates an img element",
  );
  assert.deepEqual(harness.innerHTMLWrites(), [], "innerHTML is never assigned anywhere");
  assert.equal(byClass(panel, "codex-bot-row-name")[0].textContent, hostileName);
  assert.equal(byClass(panel, "codex-bot-message-text")[0].textContent, hostileText);
});

test("keeps the panel wiring idempotent and never touches native controls on dispose", async () => {
  const harness = createHarness({ collapsed: false });
  harness.api.mount();
  const anchor = harness.anchor;
  assert.equal(harness.api.mount(), true);
  assert.equal(harness.document.querySelectorAll(`#${NAV_ITEM_ID}`).length, 1, "mounting twice adds one row");
  assert.equal(harness.document.documentElement.querySelectorAll(`#${VIEW_ID}`).length, 1);

  harness.api.dispose();
  assert.equal(harness.panel(), null);
  assert.equal(harness.navItem(), null);
  assert.equal(harness.api.snapshot().mounted, false);
  assert.ok(anchor.isConnected && anchor.parentNode.tagName === "NAV", "the native anchor is left untouched");
  assert.equal(anchor.className, "rail-icon-button no-drag", "the native anchor classes are unchanged");
});

test("reads codex-bot data through the CDP bridge when the page cannot fetch the helper", async () => {
  const bot = localBot();
  const harness = createHarness({
    collapsed: false,
    fetchThrows: true,
    bridge: (path) => {
      const key = path.split("?")[0];
      if (key === "/codex-bot/summary") {
        return summaryPayload({ botCount: 1, roomCount: 0, mainBotId: bot.id });
      }
      if (key === "/codex-bot/bots") return [bot];
      if (key === "/codex-bot/rooms") return [];
      throw new Error(`unexpected bridge path ${path}`);
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  assert.equal(harness.calls.length, 0, "with the bridge present the panel never touches fetch");
  const paths = harness.bridgeCalls.map((call) => call.path);
  assert.ok(paths.includes("/codex-bot/summary"), `summary goes through the bridge: ${paths}`);
  assert.ok(paths.includes("/codex-bot/bots?limit=20"), `bots goes through the bridge: ${paths}`);
  assert.ok(paths.includes("/codex-bot/rooms?limit=20"), `rooms goes through the bridge: ${paths}`);
  assert.equal(harness.api.snapshot().phase, "ready", "bridge data reaches the ready state");
  assert.equal(byClass(harness.body("bots"), "codex-bot-row").length, 1, "bridge data renders like helper data");
  assert.equal(byClass(harness.panel(), "codex-bot-error").length, 0, "no error state with a working bridge");
});

test("keeps the error state when the bridge itself reports failure", async () => {
  const harness = createHarness({
    collapsed: false,
    fetchThrows: true,
    bridge: () => ({ status: "failed", message: "数据源未就绪" }),
  });
  harness.api.mount();
  await harness.api.refresh();

  assert.equal(harness.api.snapshot().phase, "error", "a failed bridge payload is not rendered as data");
  assert.equal(harness.statusNode().getAttribute("data-codex-bot-status"), "error");
  assert.equal(harness.panel().getAttribute("data-collapsed"), "false", "the panel keeps working");
});

// ------------------------------------------------- DS-Bot interface port
// Every block below cites the DS-Bot client source it was ported from, so a
// later shell redesign can re-check the original instead of the panel.

test("rebuilds DS-Bot's sidebar head: brand row, round refresh, 返回聊天 pill", async () => {
  // ds-bot/src/client/sidebar.js:655 (bt-brand-whale + BrandName), styles.js:119-122
  // (.bt-side-top, .bt-brand, .bt-round).
  const harness = createHarness({ collapsed: false });
  harness.api.mount();
  await harness.api.refresh();

  const view = harness.panel();
  assert.ok(byClass(view, "bt-side-top").length >= 1, "the head row uses DS-Bot's bt-side-top");
  const brand = byClass(view, "bt-brand")[0];
  assert.ok(brand, "the brand row exists");
  assert.equal(brand.textContent, "DS Bot", "the brand name is DS-Bot's own");
  assert.equal(byClass(view, "bt-brand-whale").length, 1, "the whale mark keeps its own slot");

  const refresh = harness.actionButton("refresh");
  assert.ok(refresh.classList.contains("bt-round"), "刷新 is a bt-round control");
  const back = harness.actionButton("back");
  assert.ok(back.classList.contains("bt-pill-btn"), "返回聊天 keeps its own words as a pill");
  assert.equal(back.textContent, "返回聊天");
});

test("renders every Bot as a DS-Bot bt-row with its ink mark, role chip and preview", async () => {
  // ds-bot/src/client/sidebar.js:543 (bt-row / bt-row-body / bt-row-name /
  // bt-row-preview), styles.js:138-147, inks.js INKS deepseek.
  const bot = localBot();
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 1, roomCount: 0, mainBotId: bot.id }),
      "/codex-bot/bots": [bot],
      "/codex-bot/rooms": [],
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const botsBody = harness.body("bots");
  assert.equal(byClass(botsBody, "bt-row").length, 1, "the Bot row is a bt-row");
  assert.equal(byClass(botsBody, "bt-row-body").length, 1);
  assert.equal(byClass(botsBody, "bt-row-title")[0].textContent, "Chief");
  assert.ok(byClass(botsBody, "bt-row-preview")[0].textContent.includes("MAIN"), "the role reads as the preview");
  assert.equal(byClass(botsBody, "bt-chip")[0].textContent, "主 Bot", "the Main Bot wears DS-Bot's chip");

  const mark = byClass(botsBody, "bt-mark")[0];
  const initial = byClass(mark, "bt-mark-initial")[0];
  assert.equal(initial.textContent, "C", "the mark shows the Bot's initial");
  assert.equal(initial.style.color, "var(--bt-ink-deepseek)", "the mark is painted in the Bot's own ink");
  assert.equal(
    harness.createdNodeNames().includes("IMG"),
    false,
    "the mark is text, never an image request",
  );

  await harness.api.select("bot", bot.id, "Chief");
  assert.equal(
    byClass(harness.body("bots"), "bt-row")[0].getAttribute("aria-current"),
    "page",
    "the open session is marked with aria-current, as DS-Bot does",
  );
});

test("lays the view out as DS-Bot's bt-side list beside the chat column", async () => {
  // ds-bot/src/client/sidebar.js:251 (.bt-side) and app.js's sidebar + chat split.
  const harness = createHarness({ collapsed: false });
  harness.api.mount();
  await harness.api.refresh();

  const grid = byClass(harness.panel(), "codex-bot-grid")[0];
  const side = byClass(grid, "bt-side")[0];
  assert.ok(side, "the list column is a bt-side");
  const main = byClass(grid, "codex-bot-pane-main")[0];
  assert.ok(main, "the transcript column exists");
  ["usage", "bots", "rooms"].forEach((key) => {
    assert.ok(
      byAttribute(side, "data-codex-bot-body", key).length === 1,
      `${key} lives in the bt-side column`,
    );
  });
  assert.equal(
    byAttribute(main, "data-codex-bot-body", "messages").length,
    1,
    "messages live in the chat column",
  );
});

test("renders the usage section as DS-Bot's bt-us stat cards and ranked card", async () => {
  // ds-bot/src/client/usage-page.js:151-231 (.bt-us-stats / .bt-us-stat /
  // .bt-us-card / .bt-us-row / .bt-us-track) and styles.js:16 tokens.
  const bot = localBot({ usage: { input: 1200, output: 340, total: 1540 } });
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({
        botCount: 1,
        roomCount: 0,
        usage: {
          today: { input: 1200, output: 340, total: 1540 },
          week: { input: 9000, output: 1000, total: 10000 },
        },
      }),
      "/codex-bot/bots": [bot],
      "/codex-bot/rooms": [],
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const body = harness.body("usage");
  assert.equal(byClass(body, "bt-us-stats").length, 1, "the stat block is a bt-us-stats card");
  assert.deepEqual(
    byClass(body, "bt-us-stat-label").map((node) => node.textContent),
    ["今日", "7 天"],
    "the two ranges keep DS-Bot's labels",
  );
  assert.ok(byClass(body, "bt-us-stat-value")[0].textContent.startsWith("1,540"), "the total is the headline number");
  assert.ok(
    textOf(byClass(body, "bt-us-stat-value")[0]).includes("输入 1,200 · 输出 340 · 合计 1,540"),
    "the input/output split stays readable",
  );
  assert.equal(byClass(body, "bt-us-card").length, 1, "the ranking is a bt-us-card");
  assert.equal(byClass(body, "bt-us-row").length, 1, "each ranked Bot is a bt-us-row");
  const ranked = byClass(body, "bt-us-row-num")[0];
  assert.equal(ranked.children[0].textContent, "1,540", "the ranked total");
  assert.equal(ranked.children[1].textContent, "100%", "its share of the ranking");
  assert.equal(
    byClass(body, "bt-us-track")[0].children[0].style.width,
    "100%",
    "the track fill is relative to the top Bot",
  );
});

test("renders room rows with a bt-cluster and the member chip", async () => {
  // ds-bot/src/client/sidebar.js:514 (group rows), mark.js:138 RoomAvatar,
  // i18n.js t('{count} members') = '{count} 位成员'.
  const room = {
    id: "room-1",
    name: "研究组",
    memberCount: 3,
    lastMessageAt: 1750000000000,
    preview: "今天先对齐一下口径",
  };
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 0, roomCount: 1 }),
      "/codex-bot/bots": [],
      "/codex-bot/rooms": [room],
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  const roomsBody = harness.body("rooms");
  assert.equal(byClass(roomsBody, "bt-row").length, 1);
  assert.equal(byClass(roomsBody, "bt-row-title")[0].textContent, "研究组");
  assert.equal(byClass(roomsBody, "bt-cluster").length, 1, "a group seats its members in a cluster");
  assert.equal(byClass(roomsBody, "bt-chip")[0].textContent, "3 位成员");
  assert.ok(
    byClass(roomsBody, "bt-row-preview")[0].textContent.includes("今天先对齐一下口径"),
    "the room preview is shown",
  );
});

test("renders messages as DS-Bot bubbles and the user's own turn as a bt-ububble", async () => {
  // ds-bot/src/client/cells.js:69 (bt-msg / bt-msg-line / bt-bubble),
  // :413 (bt-urow / bt-ububble), :520 (bt-time-sep).
  const bot = localBot();
  const rows = [
    { who: "Chief", text: "第一条", time: 1750000000000, seq: 1 },
    { who: "你", text: "收到", time: 1750000100000, seq: 2 },
  ];
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 1, roomCount: 0, mainBotId: bot.id }),
      "/codex-bot/bots": [bot],
      "/codex-bot/rooms": [],
      "/codex-bot/messages": (path) => (path.includes(encodeURIComponent(bot.id)) ? rows : []),
    },
  });
  harness.api.mount();
  await harness.api.refresh();
  await harness.api.select("bot", bot.id, "Chief");

  const body = harness.body("messages");
  const bubbles = byClass(body, "bt-bubble");
  assert.equal(bubbles.length, 1, "the Bot turn is a bt-bubble");
  assert.equal(bubbles[0].textContent, "第一条");
  assert.equal(
    byClass(body, "codex-bot-message-text")[0],
    bubbles[0],
    "the test-facing hook stays on the bubble itself",
  );
  assert.equal(byClass(body, "bt-group-name").length, 1, "the Bot turn keeps a bt-group-name head");
  assert.equal(byClass(body, "bt-msg-line").length, 1);
  assert.equal(byClass(body, "bt-urow").length, 1, "the user's turn is right-aligned");
  assert.equal(byClass(body, "bt-ububble")[0].textContent, "收到");
  const seps = byClass(body, "bt-time-sep");
  assert.equal(seps.length, 1, "one day separator for one day");
  assert.match(seps[0].textContent, /^\d{4}-\d{2}-\d{2}$/);
});

test("keeps DS-Bot's own empty copy in every block", async () => {
  // i18n.js t('Setting up your Main Bot…'), sidebar.js:296 .bt-hint,
  // header.js:109 t('New chat').
  const harness = createHarness({
    collapsed: false,
    routes: {
      "/codex-bot/summary": summaryPayload({ botCount: 0, roomCount: 0 }),
      "/codex-bot/bots": [],
      "/codex-bot/rooms": [],
      "/codex-bot/messages": [],
    },
  });
  harness.api.mount();
  await harness.api.refresh();

  assert.ok(textOf(harness.body("bots")).includes("暂无 Bot"));
  assert.equal(
    byClass(harness.body("bots"), "bt-hint")[0].textContent,
    "正在设置你的主 Bot…",
    "the sidebar hint is DS-Bot's copy",
  );
  assert.ok(textOf(harness.body("rooms")).includes("暂无房间"));
  assert.equal(
    byClass(harness.panel(), "codex-bot-section-title")[3].textContent,
    "新对话",
    "with nothing open the chat head reads 新对话",
  );
  // An open session with no rows yet keeps DS-Bot's own empty copy.
  await harness.api.select("bot", "session-empty", "空会话");
  assert.ok(textOf(harness.body("messages")).includes("该会话暂无消息"));
  assert.equal(
    byClass(harness.panel(), "codex-bot-section-title")[3].textContent,
    "空会话",
    "the chat head wears the open session's name, as bt-pill-name does",
  );
});