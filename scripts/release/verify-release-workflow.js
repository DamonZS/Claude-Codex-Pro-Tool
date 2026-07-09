#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");

const auto = fs.readFileSync(".github/workflows/auto-release-installers.yml", "utf8");
const manual = fs.readFileSync(".github/workflows/release-assets.yml", "utf8");

function mustContain(source, needle, label) {
  assert.ok(source.includes(needle), `${label} missing: ${needle}`);
}

const windowsInstaller = fs.readFileSync("scripts/installer/windows/ClaudeCodexPro.nsi", "utf8");
const macosPackager = fs.readFileSync("scripts/installer/macos/package-dmg.sh", "utf8");

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
mustContain(windowsInstaller, 'File "${ROOT}\\dist\\windows\\app\\claude-codex-pro-manager.exe"', "Windows installer manager source");
mustContain(macosPackager, "create_app \"Claude Codex Pro\"", "macOS app bundle");
mustContain(macosPackager, "create_app \"Claude Codex Pro Manager\"", "macOS manager bundle");

for (const [label, source] of [["auto", auto], ["manual", manual]]) {
  mustContain(source, "windows-x64-setup.exe", label);
  mustContain(source, "windows-x64.zip", label);
  mustContain(source, "latest.json", label);
  mustContain(source, "Compress-Archive", label);
  mustContain(source, "ditto -c -k --sequesterRsrc", label);
  mustContain(source, "package-dmg.sh", `${label} macOS DMG build`);
  mustContain(source, "dist/macos/", `${label} macOS artifact path`);
  mustContain(source, "runs-on: windows-latest", `${label} Windows runner`);
  mustContain(source, "runner: macos-latest", `${label} macOS runner`);
  assert.ok(source.match(/runner: macos-latest/g)?.length >= 2, `${label} must use macos-latest for both macOS matrix entries`);
  mustContain(source, "uses: actions/checkout@v5", `${label} checkout action`);
  mustContain(source, "uses: actions/setup-node@v5", `${label} setup-node action`);
  mustContain(source, 'node-version: "24"', `${label} Node.js version`);
  for (const deprecated of ["windows-2025", "macos-15-intel", "macos-14", "macos-26-intel", "macos-26", "actions/checkout@v4", "actions/setup-node@v4", 'node-version: "22"']) {
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

mustContain(auto, "uses: actions/upload-artifact@v5", "auto workflow artifacts");
mustContain(auto, "uses: actions/download-artifact@v5", "auto workflow artifacts");
mustContain(auto, "name: windows-x64-release-assets", "auto Windows workflow artifact");
mustContain(auto, "name: macos-${{ matrix.arch }}-release-assets", "auto macOS workflow artifact");
mustContain(auto, "gh release upload \"$TAG\" release-assets/* --clobber --repo \"$REPO\"", "auto release upload from publish job");
mustContain(auto, "Expected 6 build assets before latest.json", "auto release asset count guard");
mustNotContain(auto, "gh release upload $env:TAG $asset.FullName $zip.FullName --clobber", "Windows job direct release upload");

console.log("release workflow contract passed");
