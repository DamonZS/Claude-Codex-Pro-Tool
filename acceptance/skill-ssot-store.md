# Skills 单一真源（SSOT）存储 — 通过标准

验证对象：`spec/skill-ssot-store.md`。全部用例位于 `crates/claude-codex-pro-core/tests/skill_store.rs`，使用 `tempfile`，不接触真实用户目录。

## 通过标准

1. **导入后可列出**：把含 `SKILL.md` 的 `demo-skill` 目录导入，返回条目 `id == "demo-skill"`、`name` 取 frontmatter `name`（缺失时为 id）、`source_app` 为传入值、`imported_at` 能被 `chrono::DateTime::parse_from_rfc3339` 解析；`<home>/skills/demo-skill/SKILL.md` 存在且字节与源一致，子目录与普通文件同样被复制；`list_store_skills` 返回同一条目；`<home>/skills/index.json` 可解析为 `{version: 1, entries: [...]}`。
2. **缺少 SKILL.md 被拒**：源目录不含 `SKILL.md` 时 `import_skill_to_store` 返回错误，错误信息包含 `SKILL.md`，且 `<home>/skills` 下不产生该 id 目录。
3. **重复导入被拒**：同 id 第二次导入返回错误且信息包含目标路径；已入库 `SKILL.md` 字节内容不变（无静默覆盖）。
4. **同步到 codex**：`sync_skill_to_app(home, "demo-skill", "codex")` 返回 `<user_home>/.codex/skills/demo-skill`，该目录含 `SKILL.md` 且内容与真源一致。
5. **同步到 claude**：`app = "claude"` 返回 `<user_home>/.claude/skills/demo-skill`，内容一致；未知应用（例如 `"workbuddy"`）返回错误且信息包含该应用名。
6. **目标已存在被拒**：预先创建 `<user_home>/.codex/skills/demo-skill` 后再同步，返回错误且信息包含目标路径，已存在的 `SKILL.md` 不被改写。
7. **ID 逃逸被拒**：`id` 为 `"../escape"`、`"a/b"`、`".."`、`""` 时 `sync_skill_to_app` 均返回错误，信息包含 `Skill ID`，且不发生任何目录创建；`import_skill_to_store` 对名称为 `.` 或含 `..` 的源目录同样拒绝（在 Windows 上不可创建时用可创建的等价名验证）。
8. **索引损坏可恢复**：手写非法 JSON 到 `index.json` 后调用 `list_store_skills`，仍按目录列出全部 Skill，并把 `index.json` 重写为合法 JSON（`version == 1`，包含该 id）；删除某个真源目录后再次列出，该 id 不再出现。
9. **同步时真源缺失被拒**：`id` 不在真源库时返回错误且信息包含该 id。
10. **符号链接跳过（仅 unix）**：源目录内放一个指向文件的符号链接，导入后目标目录不含该链接；Windows 上不执行该断言（`#[cfg(unix)]`）。

## 必需验证方式

- `cargo test -p claude-codex-pro-core --test skill_store` 全部用例通过（至少 8 个）。
- `cargo fmt --all -- --check` 退出码 0（如环境无 rustfmt 则在报告中说明）。

## 证据要求

- 测试命令与通过用例数原文（例如 `test result: ok. 9 passed; 0 failed`）。
- 明确指出 CCP home 解析函数与文件行号（`crates/claude-codex-pro-core/src/paths.rs` 中 `default_app_state_dir`）。

## 非目标 / 不在本次检查范围

- 不要求 `cargo build --release`、不要求前端 `npm run check` 或 `vite:build`（本规格不改前端与命令层）。
- 不验证真实 `~/.codex`、`~/.claude` 目录行为，不验证应用侧实际识别 Skill。
- 不验证删除、重命名、去重、双向同步与索引版本迁移。