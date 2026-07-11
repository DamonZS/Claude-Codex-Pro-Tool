# English README

## Background

The repository uses `README.md` as its Chinese landing page and already links to `README_EN.md`, but the English document is substantially shorter and no longer reflects the current product, provider routing, Pangu Memory, GPT-5.6 injection fix, build workflow, or release process.

## Goals

- Keep `README.md` as the GitHub default Chinese README.
- Rewrite `README_EN.md` as a complete, readable English companion to the current Chinese README.
- Preserve the language switch at the top of both documents.
- Cover the current product purpose, major features, GPT-5.6 injection fix, safety boundaries, data locations, FAQ, development commands, packaging, GitHub Actions, repository structure, license, and project disclaimer.
- Keep commands, paths, model IDs, filenames, workflow names, and technical identifiers unchanged.

## Non-goals

- Do not change runtime behavior, build scripts, release automation, versioning, or application UI.
- Do not rewrite or replace the Chinese README.
- Do not change license, attribution, repository ownership, or maintainer information.

## Target readers

- English-speaking users evaluating or installing the application.
- Developers building or testing the Rust + Tauri + React workspace.
- Maintainers diagnosing injection, provider, plugin, packaging, or release behavior.

## Documentation requirements

- Use concise English headings and practical descriptions.
- Lead with the product purpose and highlighted GPT-5.6 capability.
- Keep install and verification commands copyable.
- Explain that injection removes the Codex client-side selection restriction but cannot add unsupported upstream model capability.
- Explain review, backup, redaction, and third-party execution boundaries.
- Link back to `README.md`, `LICENSE`, `RULES.md`, `MAINTAINERS.md`, Releases, Issues, and the canonical repository URL.

## Deliverables

- Updated `README_EN.md`.
- Matching acceptance document.
