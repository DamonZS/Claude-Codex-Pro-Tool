# 验收标准：Codex 更新与注入回归修复

验证对象：spec/codex-update-and-injection-regressions.md

## 通过 / 失败标准

### A. Windows 更新安装器可正常拉起

通过：
- launch_installer() 在 Windows 下不再直接对安装包使用 Command::spawn()。
- 存在面向本地路径的 Windows Shell 打开 helper，并由 launch_installer() 调用。
- perform_update() 的返回结构不变，下载完成后仍标记 launched: true。

失败：
- Windows 下仍直接 spawn 安装包。
- 需要提权的安装器仍可能因 740 / 直接 CreateProcess 而失败。

### B. 注入弹窗不再出现 CCP 模型选择注入

通过：
- 注入脚本不再扫描 Codex 模型菜单或安装模型状态补丁。
- CCP 弹窗节点不包含模型候选、`CCP 模型增强` 组或请求模型覆盖。
- Codex 原生模型菜单仍可由 Codex 自身正常渲染。

失败：
- 仍有模型菜单扫描、模型候选追加、状态补丁或请求模型覆盖路径。
- 仅提升 z-index，却保留 CCP 模型选择注入。

### C. Codex 不再误翻成“代码”

通过：
- 注入脚本不再包含宽泛的 ["Code", "代码"] 映射。
- 注入脚本保留 Codex / Claude / Claude Code 的品牌词保护。
- 中文覆盖层仍保留其他既有翻译能力。

失败：
- 仍保留 ["Code", "代码"] 这类会误伤品牌词的映射。
- 通过直接关闭整个中文覆盖层来规避问题。

## 必需验证

    cargo fmt --check
    cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test updater -- --nocapture
    cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture
    npm --prefix apps/claude-codex-pro-manager run check
    npm --prefix apps/claude-codex-pro-manager run vite:build

如构建或测试被运行中的 Manager / Codex 占用，可先核实对应可执行文件路径，再结束阻塞进程后重跑。

## 完成证据

- 定向 Rust 测试输出。
- 前端类型检查与构建输出。
- 相关源码 diff，能看出：
  - Windows 安装包改为 Shell 打开本地路径
  - Codex 模型选择注入的负向契约
  - ["Code", "代码"] 映射已移除或收窄到不会误伤品牌词

## 非目标

- 不要求在本地真正执行 GitHub Release 下载。
- 不要求改动管理工具其他页面。
- 不要求重做 control deck 设计。
