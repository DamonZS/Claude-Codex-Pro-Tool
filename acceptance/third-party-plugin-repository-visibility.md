# 验收标准：工具与插件第三方插件仓库入口可见性修复

验证对象：`spec/third-party-plugin-repository-visibility.md`

## 验收项

1. 规格文档存在。
   - 通过标准：`spec/third-party-plugin-repository-visibility.md` 存在。
   - 证据：文件存在检查。

2. 验收标准存在。
   - 通过标准：`acceptance/third-party-plugin-repository-visibility.md` 存在。
   - 证据：文件存在检查。

3. “第三方插件仓库”入口在工具与插件页面可见。
   - 通过标准：相关 UI 源码包含“第三方插件仓库”文案，并位于工具/插件页面渲染路径。
   - 证据：源码检查、测试或截图。

4. 入口不破坏已有插件入口。
   - 通过标准：插件中心、Ponytail、Codex 插件仓库等既有入口未被删除。
   - 证据：源码检查或 UI 验证。

5. 项目静态检查通过。
   - 通过标准：前端类型检查或等价验证通过。
   - 证据：命令输出。

6. 桌面程序加载的前端产物包含该入口。
   - 通过标准：Tauri 桌面程序使用的构建产物或重新构建后的桌面窗口包含“第三方插件仓库”入口。
   - 证据：`npm run vite:build`、`cargo build -p claude-codex-pro-manager`、桌面窗口重启后截图或可见性检查。

7. 本次修复不修改无关模块。
   - 通过标准：变更集中在相关文档和工具/插件 UI 代码。
   - 证据：`git status --short` 和 diff 检查。

## 必需验证

至少运行：

```powershell
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml
```

如该命令因仓库既有无关改动失败，必须说明失败位置，并提供更窄的源码/渲染路径验证证据。

## 不在范围内

- 完整 release 构建。
- 修改插件安装后端策略。
- 修复无关脏工作区文件。
