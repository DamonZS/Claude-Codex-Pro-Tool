---
name: harness-engineering
description: Use when a coding task needs spec-first planning, acceptance criteria, verification evidence, AI agent workflow design, sub-agent coordination, disciplined project delivery, or project workflow initialization.
---

# Harness Engineering

## Core Rule

Treat every non-trivial task as a controlled engineering loop:

1. Read project context.
2. Define or locate the spec.
3. Define or locate acceptance criteria.
4. Implement the smallest necessary change.
5. Verify with real evidence.
6. Report against the acceptance criteria.

Do not claim completion without verification evidence.

## Initialization Mode

When the user invokes this skill in a project and asks to initialize, bootstrap, or set up Harness Engineering:

1. Inspect the target project root.
2. Generate the Harness Engineering baseline documents:
   - `AGENTS.md`
   - `docs/harness-engineering-theory.md`
   - `spec/harness-engineering-baseline.md`
   - `acceptance/harness-engineering-baseline.md`
3. Do not overwrite an existing `AGENTS.md` unless the user explicitly asks for overwrite.
4. Prefer running `scripts/init_harness.py <project-root>` for deterministic initialization.
5. After initialization, verify the four files exist and report the generated paths.

If the script is unavailable, use `references/initialization.md` to create the same baseline manually.

## Decision Guide

Use the lightest process that protects the task:

| Task type | Examples | Required workflow |
| --- | --- | --- |
| Micro | run a command, explain code, typo fix | Use existing context, verify if claiming a result |
| Normal | local bug fix, small UI copy change | Summarize scope and run targeted verification |
| Important | page, API, module, cross-file behavior | Create/read `spec/*.md` and `acceptance/*.md` first |
| High risk | auth, secrets, deletion, installer, release, migration | Full spec, acceptance, rollback/risk notes, stronger verification |

When uncertain, classify upward.

## Required Workflow

### 1. Gather Context

Read in this order when present:

- `AGENTS.md`
- `README.md`
- Relevant `docs/`
- Relevant `spec/*.md`
- Matching `acceptance/*.md`
- Related source, config, and tests

If project docs conflict, follow the higher-priority project rule and state the conflict.

### 2. Fill Missing Harness

If a non-trivial task lacks a spec or acceptance criteria:

- Create or update a spec before implementation.
- Create or update matching acceptance criteria before delivery.
- Keep both scoped to the current task.

Use `references/templates.md` for templates.

### 3. Summarize Before Implementing

Before code changes, summarize:

- Current goal.
- Files likely to change.
- Files or behaviors that must not change.
- Acceptance criteria.
- Key risks.

### 4. Implement Narrowly

Make the smallest change that satisfies the spec. Avoid unrelated refactors, dependency changes, production config changes, and cleanup outside scope.

### 5. Verify

Run the narrowest real verification that proves the acceptance criteria. Broaden verification for shared, security-sensitive, release, migration, or user-visible behavior.

Never invent test, build, run, screenshot, or log results.

### 6. Report

Final response must include:

1. Task conclusion.
2. Files changed and why.
3. Verification commands/results.
4. Acceptance criteria mapping.
5. Remaining risks or follow-up.

## Sub-Agent Use

For complex work, split roles:

- Spec Agent: produces `spec/*.md`.
- Acceptance Agent: produces `acceptance/*.md`.
- Implementation Agent: changes code/config/tests.
- Test Agent: verifies against acceptance criteria.
- Review Agent: checks drift, omissions, quality, and risk.

Do not let the same role silently redefine the target, implement it, and judge completion.

## References

Read only what is needed:

- `references/theory.md`: deeper theory, task layering, failure modes, and metrics.
- `references/initialization.md`: project initialization behavior and generated baseline expectations.
- `references/templates.md`: spec, acceptance, task summary, and final report templates.
- `references/checklists.md`: quick checklists for creation, implementation, verification, and review.

## Common Mistakes

| Mistake | Correction |
| --- | --- |
| Coding before the spec is clear | Stop and write/update the spec |
| Acceptance says "works correctly" | Replace with observable pass/fail criteria |
| Reporting "done" after reading code | Run verification or state why it cannot run |
| Expanding scope during implementation | Move extra ideas to follow-up |
| Treating failed verification as a footnote | Fix it or report it as unresolved risk |
