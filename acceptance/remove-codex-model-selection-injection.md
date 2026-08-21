# 移除 Codex 模型选择注入验收标准

对应规格：`spec/remove-codex-model-selection-injection.md`

## 通过标准

1. 注入脚本不再包含模型菜单候选、注入组、`CCP 模型增强` 或模型菜单观察器。
2. 注入脚本不再安装或执行模型 JSON、Statsig、React、`model/list`、`list-models-for-host` 补丁。
3. 注入脚本不再保存 CCP 模型选择，也不覆盖请求模型字段。
4. 前端设置页、客户端能力摘要和设置类型不再包含模型白名单解锁。
5. Core 设置不再序列化或合并 `codexAppModelWhitelistUnlock`。
6. 服务层级控制相关测试仍通过，且没有新增模型选择注入路径。

## 必需验证

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test bridge_routes -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
git diff --check
```

如需交付用户验证，结束仓库 `target/release` 中正在运行的 CCP 进程后执行 `cargo build --release`，重启 Codex，再检查模型菜单只显示 Codex 原生选项。

## 非目标检查

- 不要求修改 Codex 官方应用文件。
- 不要求验证供应商是否提供某个具体模型。
- 不要求删除服务层级控制的只读当前模型状态读取。
