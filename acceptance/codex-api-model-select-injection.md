# Codex API 模型选择注入验收标准

对应规格：`spec/codex-api-model-select-injection.md`

## 通过标准

1. `modelWhitelistUnlock` 开启时，自定义模型仍可通过状态补丁和模型菜单使用。
2. 真实模型菜单能显示 `/codex-model-catalog` 返回的模型，模型项带有 `CCP 模型增强` 标识。
3. 包含真实模型菜单的 Radix 外层包装、共享 portal 和父级 dialog 不会被识别为模型菜单。
4. 普通菜单、权限菜单、推理强度菜单、项目菜单、分支菜单、启动模式菜单和 CCP 自身弹窗不会出现 CCP 模型组，即使其项目名、分支名或选项文本包含 `Codex`。
5. 同一 DOM 重复扫描不会产生重复模型组或重复模型项。
6. 已经落在错误父容器中的旧模型组会在下一轮扫描中移除，合法模型菜单中的组保留。
7. 模型菜单关闭后，页面正文、首页卡片、对话区域和输入区不残留模型名称。
8. 关闭模型解锁时删除 DOM 兜底项，不删除或修改 Codex 原生模型项。
9. 模型选择和请求覆盖的既有行为不回归。
10. 测试和截图不包含 Token、API Key、敏感地址或私密会话全文。

## 必需验证

- DOM 契约测试必须构造“模型菜单嵌套在 Radix 外层包装中”的场景，并证明只有内层真实菜单入选。
- DOM 契约测试必须覆盖普通菜单、含 `Codex` 文本的项目/分支/启动模式菜单，以及旧游离注入组清理。
- 运行：

```powershell
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml --test cdp_bridge -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
git diff --check
```

- 结束本项目旧进程后运行 `cargo build --release`，产物必须写入默认 `target/release`。
- 启动新版 Manager，由用户点击“重启 Codex”后手动检查模型、权限、推理强度、项目和设置菜单。

## 证据

- 自动化测试输出。
- 默认 release 产物的修改时间与路径。
- 实际 Codex 截图：模型菜单正常显示增强模型，其他菜单及页面无游离模型名称。

## 非目标检查

- 不要求验证供应商是否真实提供未配置的新模型。
- 不要求重设计 Codex 原生模型菜单。
- 不要求修改 Claude Desktop 汉化或管理工具其他页面。
