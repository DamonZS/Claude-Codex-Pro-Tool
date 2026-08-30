#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const auto = fs.readFileSync(".github/workflows/auto-release-installers.yml", "utf8");
const manual = fs.readFileSync(".github/workflows/release-assets.yml", "utf8");

function mustContain(source, needle, label) {
  assert.ok(source.includes(needle), `${label} missing: ${needle}`);
}

const windowsInstaller = fs.readFileSync("scripts/installer/windows/ClaudeCodexPro.nsi", "utf8");
const macosPackager = fs.readFileSync("scripts/installer/macos/package-dmg.sh", "utf8");
const multicaCore = fs.readFileSync("crates/claude-codex-pro-core/src/multica.rs", "utf8");
const multicaStager = fs.readFileSync("scripts/release/stage-multica-runtime.mjs", "utf8");

function mustNotContain(source, needle, label) {
  assert.ok(!source.includes(needle), `${label} must not contain: ${needle}`);
}

const forbiddenReleaseInputs = [
  "settings.json",
  "relayProfiles",
  "relay-profiles",
  "memory_assist.sqlite",
  "auth.json",
  "credentials",
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
  "sk-",
  "%APPDATA%",
  "$APPDATA",
  "$HOME/.codex",
  "$HOME/.claude",
  "~/.codex",
  "~/.claude",
  "Library/Application Support",
];

for (const [label, source] of [["auto workflow", auto], ["manual workflow", manual]]) {
  for (const forbidden of forbiddenReleaseInputs) {
    mustNotContain(source, forbidden, label);
  }
  mustContain(source, "dist/windows/app/*", `${label} Windows ZIP source`);
  mustContain(source, "dist/macos/stage", `${label} macOS ZIP source`);
}

for (const forbidden of ["settings.json", "relayProfiles", "memory_assist.sqlite", "auth.json", "OPENAI_API_KEY", "ANTHROPIC_API_KEY", "sk-"]) {
  mustNotContain(windowsInstaller, forbidden, "Windows installer");
  mustNotContain(macosPackager, forbidden, "macOS packager");
}

mustContain(windowsInstaller, 'File "${ROOT}\\dist\\windows\\app\\claude-codex-pro.exe"', "Windows installer app source");
mustNotContain(windowsInstaller, 'File "${ROOT}\\dist\\windows\\app\\claude-codex-pro-manager.exe"', "Windows installer legacy manager source");
mustContain(windowsInstaller, 'File "${ROOT}\\dist\\windows\\app\\claude-codex-pro-mcp.exe"', "Windows installer MCP source");
mustContain(windowsInstaller, 'Delete "$INSTDIR\\claude-codex-pro-mcp.exe"', "Windows installer MCP uninstall");
mustContain(macosPackager, "create_app \"Claude Codex Pro\"", "macOS app bundle");
mustNotContain(macosPackager, "create_app \"Claude Codex Pro Manager\"", "macOS legacy manager bundle");
mustContain(macosPackager, 'install_app_runtime "claude-codex-pro-mcp"', "macOS MCP source");
mustContain(macosPackager, 'local destination="$STAGE/Claude Codex Pro.app/Contents/MacOS/$runtime_name"', "macOS MCP destination");
mustContain(macosPackager, 'sign_and_verify_binary "MCP runtime" "$mcp_runtime"', "macOS MCP signing and immediate verification");
mustContain(macosPackager, 'sign_and_verify_binary "main executable" "$main_executable"', "macOS main executable signing and immediate verification");
mustContain(macosPackager, 'codesign --force --sign - "$app_dir"', "macOS app bundle signing");
mustContain(macosPackager, 'codesign --verify --deep --strict --verbose=4 "$app_dir"', "macOS deep app bundle verification");
mustNotContain(macosPackager, 'codesign --force --deep --sign - "$app_dir"', "macOS deprecated deep app bundle signing");

