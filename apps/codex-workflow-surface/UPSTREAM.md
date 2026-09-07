# External Multica UI Source

The Codex workflow surface resolves `@multica/*` directly from the external
`D:/Project/multica/packages/` checkout (override with `MULTICA_SOURCE_ROOT`).
No upstream source snapshot is stored in this repository. CCP-specific mount,
transport and API-adapter code lives in this project under `src/`.

The complete upstream `LICENSE` and `NOTICE` are in
`docs/third-party/multica/` and must accompany a release containing this
source or a compiled derivative.
