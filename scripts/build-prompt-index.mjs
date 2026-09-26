#!/usr/bin/env node
// 扫描 Prompt/ 生成 index.json；--check 只校验清单与目录一致。
//
//   node scripts/build-prompt-index.mjs           # 重新生成
//   node scripts/build-prompt-index.mjs --check   # 不一致则非零退出（CI 用）

import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const promptRoot = fileURLToPath(new URL("../Prompt/", import.meta.url));
const indexPath = path.join(promptRoot, "index.json");
const checkOnly = process.argv.includes("--check");

function listFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listFiles(full));
    else if (entry.isFile()) out.push(path.relative(promptRoot, full).split(path.sep).join("/"));
  }
  return out;
}

// 可选 front matter：--- 之间用 `key: value`。缺省时用文件名做标题。
function frontMatter(text) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---/.exec(text);
  if (!match) return {};
  const meta = {};
  const lines = match[1].split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const kv = /^([A-Za-z_][\w-]*)\s*:\s*(.*)$/.exec(lines[index].trim());
    if (!kv) continue;
    const key = kv[1].toLowerCase();
    let value = kv[2].trim();
    if (value === ">" || value === "|-") {
      const folded = [];
      while (index + 1 < lines.length && /^\s+/.test(lines[index + 1])) {
        folded.push(lines[index + 1].trim());
        index += 1;
      }
      value = value === ">" ? folded.join(" ") : folded.join("\n");
    }
    meta[key] = value.replace(/^["']|["']$/g, "");
  }
  return meta;
}

// 允许的目标 id；与 core 的 client_deploy 注册表保持一致。
const TARGET_IDS = ["codex", "claude-code", "deepseek-harness", "zcode", "workbuddy", "workbuddy-cn", "cursor"];

// 每个客户端内容不同，因此提示词要声明适用目标。
// `targets: codex, zcode` 表示只投放到这两个；缺省表示适配全部目标。
function parseTargets(raw, file) {
  if (!raw) return [...TARGET_IDS];
  const list = raw
    .split(/[,\s]+/)
    .map((value) => value.trim())
    .filter(Boolean);
  const unknown = list.filter((id) => !TARGET_IDS.includes(id));
  if (unknown.length) {
    throw new Error(`${file} 的 targets 含未知目标：${unknown.join("、")}`);
  }
  if (!list.length) return [...TARGET_IDS];
  return list;
}

function collectPrompts() {
  const dir = path.join(promptRoot, "prompts");
  let names;
  try {
    names = readdirSync(dir).filter((name) => name.endsWith(".md"));
  } catch {
    return [];
  }
  return names.sort().map((name) => {
    const meta = frontMatter(readFileSync(path.join(dir, name), "utf8"));
    return {
      id: name.replace(/\.md$/, ""),
      title: meta.title || meta.name || name.replace(/\.md$/, ""),
      category: meta.category || "未分类",
      description: meta.description || "",
      version: meta.version || "",
      targets: parseTargets(meta.targets, name),
      skills: meta.skills ? meta.skills.split(/[;,\s]+/).filter(Boolean) : [],
      tools: meta.tools ? meta.tools.split(/[;,\s]+/).filter(Boolean) : [],
      path: `prompts/${name}`,
    };
  });
}

function collectSkills() {
  const dir = path.join(promptRoot, "skills");
  let names;
  try {
    names = readdirSync(dir, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort();
  } catch {
    return [];
  }
  return names.map((name) => {
    const skillDir = path.join(dir, name);
    const entryFile = path.join(skillDir, "SKILL.md");
    let meta = {};
    try {
      meta = frontMatter(readFileSync(entryFile, "utf8"));
    } catch {
      // 没有 SKILL.md 的目录仍列出，安装端会跳过。
    }
    return {
      name,
      description: meta.description || "",
      path: `skills/${name}`,
      files: listFiles(skillDir),
    };
  });
}

function collectTools() {
  const dir = path.join(promptRoot, "tools");
  let names;
  try {
    names = readdirSync(dir, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort();
  } catch {
    return [];
  }
  return names.map((name) => {
    if (!/^[a-z0-9][a-z0-9-]*$/.test(name)) throw new Error(`工具 ID 不合法：${name}`);
    const toolDir = path.join(dir, name);
    const metadataPath = path.join(toolDir, "tool.json");
    const metadata = JSON.parse(readFileSync(metadataPath, "utf8"));
    if (metadata.id !== name) throw new Error(`${name}/tool.json 的 id 与目录名不一致`);
    for (const key of ["title", "version", "description", "sourceRepo", "sourceRevision", "licenseId", "licenseFile"]) {
      if (typeof metadata[key] !== "string" || !metadata[key].trim()) {
        throw new Error(`${name}/tool.json 缺少 ${key}`);
      }
    }
    if (!/^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(metadata.sourceRepo)) {
      throw new Error(`${name}/tool.json 的 sourceRepo 必须是 GitHub 仓库 HTTPS 地址`);
    }
    if (!/^[a-f0-9]{40}$/i.test(metadata.sourceRevision)) {
      throw new Error(`${name}/tool.json 的 sourceRevision 必须是完整 Git commit`);
    }
    if (!Array.isArray(metadata.platforms) || metadata.platforms.length === 0) {
      throw new Error(`${name}/tool.json 必须声明 platforms`);
    }

    const files = [];
    const walk = (current) => {
      for (const entry of readdirSync(current, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
        const full = path.join(current, entry.name);
        if (entry.isSymbolicLink()) throw new Error(`${name} 不得包含符号链接：${full}`);
        if (entry.isDirectory()) walk(full);
        else if (entry.isFile() && full !== metadataPath) {
          const bytes = readFileSync(full);
          files.push({
            path: path.relative(promptRoot, full).split(path.sep).join("/"),
            size: bytes.length,
            sha256: createHash("sha256").update(bytes).digest("hex"),
          });
        }
      }
    };
    walk(toolDir);
    const licensePath = `tools/${name}/${metadata.licenseFile}`;
    if (!files.some((file) => file.path === licensePath)) {
      throw new Error(`${name} 缺少许可证文件 ${metadata.licenseFile}`);
    }
    return {
      id: name,
      title: metadata.title,
      version: metadata.version,
      description: metadata.description,
      sourceRepo: metadata.sourceRepo,
      sourceRevision: metadata.sourceRevision,
      licenseId: metadata.licenseId,
      licensePath,
      platforms: metadata.platforms,
      size: files.reduce((sum, file) => sum + file.size, 0),
      files,
    };
  });
}

const manifest = { version: 1, prompts: collectPrompts(), skills: collectSkills(), tools: collectTools() };
const serialized = `${JSON.stringify(manifest, null, 2)}\n`;

if (checkOnly) {
  let current = "";
  try {
    current = readFileSync(indexPath, "utf8");
  } catch {
    console.error("Prompt/index.json 不存在，请运行 node scripts/build-prompt-index.mjs");
    process.exit(1);
  }
  if (current !== serialized) {
    console.error("Prompt/index.json 与 Prompt/ 实际内容不一致。");
    console.error("运行 node scripts/build-prompt-index.mjs 重新生成后提交。");
    process.exit(1);
  }
  console.log(`Prompt 清单一致：${manifest.prompts.length} 个提示词，${manifest.skills.length} 个技能，${manifest.tools.length} 个工具包。`);
  process.exit(0);
}

writeFileSync(indexPath, serialized, "utf8");
console.log(`已写入 Prompt/index.json：${manifest.prompts.length} 个提示词，${manifest.skills.length} 个技能，${manifest.tools.length} 个工具包。`);