for (const [label, source] of [["auto", auto], ["manual", manual]]) {
  mustContain(source, "Copy-Item target/release/claude-codex-pro-mcp.exe dist/windows/app/", `${label} Windows MCP staging`);
  mustContain(source, 'app="dist/macos/stage/Claude Codex Pro.app"', `${label} macOS app verification`);
  mustContain(source, "for runtime in claude-codex-pro claude-codex-pro-mcp", `${label} macOS runtime verification`);
  mustNotContain(source, "target/release/claude-codex-pro-manager", `${label} legacy manager staging`);
  mustNotContain(source, "Claude Codex Pro Manager.app", `${label} legacy manager app`);
  mustContain(source, "windows-x64-setup.exe", label);
  mustContain(source, "windows-x64.zip", label);
  mustContain(source, "latest.json", label);
  mustContain(source, "Compress-Archive", label);
  mustContain(source, "ditto -c -k --sequesterRsrc", label);
  mustContain(source, "package-dmg.sh", `${label} macOS DMG build`);
  mustContain(source, "stage-multica-runtime.mjs", `${label} Multica runtime staging`);
  mustContain(source, "--target x86_64-pc-windows-msvc --destination dist/windows/app/resources/multica", `${label} Windows Multica staging`);
  mustContain(source, '--target "${{ matrix.target }}"', `${label} macOS Multica target staging`);
  mustContain(source, 'target/multica-runtime/${{ matrix.target }}', `${label} macOS Multica staging destination`);
  mustContain(source, "MULTICA_RESOURCE_DIR=", `${label} macOS Multica resource injection`);
  mustContain(source, "dist/macos/", `${label} macOS artifact path`);
  mustContain(source, "runs-on: windows-latest", `${label} Windows runner`);
  mustContain(source, "runner: macos-15-intel", `${label} macOS x64 Intel runner`);
  mustContain(source, "runner: macos-latest", `${label} macOS arm64 runner`);
  assert.equal(source.match(/runner: macos-latest/g)?.length, 1, `${label} must reserve macos-latest for arm64`);
  mustContain(source, "uses: actions/checkout@v5", `${label} checkout action`);
  mustContain(source, "uses: actions/setup-node@v5", `${label} setup-node action`);
  mustContain(source, 'node-version: "24"', `${label} Node.js version`);
  for (const deprecated of ["windows-2025", "macos-14", "macos-26-intel", "macos-26", "actions/checkout@v4", "actions/setup-node@v4", 'node-version: "22"']) {
    mustNotContain(source, deprecated, `${label} deprecated runner/action`);
  }
}

mustContain(auto, "dist/macos/*.dmg", "auto macOS DMG artifact upload");
mustContain(auto, "dist/macos/*.zip", "auto macOS ZIP artifact upload");
mustContain(auto, "macos-${{ matrix.arch }}.zip", "auto macOS ZIP naming");
mustContain(manual, "macos-${{ matrix.arch }}.dmg", "manual macOS DMG artifact path");
mustContain(manual, "macos-${{ matrix.arch }}.zip", "manual macOS ZIP artifact path");

mustContain(auto, "## 更新内容", "auto release notes");
mustContain(auto, "## 验证", "auto release notes");
mustContain(auto, "## 构建产物说明", "auto release notes");
mustNotContain(auto, "## Assets 9", "auto release notes");
mustNotContain(auto, "Source code (zip)", "auto release notes");
mustNotContain(auto, "Source code (tar.gz)", "auto release notes");
mustNotContain(auto, "claude-codex-pro-${version}-macos-arm64.dmg", "auto release notes");
mustContain(auto, 'version="${tag#v}"', "auto release version variable");
mustContain(auto, 'gh release edit "$TAG"', "auto release update existing notes");
assert.ok(!auto.includes('Release $TAG already exists; assets will be replaced.\n            exit 0'), "auto release must not skip notes update for existing draft");

mustContain(auto, "gh release list --repo \"$REPO\" --exclude-drafts --exclude-pre-releases", "auto release published-tag source");
mustContain(auto, "node scripts/release/next-release-tag.js \"${published_tags[@]}\"", "auto release version from published releases");
mustContain(auto, "Deleting orphan release tag $tag before recreating it for this build.", "auto release orphan tag cleanup");
mustContain(auto, "git push origin \":refs/tags/$tag\"", "auto release orphan remote tag cleanup");
mustContain(auto, "gh api --method DELETE \"repos/$REPO/git/refs/tags/$TAG\" || true", "auto release failed tag cleanup");
mustContain(auto, "SHA: ${{ github.sha }}", "auto release current SHA input");
mustContain(auto, 'tag_sha="$(git rev-list -n 1 "$tag" 2>/dev/null || true)"', "auto draft tag SHA resolution");
mustContain(auto, 'if [ "$tag_sha" != "$SHA" ]; then', "auto draft tag SHA validation");
mustContain(auto, 'gh api --method DELETE "repos/$REPO/releases/$release_id"', "auto stale draft cleanup");
mustContain(auto, "always() && (failure() || cancelled())", "auto failed or cancelled cleanup");

for (const [label, source] of [["auto", auto], ["manual", manual]]) {
  mustContain(source, 'const releaseUrl = `https://github.com/${repo}/releases/tag/${tag}`;', `${label} stable release URL`);
  mustContain(source, "url: releaseUrl", `${label} latest.json release URL`);
  mustNotContain(source, "url: release.url ||", `${label} draft release URL fallback`);
  mustNotContain(source, "--json assets,body,tagName,url", `${label} release API URL input`);
  mustNotContain(source, "/untagged-", `${label} draft release URL`);
}

