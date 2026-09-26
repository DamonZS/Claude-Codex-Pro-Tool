#!/usr/bin/env python3
"""Initialize Harness Engineering baseline docs in a project."""

from __future__ import annotations

import argparse
from pathlib import Path
from textwrap import dedent


def detect_project(root: Path) -> dict[str, bool]:
    return {
        "readme": (root / "README.md").exists(),
        "package": (root / "package.json").exists(),
        "cargo": (root / "Cargo.toml").exists(),
        "python": (root / "pyproject.toml").exists() or (root / "requirements.txt").exists(),
        "src": (root / "src").exists(),
        "apps": (root / "apps").exists(),
        "crates": (root / "crates").exists(),
        "tests": (root / "tests").exists(),
    }


def write_file(path: Path, content: str, force: bool) -> str:
    if path.exists() and not force:
        return f"skip existing {path}"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")
    return f"write {path}"


def validation_commands(info: dict[str, bool]) -> str:
    commands = ["git status --short"]
    if info["package"]:
        commands.append("npm test")
        commands.append("npm run build")
    if info["cargo"]:
        commands.append("cargo fmt --check")
        commands.append("cargo test --workspace")
    if info["python"]:
        commands.append("python -m pytest")
    return "\n".join(f"- `{cmd}`" for cmd in commands)


def ag_agents(project_name: str, info: dict[str, bool]) -> str:
    dirs = [
        "- `docs/`: 项目文档与方法论说明。",
        "- `spec/`: 任务与功能规格文档。",
        "- `acceptance/`: 与规格文档匹配的验收标准。",
    ]
    if info["src"]:
        dirs.insert(0, "- `src/`: 主要源码目录。")
    if info["apps"]:
        dirs.insert(0, "- `apps/`: 应用或前端/后端子项目目录。")
    if info["crates"]:
        dirs.insert(0, "- `crates/`: Rust crate 或核心库目录。")
    if info["tests"]:
        dirs.append("- `tests/`: 测试目录。")

    return dedent(
        f"""\
        # AGENTS.md

        ## 项目目的

        `{project_name}` 使用 Harness Engineering 工作方式组织 AI 代理协作。代理必须基于清晰上下文、明确规格、可验证验收标准和真实验证证据工作，不得一边猜需求一边修改代码。

        如果项目目的尚未在 README 或业务文档中明确，首次开发任务应先补充项目目的和关键边界。

        ## 目录结构

        {chr(10).join(dirs)}

        ## 必读文档

        开始任何重要开发任务前，必须先阅读并理解：

        1. `AGENTS.md`
        2. `README.md`（如果存在）
        3. `docs/harness-engineering-theory.md`
        4. `spec/` 下的相关规格文档
        5. `acceptance/` 下的匹配验收标准
        6. 与任务相关的源码、配置和测试

        如果任务没有相关规格文档或验收标准，必须先补齐。

        ## 规格文档规则

        规格文档存放在 `spec/` 下。重要功能、页面、接口、模块、配置、数据迁移或重构任务，必须先有规格文档再实现。

        一个合格规格文档应包含：标题、背景、目标、非目标、用户视角、功能要求、界面/交互要求、数据与接口要求、技术约束和交付范围。

        ## 验收标准规则

        验收标准存放在 `acceptance/` 下，并尽量与规格文档一一对应：

        ```text
        spec/feature-name.md
        acceptance/feature-name.md
        ```

        每个验收标准必须说明通过/失败标准、验证方式和所需证据。

        ## 标准任务流程

        1. 理解任务：读项目规则、规格、验收标准和相关代码。
        2. 总结上下文：说明目标、预计改动、禁止改动、验收标准和风险。
        3. 实施开发：按规格做最小必要改动。
        4. 验证：运行真实测试、构建、手动检查、日志或截图验证。
        5. 交付：对照验收标准说明结果和剩余风险。

        ## 验证命令

        根据任务选择最窄但足够证明结果的验证。当前项目可优先考虑：

        {validation_commands(info)}

        如果无法运行验证，必须说明原因和剩余风险。

        ## 安全边界

        - 不得删除用户数据、生产配置或重要本地状态，除非用户明确要求且规格文档覆盖该操作。
        - 不得把 API key、token 或授权材料写入日志、文档、测试或记忆文件。
        - 不得自动信任第三方脚本、插件、hooks 或下载归档。
        - 不得回滚无关工作区改动。

        ## 交付格式

        最终回答必须包含：

        1. 任务结论
        2. 修改内容
        3. 验证结果
        4. 对照验收标准
        5. 风险与后续建议
        """
    )


