# Codex 注入内容边界验收标准

对应规格：`spec/codex-injection-content-boundary.md`

## 通过标准

1. `renderer-inject.js` 不再把 `claudeAppChineseOverlayEnabled` 映射到 Codex 中文覆盖功能。
2. Codex 主扫描流程不再调用通用文本翻译观察器或全页翻译刷新函数。
3. 即使遗留翻译函数被误调用，也不能继续对传入文本执行词表 `replaceAll` 或返回翻译结果。
4. 注入脚本仍包含 CCP 自有状态入口、模型选择器修复和盘古记忆入口，现有功能契约测试通过。
5. Claude 中文注入脚本与 Claude 本机汉化资源未被本任务修改。
6. 完整 release 构建输出到 `target/release/claude-codex-pro-manager.exe`，并使用该产物启动 Manager。

## 必需验证

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo build --release
```

## 手动检查

- 通过新构建的 Manager 重启 Codex。
- 在 Codex 输入区输入包含 `Project`、`Settings`、`Open`、`Save` 等词的英文句子，显示保持原文。
- 打开英文项目名、英文对话正文、代码块、路径和终端输出，内容保持原文。
- 打开模型选择器和 CCP 注入弹窗，确认核心注入功能仍可用。

## 完成证据

- 定向 Rust 测试和 Windows 契约测试通过。
- 前端类型检查、Vite 构建、格式检查和 release 构建通过。
- 最终可执行文件路径、修改时间和文件大小已核对。

## 非范围检查

- 不要求 Codex 原生界面提供中文翻译。
- 不验收 Claude 中文窗口或 Claude 本机资源补丁的翻译完整度。
