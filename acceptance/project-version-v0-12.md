# 验收标准：项目版本号统一为 V0.12

验证对象：`spec/project-version-v0-12.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/project-version-v0-12.md` 存在。

2. 验收文档存在
   - 通过标准：`acceptance/project-version-v0-12.md` 存在。

3. 对外版本常量正确
   - 通过标准：`claude_codex_pro_core::version::VERSION` 默认返回 `V0.12`。
   - 通过标准：测试覆盖不再默认显示 `1.2.9`。

4. Codex 注入显示正确
   - 通过标准：注入脚本包含 `CCP ${claudeCodexProVersion}` 和 `Claude Codex Pro ${claudeCodexProVersion}`。
   - 通过标准：注入脚本序列化的版本值包含 `V0.12`。

5. 管理工具关于页版本正确
   - 通过标准：`backend_version` 返回 `V0.12`。
   - 通过标准：前端 mock/update 当前版本不再使用 `1.2.9-preview`。

6. 发布递增规则正确
   - 通过标准：`scripts/release/next-release-tag.js` 仍验证 `V0.01 -> V0.02 -> ... -> V0.99 -> V1.00`。
   - 通过标准：测试确认 `V0.12` 新于旧 semver `1.2.9`。

7. 验证通过
   - 通过标准：运行版本相关 Rust 测试通过。
   - 通过标准：前端检查通过。
   - 通过标准：前端构建通过。
   - 通过标准：manager 与 launcher debug 构建通过。

## 不在范围内

- 不检查 GitHub Release 实际发布。
- 不检查用户本机已安装旧版本是否自动覆盖。
- 不验证 Claude 中文注入。
