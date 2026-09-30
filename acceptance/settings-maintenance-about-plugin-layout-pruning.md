# 验收：设置、维护与插件页精简

对应 `spec/settings-maintenance-about-plugin-layout-pruning.md`。

1. 设置源码不含设置文件位置、Codex 增强矩阵、Claude 一键汉化、CLI 命令包装器及偏好设置摘要面板；设置页保留关于/更新内容和运行日志。
2. 路由目录不再提供独立关于入口；旧 `about` 路由归一到 `settings`。
3. 维护源码不含检查修复、入口管理和自动接管三个面板及其前端动作调用。
4. 插件页不含顶部统一管理说明副标题，Codex/Claude 插件仓库状态卡位于插件内容之后。
5. 工具/插件、维护和设置页面均不渲染全局页面标题带；概览、供应商、会话等页面维持既有标题行为。
6. CLI 包装器和 Claude 汉化入口不再由前端刷新或渲染；旧配置字段仍可通过 BackendSettings 兼容读取。
7. 运行前端类型检查、Vite 构建、Rust 格式检查和 Manager 源码契约测试；记录 Release 构建产物。
