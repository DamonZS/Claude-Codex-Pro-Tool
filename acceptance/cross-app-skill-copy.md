# 跨应用复制 Skill — 通过标准

1. **目标应用缺该 Skill 时复制成功**：临时 home 下只在 `codex/skills/demo-skill/SKILL.md` 放置 Skill，调用 `copy_skill_to_app(&roots, "demo-skill", "claude")` 后，`claude/skills/demo-skill/SKILL.md` 存在且内容与源一致。
2. **复制后清单反映可用**：同一调用的返回值里 `kind=="skill"` 且 `id=="demo-skill"` 的资产 `claude.available == true`。
3. **源不被修改**：复制前后 `codex/skills/demo-skill/SKILL.md` 字节内容一致，源目录保持不变。
4. **已存在则拒绝**：`claude/skills/demo-skill` 已存在时不覆盖，返回错误且错误信息包含目标路径。
5. **未知目标应用拒绝**：`target="workbuddy"` 返回错误且信息包含 `workbuddy`。
6. **未找到 Skill 拒绝**：`id` 不存在时返回错误且信息包含该 id。
7. **编译与测试**：`cargo test -p claude-codex-pro-core --test unified_tool_inventory` 全绿；`cargo fmt --all -- --check` 退出码 0。
8. **前端可用**：`npm --prefix apps/claude-codex-pro-manager run check` 退出码 0；`vite:build` 退出码 0。

## 非通过条件

- 复制时自动删除或移动源目录。
- 复制时改写目标应用的其它 Skill 或配置。