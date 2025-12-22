#!/usr/bin/env bash
set -euo pipefail

# Build pproxy with embedded commit/build timestamp.
# These are read at compile time via option_env!("PPROXY_COMMIT") and option_env!("PPROXY_BUILD_UNIX").

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Optional verbosity: ./build.sh -v | -vv
CARGO_VERBOSE=""
case "${1:-}" in
  -v)  CARGO_VERBOSE="-v"; shift ;;
  -vv) CARGO_VERBOSE="-vv"; shift ;;
esac

PPROXY_COMMIT="unknown"
if command -v git >/dev/null 2>&1; then
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    PPROXY_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  fi
fi

PPROXY_BUILD_UNIX="$(date -u +%s 2>/dev/null || echo 0)"

echo "Building pproxy (commit=${PPROXY_COMMIT}, build_unix=${PPROXY_BUILD_UNIX})"

# Best-effort: stop a running pproxy that might be using resources.
# (Linux can overwrite running binaries, but this keeps behavior consistent.)
if command -v pkill >/dev/null 2>&1; then
  pkill -f "(^|/)pproxy(\\.exe)?$" >/dev/null 2>&1 || true
fi

PPROXY_COMMIT="$PPROXY_COMMIT" \
PPROXY_BUILD_UNIX="$PPROXY_BUILD_UNIX" \
  cargo build --release ${CARGO_VERBOSE} "$@"

if [[ -f "pproxy.yaml" ]]; then
  cp -f "pproxy.yaml" "target/release/pproxy.yaml"
fi

echo "Done: target/release/pproxy"
