# Multica Attribution and Integration Boundary

Claude Codex Pro Tool's Codex workspace is **Built on Multica**.

- Upstream: <https://github.com/multica-ai/multica>
- Reviewed source revision: `9fce92f427694d7d303258aa281b05c902a95ba9`
- Copyright: `Copyright 2025-2026 Multica, Inc.`
- License and NOTICE snapshots: `docs/third-party/multica/`
- `LICENSE` SHA-256: `7505297A2A4BF866354D482699DB44EF8B50E8893674E674C5844D3F75DA4464`
- `NOTICE` SHA-256: `431EC97BF0002E9ADDDB31431750C2000A5895586FE1EE56A4D1D02B5DD8B71C`

## Integration Boundary

This repository currently carries Multica license and attribution snapshots only. Any later copied or derived Multica UI source must be recorded in `docs/third-party/multica/SOURCE_MANIFEST.md`, retain upstream attribution and visual branding, and communicate only with the currently open Codex page Host API.

The default integration does not download, package, launch, supervise, or require the Multica server, daemon, CLI, database, or Web application. It also does not register or launch a second Codex runtime. Any legacy external-runtime compatibility remains a separate, explicitly enabled path.

If a future change includes or derives from upstream Multica source, the complete Multica License, NOTICE, branding, modification notices, and applicable commercial-use conditions must be reviewed again before release.
