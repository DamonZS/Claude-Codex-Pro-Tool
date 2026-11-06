# Skills 单一真源（SSOT）存储

## 背景

cc-switch 的 Skills 管理把一份真源目录当成唯一权威副本，再按需分发到各应用。CCP 当前只能把 Skill 从一个应用目录一次性复制到另一个应用目录（见 `spec/cross-app-skill-copy.md`），没有"先入库再分发"的能力：同一个 Skill 在 Codex 与 Claude 各存一份，用户无法知道哪份是最新的，也无法一次导入、多处同步。

本规格实现 cc-switch `src-tauri/src/services/skill.rs` 的最小切片：一个真源目录、一个索引文件、导入与同步两个动作。只做存储与复制，不做 UI、不做应用侧启用状态管理、不做删除与去重。

## 目标

- 在 `<CCP home>/skills/` 下维护 Skill 的唯一权威副本，每个 Skill 一个目录 `<CCP home>/skills/<id>/`，其中必须含 `SKILL.md`。
- 在 `<CCP home>/skills/index.json` 维护轻量索引，记录 `id`、显示名、来源应用与导入时间。
- 提供四个公开函数（签名固定，供 Lead 接线）：真源根目录解析、列出、导入、同步到应用。
- 索引每次以磁盘目录为准校正；索引损坏时不阻塞使用，按目录重建并写回。

## 非目标

- 不做 UI、不新增 Tauri 命令、不修改前端；本规格交付的是 core 模块与集成测试。
- 不做删除、重命名、去重、双向同步、版本比较或依赖安装。
- 不接管 `~/.codex/skills` 与 `~/.claude/skills` 的既有内容，不修改目标应用已有 Skill。
- 不写 `.ccp-disabled` 快照，不改 `unified_tool_inventory.rs` 的扫描与开关行为。
- 不新增 Cargo 依赖（复用 chrono / serde_json / tempfile）。

## 用户视角

1. 用户有一个本地 Skill 目录（含 `SKILL.md`），调用导入后它进入 CCP 真源库，列表可见其名称、来源与导入时间。
2. 用户重复导入同名 Skill 时被明确拒绝，而不是静默覆盖已入库的副本。
3. 用户把真源库里的某个 Skill 同步到 Codex 或 Claude 的 `skills` 目录后，对应应用即可发现该 Skill。
4. 用户手工删掉某个真源目录后再次列出，该 Skill 从索引中消失；索引文件被写坏时，列表仍按磁盘目录恢复。

## 功能要求

- F1 真源根目录：`skill_store_root(home) = home.join("skills")`，其中 `home` 是调用方用 `crate::paths::default_app_state_dir()` 解析出的 CCP home（默认 `~/.claude-codex-pro`），不得在模块内硬编码该路径。
- F2 索引结构：`{ "version": 1, "entries": [ { "id", "name", "sourceApp", "importedAt" } ] }`；`version` 取 `STORE_VERSION = 1`。
- F3 导入前置校验：`source_dir` 必须是已存在目录且含普通文件 `SKILL.md`；`id` 取自源目录最后一段目录名。
- F4 ID 校验：`id` 必须是非空单段目录名。空串、`.`、`..`、含路径分隔符（`/`、`\`）、含 `..`、带盘符前缀等一律拒绝，并返回包含该 id 的明确中文错误，防止路径逃逸。
- F5 重复导入：`<root>/<id>` 已存在时拒绝，错误信息包含目标路径，不覆盖、不合并。
- F6 复制语义：递归复制普通文件与目录，跳过符号链接（不跟随、不复制链接本身）；不修改源目录。
- F7 导入时间：`chrono::Utc::now().to_rfc3339()`。
- F8 列表：`list_store_skills` 以磁盘目录为准重建条目；只有"目录 + 含 `SKILL.md`"的条目算 Skill，符号链接与缺 `SKILL.md` 的目录被忽略；条目按 `id` 升序。
- F9 索引校正：列表后若计算结果与现有 `index.json` 字节不一致则用 `crate::settings::atomic_write` 写回；`index.json` 缺失、无法读取或 JSON 损坏时视为空索引并按目录重建。
- F10 索引元数据：`name` 始终来自磁盘 `SKILL.md` 的 frontmatter `name:`（缺失时回退为 `id`）；`source_app`、`imported_at` 优先沿用索引中已有记录，磁盘上多出来的目录以 `source_app = "unknown"` 与目录修改时间（RFC3339）补齐。
- F11 同步：`app` 仅接受 `"codex"` 与 `"claude"`，其它值返回"不支持的 Skill 目标应用：<app>"；目标目录分别为 `~/.codex/skills/<id>` 与 `~/.claude/skills/<id>`。
- F12 同步前置：真源库中必须存在 `<root>/<id>/SKILL.md`，否则返回"真源库中未找到 Skill：<id>"；目标目录已存在时拒绝并给出完整路径。
- F13 同步返回目标 Skill 目录（`.../skills/<id>`），同步完成后不做应用侧清单扫描。

## 数据与接口要求

```rust
pub struct SkillStoreEntry {
    pub id: String,
    pub name: String,
    pub source_app: String,
    pub imported_at: String,
}

