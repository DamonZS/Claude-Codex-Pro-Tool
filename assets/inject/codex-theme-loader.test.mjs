import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const LOADER_GLOBAL = "__CLAUDE_CODEX_PRO_CODEX_THEME_LOADER__";
const PAYLOAD_GLOBAL = "__CLAUDE_CODEX_PRO_CODEX_THEME_PAYLOAD__";
const RESULT_GLOBAL = "__CLAUDE_CODEX_PRO_CODEX_THEME_RESULT__";
const STYLE_ID = "claude-codex-pro-codex-theme";
const STYLE_SELECTOR = `style[id="${STYLE_ID}"]`;
const ASSET_VARIABLE = "--ccp-theme-art";
const CSS_VARIABLE = "--ccp-theme-accent";
const ROOT_ATTRIBUTE = "data-ccp-theme-surface";
const THEME_ID_ATTRIBUTE = "data-ccp-codex-theme-id";
const THEME_GENERATION_ATTRIBUTE = "data-ccp-codex-theme-generation";
const LOADER_SOURCE = readFileSync(
  fileURLToPath(new URL("./codex-theme-loader.js", import.meta.url)),
  "utf8",
);

class CSSStyleDeclarationStub {
  constructor(owner) {
    this.owner = owner;
    this.values = new Map();
    this.priorities = new Map();
  }

  get length() {
    return this.values.size;
  }

  item(index) {
    return [...this.values.keys()][index] ?? "";
  }

  getPropertyValue(name) {
    return this.values.get(name) ?? "";
  }

  getPropertyPriority(name) {
    return this.priorities.get(name) ?? "";
  }

  setProperty(name, value, priority = "") {
    this.values.set(String(name), String(value));
    this.priorities.set(String(name), String(priority));
    this.owner.syncStyleAttribute();
  }

  removeProperty(name) {
    const previous = this.getPropertyValue(name);
    this.values.delete(name);
    this.priorities.delete(name);
    this.owner.syncStyleAttribute();
    return previous;
  }

  replaceFromAttribute(cssText) {
    this.values.clear();
    this.priorities.clear();
    for (const declaration of String(cssText).split(";")) {
      const colon = declaration.indexOf(":");
      if (colon < 0) {
        continue;
      }
      const name = declaration.slice(0, colon).trim();
      let value = declaration.slice(colon + 1).trim();
      if (!name) {
        continue;
      }
      let priority = "";
      if (/\s*!important$/i.test(value)) {
        value = value.replace(/\s*!important$/i, "").trim();
        priority = "important";
      }
      this.values.set(name, value);
      this.priorities.set(name, priority);
    }
  }

  serialize() {
    return [...this.values.entries()]
      .map(([name, value]) => {
        const priority = this.priorities.get(name);
        return `${name}: ${value}${priority ? ` !${priority}` : ""};`;
      })
      .join(" ");
  }
}

class ClassListStub {
  constructor() {
    this.values = new Set();
  }

  add(...names) {
    for (const name of names) {
      this.values.add(String(name));
    }
  }

  remove(...names) {
    for (const name of names) {
      this.values.delete(String(name));
    }
  }

  contains(name) {
    return this.values.has(String(name));
  }
}

class ElementStub {
  constructor(tagName, ownerDocument) {
    this.tagName = String(tagName).toUpperCase();
    this.ownerDocument = ownerDocument;
    this.parentNode = null;
    this.children = [];
    this.attributes = new Map();
    this.classList = new ClassListStub();
    this.style = new CSSStyleDeclarationStub(this);
    this.textContent = "";
  }

  get id() {
    return this.getAttribute("id") ?? "";
  }

  set id(value) {
    this.setAttribute("id", value);
  }

  get isConnected() {
    return this === this.ownerDocument.documentElement
      || Boolean(this.parentNode?.isConnected);
  }

