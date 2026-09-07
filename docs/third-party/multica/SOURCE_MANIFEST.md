# Multica Source Manifest

## Locked upstream source

- Upstream: <https://github.com/multica-ai/multica>
- Remote: `https://github.com/multica-ai/multica.git`
- Reviewed commit: `9fce92f427694d7d303258aa281b05c902a95ba9`
- Reviewed branch: `main`
- Review date: `2026-09-04`
- `LICENSE` SHA-256: `7505297A2A4BF866354D482699DB44EF8B50E8893674E674C5844D3F75DA4464`
- `NOTICE` SHA-256: `431EC97BF0002E9ADDDB31431750C2000A5895586FE1EE56A4D1D02B5DD8B71C`

## Current tracked material

| CCP path | Upstream path | Status | Modification notice |
| --- | --- | --- | --- |
| `docs/third-party/multica/LICENSE` | `LICENSE` | Exact copy | None |
| `docs/third-party/multica/NOTICE` | `NOTICE` | Exact copy | None |
| `apps/codex-workflow-surface/vendor/multica/packages/core/` | `packages/core/` (`080b3e46b90508077db4e41a5349b4246dbc83d8`) | Derived build snapshot | `tsconfig.json` extends the derived bundle root instead of the unavailable upstream monorepo preset; source/runtime code remains unchanged. |
| `apps/codex-workflow-surface/vendor/multica/packages/ui/` | `packages/ui/` (`e8097a7c25b2084cc549ef5627fa17661c713c72`) | Derived build snapshot | `tsconfig.json` extends the derived bundle root instead of the unavailable upstream monorepo preset; source/runtime code remains unchanged. |
| `apps/codex-workflow-surface/vendor/multica/packages/views/` | `packages/views/` (`e161cf9594aff8259e89f4a9eb89606308d3343b`) | Derived build snapshot | `tsconfig.json` extends the derived bundle root instead of the unavailable upstream monorepo preset; source/runtime code remains unchanged. |
| `apps/codex-workflow-surface/vendor/multica/packages/tsconfig/` | `packages/tsconfig/` (`92ee3e5aaa8f7dda0243ffa8b647cf0075796ad6`) | Exact source-tree copy | None |

The local `.upstream-multica/` checkout is an ignored audit input, not a
build or release dependency. The vendored source is the release input; it is
currently an unmodified snapshot and is not yet mounted into Codex until the
typed Runtime Adapter and the package build closure are complete.

## Rules for the first UI port

Before adding a derived file, append one row recording its exact upstream
path, source SHA-256, destination path, and the functional modifications made
for the Codex Runtime Adapter. Retain the upstream copyright header where one
exists and add a conspicuous CCP modification notice in the derived file.

The derived UI must retain the Multica product name, logo, copyright and
attribution required by the upstream license. Every release containing a
derived file must ship this directory's complete `LICENSE` and `NOTICE`.
