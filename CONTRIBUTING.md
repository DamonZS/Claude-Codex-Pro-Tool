# 贡献指南 / Contributing

欢迎贡献。这个仓库的代码是 **MIT**：提 PR 即表示你同意按 MIT 授权你的贡献。

## 先看这三条

- 想聊用法、想问方向、不确定是不是 bug：开 [Discussions](https://github.com/DamonZS/Claude-Codex-Pro-Tool/discussions)。
- 想报 bug：用 issue 模板，带上版本号、系统、复现步骤和相关日志。
- 想改代码：先开一个 issue 说清你要做什么、打算怎么改，再动手 —— 免得白写。

## 本地开发

```bash
git clone https://github.com/DamonZS/Claude-Codex-Pro-Tool.git
cd Claude-Codex-Pro-Tool
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:dev   # 只跑前端，浏览器里预览
npm --prefix apps/claude-codex-pro-manager run dev        # Tauri 桌面应用
```

## 交付前自检

```bash
npm --prefix apps/claude-codex-pro-manager run check        # TypeScript
npm --prefix apps/claude-codex-pro-manager run test         # 前端单元测试
npm --prefix apps/claude-codex-pro-manager run vite:build   # 前端构建
cargo test -p claude-codex-pro-core --lib                   # 核心逻辑
cargo test -p claude-codex-pro-manager --test windows_subsystem   # 界面契约
cargo fmt --check
```

改动大、或者碰到共享逻辑（`crates/claude-codex-pro-core`）、发布脚本、数据迁移、用户可见工作流时，请把验证范围扩大到自己能证明它没问题。

## 风格

- Rust：`edition 2024`，先 `cargo fmt` 再提交。
- 前端：TypeScript + React，跟随现有文件的写法；界面文案中文优先，术语保持英文（Codex、Claude、API Key、Token 等）。
- 文档：中文为主，`README_EN.md` 保持同步。
- 不要提交本机状态：`target/`、`node_modules/`、`dist/`、`.dsh-ops/`、`postgres/`、`logs/` 都不进仓库。

## PR 里请写清四件事

1. 解决了什么问题（或加了什么功能）。
2. 改了哪些文件、为什么这样改。
3. 你跑了什么验证，结果如何。
4. 有没有你做不到、需要 reviewer 帮忙确认的部分 —— 如实写，没人会因此扣分。

## English

Contributions are welcome. The code is **MIT**; opening a PR means you agree to license your contribution under MIT.

- Questions and ideas → [Discussions](https://github.com/DamonZS/Claude-Codex-Pro-Tool/discussions).
- Bugs → the issue template, with version, OS, reproduction steps and logs.
- Code changes → open an issue first so we can agree on the shape.
- Before a PR: run the checks above and describe what you verified (and what you could not).

## 许可

MIT。详见 [LICENSE](LICENSE)、[TRADEMARK.md](TRADEMARK.md)、[RULES.md](RULES.md)：代码自由，名称、Logo 与官方发布渠道保留给项目。