mustContain(auto, "uses: actions/upload-artifact@v5", "auto workflow artifacts");
mustContain(auto, "uses: actions/download-artifact@v5", "auto workflow artifacts");
mustContain(auto, "name: windows-x64-release-assets", "auto Windows workflow artifact");
mustContain(auto, "name: macos-${{ matrix.arch }}-release-assets", "auto macOS workflow artifact");
mustContain(auto, "gh release upload \"$TAG\" release-assets/* --clobber --repo \"$REPO\"", "auto release upload from publish job");
mustContain(auto, "Expected 6 build assets before latest.json", "auto release asset count guard");
mustNotContain(auto, "gh release upload $env:TAG $asset.FullName $zip.FullName --clobber", "Windows job direct release upload");

mustContain(windowsInstaller, 'File /r "${ROOT}\\dist\\windows\\app\\resources"', "Windows installer Multica resources");
mustContain(windowsInstaller, 'RMDir /r "$INSTDIR\\resources"', "Windows installer Multica resource uninstall");
mustContain(macosPackager, 'MULTICA_RESOURCE_DIR="${MULTICA_RESOURCE_DIR:-}"', "macOS Multica resource input");
mustContain(macosPackager, 'Contents/Resources/multica', "macOS Multica resource destination");

const coreVersion = multicaCore.match(/const MANAGED_RUNTIME_VERSION: &str = "([^"]+)";/)?.[1];
const stagedVersion = multicaStager.match(/const VERSION = "([^"]+)";/)?.[1];
const stagedTag = multicaStager.match(/const TAG = "([^"]+)";/)?.[1];
assert.ok(coreVersion, "Rust Multica manifest version is missing");
assert.equal(stagedVersion, coreVersion, "Rust and staging Multica versions must match");
assert.equal(stagedTag, `v${coreVersion}`, "Multica staging tag must match the fixed version");

const coreDigests = new Map(
  [...multicaCore.matchAll(/const (MANAGED_RUNTIME_[A-Z0-9_]+_SHA256): &str =\s*"([a-f0-9]{64})";/g)]
    .map((match) => [match[1], match[2]]),
);
const coreAssets = new Map(
  [...multicaCore.matchAll(/"([^"]+)"(?:\s*\|\s*"[^"]+")?\s*=>\s*\(\s*"([^"]+)",\s*"([^"]+)",\s*(MANAGED_RUNTIME_[A-Z0-9_]+_SHA256),\s*\)/g)]
    .map((match) => [match[2], { sourceTarget: match[1], name: match[3], sha256: coreDigests.get(match[4]) }]),
);
const stagedAssets = new Map(
  [...multicaStager.matchAll(/"([^"]+)":\s*\{\s*name:\s*"([^"]+)",\s*sha256:\s*"([a-f0-9]{64})",\s*\}/g)]
    .map((match) => [match[1], { name: match[2], sha256: match[3] }]),
);
const supportedTargets = [
  "aarch64-apple-darwin",
  "aarch64-pc-windows-msvc",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "x86_64-unknown-linux-gnu",
];
assert.deepEqual([...coreAssets.keys()].sort(), supportedTargets, "Rust Multica manifest must contain exactly six supported targets");
assert.deepEqual([...stagedAssets.keys()].sort(), supportedTargets, "staging Multica manifest must contain exactly six supported targets");
for (const target of supportedTargets) {
  const coreAsset = coreAssets.get(target);
  assert.equal(coreAsset.sourceTarget, target, `Rust Multica target alias must preserve ${target}`);
  assert.ok(coreAsset.sha256, `Rust Multica digest is missing for ${target}`);
  assert.deepEqual(stagedAssets.get(target), { name: coreAsset.name, sha256: coreAsset.sha256 }, `Multica asset mismatch for ${target}`);
}
mustNotContain(multicaStager, "path=${", "Multica staging logs");

