# 验收标准：工作区脏数据与代码清理

验证对象：`spec/worktree-dirty-cleanup.md`

## 验收项

1. 规格文档存在
   - 通过标准：`spec/worktree-dirty-cleanup.md` 存在。
   - 证据：文件存在检查或 Git 状态。

2. 验收文档存在
   - 通过标准：`acceptance/worktree-dirty-cleanup.md` 存在。
   - 证据：文件存在检查或 Git 状态。

3. 本地运行垃圾数据已清理
   - 通过标准：`Files/WindowsApps/Claude_1.15962.0.0_x64__pzs8sxrjxfjjc` 不再出现在 `git status --short` 或 `git ls-files --others --exclude-standard` 中。
   - 证据：Git 状态输出。

4. 同类垃圾目录已被忽略
   - 通过标准：`.gitignore` 包含 `/Files/`。
   - 证据：源码检查。

5. 没有明显冲突标记或空白错误
   - 通过标准：`git diff --check` 通过。
   - 证据：命令输出。

6. Rust 格式检查通过
   - 通过标准：`cargo fmt --check` 通过。
   - 证据：命令输出。

7. 前端类型检查通过
   - 通过标准：`npm --prefix apps/claude-codex-pro-manager run check` 通过。
   - 证据：命令输出。

8. 管理工具 Rust 构建通过
   - 通过标准：`cargo build -p claude-codex-pro-manager` 通过。
   - 证据：命令输出。

9. 脏改动已分组整理
   - 通过标准：`docs/worktree-dirty-inventory.md` 存在，并按文档、代码、测试、工作区清理等维度给出提交分组建议。
   - 证据：文件存在检查和文档内容。

10. 代码文件头无异常 BOM
   - 通过标准：`apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 第一行以 `use std::collections::BTreeMap;` 开始。
   - 证据：源码检查。

## 不在范围内

- 回滚或拆分全部既有脏改动。
- 删除用户本地 Codex、Claude、供应商、盘古记忆或登录状态。
- 完整运行 `cargo test --workspace`。
