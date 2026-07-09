#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");

const auto = fs.readFileSync(".github/workflows/auto-release-installers.yml", "utf8");
const manual = fs.readFileSync(".github/workflows/release-assets.yml", "utf8");

function mustContain(source, needle, label) {
  assert.ok(source.includes(needle), `${label} missing: ${needle}`);
}

for (const [label, source] of [["auto", auto], ["manual", manual]]) {
  mustContain(source, "windows-x64-setup.exe", label);
  mustContain(source, "windows-x64.zip", label);
  mustContain(source, "macos-x64.dmg", label);
  mustContain(source, "macos-x64.zip", label);
  mustContain(source, "macos-arm64.dmg", label);
  mustContain(source, "macos-arm64.zip", label);
  mustContain(source, "latest.json", label);
  mustContain(source, "Compress-Archive", label);
  mustContain(source, "ditto -c -k --sequesterRsrc", label);
}

mustContain(auto, "## ????", "auto release notes");
mustContain(auto, "## ??", "auto release notes");
mustContain(auto, "## Assets 9", "auto release notes");
mustContain(auto, 'version="${TAG#v}"', "auto release version variable");
mustContain(auto, 'gh release edit "$TAG"', "auto release update existing notes");
assert.ok(!auto.includes('Release $TAG already exists; assets will be replaced.\n            exit 0'), "auto release must not skip notes update for existing draft");

console.log("release workflow contract passed");
