# Codex 主题换行符 CI 回归修复

## 背景

GitHub Actions 的 Windows Rust 测试在
`repository_theme_directories_and_archives_compile_to_the_same_payload` 失败。Windows
检出的主题目录中 `theme.css` 使用 CRLF，而仓库内 ZIP 归档保留 LF；主题运行时载荷直接保留源文件换行符，导致内容相同的目录包与归档包编译出不同的
`CodexThemePayload.css`。macOS 检出内容与归档换行符一致，因此没有暴露该差异。

## 目标

本次包含：

- 主题 CSS 在校验和生成运行时载荷时统一规范为 LF。
- 将 CRLF 规范为 LF，并将剩余的独立 CR 规范为 LF。
- 保证同一主题从目录或 ZIP 导入时生成确定一致的运行时载荷。
- 增加不依赖 Git 检出配置的换行符回归测试。

本次不包含：

- 不修改 GitHub Actions、安装器、发布版本或产物命名。
- 不改写仓库主题目录或 ZIP 内的主题资源。
- 不改变图片、字体等二进制资源的读取、Data URI 编译或完整性哈希。
- 不调整主题 UI、manifest 契约、用户数据或非主题功能。

## 用户视角描述

用户在 Windows 或 macOS 上从内容相同的主题目录或 ZIP 导入并应用主题时，Codex
收到的 CSS 运行时载荷应一致，不因操作系统或 Git 换行符策略产生行为差异。

## 功能要求

- CSS 文件仍必须是合法 UTF-8，并继续通过现有 `validate_css` 校验。
- CSS 进入主题校验返回值和 `CodexThemePayload` 前必须采用 LF 规范形式。
- 规范化必须覆盖 CRLF 和独立 CR，且不得改变其他字符。
- 规范化仅作用于已解码的 CSS 文本，不得作用于二进制资源或 Data URI 内容。
- 现有仓库主题目录/归档一致性测试必须在 Windows 通过。
- 新增的定向测试必须在任意平台显式构造不同换行符，证明最终 payload 相等并采用 LF。

## 数据与接口要求

- 不新增或修改 Tauri 命令、前端类型、状态文件字段及 manifest 字段。
- `CodexThemePayload.css` 的规范形式固定为 LF。
- 主题目录的完整性 SHA-256 继续基于磁盘原始字节计算，本任务不改变其语义。

## 技术约束

- 修改范围限定在 `crates/claude-codex-pro-core/src/codex_theme.rs` 及对应规格、验收文档。
- 复用现有主题导入、应用和 payload 生成路径，不增加依赖。
- 保持现有安全校验、路径校验、资源类型校验和回滚逻辑不变。

## 交付范围

- `crates/claude-codex-pro-core/src/codex_theme.rs`
- `spec/fix-codex-theme-line-ending-ci-regression.md`
- `acceptance/fix-codex-theme-line-ending-ci-regression.md`
