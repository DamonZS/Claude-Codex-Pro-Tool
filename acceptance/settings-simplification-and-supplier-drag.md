# 验收标准：设置页精简与供应商 cc-switch 风格改造

验证对象：`spec/settings-simplification-and-supplier-drag.md`

## 验收项

1. 文档存在且可读
   - 通过标准：`spec/settings-simplification-and-supplier-drag.md` 与本文件存在，且为可读 UTF-8 中文。
   - 证据：文件读取或源代码检查。

2. 设置页指定卡片已删除
   - 通过标准：`SettingsScreen` 不再渲染 `Codex 启动参数`、`图片覆盖`、`盘古记忆`、`安全边界` 四个独立卡片。
   - 证据：`windows_subsystem` 回归测试或源码检查。

3. 设置页保留核心卡片
   - 通过标准：设置文件位置、Codex 增强矩阵、Claude 一键汉化、CLI Wrapper 和运行日志仍存在。
   - 证据：`windows_subsystem` 回归测试或源码检查。

4. 供应商拖拽源可稳定识别
   - 通过标准：供应商拖拽使用 pointer 事件、pointer capture 与窗口级 pointermove/pointerup 监听，不依赖 WebView 原生 HTML drag/drop。
   - 证据：`windows_subsystem` 回归测试或源码检查。

5. 供应商排序保存逻辑不变
   - 通过标准：释放拖拽后仍调用 `saveSupplierOrder`，并通过 `actions.saveSettings` 保存 `relayProfiles` 顺序；保存失败时回滚。
   - 证据：`windows_subsystem` 回归测试或源码检查。

6. 供应商卡片视觉对齐 cc-switch
   - 通过标准：供应商列表使用 cc-switch 式轻量卡片、左侧拖拽柄、供应商头像、名称与 URL 分层、轻量图标按钮、高亮“使用”按钮；右侧图标顺序包含使用、编辑、复制、连通检测、用量、删除；不得出现“不写 API 文件”误导文案。
   - 证据：样式类检查、前端 check/build，必要时截图人工确认。

7. 拖拽过程有卡片跟随鼠标
   - 通过标准：拖拽期间存在 `supplierDragOverlay`，镜像卡片使用 `position: fixed`，并随 `pointermove` 更新 top；原卡片以 `drag-source` 占位。
   - 证据：`windows_subsystem` 回归测试或源码检查。

8. Claude 供应商编辑页结构对齐 cc-switch
   - 通过标准：Claude / Claude Desktop 供应商进入专用编辑页，包含返回标题、供应商名称、备注、官网链接、API Key 眼睛按钮、请求地址、完整 URL 标识、高级选项、API 格式、认证字段、模型映射、默认兜底模型、自定义 User-Agent、Header 覆盖、Body 覆盖、配置 JSON、连通检测配置、计费配置、底部保存条。
   - 证据：`windows_subsystem` 回归测试、前端 check/build。

9. Claude 编辑页保持管理工具色调
   - 通过标准：Claude 编辑页使用 `supplier-ccswitch-editor` 相关深色样式，不引入 cc-switch 浅色背景，不改变全局主题。
   - 证据：样式检查或截图人工确认。

10. 第三方导入字段不丢失
    - 通过标准：从 cc-switch 导入或编辑保存后，API Key、目标应用、API 格式、路由、模型映射和原始配置内容可保留。
    - 证据：类型/设置字段检查、相关回归测试或手动导入验证。

11. 前端类型检查通过
    - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 成功。
    - 证据：命令输出。

12. 前端生产构建通过
    - 通过标准：`npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
    - 证据：命令输出。

13. Manager UI 回归测试通过
    - 通过标准：定向 `windows_subsystem` 供应商测试成功。
    - 证据：命令输出。

14. Debug 管理工具重新构建
    - 通过标准：`cargo build -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功，`target/debug/claude-codex-pro-manager.exe` 更新时间变化。
    - 证据：命令输出与文件时间。

## 不在范围内

- 删除盘古记忆其它页面能力。
- 删除后端设置字段。
- 修改 Claude 中文注入脚本。
- 新增 UI 依赖。
