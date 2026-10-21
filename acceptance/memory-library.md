# 验收标准：记忆库页面与记忆编辑弹窗

对应规格：`spec/memory-library.md`

## 通过标准

- [ ] 一级导航出现「记忆库」，页面包含 4 张统计卡（记忆总数 / 画像 / 蒸馏沉淀 / 最近更新），
      数值与副文案符合规格 1。
- [ ] 搜索框 placeholder 为「搜索记忆标题、正文或来源」；筛选标签为「全部 N / 画像 N /
      任务记忆 N」并有「重置」；搜索与筛选即时生效，列表头显示「共 N 条记忆」。
- [ ] 记忆卡片显示类型、标题（单行省略）、正文 4 行摘要、更新时间与「导出 MD / 编辑 / 删除」。
- [ ] 空态文案为「还没有记忆条目」+「在蒸馏工作台选择「画像」或「任务记忆」，产物会自动沉淀到这里。」
- [ ] 点「编辑」打开「编辑记忆」弹窗：类型选项、一句话标题、说明、取消 / 导出 MD / 保存；
      标题或正文为空时保存禁用。
- [ ] 保存后卡片标题与正文立即更新，记忆文件同步重写；删除后卡片消失且计数同步。
- [ ] 导出 MD 先弹出文件夹选择，取消不写任何文件；选中后在所选文件夹生成带 `title/type/source/updatedAt`
      frontmatter 与完整正文的文件，命令返回非空路径；选择的文件夹不存在时命令报错而不是静默回退。
- [ ] 蒸馏工作台历史卡片「去记忆库查看」跳转到记忆库页面，不再打开资源管理器。

## 必需证据

- `cargo test -p claude-codex-pro-manager --lib` 通过，且包含：
  - `memory_library_payload_filters_and_counts`：只保留 persona/memory 且非 cancelled，计数正确。
  - `memory_markdown_has_frontmatter_and_body`：frontmatter 字段与正文完整。
  - `memory_export_stem_is_safe`：导出文件名去除非法字符，空标题回退 `memory`。
- `npm --prefix apps/claude-codex-pro-manager run check` exit 0。
- `npm --prefix apps/claude-codex-pro-manager run vite:build` 成功。
- `cargo build --release` 成功，默认 Release 可执行文件存在。
- 实机确认（需用户执行）：蒸馏一条画像/任务记忆后进入记忆库能看到条目、能编辑保存、能导出与删除。

## 非目标

- 不做新增记忆按钮、知识库检索、向量化、Jarvis 洞察卡。
- 不改变蒸馏提示词、候选自动批准和记忆文件目录结构。
