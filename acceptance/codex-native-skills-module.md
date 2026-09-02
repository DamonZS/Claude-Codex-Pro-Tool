# Skills 模块原生清单验收

对应规格：`spec/codex-native-skills-module.md`

## 通过标准

1. 当 `skills` 本地集合为空而 bootstrap 含 `codex_native_skills` 时，页面显示原生 Skill 条目及只读来源标识。
2. 原生条目不显示可执行、绑定或信任成功状态；Host 不支持执行时相关按钮保持禁用。
3. 本地 Skills 集合非空时不重复混入原生快照。

## 验证方式

- `node --check assets/inject/renderer-inject.js`
- `npm --prefix apps/claude-codex-pro-manager run check`
- `cargo test -p claude-codex-pro-core multica_workspace --lib -- --nocapture`
- 静态检查确认 fallback 仅引用 `codex_native_skills`。

## 非目标

- 不实现原生 Skill 的编辑、安装、执行或信任写入。
