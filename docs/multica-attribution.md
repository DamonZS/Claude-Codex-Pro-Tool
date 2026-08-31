# Multica Attribution and Integration Boundary

Claude Codex Pro Tool's Codex workspace is **Built on Multica**.

- Upstream: <https://github.com/multica-ai/multica>
- Reviewed source revision: `c1e1f11e21cc2ee7dca0ac506a21cb64cca87af4`
- Revision date: `2026-08-31T06:26:15Z`
- Copyright: `Copyright 2025-2026 Multica, Inc.`
- License and NOTICE snapshots: `docs/third-party/multica/`
- `LICENSE` SHA-256: `0E42D37BB02DC61F270C5A0528D489DA76E5A578B209856F2E95EE4D60AACDBE`
- `NOTICE` SHA-256: `763619B43AE4F18C43BEF5284C04A5739F84CD9F935C0123CF67834541EC3D9A`

## Integration Boundary

This repository does not copy or embed Multica's Web, desktop, mobile, or shared UI source. The Codex workspace UI and its local storage/execution adapters are implemented in CCP and communicate only with the currently open Codex page Host API.

The default integration does not download, package, launch, supervise, or require the Multica server, daemon, CLI, database, or Web application. It also does not register or launch a second Codex runtime. Any legacy external-runtime compatibility remains a separate, explicitly enabled path.

If a future change includes or derives from upstream Multica source, the complete Multica License, NOTICE, branding, modification notices, and applicable commercial-use conditions must be reviewed again before release.
