# Harness Engineering Theory

Harness Engineering is an AI-agent development method that reduces uncertainty by wrapping work in context, specs, acceptance criteria, constrained implementation, and real verification.

## Problem

AI coding failures often come from loose task systems:

- Unclear goals cause requirement invention.
- Unclear scope causes unrelated changes.
- Unclear acceptance causes subjective completion.
- Missing context causes hidden contract breakage.
- Missing verification turns assumptions into reported facts.
- One agent acting as spec writer, implementer, tester, and judge increases self-confirmation bias.

## Principles

- Understand before modifying.
- Specify before implementing.
- Define acceptance before delivery.
- Verify before reporting.
- Prefer smallest necessary changes.
- Evidence beats confidence.
- Escalate process depth as risk rises.

## Task Layering

| Layer | Signal | Harness depth |
| --- | --- | --- |
| Micro | command, explanation, typo | No new files required; still avoid invented results |
| Normal | small local change | Lightweight summary and targeted validation |
| Important | new feature, API, module, multi-file behavior | Formal spec + matching acceptance |
| High risk | auth, secrets, deletion, installers, release, migration, third-party scripts | Full spec/acceptance, rollback notes, stronger validation, review |

## Document Loop

1. Demand enters.
2. Context is read.
3. Demand becomes spec.
4. Spec becomes acceptance.
5. Acceptance drives implementation.
6. Verification checks acceptance.
7. Delivery maps work back to acceptance.
8. New constraints are folded back into docs when useful.

## Role Boundaries

- Spec role defines what and what not.
- Acceptance role defines pass/fail and evidence.
- Implementation role makes narrow changes.
- Test role gathers proof.
- Review role checks drift, omissions, and risk.

## Failure Modes

| Failure | Correction |
| --- | --- |
| No spec before code | Stop implementation and create one |
| Spec is a wish list | Convert to executable requirements |
| Acceptance is subjective | Add observable pass/fail criteria |
| Verification is skipped | Run it or report inability/risk |
| Scope expands | return to delivery scope |
| Sub-agents overlap | Define inputs, outputs, and forbidden scope |

## Metrics

Use these to judge whether the harness is helping:

- Rework caused by misunderstood requirements.
- Acceptance criteria hit rate.
- Percentage of deliveries with real evidence.
- Frequency of unrelated changes.
- Defects found after delivery.
- Reuse of specs, acceptance files, and templates.
- Traceability from change to spec to acceptance.
