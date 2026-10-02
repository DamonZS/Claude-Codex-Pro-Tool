#!/usr/bin/env node
// Builds the "更新内容" list for an auto release from the commit subjects
// between the previous published release tag and the commit being released.
//
// Usage: node scripts/release/release-changelog.mjs <current-tag> <sha> [repo] [previous-tag ...]
// The trailing tags are the published (non-draft) release tags; the nearest
// one that is an ancestor of <sha> is used as the starting point.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

const RELEASE_TAG = /^[vV](\d+)\.(\d{2})$/;
export const MAX_ENTRIES = 50;
const MAX_SUBJECT = 160;

function tagValue(tag) {
  const match = String(tag).trim().match(RELEASE_TAG);
  return match ? Number(match[1]) * 100 + Number(match[2]) : null;
}

/** Newest-first list of release tags other than `current`. */
export function candidateTags(tags, current) {
  return [...new Set(tags.map((tag) => String(tag).trim()))]
    .filter((tag) => tag && tag !== current && tagValue(tag) !== null)
    .sort((left, right) => tagValue(right) - tagValue(left));
}

/** One markdown bullet per commit subject; subjects only, never bodies. */
export function formatChangelog(commits, { repo, previousTag, currentTag } = {}) {
  if (!commits.length) {
    return "- 与上一版本相比没有新的代码提交。";
  }
  const lines = commits.slice(0, MAX_ENTRIES).map(({ sha, subject }) => {
    let text = subject.replace(/\s+/g, " ").trim().replace(/^[-*+]\s+/, "");
    if ([...text].length > MAX_SUBJECT) text = `${[...text].slice(0, MAX_SUBJECT).join("")}…`;
    const short = sha.slice(0, 7);
    const ref = repo ? `[${short}](https://github.com/${repo}/commit/${sha})` : short;
    return `- ${text}（${ref}）`;
  });
  if (commits.length > MAX_ENTRIES) {
    lines.push(`- 以及另外 ${commits.length - MAX_ENTRIES} 个提交`);
  }
  if (repo && previousTag && currentTag) {
    lines.push("", `完整变更：https://github.com/${repo}/compare/${previousTag}...${currentTag}`);
  }
  return lines.join("\n");
}

function git(args) {
  return execFileSync("git", args, { encoding: "utf8" });
}

function isAncestor(tag, sha) {
  try {
    execFileSync("git", ["merge-base", "--is-ancestor", tag, sha], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

/** Non-merge commits in `previous..sha` (or the last MAX_ENTRIES when there is no previous release). */
export function readCommits(previousTag, sha) {
  const range = previousTag ? [`${previousTag}..${sha}`] : ["-n", String(MAX_ENTRIES + 1), sha];
  const out = git(["log", "--no-merges", "--format=%H%x1f%s%x1e", ...range]);
  return out
    .split("\x1e")
    .map((record) => record.trim())
    .filter(Boolean)
    .map((record) => {
      const [hash, subject = ""] = record.split("\x1f");
      return { sha: hash.trim(), subject };
    })
    .filter((commit) => commit.sha && commit.subject.trim());
}

function main([currentTag, sha, repo, ...publishedTags]) {
  if (!currentTag || !sha) {
    throw new Error("usage: release-changelog.mjs <current-tag> <sha> [repo] [previous-tag ...]");
  }
  const previousTag = candidateTags(publishedTags, currentTag).find((tag) => isAncestor(tag, sha)) ?? null;
  const commits = readCommits(previousTag, sha);
  process.stdout.write(`${formatChangelog(commits, { repo, previousTag, currentTag })}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2));
}
