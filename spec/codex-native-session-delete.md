# Codex 原生会话删除恢复

## 背景

CCP 的 Renderer 注入层曾在 Codex 会话行上创建删除按钮、接管点击事件并调用本地 `/delete` 端点。该实现会与 Codex 原生删除后的本地状态竞争，并显示 CCP 的右下角提示。

## 目标

- 取消 CCP 注入层的会话删除控制和事件接管。
- 删除操作完全由 Codex 原生界面和原生存储流程处理。
- 新注入运行时移除旧注入版本遗留的 CCP 删除 UI。

## 非目标

- 不修改 Codex 原生会话数据、删除逻辑或 `instruction-inject.sh`。
- 不改变导出、项目移动等独立会话增强。

## 验收范围

- Renderer 的有效设置始终关闭 `sessionDelete`。
- Renderer 不安装会话删除事件委托，且不创建 CCP 删除按钮。
- 定向注入契约测试通过。
