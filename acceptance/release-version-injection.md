# Acceptance: Release version injection and auto-publish stabilization

Spec: `spec/release-version-injection.md`

## Pass criteria

1. Public release version is not hard-coded in source
   - `crates/claude-codex-pro-core/src/version.rs` defaults to `dev-<Cargo version>`.
   - `VERSION` still prioritizes `CLAUDE_CODEX_PRO_RELEASE_VERSION`.

2. Auto release version injection is present
   - `auto-release-installers.yml` sets `CLAUDE_CODEX_PRO_RELEASE_VERSION: ${{ needs.prepare-release.outputs.tag }}`.
   - prepare job fetches tags, reads published release tags, and uses `next-release-tag.js` to increment versions.
   - orphan tags from failed draft releases are deleted before reuse.

3. Release notes are Chinese and not misleading
   - Contains `## 更新内容`, `## 验证`, `## 构建产物说明`.
   - Does not contain `## Assets 9`, `Source code (zip)`, or `Source code (tar.gz)` body enumeration.

4. Windows direct release upload failure path is removed
   - Windows job no longer calls `gh release upload $env:TAG ...`.
   - Windows/macOS jobs use `actions/upload-artifact@v5`.
   - publish job uses `actions/download-artifact@v5` and uploads to the Release from Linux.
   - publish job validates 6 app build assets before `latest.json`.

5. Required verification passes
   - `node scripts/release/verify-release-workflow.js`
   - `cargo test -p claude-codex-pro-core --manifest-path Cargo.toml exposes_project_release_version -- --nocapture`
   - `cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem github_auto_release_workflow_builds_installers_with_v0_tags -- --nocapture`
   - `npm --prefix apps/claude-codex-pro-manager run check`
   - `npm --prefix apps/claude-codex-pro-manager run vite:build`
   - `cargo fmt --check`
   - `git diff --check`

## Out of scope

- Local creation of a real GitHub Release.
- Local execution of hosted Windows/macOS runners.
- Verification of installed legacy app upgrades.
