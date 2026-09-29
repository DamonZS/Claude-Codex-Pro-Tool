#!/usr/bin/env sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
target=${1:?Usage: ROLLBACK.sh TARGET_COPY_DIRECTORY}
[ -f "$target/.ccp-transaction-copy" ] || { printf '%s\n' 'Expected a disposable transaction copy'; exit 2; }
cp -R "$here/baseline/." "$target/"
rm -f "$target/apps/claude-codex-pro-manager/src/lib/overviewUsage.ts"
rm -f "$target/scripts/test-overview-usage.cjs"
printf '%s\n' 'Pristine baseline bytes restored to target copy'
