#!/usr/bin/env bash
set -euo pipefail
target="${1:?target copy path required}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
cp -- "$script_dir/BASELINE_FILE.tsx" "$target"
