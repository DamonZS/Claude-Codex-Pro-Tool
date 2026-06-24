# Contributing to Claude Codex Pro Tool

Thank you for your interest in contributing to Claude Codex Pro Tool.

## Development Setup

1. Clone the repository

```bash
git clone https://github.com/DamonZS/Claude-Codex-Pro-Tool.git
cd Claude-Codex-Pro-Tool
```

2. Install the Rust toolchain

Make sure Rust 1.85 or newer is installed.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version
```

3. Install frontend dependencies

```bash
cd apps/claude-codex-pro-manager
npm install
cd ../..
```

4. Build the project

```bash
cargo build --release
```

5. Run tests

```bash
cargo test --workspace
cd apps/claude-codex-pro-manager
npm run check
```

## Project Structure

```text
Claude-Codex-Pro-Tool/
  apps/
    claude-codex-pro-launcher/
    claude-codex-pro-manager/
  crates/
    claude-codex-pro-core/
    claude-codex-pro-data/
  scripts/
  docs/
  README.md
  README_EN.md
```

## Making Changes

1. Create a feature branch

```bash
git checkout -b feat/your-feature-name
```

2. Make your changes

- Keep changes focused.
- Add or update tests when behavior changes.
- Update documentation when user-facing behavior changes.

3. Run validation

```bash
cargo test --workspace
cd apps/claude-codex-pro-manager
npm run check
npm run vite:build
```

## Code Style

- Format Rust with `cargo fmt`.
- Prefer clear names and direct control flow.
- Add comments only when they carry real context.

## Pull Requests

1. Fork the repository.
2. Create a branch from the latest `main`.
3. Keep the diff scoped to one problem or feature.
4. Include validation details in the PR description.
5. Link related issues when relevant.

## Reporting Issues

- Use GitHub Issues for bugs and feature requests.
- Include OS, Codex version, and reproduction steps.
- When the issue involves relay injection or provider switching, include the minimum config needed to reproduce without exposing secrets.

## License

By contributing, you agree that your contributions are licensed under the repository license.

## Repository Rules

This repository uses a custom source-available restricted license. It is not an
OSI-approved open source license.

Do not submit code, documentation, assets, workflow changes, generated patches,
or metadata changes unless DamonZS or an authorized maintainer has requested or
approved them. Unauthorized modifications are prohibited whether they are made
manually, with AI, with scripts, with codemods, or with any other automation.

Author information, repository ownership, product name, branding, license,
rules, release metadata, sponsorship identities, and payment identities must not
be removed, replaced, hidden, or rewritten.

These restrictions do not limit DamonZS, the repository owner, authorized
maintainers, or tools acting under their instruction. Official maintainers may
continue using AI assistants, scripts, CI, codemods, formatting tools, and manual
editing to develop this project.

Authorized maintainers are recorded in [MAINTAINERS.md](MAINTAINERS.md).

See [LICENSE](LICENSE), [RULES.md](RULES.md), and [MAINTAINERS.md](MAINTAINERS.md).
