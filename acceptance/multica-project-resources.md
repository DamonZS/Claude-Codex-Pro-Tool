# Project Resource 独立资源验收

验证对象：`spec/multica-project-resources.md`

## 通过标准

1. Rust workspace store 能持久化并分页查询 `project_resources`，且不会把它并入 `projects` 集合。
2. `github_repo`、`local_directory` 资源按上游字段验证；未知类型、空 URL、非法执行模式失败。
3. Project Resource 的 `project_id` 必须指向同一 workspace 的已存在 Project。
4. renderer 模块列表包含“项目资源”，编辑器能读写资源类型、引用 JSON、标签和排序。

## 必需验证

```powershell
cargo test -p claude-codex-pro-core --lib multica_workspace -- --nocapture
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
git diff --check
```

## 非范围

上游 daemon 的真实目录锁、worktree 创建和远程 Git checkout 仍属于 daemon 执行链，不由此处伪造。
