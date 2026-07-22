(() => {
  "use strict";

  const LOADER_VERSION = 4;
  const PAYLOAD_GLOBAL = "__CLAUDE_CODEX_PRO_CODEX_THEME_PAYLOAD__";
  const LOADER_GLOBAL = "__CLAUDE_CODEX_PRO_CODEX_THEME_LOADER__";
  const RESULT_GLOBAL = "__CLAUDE_CODEX_PRO_CODEX_THEME_RESULT__";
  const STYLE_ID = "claude-codex-pro-codex-theme";
  const STYLE_SELECTOR = `style[id="${STYLE_ID}"]`;
  const STYLE_OWNER_ATTRIBUTE = "data-ccp-theme-loader-version";
  const STYLE_GENERATION_ATTRIBUTE = "data-codex-theme-generation";
  const STYLE_THEME_ID_ATTRIBUTE = "data-codex-theme-id";
  const ROOT_METADATA = Object.freeze({
    "data-ccp-codex-theme-active": "true",
    "data-ccp-codex-theme-generation": null,
    "data-ccp-codex-theme-id": null,
  });
  const DATA_URI_PATTERN = /^data:(image\/(?:png|jpeg|webp));base64,([A-Za-z0-9+/]+={0,2})$/;
  const CSS_IMPORT_PATTERN = /@import(?:\s|\/\*[\s\S]*?\*\/)*(?:url\s*\(|["'])/i;
  const CSS_URL_PATTERN = /url\s*\(\s*(["']?)([\s\S]*?)\1\s*\)/gi;
  const SAFE_CSS_URL_PATTERN = /^(?:data:image\/(?:png|jpeg|webp);base64,[A-Za-z0-9+/]+={0,2}|blob:|(?:\.\.?\/|\/(?!\/))[^\\]*|#[^\s]*)$/i;

  const hasOwn = (value, key) => Object.prototype.hasOwnProperty.call(value, key);
  const emptyRecord = () => Object.create(null);
  const isRecord = (value) => value !== null && typeof value === "object" && !Array.isArray(value);

  const pick = (value, ...keys) => {
    for (const key of keys) {
      if (hasOwn(value, key)) {
        return value[key];
      }
    }
    return undefined;
  };

  const normalizeStringMap = (rawValue, label) => {
    if (rawValue === undefined) {
      return emptyRecord();
    }
    if (!isRecord(rawValue)) {
      throw new TypeError(`${label} must be an object`);
    }
    const normalized = emptyRecord();
    for (const key of Object.keys(rawValue).sort()) {
      if (typeof rawValue[key] !== "string") {
        throw new TypeError(`${label}.${key} must be a string`);
      }
      normalized[key] = rawValue[key];
    }
    return normalized;
  };

  const normalizeClassNames = (rawValue, label = "root_attributes.classes") => {
    if (rawValue === undefined) {
      return [];
    }
    if (!Array.isArray(rawValue)) {
      throw new TypeError(`${label} must be an array`);
    }
    const classNames = new Set();
    for (const value of rawValue) {
      if (typeof value !== "string" || !value) {
        throw new TypeError(`${label} entries must be non-empty strings`);
      }
      classNames.add(value);
    }
    return [...classNames].sort();
  };

  const normalizeRootAttributes = (payload) => {
    const rawRoot = pick(payload, "rootAttributes", "root_attributes");
    const topLevelClasses = pick(payload, "rootClasses", "root_classes");
    if (rawRoot === undefined) {
      return {
        classes: normalizeClassNames(topLevelClasses),
        attributes: emptyRecord(),
      };
    }
    if (!isRecord(rawRoot)) {
      throw new TypeError("root_attributes must be an object");
    }

    const usesStructuredShape = hasOwn(rawRoot, "classes") || hasOwn(rawRoot, "attributes");
    if (!usesStructuredShape) {
      return {
        classes: normalizeClassNames(topLevelClasses),
        attributes: normalizeStringMap(rawRoot, "root_attributes.attributes"),
      };
    }
    for (const key of Object.keys(rawRoot)) {
      if (key !== "classes" && key !== "attributes") {
        throw new TypeError(`root_attributes.${key} is not supported`);
      }
    }
    if (topLevelClasses !== undefined) {
      throw new TypeError("root classes must have one source");
    }
    return {
      classes: normalizeClassNames(rawRoot.classes),
      attributes: normalizeStringMap(rawRoot.attributes, "root_attributes.attributes"),
    };
  };

  const validateAssetDataUri = (dataUri, name) => {
    const match = DATA_URI_PATTERN.exec(dataUri);
    if (!match || match[2].length % 4 !== 0) {
      throw new TypeError(`asset_data_uris.${name} must be a canonical image Data URI`);
    }
    return {
      mimeType: match[1],
      base64: match[2],
    };
  };

  const dataUriToBlobAsset = (dataUri, name) => {
    const { mimeType, base64 } = validateAssetDataUri(dataUri, name);
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    const blob = new Blob([bytes], { type: mimeType });
    const objectUrl = URL.createObjectURL(blob);
    return {
      dataUri,
      mimeType,
      blobSize: blob.size,
      objectUrl,
      cssValue: `url("${objectUrl}")`,
      revoked: false,
    };
  };

  const normalizeGeneration = (value) => {
    if (typeof value === "string" && /^\d+$/.test(value)) {
      return value;
    }
    if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0) {
      return String(value);
    }
    throw new TypeError("generation must be a non-negative integer");
  };

  const validateThemeCss = (css) => {
    if (CSS_IMPORT_PATTERN.test(css)) {
      throw new TypeError("theme CSS must not contain @import");
    }
    CSS_URL_PATTERN.lastIndex = 0;
    for (const match of css.matchAll(CSS_URL_PATTERN)) {
      const resource = match[2].trim();
      if (!resource || !SAFE_CSS_URL_PATTERN.test(resource)) {
        throw new TypeError("theme CSS contains an unsafe resource URL");
      }
    }
  };

  const normalizePayload = (rawPayload) => {
    if (!isRecord(rawPayload)) {
      throw new TypeError("theme payload must be an object");
    }
    const rawThemeId = pick(rawPayload, "themeId", "theme_id") ?? "default";
    if (typeof rawThemeId !== "string" || !rawThemeId) {
      throw new TypeError("theme_id must be a non-empty string");
    }
    const rawCss = pick(
      rawPayload,
      "css",
      "style",
      "styleContents",
      "style_contents",
    ) ?? "";
    if (typeof rawCss !== "string") {
      throw new TypeError("theme CSS must be a string");
    }
    validateThemeCss(rawCss);
    const rawDefault = pick(rawPayload, "isDefault", "is_default");
    if (rawDefault !== undefined && typeof rawDefault !== "boolean") {
      throw new TypeError("is_default must be a boolean");
    }

    const themeId = rawThemeId;
    const generation = normalizeGeneration(rawPayload.generation ?? 0);
    const isDefault = rawDefault === true || themeId === "default";
    const cssVariables = normalizeStringMap(
      pick(rawPayload, "cssVariables", "css_variables"),
      "css_variables",
    );
    const rootAttributes = normalizeRootAttributes(rawPayload);
    const assetDataUris = normalizeStringMap(
      pick(
        rawPayload,
        "assetDataUris",
        "asset_data_uris",
        "assetVariables",
        "asset_variables",
      ),
      "asset_data_uris",
    );
    for (const name of Object.keys(assetDataUris)) {
      if (hasOwn(cssVariables, name)) {
        throw new TypeError(`${name} is declared by both CSS and asset variables`);
      }
      validateAssetDataUri(assetDataUris[name], name);
    }
    for (const name of Object.keys(ROOT_METADATA)) {
      if (hasOwn(rootAttributes.attributes, name)) {
        throw new TypeError(`${name} is reserved for Theme Loader metadata`);
      }
    }

    if (isDefault) {
      if (
        rawCss
        || Object.keys(cssVariables).length
        || rootAttributes.classes.length
        || Object.keys(rootAttributes.attributes).length
        || Object.keys(assetDataUris).length
      ) {
        throw new TypeError("the default theme payload must not declare theme state");
      }
    } else if (!rawCss.trim()) {
      throw new TypeError("theme CSS is empty");
    }

    const signature = JSON.stringify([
      themeId,
      generation,
      rawCss,
      isDefault,
      Object.entries(cssVariables),
      rootAttributes.classes,
      Object.entries(rootAttributes.attributes),
      Object.entries(assetDataUris),
    ]);
    return {
      themeId,
      generation,
      css: rawCss,
      isDefault,
      cssVariables,
      rootClasses: rootAttributes.classes,
      rootAttributes: rootAttributes.attributes,
      assetDataUris,
      assetCssVariables: emptyRecord(),
      signature,
    };
  };

  const generationOrder = (value) => {
    try {
      return /^\d+$/.test(value) ? BigInt(value) : null;
    } catch (_) {
      return null;
    }
  };

  const createOwnership = () => ({
    cssVariables: emptyRecord(),
    assetVariables: emptyRecord(),
    rootAttributes: emptyRecord(),
    metadataAttributes: emptyRecord(),
    rootClasses: emptyRecord(),
    rootStyleAttribute: null,
  });

  const createState = () => ({
    generation: null,
    themeId: null,
    isDefault: true,
    payloadSignature: null,
    expected: null,
    ownership: createOwnership(),
    assets: emptyRecord(),
    lastConflicts: [],
  });

  const previous = window[LOADER_GLOBAL];
  const previousState = previous && isRecord(previous.state) ? previous.state : null;
  let legacyAssetRegistry = previous?.version !== LOADER_VERSION && previousState
    ? previousState.assets
    : null;
  const state = previous?.version === LOADER_VERSION && previousState
    ? previousState
    : createState();

  const ensureOwnershipShape = () => {
    if (!isRecord(state.ownership)) {
      state.ownership = createOwnership();
      return;
    }
    for (const key of [
      "cssVariables",
      "assetVariables",
      "rootAttributes",
      "metadataAttributes",
      "rootClasses",
    ]) {
      if (!isRecord(state.ownership[key])) {
        state.ownership[key] = emptyRecord();
      }
    }
    if (!hasOwn(state.ownership, "rootStyleAttribute")) {
      state.ownership.rootStyleAttribute = null;
    }
  };
  ensureOwnershipShape();
  if (!isRecord(state.assets)) {
    state.assets = emptyRecord();
  }

  const revokeAssetRecord = (record) => {
    if (!isRecord(record) || record.revoked === true) {
      return;
    }
    if (typeof record.objectUrl === "string" && record.objectUrl.startsWith("blob:")) {
      URL.revokeObjectURL(record.objectUrl);
    }
    record.revoked = true;
  };

  const revokeAssetRegistry = (registry, keepRecords = new Set()) => {
    if (!isRecord(registry)) {
      return 0;
    }
    let released = 0;
    for (const record of Object.values(registry)) {
      if (
        isRecord(record)
        && record.revoked !== true
        && !keepRecords.has(record)
      ) {
        revokeAssetRecord(record);
        released += 1;
      }
    }
    return released;
  };

  const assetRecordIsReferenced = (record) => {
    if (!isRecord(record) || typeof record.objectUrl !== "string") {
      return false;
    }
    const root = document.documentElement;
    const style = root.style;
    for (let index = 0; index < style.length; index += 1) {
      const name = typeof style.item === "function" ? style.item(index) : style[index];
      if (style.getPropertyValue(name).includes(record.objectUrl)) {
        return true;
      }
    }
    return (root.getAttribute("style") ?? "").includes(record.objectUrl);
  };

  const releaseAssetObjects = () => {
    const retained = emptyRecord();
    let released = 0;
    for (const [name, record] of Object.entries(state.assets)) {
      if (assetRecordIsReferenced(record)) {
        retained[name] = record;
      } else if (isRecord(record) && record.revoked !== true) {
        revokeAssetRecord(record);
        released += 1;
      }
    }
    state.assets = retained;
    return released;
  };

  const assetRecordIsUsable = (record, dataUri) => isRecord(record)
    && record.dataUri === dataUri
    && record.revoked !== true
    && typeof record.objectUrl === "string"
    && record.objectUrl.startsWith("blob:")
    && record.cssValue === `url("${record.objectUrl}")`
    && Number.isFinite(record.blobSize)
    && record.blobSize > 0;

  const stageAssets = (payload, sourceRegistry, createdRecords) => {
    const requestedNames = new Set(Object.keys(payload.assetDataUris));
    const staged = emptyRecord();
    const cssVariables = emptyRecord();
    for (const name of [...requestedNames].sort()) {
      const dataUri = payload.assetDataUris[name];
      let record = isRecord(sourceRegistry) ? sourceRegistry[name] : null;
      if (!assetRecordIsUsable(record, dataUri)) {
        record = dataUriToBlobAsset(dataUri, name);
        createdRecords.push(record);
      }
      staged[name] = record;
      cssVariables[name] = record.cssValue;
    }
    payload.assetCssVariables = cssVariables;
    return staged;
  };

  const metadataFor = (payload) => ({
    ...ROOT_METADATA,
    "data-ccp-codex-theme-generation": payload.generation,
    "data-ccp-codex-theme-id": payload.themeId,
  });

  const captureAttribute = (group, name, writtenValue) => {
    const root = document.documentElement;
    group[name] = {
      hadOriginal: root.hasAttribute(name),
      originalValue: root.getAttribute(name),
      writtenValue,
    };
    if (root.getAttribute(name) !== writtenValue) {
      root.setAttribute(name, writtenValue);
    }
  };

  const inlineStyleHas = (style, name) => {
    for (let index = 0; index < style.length; index += 1) {
      const item = typeof style.item === "function" ? style.item(index) : style[index];
      if (item === name) {
        return true;
      }
    }
    return style.getPropertyValue(name) !== "";
  };

  const captureVariable = (group, name, writtenValue) => {
    const root = document.documentElement;
    const style = root.style;
    group[name] = {
      hadOriginal: inlineStyleHas(style, name),
      originalValue: style.getPropertyValue(name),
      originalPriority: style.getPropertyPriority(name),
      writtenValue,
      writtenPriority: "",
    };
    if (
      style.getPropertyValue(name) !== writtenValue
      || style.getPropertyPriority(name) !== ""
    ) {
      style.setProperty(name, writtenValue);
    }
  };

  const captureRootClass = (name) => {
    const root = document.documentElement;
    const originallyPresent = root.classList.contains(name);
    state.ownership.rootClasses[name] = {
      originallyPresent,
      writtenValue: true,
    };
    if (!originallyPresent) {
      root.classList.add(name);
    }
  };

  const ownedStyleNodes = () => [...document.querySelectorAll(STYLE_SELECTOR)];

  const convergeOwnedStyle = (payload) => {
    const styleNodes = ownedStyleNodes();
    const style = styleNodes[0] || document.createElement("style");
    for (const duplicate of styleNodes.slice(1)) {
      duplicate.remove();
    }
    if (style.id !== STYLE_ID) {
      style.id = STYLE_ID;
    }
    if (style.getAttribute(STYLE_OWNER_ATTRIBUTE) !== String(LOADER_VERSION)) {
      style.setAttribute(STYLE_OWNER_ATTRIBUTE, String(LOADER_VERSION));
    }
    if (style.getAttribute(STYLE_GENERATION_ATTRIBUTE) !== payload.generation) {
      style.setAttribute(STYLE_GENERATION_ATTRIBUTE, payload.generation);
    }
    if (style.getAttribute(STYLE_THEME_ID_ATTRIBUTE) !== payload.themeId) {
      style.setAttribute(STYLE_THEME_ID_ATTRIBUTE, payload.themeId);
    }
    if (style.textContent !== payload.css) {
      style.textContent = payload.css;
    }
    if (!style.isConnected) {
      (document.head || document.documentElement).appendChild(style);
    }
    return style;
  };

  const removeOwnedStyles = () => {
    const styleNodes = ownedStyleNodes();
    for (const style of styleNodes) {
      style.remove();
    }
    return styleNodes.length;
  };

  const cloneRecordMap = (source) => {
    const clone = emptyRecord();
    if (!isRecord(source)) {
      return clone;
    }
    for (const [name, record] of Object.entries(source)) {
      clone[name] = isRecord(record) ? { ...record } : record;
    }
    return clone;
  };

  const cloneOwnership = (source) => ({
    cssVariables: cloneRecordMap(source?.cssVariables),
    assetVariables: cloneRecordMap(source?.assetVariables),
    rootAttributes: cloneRecordMap(source?.rootAttributes),
    metadataAttributes: cloneRecordMap(source?.metadataAttributes),
    rootClasses: cloneRecordMap(source?.rootClasses),
    rootStyleAttribute: isRecord(source?.rootStyleAttribute)
      ? { ...source.rootStyleAttribute }
      : null,
  });

  const captureStateTransaction = () => ({
    generation: state.generation,
    themeId: state.themeId,
    isDefault: state.isDefault,
    payloadSignature: state.payloadSignature,
    expected: state.expected,
    ownership: cloneOwnership(state.ownership),
    assets: { ...state.assets },
    lastConflicts: Array.isArray(state.lastConflicts)
      ? state.lastConflicts.map((conflict) => ({ ...conflict }))
      : [],
  });

  const restoreStateTransaction = (transaction) => {
    state.generation = transaction.generation;
    state.themeId = transaction.themeId;
    state.isDefault = transaction.isDefault;
    state.payloadSignature = transaction.payloadSignature;
    state.expected = transaction.expected;
    state.ownership = cloneOwnership(transaction.ownership);
    state.assets = { ...transaction.assets };
    state.lastConflicts = transaction.lastConflicts.map((conflict) => ({ ...conflict }));
  };

  const captureStyleNode = (node) => {
    const attributeNames = [
      "id",
      STYLE_OWNER_ATTRIBUTE,
      STYLE_GENERATION_ATTRIBUTE,
      STYLE_THEME_ID_ATTRIBUTE,
    ];
    return {
      node,
      parent: node.parentNode,
      nextSibling: node.nextSibling ?? null,
      textContent: node.textContent,
      attributes: Object.fromEntries(attributeNames.map((name) => [name, {
        present: node.hasAttribute(name),
        value: node.getAttribute(name),
      }])),
    };
  };

  const captureDomTransaction = (payload) => {
    const root = document.documentElement;
    const attributeNames = new Set([
      ...Object.keys(state.ownership.rootAttributes),
      ...Object.keys(state.ownership.metadataAttributes),
      ...Object.keys(payload.rootAttributes),
      ...Object.keys(metadataFor(payload)),
    ]);
    const classNames = new Set([
      ...Object.keys(state.ownership.rootClasses),
      ...payload.rootClasses,
    ]);
    return {
      rootStyle: {
        present: root.hasAttribute("style"),
        value: root.getAttribute("style"),
      },
      attributes: Object.fromEntries([...attributeNames].map((name) => [name, {
        present: root.hasAttribute(name),
        value: root.getAttribute(name),
      }])),
      classes: Object.fromEntries(
        [...classNames].map((name) => [name, root.classList.contains(name)]),
      ),
      styles: ownedStyleNodes().map(captureStyleNode),
    };
  };

  const restoreDomTransaction = (transaction) => {
    const root = document.documentElement;
    if (transaction.rootStyle.present) {
      root.setAttribute("style", transaction.rootStyle.value ?? "");
    } else {
      root.removeAttribute("style");
    }
    for (const [name, record] of Object.entries(transaction.attributes)) {
      if (record.present) {
        root.setAttribute(name, record.value ?? "");
      } else {
        root.removeAttribute(name);
      }
    }
    for (const [name, present] of Object.entries(transaction.classes)) {
      if (present) {
        root.classList.add(name);
      } else {
        root.classList.remove(name);
      }
    }

    removeOwnedStyles();
    for (const styleState of transaction.styles) {
      const node = styleState.node;
      node.textContent = styleState.textContent;
      for (const [name, record] of Object.entries(styleState.attributes)) {
        if (record.present) {
          node.setAttribute(name, record.value ?? "");
        } else {
          node.removeAttribute(name);
        }
      }
      const parent = styleState.parent;
      if (parent && typeof parent.insertBefore === "function") {
        const sibling = styleState.nextSibling;
        parent.insertBefore(node, sibling?.parentNode === parent ? sibling : null);
      } else if (parent && typeof parent.appendChild === "function") {
        parent.appendChild(node);
      }
    }
  };

  const activeAssetRecords = (registry) => new Set(
    isRecord(registry) ? Object.values(registry).filter(isRecord) : [],
  );

  const releaseSupersededAssets = (registries, activeRegistry) => {
    const keepRecords = activeAssetRecords(activeRegistry);
    const seen = new Set();
    let released = 0;
    for (const registry of registries) {
      if (!isRecord(registry)) {
        continue;
      }
      for (const record of Object.values(registry)) {
        if (!isRecord(record) || seen.has(record)) {
          continue;
        }
        seen.add(record);
        if (
          !keepRecords.has(record)
          && record.revoked !== true
          && !assetRecordIsReferenced(record)
        ) {
          revokeAssetRecord(record);
          released += 1;
        }
      }
    }
    return released;
  };

  const variableMatchesWrittenValue = (name, record) => {
    const style = document.documentElement.style;
    return style.getPropertyValue(name) === record.writtenValue
      && style.getPropertyPriority(name) === record.writtenPriority;
  };

  const attributeMatchesWrittenValue = (name, record) => {
    const root = document.documentElement;
    return root.hasAttribute(name) && root.getAttribute(name) === record.writtenValue;
  };

  const restoreVariableGroup = (kind, group, conflicts) => {
    const style = document.documentElement.style;
    for (const name of Object.keys(group)) {
      const record = group[name];
      if (!variableMatchesWrittenValue(name, record)) {
        conflicts.push({ kind, name });
        continue;
      }
      if (record.hadOriginal) {
        style.setProperty(name, record.originalValue, record.originalPriority);
      } else {
        style.removeProperty(name);
      }
    }
  };

  const restoreAttributeGroup = (kind, group, conflicts) => {
    const root = document.documentElement;
    for (const name of Object.keys(group)) {
      const record = group[name];
      if (!attributeMatchesWrittenValue(name, record)) {
        conflicts.push({ kind, name });
        continue;
      }
      if (record.hadOriginal) {
        root.setAttribute(name, record.originalValue);
      } else {
        root.removeAttribute(name);
      }
    }
  };

  // Restoration is value-conditional: a value that drifted after application is no longer ours.
  const restoreOwnedRootState = () => {
    const root = document.documentElement;
    const conflicts = [];
    const variableConflictStart = conflicts.length;
    restoreVariableGroup("css_variable", state.ownership.cssVariables, conflicts);
    restoreVariableGroup("asset_variable", state.ownership.assetVariables, conflicts);
    const variablesDrifted = conflicts.length !== variableConflictStart;
    restoreAttributeGroup("root_attribute", state.ownership.rootAttributes, conflicts);
    restoreAttributeGroup("metadata_attribute", state.ownership.metadataAttributes, conflicts);

    for (const name of Object.keys(state.ownership.rootClasses)) {
      const record = state.ownership.rootClasses[name];
      if (root.classList.contains(name) !== record.writtenValue) {
        conflicts.push({ kind: "root_class", name });
      } else if (!record.originallyPresent) {
        root.classList.remove(name);
      }
    }

    const styleAttribute = state.ownership.rootStyleAttribute;
    if (!variablesDrifted && styleAttribute) {
      if (!styleAttribute.hadOriginal && root.getAttribute("style") === "") {
        root.removeAttribute("style");
      } else if (
        styleAttribute.hadOriginal
        && styleAttribute.originalValue === ""
        && !root.hasAttribute("style")
      ) {
        root.setAttribute("style", "");
      }
    }
    state.ownership = createOwnership();
    return conflicts;
  };

  const difference = (kind, name = null) => name ? `${kind}:${name}` : kind;

  const inspectActivePayload = (payload) => {
    const root = document.documentElement;
    const differences = [];
    const styleNodes = ownedStyleNodes();
    if (styleNodes.length !== 1) {
      differences.push(difference("style_count"));
    }
    const style = styleNodes[0];
    if (style) {
      if (style.textContent !== payload.css) {
        differences.push(difference("style_text"));
      }
      if (style.getAttribute(STYLE_OWNER_ATTRIBUTE) !== String(LOADER_VERSION)) {
        differences.push(difference("style_owner"));
      }
      if (style.getAttribute(STYLE_GENERATION_ATTRIBUTE) !== payload.generation) {
        differences.push(difference("style_generation"));
      }
      if (style.getAttribute(STYLE_THEME_ID_ATTRIBUTE) !== payload.themeId) {
        differences.push(difference("style_theme_id"));
      }
    }
    for (const [name, value] of Object.entries(payload.cssVariables)) {
      if (
        root.style.getPropertyValue(name) !== value
        || root.style.getPropertyPriority(name) !== ""
      ) {
        differences.push(difference("css_variable", name));
      }
    }
    for (const [name, value] of Object.entries(payload.assetCssVariables)) {
      if (!assetRecordIsUsable(state.assets[name], payload.assetDataUris[name])) {
        differences.push(difference("asset_object", name));
      }
      if (
        root.style.getPropertyValue(name) !== value
        || root.style.getPropertyPriority(name) !== ""
      ) {
        differences.push(difference("asset_variable", name));
      }
    }
    for (const [name, value] of Object.entries(payload.rootAttributes)) {
      if (!root.hasAttribute(name) || root.getAttribute(name) !== value) {
        differences.push(difference("root_attribute", name));
      }
    }
    for (const [name, value] of Object.entries(metadataFor(payload))) {
      if (!root.hasAttribute(name) || root.getAttribute(name) !== value) {
        differences.push(difference("metadata_attribute", name));
      }
    }
    for (const name of payload.rootClasses) {
      if (!root.classList.contains(name)) {
        differences.push(difference("root_class", name));
      }
    }
    return differences;
  };

  const assertOwnershipComplete = (payload) => {
    for (const name of Object.keys(payload.cssVariables)) {
      if (!hasOwn(state.ownership.cssVariables, name)) {
        throw new Error(`missing CSS variable ownership for ${name}`);
      }
    }
    for (const name of Object.keys(payload.assetCssVariables)) {
      if (!hasOwn(state.ownership.assetVariables, name)) {
        throw new Error(`missing asset variable ownership for ${name}`);
      }
    }
    for (const name of Object.keys(payload.rootAttributes)) {
      if (!hasOwn(state.ownership.rootAttributes, name)) {
        throw new Error(`missing root attribute ownership for ${name}`);
      }
    }
    for (const name of Object.keys(metadataFor(payload))) {
      if (!hasOwn(state.ownership.metadataAttributes, name)) {
        throw new Error(`missing metadata ownership for ${name}`);
      }
    }
    for (const name of payload.rootClasses) {
      if (!hasOwn(state.ownership.rootClasses, name)) {
        throw new Error(`missing root class ownership for ${name}`);
      }
    }
  };

  const repairActivePayload = (payload) => {
    assertOwnershipComplete(payload);
    const before = inspectActivePayload(payload);
    if (!before.length) {
      return { before, after: [] };
    }
    convergeOwnedStyle(payload);
    const root = document.documentElement;
    for (const [name, record] of Object.entries(state.ownership.cssVariables)) {
      if (!variableMatchesWrittenValue(name, record)) {
        root.style.setProperty(name, record.writtenValue, record.writtenPriority);
      }
    }
    for (const [name, record] of Object.entries(state.ownership.assetVariables)) {
      const expectedValue = payload.assetCssVariables[name];
      if (typeof expectedValue === "string" && record.writtenValue !== expectedValue) {
        record.writtenValue = expectedValue;
        record.writtenPriority = "";
      }
      if (!variableMatchesWrittenValue(name, record)) {
        root.style.setProperty(name, record.writtenValue, record.writtenPriority);
      }
    }
    for (const [name, record] of Object.entries(state.ownership.rootAttributes)) {
      if (!attributeMatchesWrittenValue(name, record)) {
        root.setAttribute(name, record.writtenValue);
      }
    }
    for (const [name, record] of Object.entries(state.ownership.metadataAttributes)) {
      if (!attributeMatchesWrittenValue(name, record)) {
        root.setAttribute(name, record.writtenValue);
      }
    }
    for (const [name, record] of Object.entries(state.ownership.rootClasses)) {
      if (root.classList.contains(name) !== record.writtenValue) {
        root.classList.add(name);
      }
    }
    return { before, after: inspectActivePayload(payload) };
  };

  const applyFreshPayload = (payload, stagedAssets = emptyRecord()) => {
    state.generation = payload.generation;
    state.themeId = payload.themeId;
    state.isDefault = payload.isDefault;
    state.payloadSignature = payload.signature;
    state.expected = null;
    state.ownership = createOwnership();
    state.assets = stagedAssets;

    if (payload.isDefault) {
      state.expected = payload;
      return ownedStyleNodes().length === 0;
    }

    state.expected = payload;
    const root = document.documentElement;
    const variableCount = Object.keys(payload.cssVariables).length
      + Object.keys(payload.assetCssVariables).length;
    if (variableCount) {
      state.ownership.rootStyleAttribute = {
        hadOriginal: root.hasAttribute("style"),
        originalValue: root.getAttribute("style"),
      };
    }
    convergeOwnedStyle(payload);
    for (const [name, value] of Object.entries(payload.cssVariables)) {
      captureVariable(state.ownership.cssVariables, name, value);
    }
    for (const [name, value] of Object.entries(payload.assetCssVariables)) {
      captureVariable(state.ownership.assetVariables, name, value);
    }
    for (const [name, value] of Object.entries(payload.rootAttributes)) {
      captureAttribute(state.ownership.rootAttributes, name, value);
    }
    for (const [name, value] of Object.entries(metadataFor(payload))) {
      captureAttribute(state.ownership.metadataAttributes, name, value);
    }
    for (const name of payload.rootClasses) {
      captureRootClass(name);
    }
    return inspectActivePayload(payload).length === 0;
  };

  const ownershipSummary = () => ({
    cssVariables: Object.keys(state.ownership.cssVariables),
    assetVariables: Object.keys(state.ownership.assetVariables),
    rootAttributes: Object.keys(state.ownership.rootAttributes),
    metadataAttributes: Object.keys(state.ownership.metadataAttributes),
    rootClasses: Object.keys(state.ownership.rootClasses),
  });

  const assetSummary = () => Object.fromEntries(
    Object.entries(state.assets).map(([name, record]) => [name, {
      objectUrl: record.objectUrl,
      cssValue: record.cssValue,
      mimeType: record.mimeType,
      blobSize: record.blobSize,
      revoked: record.revoked === true,
    }]),
  );

  const snapshot = () => {
    const payload = state.expected;
    const styleNodes = ownedStyleNodes();
    const style = styleNodes[0] || null;
    let differences = [];
    if (payload && !payload.isDefault) {
      differences = inspectActivePayload(payload);
    } else if (styleNodes.length) {
      differences = [difference("style_count")];
    }
    return {
      loaderVersion: LOADER_VERSION,
      generation: state.generation,
      themeId: state.themeId,
      isDefault: state.isDefault,
      active: !!payload && !payload.isDefault && differences.length === 0,
      stylePresent: !!style,
      styleCount: styleNodes.length,
      styleGeneration: style?.getAttribute(STYLE_GENERATION_ATTRIBUTE) ?? null,
      styleThemeId: style?.getAttribute(STYLE_THEME_ID_ATTRIBUTE) ?? null,
      cssLength: style?.textContent?.length ?? 0,
      cssVariables: payload ? Object.keys(payload.cssVariables) : [],
      assetVariables: payload ? Object.keys(payload.assetDataUris) : [],
      rootAttributes: payload ? Object.keys(payload.rootAttributes) : [],
      rootClasses: payload ? [...payload.rootClasses] : [],
      assetObjects: assetSummary(),
      differences,
      ownership: ownershipSummary(),
      conflictCount: Array.isArray(state.lastConflicts) ? state.lastConflicts.length : 0,
    };
  };

  const result = (ok, status, details = {}) => ({
    ok,
    status,
    ...details,
    ...snapshot(),
  });

  const copyOwnershipGroup = (target, source) => {
    if (!isRecord(source)) {
      return;
    }
    for (const [name, record] of Object.entries(source)) {
      if (isRecord(record)) {
        target[name] = { ...record };
      }
    }
  };

  const migrateV2Ownership = () => {
    if (previous?.version !== 2 || !previousState) {
      return;
    }
    state.generation = previousState.generation ?? null;
    state.themeId = previousState.themeId ?? null;
    state.isDefault = previousState.isDefault === true;
    const added = new Set(normalizeClassNames(previousState.addedRootClasses, "v2 classes"));
    for (const name of normalizeClassNames(previousState.rootClasses, "v2 classes")) {
      state.ownership.rootClasses[name] = {
        originallyPresent: !added.has(name),
        writtenValue: true,
      };
    }
    const root = document.documentElement;
    for (const name of Object.keys(ROOT_METADATA)) {
      if (root.hasAttribute(name)) {
        state.ownership.metadataAttributes[name] = {
          hadOriginal: false,
          originalValue: null,
          writtenValue: root.getAttribute(name),
        };
      }
    }
  };
  migrateV2Ownership();

  const migrateV3Ownership = () => {
    if (previous?.version !== 3 || !previousState || !isRecord(previousState.ownership)) {
      return;
    }
    state.generation = previousState.generation ?? null;
    state.themeId = previousState.themeId ?? null;
    state.isDefault = previousState.isDefault === true;
    copyOwnershipGroup(
      state.ownership.cssVariables,
      previousState.ownership.cssVariables,
    );
    copyOwnershipGroup(
      state.ownership.rootAttributes,
      previousState.ownership.rootAttributes,
    );
    copyOwnershipGroup(
      state.ownership.metadataAttributes,
      previousState.ownership.metadataAttributes,
    );
    copyOwnershipGroup(
      state.ownership.rootClasses,
      previousState.ownership.rootClasses,
    );

    const rootStyle = document.documentElement.style;
    if (isRecord(previousState.ownership.assetVariables)) {
      for (const [name, record] of Object.entries(previousState.ownership.assetVariables)) {
        if (!isRecord(record)) {
          continue;
        }
        // Chromium can discard a very long Data URI assignment entirely. In that case
        // there is no original value to preserve and the v3 write must not become one.
        if (inlineStyleHas(rootStyle, name) || variableMatchesWrittenValue(name, record)) {
          state.ownership.assetVariables[name] = { ...record };
        }
      }
    }
    if (isRecord(previousState.ownership.rootStyleAttribute)) {
      state.ownership.rootStyleAttribute = {
        ...previousState.ownership.rootStyleAttribute,
      };
    }
  };
  migrateV3Ownership();

  const apply = (rawPayload) => {
    let payload;
    try {
      payload = normalizePayload(rawPayload);
    } catch (error) {
      return result(false, "failed", {
        reason: "invalid_payload",
        message: error instanceof Error ? error.message : String(error),
      });
    }

    const currentOrder = state.generation === null ? null : generationOrder(String(state.generation));
    const nextOrder = generationOrder(payload.generation);
    if (currentOrder !== null && nextOrder !== null && nextOrder < currentOrder) {
      return result(true, "stale");
    }

    const samePayload = state.generation === payload.generation
      && state.themeId === payload.themeId
      && state.isDefault === payload.isDefault
      && state.payloadSignature === payload.signature;

    const createdRecords = [];
    const sourceAssets = emptyRecord();
    if (isRecord(legacyAssetRegistry)) {
      Object.assign(sourceAssets, legacyAssetRegistry);
    }
    if (isRecord(state.assets)) {
      Object.assign(sourceAssets, state.assets);
    }

    let stagedAssets;
    try {
      stagedAssets = stageAssets(payload, sourceAssets, createdRecords);
    } catch (error) {
      for (const record of createdRecords) {
        revokeAssetRecord(record);
      }
      return result(false, "failed", {
        reason: "asset_materialization_failed",
        message: error instanceof Error ? error.message : String(error),
      });
    }

    const stateTransaction = captureStateTransaction();
    const domTransaction = captureDomTransaction(payload);
    const rollback = () => {
      restoreDomTransaction(domTransaction);
      restoreStateTransaction(stateTransaction);
      for (const record of createdRecords) {
        revokeAssetRecord(record);
      }
    };

    try {
      if (samePayload && !payload.isDefault) {
        state.lastConflicts = [];
        state.assets = stagedAssets;
        state.expected = payload;
        const repair = repairActivePayload(payload);
        if (repair.after.length) {
          const differences = [...repair.after];
          rollback();
          return result(false, "failed", {
            reason: "repair_verification_failed",
            differences,
          });
        }
        releaseSupersededAssets([stateTransaction.assets], state.assets);
        return result(true, repair.before.length ? "repaired" : "healthy", {
          repairedDifferences: repair.before,
        });
      }

      const hadOwnedState = Object.values(ownershipSummary()).some((items) => items.length);
      const conflicts = restoreOwnedRootState();
      if (conflicts.length) {
        rollback();
        state.lastConflicts = conflicts;
        return result(false, "ownership_conflict", { conflicts });
      }

      removeOwnedStyles();
      const verified = applyFreshPayload(payload, stagedAssets);
      if (!verified) {
        rollback();
        return result(false, "failed", { reason: "apply_verification_failed" });
      }

      state.lastConflicts = [];
      const releasedAssets = releaseSupersededAssets(
        [stateTransaction.assets, legacyAssetRegistry],
        state.assets,
      );
      legacyAssetRegistry = null;
      if (samePayload && payload.isDefault) {
        const removedState = hadOwnedState || domTransaction.styles.length > 0 || releasedAssets > 0;
        return result(true, removedState ? "repaired" : "healthy");
      }
      return result(true, "applied");
    } catch (error) {
      rollback();
      return result(false, "failed", {
        reason: "loader_error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const loader = Object.freeze({
    version: LOADER_VERSION,
    payloadGlobal: PAYLOAD_GLOBAL,
    resultGlobal: RESULT_GLOBAL,
    styleId: STYLE_ID,
    state,
    apply,
    dataUriToBlobAsset,
    releaseAssetObjects,
    normalizePayload,
    snapshot,
  });
  window[LOADER_GLOBAL] = loader;
  window[RESULT_GLOBAL] = loader.apply(window[PAYLOAD_GLOBAL]);
  return window[RESULT_GLOBAL];
})();
