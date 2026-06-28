# 验收标准：Harness Engineering 理论完善

验证对象：`spec/harness-engineering-theory.md`

## 验收项

1. 规格文档存在。
   - 通过标准：`spec/harness-engineering-theory.md` 存在。
   - 验证证据：文件存在检查。

2. 验收标准存在。
   - 通过标准：`acceptance/harness-engineering-theory.md` 存在。
   - 验证证据：文件存在检查。

3. 理论文档存在。
   - 通过标准：`docs/harness-engineering-theory.md` 存在。
   - 验证证据：文件存在检查。

4. 理论文档为中文。
   - 通过标准：正文主体使用中文；路径、命令、产品名、框架名和必要英文术语可保留。
   - 验证证据：内容检查。

5. 理论文档解释方法论定义和目标。
   - 通过标准：文档包含“定义”和“目标”相关章节或等价内容。
   - 验证证据：内容检查。

6. 理论文档包含核心原则。
   - 通过标准：文档至少说明先理解后修改、先规格后开发、先验收后交付、先验证后汇报、最小改动等原则。
   - 验证证据：内容检查。

7. 理论文档包含任务分层。
   - 通过标准：文档至少区分微任务、普通任务、重要任务和高风险任务，并说明各自需要的文档/验证深度。
   - 验证证据：内容检查。

8. 理论文档说明角色边界。
   - 通过标准：文档说明规格、验收、实现、测试、评审各自负责什么和不负责什么。
   - 验证证据：内容检查。

9. 理论文档说明闭环流程。
   - 通过标准：文档覆盖需求进入、规格化、验收化、实施、验证和复盘。
   - 验证证据：内容检查。

10. 理论文档说明失败模式和纠偏策略。
    - 通过标准：文档列出常见失败模式，并给出对应处理方式。
    - 验证证据：内容检查。

11. 理论文档说明度量指标。
    - 通过标准：文档给出可用于判断流程有效性的指标。
    - 验证证据：内容检查。

12. `AGENTS.md` 包含理论文档入口。
    - 通过标准：`AGENTS.md` 提到 `docs/harness-engineering-theory.md`。
    - 验证证据：内容检查。

13. 本次变更不修改应用源码。
    - 通过标准：本次新增或修改仅限 `AGENTS.md`、`spec/`、`acceptance/` 和 `docs/` 下的 Markdown 文档。
    - 验证证据：`git status --short`，并区分仓库中原本已有的无关源码改动。

## 必需验证

运行或执行：

```bash
Test-Path spec/harness-engineering-theory.md
Test-Path acceptance/harness-engineering-theory.md
Test-Path docs/harness-engineering-theory.md
Select-String -Path AGENTS.md -Pattern 'docs/harness-engineering-theory.md'
git status --short
```

随后人工检查理论文档是否满足以上内容要求。

## 不在范围内

- 运行应用构建。
- 运行测试套件。
- 修改源码。
- 编写自动化 lint 或 CI 检查。