  appendChild(child) {
    child.remove();
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  remove() {
    if (!this.parentNode) {
      return;
    }
    const index = this.parentNode.children.indexOf(this);
    if (index >= 0) {
      this.parentNode.children.splice(index, 1);
    }
    this.parentNode = null;
  }

  hasAttribute(name) {
    return this.attributes.has(String(name));
  }

  getAttribute(name) {
    return this.attributes.get(String(name)) ?? null;
  }

  setAttribute(name, value) {
    const normalizedName = String(name);
    const normalizedValue = String(value);
    this.attributes.set(normalizedName, normalizedValue);
    if (normalizedName === "style") {
      this.style.replaceFromAttribute(normalizedValue);
    }
  }

  removeAttribute(name) {
    const normalizedName = String(name);
    this.attributes.delete(normalizedName);
    if (normalizedName === "style") {
      this.style.replaceFromAttribute("");
    }
  }

  syncStyleAttribute() {
    this.attributes.set("style", this.style.serialize());
  }
}

class DocumentStub {
  constructor() {
    this.documentElement = new ElementStub("html", this);
    this.head = new ElementStub("head", this);
    this.documentElement.appendChild(this.head);
  }

  createElement(tagName) {
    return new ElementStub(tagName, this);
  }

  querySelectorAll(selector) {
    const match = /^style\[id="([^"]+)"\]$/.exec(selector);
    if (!match) {
      return [];
    }
    const matches = [];
    const visit = (element) => {
      if (element.tagName === "STYLE" && element.id === match[1] && element.isConnected) {
        matches.push(element);
      }
      for (const child of element.children) {
        visit(child);
      }
    };
    visit(this.documentElement);
    return matches;
  }
}

class BlobStub {
  constructor(parts = [], options = {}) {
    this.type = String(options.type ?? "");
    this.size = parts.reduce((total, part) => {
      if (typeof part === "string") {
        return total + Buffer.byteLength(part);
      }
      if (typeof part?.byteLength === "number") {
        return total + part.byteLength;
      }
      if (typeof part?.size === "number") {
        return total + part.size;
      }
      return total;
    }, 0);
  }
}

function themePayload({
  themeId = "theme-a",
  generation = 1,
  accent = "#0a84ff",
  assetBase64 = "dGhlbWUtYQ==",
} = {}) {
  return {
    theme_id: themeId,
    generation,
    css: `:root[data-ccp-codex-theme-id="${themeId}"] { color: var(${CSS_VARIABLE}); }`,
    css_variables: {
      [CSS_VARIABLE]: accent,
    },
    root_attributes: {
      classes: [`ccp-${themeId}`],
      attributes: {
        [ROOT_ATTRIBUTE]: themeId,
      },
    },
    asset_variables: {
      [ASSET_VARIABLE]: `data:image/png;base64,${assetBase64}`,
    },
  };
}

function defaultPayload(generation) {
  return {
    theme_id: "default",
    generation,
  };
}

function ownedStyle(document) {
  const styles = document.querySelectorAll(STYLE_SELECTOR);
  assert.equal(styles.length, 1, "exactly one owned theme style must remain");
  return styles[0];
}

function plain(value) {
  return JSON.parse(JSON.stringify(value));
}

function createHarness(initialPayload) {
  const document = new DocumentStub();
  const events = [];
  const revoked = [];
  const created = [];
  let nextObjectUrl = 1;
  let createFailuresRemaining = 0;

  const domSnapshot = () => {
    const style = document.querySelectorAll(STYLE_SELECTOR)[0] ?? null;
    return {
      styleThemeId: style?.getAttribute("data-codex-theme-id") ?? null,
      rootThemeId: document.documentElement.getAttribute(THEME_ID_ATTRIBUTE),
      assetValue: document.documentElement.style.getPropertyValue(ASSET_VARIABLE),
    };
  };

  const url = {
    createObjectURL(blob) {
      if (createFailuresRemaining > 0) {
        createFailuresRemaining -= 1;
        events.push({ type: "create_failed", dom: domSnapshot() });
        throw new Error("createObjectURL fixture failure");
      }
      const objectUrl = `blob:ccp-theme-test/${nextObjectUrl}`;
      nextObjectUrl += 1;
      created.push(objectUrl);
      events.push({
        type: "create",
        url: objectUrl,
        blobSize: blob.size,
        dom: domSnapshot(),
      });
      return objectUrl;
    },
    revokeObjectURL(objectUrl) {
      revoked.push(objectUrl);
      events.push({ type: "revoke", url: objectUrl, dom: domSnapshot() });
    },
  };

  const window = {
    [PAYLOAD_GLOBAL]: initialPayload,
  };
  window.window = window;
  window.document = document;

  const context = vm.createContext({
    Blob: BlobStub,
    URL: url,
    atob(value) {
      return Buffer.from(String(value), "base64").toString("binary");
    },
    document,
    window,
  });
  vm.runInContext(LOADER_SOURCE, context, {
    filename: fileURLToPath(new URL("./codex-theme-loader.js", import.meta.url)),
  });

  return {
    created,
    document,
    events,
    initialResult: window[RESULT_GLOBAL],
    loader: window[LOADER_GLOBAL],
    revoked,
    failNextCreateObjectURL(count = 1) {
      createFailuresRemaining = count;
    },
  };
}

