# Acceptance: English README

Validates: `spec/english-readme.md`

## Pass criteria

1. `README.md` remains the root/default Chinese document and contains a link to `README_EN.md`.
2. `README_EN.md` starts with an English landing page and links back to `README.md` as `中文`.
3. The English README covers:
   - product purpose and audience;
   - GPT-5.6 third-party API model-selection injection fix;
   - Codex, Claude, providers, plugins, sessions, Pangu Memory, scripts, maintenance, and updates;
   - safety boundaries and local data locations;
   - FAQ and build/package/release commands;
   - license, attribution, repository URL, and non-official-project disclaimer.
4. Markdown fenced code blocks are balanced and local Markdown links resolve to existing files.
5. No application source, runtime configuration, release workflow, or Chinese README content is changed by this task.

## Verification

- Inspect `git diff -- README.md README_EN.md spec/english-readme.md acceptance/english-readme.md`.
- Run a Markdown structure/link check for both README files.
- Run `git diff --check`.

## Evidence

- Command output showing balanced fences, valid local links, and a clean whitespace check.
- Final change summary listing the English documentation sections added or refreshed.
