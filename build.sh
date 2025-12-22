#!/usr/bin/env bash
set -euo pipefail

# Build pproxy with embedded build metadata from build.rs.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Optional verbosity: ./build.sh -v | -vv
CARGO_VERBOSE=""
case "${1:-}" in
  -v)  CARGO_VERBOSE="-v"; shift ;;
  -vv) CARGO_VERBOSE="-vv"; shift ;;
esac

echo "Building pproxy"

# Best-effort: stop a running pproxy that might be using resources.
# (Linux can overwrite running binaries, but this keeps behavior consistent.)
if command -v pkill >/dev/null 2>&1; then
  pkill -f "(^|/)pproxy(\\.exe)?$" >/dev/null 2>&1 || true
fi

cargo build --release ${CARGO_VERBOSE} "$@"

if [[ -f "pproxy.yaml" ]]; then
  cp -f "pproxy.yaml" "target/release/pproxy.yaml"
fi

echo "Done: target/release/pproxy"
