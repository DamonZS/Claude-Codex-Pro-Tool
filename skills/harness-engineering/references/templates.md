# Harness Engineering Templates

## Spec Template

```markdown
# <Feature or Task Name>

## Background

Why this work matters, current problem, target value.

## Goals

This includes:

- ...

This does not include:

- ...

## User Perspective

How the user or maintainer will experience the result.

## Functional Requirements

- Observable requirement.
- Edge cases.
- Permissions or constraints.

## UI / Interaction Requirements

Only when relevant.

## Data And API Requirements

Inputs, outputs, data sources, auth, errors.

## Technical Constraints

Frameworks, dependencies, architecture boundaries, forbidden changes.

## Delivery Scope

Files, tests, docs, config, artifacts.
```

## Acceptance Template

```markdown
# Acceptance Criteria: <Feature or Task Name>

Verifies: `spec/<same-name>.md`

## Criteria

1. <Behavior or artifact exists>
   - Pass condition: ...
   - Evidence: ...

## Required Verification

Commands, manual checks, screenshots, logs, or build outputs required.

## Out Of Scope

Checks that are explicitly not required.
```

## Pre-Implementation Summary

```markdown
Goal:
Likely files:
Must not change:
Acceptance criteria:
Risks:
Verification plan:
```

## Final Report

```markdown
## 1. Task Conclusion

What was completed and whether the goal is satisfied.

## 2. Changes

Files changed and why.

## 3. Verification

Commands/checks run and results.

## 4. Acceptance Mapping

Which criteria passed, failed, or were not run.

## 5. Risks And Follow-Up

Remaining risks and useful next steps.
```
