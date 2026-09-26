# Project Initialization

Use this reference when the user asks to initialize Harness Engineering in a project.

## Trigger Phrases

- "初始化"
- "初始化这套工作流"
- "set up Harness Engineering"
- "initialize this project with $harness-engineering"
- "在这个项目生成相应的文档标准"

## Expected Output

Create or complete:

- `AGENTS.md`: project-level agent contract.
- `docs/harness-engineering-theory.md`: method overview for maintainers.
- `spec/harness-engineering-baseline.md`: spec for installing the workflow baseline.
- `acceptance/harness-engineering-baseline.md`: acceptance criteria for the baseline.

## Safety Rules

- Never overwrite an existing `AGENTS.md` unless the user explicitly asks for overwrite.
- If `spec/`, `acceptance/`, or `docs/` already exist, add the baseline files without deleting existing contents.
- Keep generated text generic when project details are unknown.
- Mention assumptions in the final response.

## Preferred Command

From the skill directory:

```bash
python scripts/init_harness.py /path/to/project
```

Overwrite existing baseline files only when explicitly requested:

```bash
python scripts/init_harness.py /path/to/project --force
```

## Manual Fallback

If the script cannot run, create the same files manually:

1. `AGENTS.md`
   - Project purpose.
   - Directory map.
   - Required documents.
   - Spec and acceptance rules.
   - Task phases.
   - Verification commands.
   - Safety boundaries.
   - Delivery format.

2. `spec/harness-engineering-baseline.md`
   - Why the workflow is being added.
   - What files are created.
   - What is out of scope.
   - Technical constraints.

3. `acceptance/harness-engineering-baseline.md`
   - File existence checks.
   - Content checks.
   - No source-code-change check.
   - Required verification commands.

4. `docs/harness-engineering-theory.md`
   - Definition.
   - Principles.
   - Task layering.
   - Role boundaries.
   - Failure modes.
