# 验收标准：关于页更新检查与下载体验

验证对象：`spec/about-update-download-experience.md`

## 通过 / 失败标准

### A. 检查更新快速且具备回退

通过：

- Core 定义并使用 GitHub Releases API 最新发布地址。
- API、`releases/latest` Tag 重定向与 `latest.json` 任一可信来源成功即可完成检查，单一来源异常不会直接导致失败。
- API 返回匿名限流且 `latest.json` 资源域不可用时，Tag 重定向仍能返回固定仓库的最新版本和当前平台安装包地址。
- 系统代理路径缓慢或不可用时，可信 GitHub Tag 重定向可通过直连路径完成；系统代理可用时仍可参与竞速。
- 首轮全部失败时只有限重试轻量 Tag 请求；成功后短时间内重复检查不再次访问网络。
- 检查客户端使用短于普通 60 秒请求的独立超时。

失败：

- 仍只请求 `latest.json`。
- Release 资源域不可达时必须等待完整长超时后才失败。

### B. 安装包流式下载且不破坏完整文件

通过：

- 下载响应按块写入 `.part` 文件，不调用 `Response::bytes()` 读取整包。
- 安装包连接同时尝试系统代理和直连，采用首个成功响应继续流式下载。
- 失败时清理临时文件，并且不会提前覆盖同名完整安装包。
- 完整下载后才替换目标文件并调用现有安装器启动逻辑。

失败：

- 下载仍整包驻留内存。
- 中断后正式安装包是半成品，或遗留会被误启动的临时文件。

### C. 页面有即时反馈和真实进度

通过：

- Tauri 发出 `update-download-progress`，至少覆盖连接、下载、启动、完成和失败。
- 点击检查后立即显示“检查中”；点击下载后立即显示连接状态。
- 下载中显示百分比或未知总量状态，并显示已下载字节。
- 操作期间两个更新按钮禁用，完成或失败后恢复。
- Button 有 `active` 按压反馈。
- 已取得最新版本但发布正文为空时，说明区域显示成功提示，不显示“暂未检查”。

失败：

- 用户点击后长时间看不到任何变化。
- 只能看到最终成功/失败，没有下载进度。
- 操作可被重复触发或失败后永久停留在运行态。

## 必需验证

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test updater -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
```

## 完成证据

- 定向 Rust 测试通过。
- TypeScript 检查与 Vite 构建通过。
- 默认 `target/release/claude-codex-pro-manager.exe` 的修改时间来自本次构建。
- 本地启动后关于页可显示检查运行态和下载进度结构。

## 非目标

- 不要求在验收中实际下载并执行线上安装包。
- 不要求修改 GitHub Release 发布工作流。
- 不要求重做关于页或其他管理页面。