test("applies a theme successfully on first load", () => {
  const payload = themePayload();
  const harness = createHarness(payload);
  const { document, initialResult } = harness;
  const root = document.documentElement;
  const style = ownedStyle(document);
  const asset = initialResult.assetObjects[ASSET_VARIABLE];

  assert.equal(initialResult.ok, true);
  assert.equal(initialResult.status, "applied");
  assert.equal(initialResult.active, true);
  assert.equal(initialResult.themeId, payload.theme_id);
  assert.equal(style.textContent, payload.css);
  assert.equal(style.getAttribute("data-codex-theme-id"), payload.theme_id);
  assert.equal(root.style.getPropertyValue(CSS_VARIABLE), payload.css_variables[CSS_VARIABLE]);
  assert.equal(root.style.getPropertyValue(ASSET_VARIABLE), asset.cssValue);
  assert.equal(root.getAttribute(ROOT_ATTRIBUTE), payload.theme_id);
  assert.equal(root.getAttribute(THEME_ID_ATTRIBUTE), payload.theme_id);
  assert.equal(root.getAttribute(THEME_GENERATION_ATTRIBUTE), String(payload.generation));
  assert.equal(root.classList.contains(`ccp-${payload.theme_id}`), true);
  assert.deepEqual(harness.created, [asset.objectUrl]);
  assert.deepEqual(harness.revoked, []);
});

test("keeps the old theme intact when createObjectURL fails during a switch", () => {
  const oldPayload = themePayload();
  const nextPayload = themePayload({
    themeId: "theme-b",
    generation: 2,
    accent: "#10b981",
    assetBase64: "dGhlbWUtYg==",
  });
  const harness = createHarness(oldPayload);
  const root = harness.document.documentElement;
  const oldAsset = harness.initialResult.assetObjects[ASSET_VARIABLE];

  harness.failNextCreateObjectURL();
  const result = harness.loader.apply(nextPayload);

  assert.equal(result.ok, false);
  assert.equal(result.status, "failed");
  assert.equal(harness.loader.snapshot().themeId, oldPayload.theme_id);
  assert.equal(harness.loader.snapshot().active, true);
  assert.equal(ownedStyle(harness.document).textContent, oldPayload.css);
  assert.equal(root.getAttribute(ROOT_ATTRIBUTE), oldPayload.theme_id);
  assert.equal(root.getAttribute(THEME_ID_ATTRIBUTE), oldPayload.theme_id);
  assert.equal(root.style.getPropertyValue(CSS_VARIABLE), oldPayload.css_variables[CSS_VARIABLE]);
  assert.equal(root.style.getPropertyValue(ASSET_VARIABLE), oldAsset.cssValue);
  assert.equal(root.classList.contains(`ccp-${oldPayload.theme_id}`), true);
  assert.deepEqual(harness.created, [oldAsset.objectUrl]);
  assert.equal(harness.revoked.includes(oldAsset.objectUrl), false);
});

