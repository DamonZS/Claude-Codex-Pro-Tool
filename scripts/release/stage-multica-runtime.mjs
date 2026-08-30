#!/usr/bin/env node

import { createHash, randomUUID } from "node:crypto";
import { mkdir, open, readFile, rename, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { pathToFileURL } from "node:url";

const VERSION = "0.4.36";
const TAG = "v0.4.36";
const OWNER = "multica-ai";
const REPOSITORY = "multica";
const MAX_ARCHIVE_BYTES = 96 * 1024 * 1024;
const MAX_CHECKSUMS_BYTES = 1024 * 1024;
const MAX_REDIRECTS = 3;
const FETCH_TOTAL_TIMEOUT_MS = 120_000;
const MAX_FETCH_ATTEMPTS = 3;
const RETRY_BASE_DELAY_MS = 250;
const RETRYABLE_HTTP_STATUSES = new Set([408, 425, 429, 500, 502, 503, 504]);
const TRANSIENT_NETWORK_CODES = new Set([
  "EAI_AGAIN",
  "ECONNREFUSED",
  "ECONNRESET",
  "EPIPE",
  "ETIMEDOUT",
  "UND_ERR_BODY_TIMEOUT",
  "UND_ERR_CONNECT_TIMEOUT",
  "UND_ERR_HEADERS_TIMEOUT",
  "UND_ERR_SOCKET",
]);
const ALLOWED_HOSTS = new Set([
  "github.com",
  "objects.githubusercontent.com",
  "release-assets.githubusercontent.com",
  "github-releases.githubusercontent.com",
]);

const ASSETS = Object.freeze({
  "x86_64-pc-windows-msvc": {
    name: "multica-cli-0.4.36-windows-amd64.zip",
    sha256: "b96bc1df13824ed1bcb733351eb29ae570cdf3bae1f004dba45215cd011c744c",
  },
  "aarch64-pc-windows-msvc": {
    name: "multica-cli-0.4.36-windows-arm64.zip",
    sha256: "819e4839fab86a1c50af8fb755c3d5eafc78e8655931a22a2486264e0fd58ac0",
  },
  "x86_64-apple-darwin": {
    name: "multica-cli-0.4.36-darwin-amd64.tar.gz",
    sha256: "76d0e286b085cbb3f716c7ee5cfce7aee4ac223589620b0cdc5d86d5de7e8803",
  },
  "aarch64-apple-darwin": {
    name: "multica-cli-0.4.36-darwin-arm64.tar.gz",
    sha256: "ca7b62877628444bb08f8109008220616fefb275927ad741ad372114ee2f7d62",
  },
  "x86_64-unknown-linux-gnu": {
    name: "multica-cli-0.4.36-linux-amd64.tar.gz",
    sha256: "bdee5c7f574202e43d9cafe23914a384ad4e86098b98f59432faed6fdc92bfa2",
  },
  "aarch64-unknown-linux-gnu": {
    name: "multica-cli-0.4.36-linux-arm64.tar.gz",
    sha256: "e6cd65111f2a98f22d602d1db53aa506cacf906fc34ac59dc204525e34594f60",
  },
});

class StagingError extends Error {
  constructor(message, { cause, retryable = false } = {}) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = "StagingError";
    this.retryable = retryable;
  }
}

