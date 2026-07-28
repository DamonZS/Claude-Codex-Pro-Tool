# 验收标准：Codex 在线主题库与删除

对应规格：`spec/online-codex-theme-library.md`

## 通过标准

### 在线下载

- 主题中心存在“下载主题”按钮，菜单包含仓库当前 8 个官方主题。
- 未安装主题可下载；成功后出现在三列主题列表中且不会自动启用。
- 已安装主题显示“已安装”并禁用；直接调用后端同 ID 下载命令也在网络请求前失败。
- 后端不接受任意 URL，只接受允许列表中的主题 ID。
- 下载固定到经过审计的 GitHub 提交并匹配内置 SHA-256；本地 8 个 ZIP 的哈希与目录定义一致。
- 下载使用连接/总超时和流式 32 MiB 上限；HTTP 错误、超限和中断不会写入主题状态。
- ZIP manifest ID 与请求 ID 不一致时安装失败，主题库不新增错误 ID。
- 下载内容通过现有路径越界、符号链接、文件数量、文件大小、CSS、图片和 manifest 校验。

### 删除

- 每个非默认主题卡片有可识别的删除按钮。
- 默认主题没有删除操作；当前主题删除按钮禁用，后端直接删除也失败。
- 取消确认不会调用删除命令。
- 删除未启用主题后，其状态记录、正式主题目录和版本备份被移除，其他主题和当前主题不变。
- 模拟删除在文件移动后、状态提交前中断，重新打开 Store 能恢复原主题。
- 模拟状态已提交但清理未完成，重新打开 Store 能完成清理且不恢复已删除主题。
- 删除成功返回 `restart_required = false`。

### 主题制作

- 工具栏存在“制作指南”入口并使用系统默认浏览器打开官方 `Theme/README.md`。
- 文档包含最小目录结构、字段说明、CSS 隔离规则、图片要求、打包命令和导入验证步骤。
- 文档不建议直接编辑已安装主题库，也不允许远程脚本或远程 CSS。

### 回归

- 本地 ZIP/目录导入、主题应用、恢复默认、背景设置与上一版本回滚保持可用。
- 默认主题仍固定第一项，无法覆盖或删除。
- 下载或删除失败不改变当前主题 ID、generation 或 Manager 背景。
- 供应商、模型、汉化、会话和其他注入逻辑没有改动。

## 验证方式

```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml codex_theme -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo build --release
```

手动验证：

- 在主题中心下载一个未安装主题，确认通知、卡片和未启用状态。
- 再次打开下载菜单，确认该主题显示“已安装”且不可点击。
- 删除一个未启用主题，确认卡片消失；尝试删除当前主题，确认被阻止。
- 点击“制作指南”，确认由系统默认浏览器打开正确仓库文档。

## 必需证据

- 上述命令的真实结果。
- 默认 `target/release/claude-codex-pro-manager.exe` 的更新时间和路径。
- 主题中心下载菜单、已安装状态和删除按钮的本地应用截图；截图不得包含本地隐私路径或凭据。

## 非目标

- 不验证第三方仓库主题市场。
- 不要求在线编辑器或实时 CSS 设计器。
- 不要求删除当前生效主题时自动切换主题。
