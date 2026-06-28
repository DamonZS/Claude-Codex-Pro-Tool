# 验收标准：Harness Engineering Skill 封装与备份

验证对象：`spec/harness-engineering-skill.md`

## 验收项

1. 本任务规格存在。
   - 通过标准：`spec/harness-engineering-skill.md` 存在。
   - 证据：文件存在检查。

2. 本任务验收标准存在。
   - 通过标准：`acceptance/harness-engineering-skill.md` 存在。
   - 证据：文件存在检查。

3. 理论文档包含 Skill 化说明。
   - 通过标准：`docs/harness-engineering-theory.md` 提到 `$harness-engineering` 或 Skill 化使用方式。
   - 证据：内容检查。

4. Skill 已安装到本机 Codex Skill 目录。
   - 通过标准：`C:/Users/Damon/.codex/skills/harness-engineering/SKILL.md` 存在。
   - 证据：文件存在检查。

5. Skill 名称合法。
   - 通过标准：`SKILL.md` frontmatter 中 `name: harness-engineering`。
   - 证据：内容检查或验证脚本输出。

6. Skill 触发描述合法。
   - 通过标准：`description` 以 `Use when` 开头，并描述适用场景。
   - 证据：内容检查或验证脚本输出。

7. Skill 包含 UI 元数据。
   - 通过标准：`agents/openai.yaml` 存在。
   - 证据：文件存在检查。

8. Skill 包含参考资料。
   - 通过标准：`references/` 下至少有一个 Markdown 文件。
   - 证据：文件列表。

9. Skill 支持项目初始化。
   - 通过标准：`SKILL.md` 或 `references/initialization.md` 说明当用户说“初始化”、initialize、bootstrap 或 set up 时，应在项目中生成 Harness Engineering 文档基线。
   - 证据：内容检查。

10. Skill 包含初始化脚本。
    - 通过标准：`scripts/init_harness.py` 存在。
    - 证据：文件存在检查。

11. 初始化脚本可执行。
    - 通过标准：对临时目录运行初始化脚本后，生成 `AGENTS.md`、`docs/harness-engineering-theory.md`、`spec/harness-engineering-baseline.md`、`acceptance/harness-engineering-baseline.md`。
    - 证据：命令输出与文件存在检查。

12. Skill 通过基础验证。
   - 通过标准：`quick_validate.py` 对 Skill 目录执行成功，或完成等价的 frontmatter、命名和文件结构检查。
   - 证据：命令输出。

13. 桌面存在备份。
    - 通过标准：桌面存在 `harness-engineering-skill-backup-*` 目录或压缩包。
    - 证据：文件存在检查。

14. 本次变更不修改应用源码。
    - 通过标准：本次新增或修改仅限文档和 Skill 目录，不触碰应用源码。
    - 证据：`git status --short`，并区分仓库原本已有的无关源码改动。

## 必需验证

运行或执行：

```powershell
Test-Path spec/harness-engineering-skill.md
Test-Path acceptance/harness-engineering-skill.md
Test-Path C:/Users/Damon/.codex/skills/harness-engineering/SKILL.md
Test-Path C:/Users/Damon/.codex/skills/harness-engineering/agents/openai.yaml
Test-Path C:/Users/Damon/.codex/skills/harness-engineering/scripts/init_harness.py
Get-ChildItem C:/Users/Damon/.codex/skills/harness-engineering/references
python C:/Users/Damon/.codex/skills/harness-engineering/scripts/init_harness.py <temp-project-path>
python C:/Users/Damon/.codex/skills/.system/skill-creator/scripts/quick_validate.py C:/Users/Damon/.codex/skills/harness-engineering
Get-ChildItem C:/Users/Damon/Desktop -Filter 'harness-engineering-skill-backup-*'
git status --short
```

## 不在范围内

- 应用构建。
- 应用测试。
- 推送 Skill 到远程仓库。
- 修复无关源码改动。
