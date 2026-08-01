# 验收标准：Codex 供应商路由关闭与新建配置持久化

验证对象：`spec/codex-supplier-route-and-new-profile-persistence.md`

## 验收项

1. 关闭当前 Codex 路由时，源码和回归测试证明调用顺序为：撤销运行配置、保存 `nextProfiles`、显示成功提示。
2. Codex 路由关闭成功后，目标 Profile 的 `routeEnabled` 保持为 `false`；刷新设置后总开关不得回弹。
3. 撤销运行配置或保存设置失败时，不显示“已关闭 Codex 供应商路由”的完成提示。
4. 新建普通供应商时，“保存并使用”按钮不再依赖已有 `editingExisting`，能够把新 Profile 保存并设为对应目标的当前供应商。
5. 保存 Profile 后，列表目标标签切换为 `savedProfile` 的目标应用，再关闭编辑器；跨目标预设保存后卡片立即可见。
6. Codex Profile 使用非空 API Key 与 Base URL 保存到临时 `SettingsStore` 后重新加载，`api_key`、`base_url`、`upstream_base_url` 与原值一致，持久化 JSON 不直接序列化 `apiKey` 或 `baseUrl` 明文字段。
7. 不新增编辑页路由开关，不改变 Claude / Claude Desktop 既有路由与模型映射行为。
8. Codex Profile 开启路由后，Responses 与 Chat Completions 两种协议写入的活动
   provider `base_url` 都是 CCP 本地代理地址；Responses 请求由代理透明转发到 Profile
   保存的上游地址，Chat Completions 请求继续转换后转发。关闭路由后不强制劫持。
9. 旧数据中 `relayMode=official`、`importSource=cc-switch` 的 Codex Profile 填写
   Base URL 后保存，必须转为 `pureApi`；重新加载后卡片可读到相同地址，且
   `configContents` 包含活动 `model_provider` 与 provider `base_url`。
10. 打开供应商编辑器后，窗口最右侧没有 `ops-screen` 页面级纵向滚动条；编辑器标题栏和底部操作栏保持可见，只有 `.supplier-ccswitch-editor-body` 随长表单滚动。
11. 在约 `1182x852` 的桌面窗口及项目支持的最小窗口宽度下，“取消”“保存”“保存并使用”三个按钮均完整可见；滚动到表单顶部和底部时，操作栏位置不变，最后一项表单内容不被遮挡。
12. 以下验证通过：
   - `cargo test -p claude-codex-pro-manager --test windows_subsystem supplier_route_shutdown_and_feedback_use_real_operation_results -- --nocapture`
   - `cargo test -p claude-codex-pro-manager --test windows_subsystem supplier_screen_exposes_real_provider_crud_and_switching -- --nocapture`
   - `cargo test -p claude-codex-pro-core settings_store_save_and_load_preserves_codex_supplier_credentials_and_url -- --nocapture`
   - `npm --prefix apps/claude-codex-pro-manager run check`
   - `npm --prefix apps/claude-codex-pro-manager run vite:build`
   - `cargo fmt --check`
   - `cargo build --release`

## 完成证据

- 测试由失败转为通过的命令输出。
- TypeScript 检查、Vite 构建和 Rust 格式检查结果。
- 供应商编辑器在桌面窗口的截图，以及滚动前后标题栏和操作栏位置不变的浏览器测量结果。
- `target/release/claude-codex-pro.exe` 存在且更新时间晚于本次源码修改。

## 非范围

- 不要求生成 Windows 安装包或 macOS DMG。
- 不修改或清理用户现有供应商数据。
- 不自动重启正在运行的管理工具、Codex 或 Claude。
