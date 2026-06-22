#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");

const RELEASE_TAG_PATTERN = /^[vV](\d+)\.(\d{2})$/;

function parseReleaseTag(tag) {
  const match = String(tag || "").trim().match(RELEASE_TAG_PATTERN);
  if (!match) {
    return null;
  }
  return {
    tag: match[0],
    value: Number.parseInt(match[1], 10) * 100 + Number.parseInt(match[2], 10),
  };
}

function nextReleaseTag(tags) {
  const latest = tags
    .map(parseReleaseTag)
    .filter(Boolean)
    .sort((left, right) => right.value - left.value)[0];
  const nextValue = latest ? latest.value + 1 : 1;
  const major = Math.floor(nextValue / 100);
  const minor = String(nextValue % 100).padStart(2, "0");
  return `V${major}.${minor}`;
}

function readGitTags() {
  return execFileSync("git", ["tag", "--list"], { encoding: "utf8" })
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function writeGithubOutput(tag) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) {
    return;
  }
  fs.appendFileSync(outputPath, `tag=${tag}\nnext_tag=${tag}\nversion=${tag.slice(1)}\n`, "utf8");
}

function runTests() {
  assert.equal(nextReleaseTag([]), "V0.01");
  assert.equal(nextReleaseTag(["v1.2.9"]), "V0.01");
  assert.equal(nextReleaseTag(["V0.01"]), "V0.02");
  assert.equal(nextReleaseTag(["V0.01", "V0.09", "V0.03"]), "V0.10");
  assert.equal(nextReleaseTag(["V0.99"]), "V1.00");
  assert.equal(nextReleaseTag(["v0.01", "V0.02"]), "V0.03");
  assert.equal(parseReleaseTag("V10.42").value, 1042);
  assert.equal(parseReleaseTag("V1.2.9"), null);
  assert.equal(parseReleaseTag("release-1"), null);
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes("--test")) {
    runTests();
    console.log("next-release-tag tests passed");
    return;
  }

  const tags = args.length > 0 ? args : readGitTags();
  const tag = nextReleaseTag(tags);
  writeGithubOutput(tag);
  console.log(tag);
}

if (require.main === module) {
  main();
}

module.exports = {
  nextReleaseTag,
  parseReleaseTag,
};
