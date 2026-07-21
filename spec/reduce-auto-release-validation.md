# 减少 GitHub 自动发布重复验证

## 背景

`pr-build.yml` 与 `auto-release-installers.yml` 都会在 `main` 提交上执行前端类型检查、前端生产构建和 Rust 验证。自动发布的 Windows job 因重复执行 `cargo test --workspace` 而在测试结束阶段失败，导致后续 release 编译和安装包构建未执行；同一提交的 PR Build 已承担完整质量验证。

## 目标

- 保留 `pr-build.yml` 的完整验证，包括 TypeScript 检查、前端生产构建、Rust workspace 测试和 release 构建。
- 将 `auto-release-installers.yml` 聚焦于可发布产物构建和产物验证。
- 自动发布的 Windows job 不再重复执行 TypeScript 检查和 `cargo test --workspace`。
- 自动发布的 macOS matrix 不再为两个架构分别重复执行 TypeScript 检查。
- 自动发布仍执行前端生产构建、各目标平台 release 编译、安装包构建及现有产物完整性检查。
- Release Notes 只描述自动发布工作流实际执行的验证。

## 用户视角描述

代码进入 `main` 后，PR Build 继续提供完整质量门禁；自动发布直接构建并验证 Windows x64、macOS x64 和 macOS arm64 发布产物，避免重复测试阻断安装包发布。

## 功能要求

- `auto-release-installers.yml` 不包含 `npm run check`。
- `auto-release-installers.yml` 不包含 `cargo test --workspace`。
- Windows 和 macOS 自动发布 job 都保留 `npm run vite:build`。
- Windows 保留 `cargo build --release`、NSIS、ZIP 和上传步骤。
- macOS 保留目标架构 release build、DMG、ZIP、bundle/plist/codesign 验证和上传步骤。
- `pr-build.yml` 保留 `npm run check`、`npm run vite:build`、`cargo test --workspace` 和 release build。
- 自动 Release Notes 不再声称 TypeScript 检查或 workspace 测试已在发布工作流中运行。

## 技术约束

- 不修改版本号与 tag 策略。
- 不修改签名、产物命名、产物数量、发布、`latest.json` 或失败草稿清理行为。
- 不增加跨 job 前端产物共享等额外工作流复杂度。
- 不修改应用业务代码。

## 交付范围

- `.github/workflows/auto-release-installers.yml`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- `spec/reduce-auto-release-validation.md`
- `acceptance/reduce-auto-release-validation.md`
