# 安装设备匿名注册

## 背景

拓扑熵减官网需要展示 Claude Codex Pro Tool 的真实使用人数。正式 Windows 安装包由 NSIS 直接复制发布二进制和创建快捷方式，不会调用管理工具中的“安装入口”，因此注册必须接入正式安装包流程。

## 目标

- Windows 安装包完成文件、快捷方式和卸载信息写入后，触发一次安装设备注册。
- 使用主板序列号生成稳定的本地 SHA-256 安装摘要，同一电脑升级或重复安装保持相同摘要。
- 仅向统计接口发送摘要、应用版本和平台，不发送原始主板序列号。
- 注册超时、硬件标识不可用或服务异常时不阻断安装完成。

## 非目标

- 不采集用户名、IP、本地路径、供应商配置、Token、会话或盘古记忆数据。
- 不把原始主板序列号写入日志、配置或诊断文件。
- 不在管理工具 UI 增加统计设置或注册状态。
- 本次不为 macOS DMG 增加安装后脚本；macOS 无安装执行阶段，后续应另行设计首次启动注册。

## 安装流程

1. NSIS 将三个 Release 可执行文件复制到安装目录。
2. NSIS 完成快捷方式、卸载器和注册表信息写入。
3. NSIS 调用 `claude-codex-pro.exe --register-installation --app-version <VERSION>`。
4. launcher 识别专用参数后只执行注册，不启动 Codex、不获取单实例锁、不执行注入。
5. 注册命令读取 Windows `Win32_BaseBoard.SerialNumber`，规范化并过滤常见占位值。
6. 客户端对带 CCP 命名空间的规范化值做 SHA-256，生成 64 位小写十六进制摘要。
7. 客户端将摘要、版本和 `windows` 发送至官网 Worker 注册接口。
8. 无论命令成功或失败，NSIS 都继续完成安装。

## 接口

- 地址：`https://connect-worker.solitaryzj.workers.dev/api/tools/claude-codex-pro/register`
- 方法：`POST`
- Content-Type：`application/json`
- 请求：

```json
{
  "installationId": "<64 lowercase hex characters>",
  "appVersion": "0.12.0",
  "platform": "windows"
}
```

- 连接和总请求时间必须有明确上限，不能造成安装器长时间停留。
- 2xx 视为注册完成；非 2xx 只返回命令失败状态，不包含原始硬件标识。

## 标识规范

- 去除首尾空白，把内部连续空白折叠为单个空格，并转为大写。
- 空值以及 `UNKNOWN`、`DEFAULT STRING`、`TO BE FILLED BY O.E.M.`、`NONE`、`N/A`、全零等占位值不得注册。
- 哈希输入必须带固定命名空间，防止摘要在其他业务中被直接关联。
- 原始值只存在于当前进程内存中，生成摘要后不持久化。

## 错误处理

- PowerShell / CIM 不可用、输出非 UTF-8、返回占位值、网络超时和服务器错误均应快速返回失败。
- launcher 专用命令不得生成 `launcher.fatal` 中含硬件值的日志。
- NSIS 忽略注册命令退出码，安装结果以应用文件和快捷方式写入为准。

## 技术约束

- 复用现有 `reqwest`、`sha2`、`serde` 和 Tokio 依赖，不引入新的硬件信息或遥测 SDK。
- Windows 子进程使用 `CREATE_NO_WINDOW`，不得闪现 PowerShell 控制台。
- 注册逻辑置于 core 独立模块，launcher 只负责参数分流。
- 不修改管理工具 UI、Codex 启动参数语义、注入流程或发布资产结构。

## 交付范围

- core 安装注册模块和单元测试。
- launcher 专用 CLI 参数与参数分流测试。
- NSIS 安装触发与 Windows 结构测试。
- 默认 `target/release` 全量构建。
