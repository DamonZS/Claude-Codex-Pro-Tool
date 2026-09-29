#!/bin/sh
set -eu
[ "$#" -eq 1 ] || { printf '%s\n' 'usage: ROLLBACK.sh TARGET_COPY' >&2; exit 2; }
base_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cp -- "$base_dir/claude_desktop.baseline.rs" "$1"