def theory() -> str:
    return dedent(
        """\
        # Harness Engineering 理论

        Harness Engineering 是一种面向 AI 代理协作的软件开发工作法。它通过项目上下文、规格文档、验收标准、最小必要改动和真实验证证据，降低 AI 编码过程中的不确定性。

        ## 核心原则

        - 先理解，后修改。
        - 先规格，后开发。
        - 先验收，后交付。
        - 先验证，后汇报。
        - 最小必要改动。
        - 证据优先于自信。

        ## 任务分层

        - 微任务：命令查询、解释、错别字。可用轻量流程，但不得编造结果。
        - 普通任务：局部 bug、小型文案或样式调整。需要上下文总结和定向验证。
        - 重要任务：页面、接口、模块、跨文件行为。必须有 `spec/` 与 `acceptance/`。
        - 高风险任务：鉴权、密钥、删除、迁移、安装器、发布、第三方脚本。必须完整规格、验收、风险和更强验证。

        ## 角色边界

        - 规格代理：定义做什么和不做什么。
        - 验收代理：定义通过标准和证据。
        - 实现代理：按规格做最小必要改动。
        - 测试代理：按验收标准验证。
        - 评审代理：检查偏离、遗漏、质量和风险。

        ## 失败模式

        - 没有规格就写代码：停止实现，先补规格。
        - 验收标准不可验证：改成可观察的通过/失败标准。
        - 顺手重构：回到交付范围。
        - 未验证就汇报完成：运行验证或说明无法验证的风险。
        """
    )


def spec_baseline() -> str:
    return dedent(
        """\
        # Harness Engineering 文档基线

        ## 背景

        本项目需要建立 AI 代理协作的基础工作流，避免代理在目标、范围、验收和验证不清晰时直接修改代码。

        ## 目标

        本次要完成：

        - 创建 `AGENTS.md`。
        - 创建 `docs/harness-engineering-theory.md`。
        - 创建 `spec/` 和 `acceptance/` 目录。
        - 创建本文档和对应验收标准。

        本次不包含：

        - 修改应用源码。
        - 修改构建、发布、CI 或生产配置。
        - 引入新依赖。

        ## 功能要求

        - 根目录必须存在 `AGENTS.md`。
        - 必须存在 `docs/harness-engineering-theory.md`。
        - 必须存在 `spec/harness-engineering-baseline.md`。
        - 必须存在 `acceptance/harness-engineering-baseline.md`。
        - `AGENTS.md` 必须说明规格、验收、验证和交付规则。

        ## 技术约束

        - 只创建或更新文档。
        - 不改应用源码。
        - 不删除已有文档或目录。

        ## 交付范围

        - `AGENTS.md`
        - `docs/harness-engineering-theory.md`
        - `spec/harness-engineering-baseline.md`
        - `acceptance/harness-engineering-baseline.md`
        """
    )


def acceptance_baseline() -> str:
    return dedent(
        """\
        # 验收标准：Harness Engineering 文档基线

        验证对象：`spec/harness-engineering-baseline.md`

        ## 验收项

        1. `AGENTS.md` 存在。
           - 通过标准：根目录存在该文件。
           - 证据：文件存在检查。

        2. 理论文档存在。
           - 通过标准：`docs/harness-engineering-theory.md` 存在。
           - 证据：文件存在检查。

        3. 规格与验收目录存在。
           - 通过标准：`spec/` 与 `acceptance/` 存在。
           - 证据：目录存在检查。

        4. 基线规格与验收文档存在。
           - 通过标准：本文档和对应 spec 均存在。
           - 证据：文件存在检查。

        5. 未修改应用源码。
           - 通过标准：本次初始化只新增或更新文档。
           - 证据：`git status --short`。

        ## 必需验证

        ```bash
        test -f AGENTS.md
        test -f docs/harness-engineering-theory.md
        test -f spec/harness-engineering-baseline.md
        test -f acceptance/harness-engineering-baseline.md
        git status --short
        ```
        """
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Initialize Harness Engineering docs.")
    parser.add_argument("project", help="Target project directory")
    parser.add_argument("--force", action="store_true", help="Overwrite existing baseline files")
    args = parser.parse_args()

    root = Path(args.project).expanduser().resolve()
    root.mkdir(parents=True, exist_ok=True)
    info = detect_project(root)
    project_name = root.name

    writes = [
        (root / "AGENTS.md", ag_agents(project_name, info)),
        (root / "docs" / "harness-engineering-theory.md", theory()),
        (root / "spec" / "harness-engineering-baseline.md", spec_baseline()),
        (root / "acceptance" / "harness-engineering-baseline.md", acceptance_baseline()),
    ]

    for path, content in writes:
        print(write_file(path, content, args.force))

    print("Harness Engineering baseline initialized.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
