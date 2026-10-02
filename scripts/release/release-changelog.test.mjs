import assert from "node:assert/strict";
import test from "node:test";
import { MAX_ENTRIES, candidateTags, formatChangelog } from "./release-changelog.mjs";

test("candidate tags are release tags only, newest first, without the current tag", () => {
  assert.deepEqual(candidateTags(["V1.09", "v1.10", "V1.11", "nightly", "V1.08", "V1.11"], "V1.11"), [
    "v1.10",
    "V1.09",
    "V1.08",
  ]);
});

test("each commit subject becomes one bullet with a commit link", () => {
  const notes = formatChangelog(
    [
      { sha: "a".repeat(40), subject: "修复 Claude 一键汉化导致崩溃" },
      { sha: "b".repeat(40), subject: "- 供应商页覆盖多 Agent" },
    ],
    { repo: "owner/repo", previousTag: "V1.10", currentTag: "V1.11" },
  );
  assert.match(notes, /^- 修复 Claude 一键汉化导致崩溃（\[aaaaaaa\]\(https:\/\/github\.com\/owner\/repo\/commit\/a{40}\)）$/m);
  // A leading list marker in the subject is not doubled.
  assert.match(notes, /^- 供应商页覆盖多 Agent（/m);
  assert.match(notes, /完整变更：https:\/\/github\.com\/owner\/repo\/compare\/V1\.10\.\.\.V1\.11$/m);
});

test("long subjects are truncated and long lists are capped", () => {
  const long = "改".repeat(200);
  const commits = Array.from({ length: MAX_ENTRIES + 3 }, (_, index) => ({
    sha: String(index).padStart(40, "0"),
    subject: index === 0 ? long : `提交 ${index}`,
  }));
  const notes = formatChangelog(commits, {});
  const bullets = notes.split("\n").filter((line) => line.startsWith("- "));
  assert.equal(bullets.length, MAX_ENTRIES + 1);
  assert.ok(bullets[0].includes("…"));
  assert.ok(!bullets[0].includes("改".repeat(161)));
  assert.equal(bullets.at(-1), "- 以及另外 3 个提交");
});

test("no new commits yields an explicit note instead of an empty section", () => {
  assert.equal(formatChangelog([], {}), "- 与上一版本相比没有新的代码提交。");
});