async function verifyMulticaStagerNetworkContract() {
  const { fetchBounded, stage } = await import("./stage-multica-runtime.mjs");
  const downloadUrl = "https://github.com/multica-ai/multica/releases/download/v0.4.36/test-asset";

  let retryAttempts = 0;
  const retryDelays = [];
  const leakedSecret = "signed-query-secret";
  await assert.rejects(
    fetchBounded(downloadUrl, 1024, {
      fetchImpl: async (_url, { signal }) => {
        retryAttempts += 1;
        assert.ok(signal instanceof AbortSignal, "download fetch must receive a total-timeout signal");
        const cause = Object.assign(
          new Error(`socket reset at https://release-assets.githubusercontent.com/file?sig=${leakedSecret}`),
          { code: "ECONNRESET" },
        );
        throw new TypeError(`fetch failed with Bearer ${leakedSecret}`, { cause });
      },
      sleepImpl: async (delay) => retryDelays.push(delay),
      timeoutMs: 1_000,
    }),
    (error) => {
      assert.match(error.message, /download failed after 3 attempts/);
      assert.ok(!error.message.includes(leakedSecret), "download errors must redact secrets");
      assert.ok(!error.message.includes("https://"), "download errors must not expose full URLs");
      return true;
    },
  );
  assert.equal(retryAttempts, 3, "transient network failures must use a finite three-attempt budget");
  assert.equal(retryDelays.length, 2, "the downloader must not sleep after the final attempt");

  let statusAttempts = 0;
  const statusResult = await fetchBounded(downloadUrl, 1024, {
    fetchImpl: async () => {
      statusAttempts += 1;
      return statusAttempts === 1
        ? new Response("temporarily unavailable", { status: 503 })
        : new Response(Uint8Array.of(1, 2, 3), { status: 200 });
    },
    sleepImpl: async () => {},
    timeoutMs: 1_000,
  });
  assert.deepEqual([...statusResult], [1, 2, 3]);
  assert.equal(statusAttempts, 2, "retryable HTTP failures must be retried");

  let permanentAttempts = 0;
  await assert.rejects(
    fetchBounded(downloadUrl, 1024, {
      fetchImpl: async () => {
        permanentAttempts += 1;
        return new Response("missing", { status: 404 });
      },
      sleepImpl: async () => assert.fail("permanent HTTP failures must not back off or retry"),
      timeoutMs: 1_000,
    }),
    /download failed with HTTP 404/,
  );
  assert.equal(permanentAttempts, 1, "permanent HTTP failures must fail immediately");

  let redirectAttempts = 0;
  await assert.rejects(
    fetchBounded(downloadUrl, 1024, {
      fetchImpl: async () => {
        redirectAttempts += 1;
        return new Response(null, {
          status: 302,
          headers: { location: "https://example.invalid/archive?signature=do-not-log" },
        });
      },
      sleepImpl: async () => assert.fail("non-allowlisted redirects must not retry"),
      timeoutMs: 1_000,
    }),
    /download host or protocol is not allowlisted/,
  );
  assert.equal(redirectAttempts, 1, "a non-allowlisted redirect must be rejected before another request");

  let timeoutAttempts = 0;
  await assert.rejects(
    fetchBounded(downloadUrl, 1024, {
      fetchImpl: async (_url, { signal }) => {
        timeoutAttempts += 1;
        return new Promise((resolve, reject) => {
          const guard = setTimeout(() => reject(new Error("timeout signal did not fire")), 250);
          signal.addEventListener("abort", () => {
            clearTimeout(guard);
            reject(signal.reason);
          }, { once: true });
        });
      },
      sleepImpl: async () => assert.fail("the total timeout must not be retried"),
      timeoutMs: 10,
    }),
    /download timed out/,
  );
  assert.equal(timeoutAttempts, 1, "the total timeout must terminate the current download");

  const destination = fs.mkdtempSync(path.join(os.tmpdir(), "ccp-multica-stager-test-"));
  let checksumRequests = 0;
  try {
    await assert.rejects(
      stage("x86_64-pc-windows-msvc", destination, {
        fetchImpl: async () => {
          checksumRequests += 1;
          return new Response(`${"0".repeat(64)}  multica-cli-0.4.36-windows-amd64.zip\n`);
        },
        sleepImpl: async () => assert.fail("checksum validation failures must not retry"),
        timeoutMs: 1_000,
      }),
      /release checksum declaration mismatch/,
    );
    assert.equal(checksumRequests, 1, "checksum declaration failures must stop before archive download");

    const stagedAsset = stagedAssets.get("x86_64-pc-windows-msvc");
    let archiveRequests = 0;
    await assert.rejects(
      stage("x86_64-pc-windows-msvc", destination, {
        fetchImpl: async () => {
          archiveRequests += 1;
          return archiveRequests === 1
            ? new Response(`${stagedAsset.sha256}  ${stagedAsset.name}\n`)
            : new Response("corrupt archive");
        },
        sleepImpl: async () => assert.fail("archive checksum failures must not retry"),
        timeoutMs: 1_000,
      }),
      /archive checksum mismatch/,
    );
    assert.equal(archiveRequests, 2, "archive checksum failures must stop after one archive response");
  } finally {
    fs.rmSync(destination, { recursive: true, force: true });
  }
}

verifyMulticaStagerNetworkContract()
  .then(() => console.log("release workflow contract passed"))
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
