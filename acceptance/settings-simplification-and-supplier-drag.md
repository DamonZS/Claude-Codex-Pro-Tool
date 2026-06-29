# 验收标准：设置页精简与供应商拖拽修复

验证对象：`spec/settings-simplification-and-supplier-drag.md`

## 验收项

1. 文档存在
   - 通过标准：本文件与 `spec/settings-simplification-and-supplier-drag.md` 存在。
   - 证据：文件存在检查。

2. 设置页指定卡片已删除
   - 通过标准：`SettingsScreen` 不再渲染 `Codex 启动参数`、`图片覆盖`、`盘古记忆`、`安全边界`四个独立 `Panel`。
   - 证据：`windows_subsystem` 回归测试或源码检查。

3. 设置页保留核心卡片
   - 通过标准：设置文件位置、Codex 增强矩阵、Claude 一键汉化、CLI Wrapper 和运行日志仍存在。
   - 证据：`windows_subsystem` 回归测试或源码检查。

4. 供应商拖拽源可稳定识别
   - 通过标准：供应商拖拽开始时写入自定义 `dataTransfer` MIME 和 `text/plain`，拖拽悬停/释放时可从 `dataTransfer` 读取源 id。
   - 证据：`windows_subsystem` 回归测试或源码检查。

5. 供应商排序保存逻辑不变
   - 通过标准：释放拖拽后仍调用现有 `saveSupplierOrder`，并通过 `actions.saveSettings` 保存 `relayProfiles` 顺序。
   - 证据：`windows_subsystem` 回归测试或源码检查。

6. 前端类型检查通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
   - 证据：命令输出。

7. 前端生产构建通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
   - 证据：命令输出。

8. Manager UI 回归测试通过
   - 通过标准：`cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture` 成功。
   - 证据：命令输出。

9. Debug 管理工具重新构建
   - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 证据：命令输出。

## 不在范围内

- 删除盘古记忆其它页面能力。
- 删除后端设置字段。
- 修改 Claude 中文注入脚本。
- 新增依赖。