function usage(message) {
  if (message) console.error(`error: ${message}`);
  console.error("usage: node scripts/release/stage-multica-runtime.mjs --target <triple> --destination <dir>");
  process.exitCode = 2;
}

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (key !== "--target" && key !== "--destination") return usage(`unknown option ${key}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) return usage(`${key} requires a value`);
    args[key.slice(2)] = value;
    index += 1;
  }
  if (!args.target || !args.destination) return usage("--target and --destination are required");
  return args;
}

function assertAllowedUrl(url) {
  if (url.protocol !== "https:" || !ALLOWED_HOSTS.has(url.hostname.toLowerCase())) {
    throw new StagingError("download host or protocol is not allowlisted");
  }
  // GitHub release redirects carry a short-lived signed query on the CDN URL.
  // It is accepted only after the host has moved to the explicit CDN allowlist.
  if (url.username || url.password || url.hash || (url.search && url.hostname.toLowerCase() === "github.com")) {
    throw new StagingError("download URL contains forbidden credentials or query data");
  }
}

function isTransientNetworkError(error) {
  let current = error;
  for (let depth = 0; current && depth < 5; depth += 1) {
    if (typeof current.code === "string" && TRANSIENT_NETWORK_CODES.has(current.code)) return true;
    current = current.cause;
  }
  return false;
}

function normalizeDownloadError(error, signal, fallbackMessage) {
  if (signal.aborted || error?.name === "TimeoutError") {
    return new StagingError("download timed out");
  }
  if (error instanceof StagingError) return error;
  if (isTransientNetworkError(error)) {
    return new StagingError("transient network failure", { cause: error, retryable: true });
  }
  return new StagingError(fallbackMessage, { cause: error });
}

async function discardResponseBody(response) {
  await response.body?.cancel().catch(() => {});
}

async function fetchAttempt(initialUrl, limit, fetchImpl, signal) {
  let url;
  try {
    url = new URL(initialUrl);
  } catch (error) {
    throw new StagingError("invalid download URL", { cause: error });
  }

  for (let redirects = 0; redirects <= MAX_REDIRECTS; redirects += 1) {
    assertAllowedUrl(url);
    let response;
    try {
      response = await fetchImpl(url, {
        redirect: "manual",
        headers: { "User-Agent": "Claude-Codex-Pro release staging" },
        signal,
      });
    } catch (error) {
      throw normalizeDownloadError(error, signal, "download request failed");
    }

    if (response.status >= 300 && response.status < 400) {
      await discardResponseBody(response);
      if (redirects === MAX_REDIRECTS) throw new StagingError("too many redirects");
      const location = response.headers.get("location");
      if (!location) throw new StagingError("redirect missing location");
      try {
        url = new URL(location, url);
      } catch (error) {
        throw new StagingError("redirect location is invalid", { cause: error });
      }
      continue;
    }

    if (RETRYABLE_HTTP_STATUSES.has(response.status)) {
      await discardResponseBody(response);
      throw new StagingError(`download failed with retryable HTTP ${response.status}`, { retryable: true });
    }
    if (!response.ok || !response.body) {
      await discardResponseBody(response);
      throw new StagingError(`download failed with HTTP ${response.status}`);
    }
    const declared = Number(response.headers.get("content-length") || 0);
    if (Number.isFinite(declared) && declared > limit) {
      await discardResponseBody(response);
      throw new StagingError("download exceeds size limit");
    }
    const reader = response.body.getReader();
    const chunks = [];
    let total = 0;
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        total += value.byteLength;
        if (total > limit) throw new StagingError("download exceeds size limit");
        chunks.push(value);
      }
    } catch (error) {
      throw normalizeDownloadError(error, signal, "download response failed");
    } finally {
      reader.releaseLock();
    }
    const body = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      body.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return body;
  }
  throw new StagingError("too many redirects");
}

async function sleepForRetry(milliseconds, signal) {
  await delay(milliseconds, undefined, { signal });
}

async function fetchBounded(initialUrl, limit, options = {}) {
  const {
    fetchImpl = globalThis.fetch,
    sleepImpl = sleepForRetry,
    timeoutMs = FETCH_TOTAL_TIMEOUT_MS,
  } = options;
  if (typeof fetchImpl !== "function") throw new StagingError("fetch is unavailable");
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) throw new StagingError("invalid download timeout");

  // Standard Node fetch exposes AbortSignal but no portable connection-only
  // timeout. One deadline therefore bounds DNS/TCP/TLS, redirects, body reads,
  // and retry backoff for this download.
  const signal = AbortSignal.timeout(timeoutMs);
  for (let attempt = 1; attempt <= MAX_FETCH_ATTEMPTS; attempt += 1) {
    try {
      return await fetchAttempt(initialUrl, limit, fetchImpl, signal);
    } catch (error) {
      const normalized = normalizeDownloadError(error, signal, "download failed");
      if (!normalized.retryable) throw normalized;
      if (attempt === MAX_FETCH_ATTEMPTS) {
        throw new StagingError(`download failed after ${MAX_FETCH_ATTEMPTS} attempts`);
      }
      try {
        await sleepImpl(RETRY_BASE_DELAY_MS * (2 ** (attempt - 1)), signal);
      } catch (sleepError) {
        throw normalizeDownloadError(sleepError, signal, "download retry delay failed");
      }
    }
  }
  throw new StagingError("download failed");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function listedChecksum(bytes, assetName) {
  let text;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    throw new StagingError("checksum list is not valid UTF-8", { cause: error });
  }
  for (const line of text.split(/\r?\n/)) {
    const match = line.trim().match(/^([a-f0-9]{64})\s+\*?(.+?)\s*$/i);
    if (match && match[2] === assetName) return match[1].toLowerCase();
  }
  throw new StagingError("checksum entry missing");
}

async function writeAtomic(path, bytes) {
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  let handle;
  try {
    handle = await open(temporary, "wx", 0o644);
    await handle.writeFile(bytes);
    await handle.sync();
    await handle.close();
    handle = undefined;
    try {
      await rename(temporary, path);
    } catch (error) {
      // POSIX rename replaces atomically; Windows rejects replacing an
      // existing file, so remove only this known destination and retry.
      if (error.code !== "EEXIST" && error.code !== "EPERM") throw error;
      await rm(path, { force: true });
      await rename(temporary, path);
    }
  } finally {
    await handle?.close().catch(() => {});
    await rm(temporary, { force: true }).catch(() => {});
  }
}

async function stage(target, destination, fetchOptions = {}) {
  const asset = ASSETS[target];
  if (!asset) throw new StagingError("unsupported target");
  const destinationDir = resolve(destination);
  const destinationPath = resolve(destinationDir, asset.name);
  if (dirname(destinationPath) !== destinationDir) throw new StagingError("invalid destination");
  await mkdir(destinationDir, { recursive: true });

  try {
    const existing = await readFile(destinationPath);
    if (sha256(existing) === asset.sha256) {
      console.log(`reused Multica ${VERSION} ${target} ${asset.name} sha256=${asset.sha256}`);
      return;
    }
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  const base = `https://github.com/${OWNER}/${REPOSITORY}/releases/download/${TAG}`;
  const checksums = await fetchBounded(`${base}/checksums.txt`, MAX_CHECKSUMS_BYTES, fetchOptions);
  const listed = listedChecksum(checksums, asset.name);
  if (listed !== asset.sha256) throw new StagingError("release checksum declaration mismatch");
  const archive = await fetchBounded(`${base}/${asset.name}`, MAX_ARCHIVE_BYTES, fetchOptions);
  const digest = sha256(archive);
  if (digest !== asset.sha256) throw new StagingError("archive checksum mismatch");
  await writeAtomic(destinationPath, archive);
  console.log(`staged Multica ${VERSION} ${target} ${asset.name} sha256=${digest}`);
}

function errorMessageForLog(error) {
  if (error instanceof StagingError) return error.message;
  const code = typeof error?.code === "string" && /^[A-Z0-9_]+$/.test(error.code)
    ? error.code
    : undefined;
  return code ? `staging failed (${code})` : "staging failed";
}

function isMainModule() {
  return Boolean(process.argv[1]) && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
}

export { fetchBounded, stage };

if (isMainModule()) {
  const parsed = parseArgs(process.argv.slice(2));
  if (parsed && process.exitCode == null) {
    stage(parsed.target, parsed.destination).catch((error) => {
      console.error(`error: ${errorMessageForLog(error)}`);
      process.exitCode = 1;
    });
  }
}