test("repairs same-payload DOM and CSS drift while reusing the existing Blob", () => {
  const payload = themePayload();
  const harness = createHarness(payload);
  const root = harness.document.documentElement;
  const style = ownedStyle(harness.document);
  const oldAsset = harness.initialResult.assetObjects[ASSET_VARIABLE];

  style.textContent = "/* externally changed */";
  root.style.setProperty(CSS_VARIABLE, "#ef4444", "important");
  root.style.setProperty(ASSET_VARIABLE, "none", "important");
  root.setAttribute(ROOT_ATTRIBUTE, "external-value");
  root.setAttribute(THEME_ID_ATTRIBUTE, "external-theme");
  root.classList.remove(`ccp-${payload.theme_id}`);

  const result = harness.loader.apply(payload);
  const repairedAsset = result.assetObjects[ASSET_VARIABLE];

  assert.equal(result.ok, true);
  assert.equal(result.status, "repaired");
  assert.equal(result.active, true);
  assert.equal(ownedStyle(harness.document).textContent, payload.css);
  assert.equal(root.style.getPropertyValue(CSS_VARIABLE), payload.css_variables[CSS_VARIABLE]);
  assert.equal(root.style.getPropertyPriority(CSS_VARIABLE), "");
  assert.equal(root.style.getPropertyValue(ASSET_VARIABLE), oldAsset.cssValue);
  assert.equal(root.style.getPropertyPriority(ASSET_VARIABLE), "");
  assert.equal(root.getAttribute(ROOT_ATTRIBUTE), payload.theme_id);
  assert.equal(root.getAttribute(THEME_ID_ATTRIBUTE), payload.theme_id);
  assert.equal(root.classList.contains(`ccp-${payload.theme_id}`), true);
  assert.equal(repairedAsset.objectUrl, oldAsset.objectUrl);
  assert.deepEqual(harness.created, [oldAsset.objectUrl]);
  assert.deepEqual(harness.revoked, []);
});

test("revokes the old Blob only after a successful theme switch commits", () => {
  const oldPayload = themePayload();
  const nextPayload = themePayload({
    themeId: "theme-b",
    generation: 2,
    accent: "#10b981",
    assetBase64: "dGhlbWUtYg==",
  });
  const harness = createHarness(oldPayload);
  const oldAssetUrl = harness.initialResult.assetObjects[ASSET_VARIABLE].objectUrl;

  const result = harness.loader.apply(nextPayload);
  const nextAssetUrl = result.assetObjects[ASSET_VARIABLE].objectUrl;
  const nextCreateIndex = harness.events.findIndex(
    (event) => event.type === "create" && event.url === nextAssetUrl,
  );
  const oldRevokeIndex = harness.events.findIndex(
    (event) => event.type === "revoke" && event.url === oldAssetUrl,
  );
  const oldRevoke = harness.events[oldRevokeIndex];

  assert.equal(result.ok, true);
  assert.equal(result.status, "applied");
  assert.equal(result.active, true);
  assert.notEqual(nextAssetUrl, oldAssetUrl);
  assert.ok(nextCreateIndex >= 0, "the replacement Blob must be created");
  assert.ok(oldRevokeIndex > nextCreateIndex, "the old Blob must be revoked after replacement creation");
  assert.equal(oldRevoke.dom.styleThemeId, nextPayload.theme_id);
  assert.equal(oldRevoke.dom.rootThemeId, nextPayload.theme_id);
  assert.equal(oldRevoke.dom.assetValue, `url("${nextAssetUrl}")`);
  assert.equal(harness.revoked.filter((url) => url === oldAssetUrl).length, 1);
  assert.equal(harness.revoked.includes(nextAssetUrl), false);
});

test("reports default ownership conflicts without revoking a Blob still referenced by a root variable", () => {
  const payload = themePayload();
  const harness = createHarness(payload);
  const root = harness.document.documentElement;
  const oldAssetUrl = harness.initialResult.assetObjects[ASSET_VARIABLE].objectUrl;
  const externalValue = `image-set(url("${oldAssetUrl}") 1x)`;
  root.style.setProperty(ASSET_VARIABLE, externalValue);

  const result = harness.loader.apply(defaultPayload(2));
  const conflicts = plain(result.conflicts ?? []);

  assert.equal(result.status, "ownership_conflict");
  assert.equal(
    conflicts.some(
      (conflict) => conflict.kind === "asset_variable" && conflict.name === ASSET_VARIABLE,
    ),
    true,
  );
  assert.equal(root.style.getPropertyValue(ASSET_VARIABLE), externalValue);
  assert.equal(root.style.getPropertyValue(ASSET_VARIABLE).includes(oldAssetUrl), true);
  assert.equal(harness.revoked.includes(oldAssetUrl), false);
});
