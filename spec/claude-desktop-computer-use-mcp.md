# Claude Desktop Computer Use MCP

## 背景

Claude Desktop 没有内置 Computer Use（截图 + 键鼠操作）能力，Codex 有。用户希望在 CCP 中一键为 Claude Desktop 接入同等能力，用于操作本机网页、桌面应用（例如在画布网页上生成图片）。Claude Desktop 支持本地 stdio MCP 服务并能展示工具返回的图片，因此由 CCP 自带一个 Computer Use MCP 服务并注册到 Claude Desktop 配置即可实现。

## 目标

- CCP 主程序新增 `--mcp-computer-use` 运行模式，作为 stdio MCP 服务（JSON-RPC 2.0，按行分隔）。
- 提供工具：`screenshot`、`click`、`move_mouse`、`drag`、`drag_path`、`scroll`、`type_text`、`press_keys`、`cursor_position`、`wait`。
- 支持 Windows（SendInput + GDI 截图）与 macOS（CoreGraphics CGEvent + CGDisplay 截图）。
- 管理器「工具与插件」页提供开关：开启时写入 Claude Desktop `mcpServers.claude-codex-pro-computer-use`，关闭时移除。
- 安全：总开关 + 急停 + 诊断日志。

## 非目标

- 不支持 Linux（工具调用返回明确的“不支持”错误）。
- 不实现 macOS 虚拟光标/不抢鼠标；会移动真实鼠标。
- 不做多显示器选择，仅主显示器。
- 不修改 Codex 自带 `computer-use@openai-bundled` 守护逻辑（`computer_use_guard`）。
- 不自动重启 Claude Desktop。

## 功能要求

- 设置项 `claudeDesktopComputerUseEnabled`，默认 `false`。
- 每次工具调用前重新读取设置；为 `false` 时拒绝执行并提示在 CCP 中开启。
- 急停：任何动作类工具执行前，若真实鼠标位于主屏左上角 4×4 像素区域内，立即把设置写为 `false`、拒绝本次与后续调用，需用户回 CCP 重新开启。
- 截图：主显示器，缩放到不超过 1280×800（保持比例，不放大），JPEG 质量 80，以 MCP `image` 内容返回，并附带文本说明截图尺寸。所有坐标参数均为截图坐标系，服务内部按比例换算为真实屏幕坐标。
- 坐标越界（超出截图尺寸）返回工具错误，不执行动作。
- `drag_path`：按住左键依次经过 2~200 个路径点（截图坐标）后松开，用于一笔画曲线、圆、签名等；每个点都按 `drag` 同样规则换算并拒绝越界与急停保留区坐标。任何一步失败都必须先松开左键再返回错误，不得留下按下状态。
- `drag` 同样保证：移动过程中出错时先发出左键松开再返回错误。
- 动作类工具列表追加 `drag_path`（执行前做急停检查）。
- `type_text` 支持任意 Unicode 文本；`press_keys` 接受组合键数组，如 `["ctrl","c"]`、`["enter"]`。
- `wait` 秒数限制在 0~10。
- 诊断日志记录工具名、坐标、按键名、结果；不记录 `type_text` 的文本内容，只记录长度。
- 注册：写入 Claude Desktop 所有常规配置路径（复用 `claude_desktop_normal_config_paths`），以及 3p（开发模式）配置 `Claude-3p/claude_desktop_config.json`，写前备份、写后校验；`command` 为当前 CCP 可执行文件绝对路径，`args` 为 `["--mcp-computer-use"]`。
- 3p 配置仅在该文件已存在时才纳入注册与移除，避免给未使用 3p 模式的用户凭空创建 `Claude-3p` 目录；路径来源复用 `claude_desktop_threep_paths`（含 MSIX `Packages\Claude_*` 下的副本）。
- 其他使用 `claude_desktop_normal_config_paths` 的功能（插件、MCP 条目等）行为不变，本条只影响 Computer Use 的注册、移除与状态查询。
- 状态查询：返回是否开启、各配置路径是否已注册、当前平台是否支持。
- 未知方法返回 JSON-RPC `-32601`；解析失败返回 `-32700`；通知（无 id）不回复。

## UI / 交互要求

- 「工具与插件」页新增卡片「Claude Desktop Computer Use」：开关、注册状态、平台支持状态。
- 说明文案：开启后需重启 Claude Desktop；鼠标甩到屏幕左上角可急停；会移动真实鼠标；截图可能包含屏幕上的敏感信息；macOS 需在系统设置中为 Claude 授予「辅助功能」与「屏幕录制」。
- 开关操作成功/失败沿用 `notifyResult` 提示。

## 数据与接口要求

- Tauri 命令：`get_claude_desktop_computer_use_status`、`set_claude_desktop_computer_use_enabled({ enabled })`，返回 `ComputerUseStatus { enabled, supported, platform, registeredPaths, configPaths, executablePath }`。
- MCP `initialize` 回显客户端 `protocolVersion`，缺省 `2025-06-18`；`capabilities.tools` 为 `{}`。

## 技术约束

- 不新增独立可执行文件（遵循单 exe 规则），以参数模式运行。
- 不引入 `rmcp` 或截图/输入第三方 crate；Windows 复用 `windows` crate（追加 `Win32_Graphics_Gdi`、`Win32_UI_HiDpi` 特性），macOS 直接 FFI 链接 CoreGraphics/CoreFoundation/ApplicationServices。
- 写 Claude Desktop 配置必须走现有备份/校验 helper，遇到无法解析的配置拒绝覆盖。
- 动作逻辑通过 trait 抽象，协议层可用假实现做单元测试，不在测试中移动真实鼠标。

## 交付范围

- core：`claude_desktop_computer_use` 模块（协议、工具、换算、急停、平台实现、注册）。
- core：设置项；`plugin_hub` 暴露注册所需 helper。
- manager：`main.rs` 参数分支、两个 Tauri 命令、前端卡片、类型、bridge mock。
- 测试：协议与工具单元测试、注册写入/移除测试。
- 文档：本 spec 与对应 acceptance。
