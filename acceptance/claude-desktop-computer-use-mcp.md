# 验收：Claude Desktop Computer Use MCP

## 对应规格

`spec/claude-desktop-computer-use-mcp.md`

## 通过标准

1. `cargo test -p claude-codex-pro-core claude_desktop_computer_use` 全部通过，覆盖：
   - `initialize` 返回 `protocolVersion`、`serverInfo`、`capabilities.tools`。
   - `tools/list` 包含规格列出的 10 个工具（含 `drag_path`），且每个都有 `inputSchema`。
   - `drag_path`：点数少于 2、超过 200、含越界点、含急停保留区点时返回工具错误且假后端未收到任何动作；合法输入时假后端按顺序收到全部点；后端中途失败时错误被原样返回。
   - 动作类工具在急停区命中时同样拒绝 `drag_path`。
   - 未知方法返回 `-32601`；非法 JSON 返回 `-32700`；通知不产生回复。
   - 总开关关闭时 `tools/call` 返回 `isError: true` 且假后端未收到任何动作。
   - 急停区域命中时返回错误并调用关闭开关。
   - 截图坐标 → 屏幕坐标换算正确；越界坐标被拒绝。
   - `screenshot` 返回 `image` 内容，`mimeType` 为 `image/jpeg`，尺寸不超过 1280×800。
   - `type_text` 的诊断日志不含文本内容。
2. 注册测试：在临时配置文件上写入后 `mcpServers.claude-codex-pro-computer-use.args == ["--mcp-computer-use"]`，其他已有字段与其他 MCP 条目保留；移除后只删除该条目；无法解析的配置拒绝覆盖。
3. `cargo build --release` 成功；`npm --prefix apps/claude-codex-pro-manager run check` 与 `vite:build` 通过。
4. Windows 实机冒烟：用管道启动 `claude-codex-pro.exe --mcp-computer-use`，发送 `initialize`、`tools/list`、`tools/call screenshot`，能收到合法 JSON 回复与 base64 JPEG；设置关闭时 `screenshot` 返回错误。
5. `cargo test -p claude-codex-pro-manager --test windows_subsystem` 通过（单 exe 与子系统契约不被破坏）。
6. 3p 配置注册：`cargo test -p claude-codex-pro-core computer_use_config_paths` 通过，覆盖：
   - 3p 配置文件存在时，其路径被追加到 Computer Use 的配置路径列表。
   - 3p 配置文件不存在时，不追加（不会为其创建目录或文件）。
   - 与常规路径重复时不重复追加。
   - 对含 `deploymentMode`、`extraKnownMarketplaces` 等字段的 3p 配置注册后，这些字段原样保留，仅新增 `mcpServers.claude-codex-pro-computer-use`；移除后该条目消失、其余字段仍在。
7. 实机确认（需用户执行）：开启开关后 `Roaming\Claude-3p\claude_desktop_config.json` 出现 `claude-codex-pro-computer-use`，完全退出并重启 Claude Desktop 后，Code 会话能列出并调用 `screenshot` 工具。若配置已写入仍看不到工具，则根因不在注册路径，需另查。

## 验证方式与证据

- 命令输出摘要记录到最终汇报。
- 冒烟脚本输出保存到 `acceptance/evidence/claude-desktop-computer-use-mcp/`（不含截图原图）。

## 不在范围内

- macOS 实机验证（本机为 Windows，只验证 Windows 编译与逻辑；macOS 代码路径以 `cfg` 隔离，标注未实机验证）。
- Claude Desktop 内端到端调用（需要用户重启 Claude Desktop 后手动确认）。
- 多显示器、Linux。
