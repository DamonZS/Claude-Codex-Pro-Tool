# Codex 更新与注入回归修复

## 背景

最近一轮管理工具与 Codex 注入改动后，出现了 3 个直接影响发布与使用体验的回归：

1. 关于页“下载并运行安装包更新”在 Windows 上无法可靠拉起安装包。
2. Codex 注入弹窗会被模型名称或模型下拉相关浮层污染，导致底部内容被覆盖或误插入模型项。
3. 开启中文覆盖层后，Codex 品牌词会瞬时被误翻成“代码”。

这些问题都属于已有能力的稳定性修复，不应借机大改 UI、改架构或引入新依赖。

## 目标

- 修复 Windows 下更新安装包的拉起方式，使需要提权的安装器也能正常触发系统打开流程。
- 修复 Codex 模型下拉注入逻辑对 CCP 自身弹窗的误识别，避免模型菜单逻辑污染控制弹窗。
- 修复中文覆盖层对 Codex 品牌词的误翻译，不再出现 Codex -> 代码 的瞬时错误。

## 非目标

- 不重做管理工具关于页。
- 不重做 Codex 注入弹窗整体视觉方案，只做最小必要修复。
- 不新增第三方 UI 库、翻译服务或远程依赖。
- 不修改 Claude 一键汉化、盘古记忆、供应商切换等无关逻辑。

## 用户视角

- 用户在关于页点击“下载并运行安装包更新”后，Windows 能正常拉起下载好的安装器，而不是静默失败。
- 用户打开 Codex 注入弹窗时，底部区域不再被模型列表覆盖，也不会在弹窗内部插入模型增强项。
- 用户开启中文覆盖层后，Codex 品牌名保持为 Codex，不再闪成“代码”或“代码x”。

## 功能要求

### 1. Windows 更新安装器拉起

- perform_update() 下载 Release 安装包后，仍调用统一的 launch_installer()。
- Windows 下 launch_installer() 不再直接 Command::spawn() 安装包。
- Windows 下改为走系统 Shell 打开本地路径，以兼容 setup.exe / .msi 的提权与关联打开行为。
- 失败时仍返回明确错误，方便前端提示。

### 2. Codex 模型菜单候选过滤

- 模型菜单候选筛选必须显式排除 CCP 自己创建的弹窗节点。
- 至少排除：
  - .claude-codex-pro-modal-overlay
  - .claude-codex-pro-modal-content
  - .claude-codex-pro-control-deck
  - [data-claude-codex-pro-dialog="true"]
- 该排除逻辑应同时用于：
  - React 模型状态 patch 的候选节点收集
  - DOM 模型菜单候选筛选
  - 模型菜单轮询探测
- 不得因为修复而破坏真实模型下拉增强。

### 3. 中文覆盖层品牌词保护

- 不得再通过宽泛映射把 Code 子串替换进 Codex。
- Codex、Claude、Claude Code 等品牌词在覆盖层中应优先保护。
- 修复后保留现有中文覆盖层能力，避免把整个覆盖层关闭。

## UI / 交互要求

- 不大改当前控制弹窗 UI。
- 不修改当前 control deck 的导航结构、标签页和主要入口。
- 修复应优先作用于行为层与候选过滤层，而不是仅靠继续抬高 z-index 掩盖问题。

## 数据与接口要求

- 不新增前端公开 API。
- 不改变 perform_update 的返回结构。
- 不改变注入脚本对现有开关、面板和按钮的数据属性约定。

## 技术约束

- 改动限定在：
  - crates/claude-codex-pro-core/src/windows_integration.rs
  - crates/claude-codex-pro-core/src/lib.rs
  - crates/claude-codex-pro-core/src/update.rs
  - assets/inject/renderer-inject.js
  - 必要的定向测试与本 spec / acceptance
- 不回滚当前工作区中与 control deck redesign 相关的未提交内容。
- 不提交 target-rebuild/。

## 交付范围

- Windows 安装包拉起修复。
- Codex 注入弹窗与模型菜单候选隔离修复。
- 中文覆盖层品牌词误翻修复。
- 对应测试与验收文档。
