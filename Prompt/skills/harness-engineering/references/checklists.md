# Harness Engineering Checklists

## Before Editing

- Read `AGENTS.md`.
- Read relevant `README.md`, `docs/`, `spec/`, and `acceptance/`.
- Identify related source, config, tests.
- Classify task risk.
- Confirm whether new/updated spec and acceptance are needed.
- Summarize goal, files, non-goals, acceptance, risks.

## Spec Quality

- Has background and goal.
- Explicitly says what is out of scope.
- Describes user/maintainer perspective.
- Requirements are executable.
- Technical constraints are clear.
- Delivery scope is finite.

## Acceptance Quality

- Each criterion has pass/fail condition.
- Each criterion has evidence.
- Required verification is realistic.
- Non-goals are listed.
- Criteria do not depend on subjective “looks good” language.

## Implementation

- Smallest necessary change.
- No unrelated refactor.
- No unnecessary dependency.
- No production config/data deletion unless explicitly specified.
- Existing unrelated working-tree changes are not reverted.

## Verification

- Verification matches acceptance criteria.
- Failed checks are fixed or reported.
- Outputs are not invented.
- Screenshots/logs/commands are summarized accurately.

## Review

- Implementation still matches spec.
- Acceptance items are all addressed or explicitly unresolved.
- Risky behavior has rollback or mitigation notes.
- Final report includes evidence and residual risk.
