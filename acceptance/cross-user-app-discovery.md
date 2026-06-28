# 验收标准：跨用户 Codex 与 Claude 安装发现

验证对象：`spec/cross-user-app-discovery.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/cross-user-app-discovery.md` 存在。
   - 验证证据：文件存在检查。

2. 验收文档存在
   - 通过标准：`acceptance/cross-user-app-discovery.md` 存在。
   - 验证证据：文件存在检查。

3. Codex 候选路径覆盖多用户环境
   - 通过标准：源码中存在可测试的 Codex 候选路径构造逻辑，覆盖 LOCALAPPDATA、APPDATA、Program Files、Program Files (x86)、ProgramW6432 和 macOS 应用目录。
   - 验证证据：`cargo test -p claude-codex-pro-core --test launcher app_paths -- --nocapture` 通过。

4. Codex 解析优先级保留
   - 通过标准：显式传入路径和用户保存路径仍优先于自动发现。
   - 验证证据：现有 `app_paths_saved_path_is_used_when_no_explicit_path_is_provided` 测试通过。

5. Claude Desktop 候选路径覆盖多用户环境
   - 通过标准：Claude Desktop 候选路径构造覆盖运行中路径、LOCALAPPDATA、APPDATA、Program Files、Program Files (x86)、ProgramW6432、MSIX/AppX 和 macOS 应用目录。
   - 验证证据：`cargo test -p claude-codex-pro-core claude_desktop_candidate -- --nocapture` 通过。

6. Claude 启动使用增强发现
   - 通过标准：`open_claude_desktop` 在无运行中路径时仍会调用候选路径发现，再回退系统启动入口。
   - 验证证据：源码检查和 `cargo test -p claude-codex-pro-core claude_desktop_candidate -- --nocapture` 通过。

7. 构建与类型检查通过
   - 通过标准：
     - `cargo check -p claude-codex-pro-core --manifest-path Cargo.toml` 成功。
     - `cargo check -p claude-codex-pro-manager --manifest-path Cargo.toml` 成功。
   - 验证证据：命令输出。

## 不在范围内

- 真实扫描所有用户磁盘。
- 真实验证每一种第三方重打包安装器。
- 修改用户本地 Codex 或 Claude 配置。
- 发布正式安装包。
