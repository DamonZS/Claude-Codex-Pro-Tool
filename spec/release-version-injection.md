# Release version injection and auto-publish stabilization

## Background

GitHub Actions auto release failed in the Windows installer path, so the final Release was not published. The release page must keep Chinese section titles for 更新内容 and 验证, and the application should not hard-code a public release version in source code. The release tag should be generated on push and injected into the build, so the Manager About page shows the current published version.

## Goals

- Keep the tag sequence `V0.01 -> V0.02 -> ... -> V0.99 -> V1.00`, based on published releases rather than orphan tags from failed drafts.
- Do not hard-code a public `V0.xx` release in source code; local builds use `dev-<Cargo version>`.
- GitHub release builds inject `CLAUDE_CODEX_PRO_RELEASE_VERSION` from the release tag.
- The About page, injection script and update checker share the same backend version constant.
- Release notes contain Chinese headings: `更新内容`, `验证`, `构建产物说明`.
- Do not write `Assets 9` in the body; GitHub's Assets section is the download list.
- Build jobs upload workflow artifacts first; the publish job downloads them and uploads Release assets in one place.

## Non-goals

- Do not change supplier, Pangu Memory, Claude Chinese injection or other business logic.
- Do not change Cargo/Tauri internal semver to `V0.xx`.
- Do not list all 9 GitHub assets in the Release body.
- Do not package local supplier config, API keys, memory DBs or user caches.

## Functional requirements

- `DEFAULT_RELEASE_VERSION` is derived from Cargo package version as a development version.
- `VERSION` prioritizes `CLAUDE_CODEX_PRO_RELEASE_VERSION` when present.
- `auto-release-installers.yml` fetches tags, reads published release tags, and resolves the next tag from published releases.
- If a failed draft left an orphan tag for the next version, the workflow deletes that tag before recreating it for the current commit.
- Windows and macOS jobs build artifacts and upload workflow artifacts only.
- `publish-release` downloads build artifacts, verifies 6 app assets before `latest.json`, uploads them to the Release, generates `latest.json`, uploads it, then publishes the Release.
- Release notes are UTF-8 and keep Chinese section headings.

## Verification requirements

- The release workflow contract script covers Chinese notes, artifact fan-in upload, and the absence of direct Windows `gh release upload`.
- Rust regression tests cover version injection and workflow structure.
- Frontend type check and production build pass.