pub fn skill_store_root(home: &Path) -> PathBuf;
pub fn list_store_skills(home: &Path) -> anyhow::Result<Vec<SkillStoreEntry>>;
pub fn import_skill_to_store(home: &Path, source_dir: &Path, source_app: &str) -> anyhow::Result<SkillStoreEntry>;
pub fn sync_skill_to_app(home: &Path, id: &str, app: &str) -> anyhow::Result<PathBuf>;
```

- `home` 为用户主目录下的 CCP home（`~/.claude-codex-pro`）。同步目标中的 `~` 由其父目录派生，因此默认情况下即真实用户主目录；测试可把 CCP home 放在临时"用户主目录"下实现完全隔离。
- `SkillStoreEntry` 通过 serde 以 camelCase 字段序列化，供后续命令层直接回传前端；索引文件字段同样为 camelCase，但 `version` / `entries` 结构固定。
- 所有失败以 `anyhow::Error` 返回，文案沿用仓库既有中文风格，并包含涉及的具体路径或 id。

## 技术约束

- 仅改动：`crates/claude-codex-pro-core/src/skill_store.rs`、新增 `crates/claude-codex-pro-core/tests/skill_store.rs`、本规格与对应验收文档。
- 不得修改 `unified_tool_inventory.rs`、`commands.rs`、`lib.rs`、前端、`Cargo.toml`、`docs/`、`build-lead/`、`.dsh-ops/`；不执行 git 操作，不构建 release，不安装或重启应用。
- 索引落盘必须使用 `crate::settings::atomic_write`（`pub(crate)`），不得直接 `fs::write`。
- 复制实现放在本模块内（可参考但不得修改 `unified_tool_inventory.rs::copy_skill_dir`），跳过符号链接。
- 时间必须用 chrono（core 已依赖 `chrono = 0.4.45`），不得改为秒级时间戳，也不得新增依赖。
- 派生用户主目录只做一次 `home.parent()`，不读取 `CODEX_HOME`，避免同步写入真实环境之外的意外位置；该取舍在验收与风险中记录。

## 交付范围

- `crates/claude-codex-pro-core/src/skill_store.rs` 完整实现（保留 `STORE_VERSION`）。
- `crates/claude-codex-pro-core/tests/skill_store.rs` 覆盖导入、拒绝、同步、ID 校验与索引损坏恢复。
- `spec/skill-ssot-store.md` 与 `acceptance/skill-ssot-store.md`。

## 变更记录

- 2026-11-06：新建。来源 cc-switch `src-tauri/src/services/skill.rs`（SSOT + 按需同步）的最小切片；与 `spec/cross-app-skill-copy.md` 的区别是引入真源目录与索引。