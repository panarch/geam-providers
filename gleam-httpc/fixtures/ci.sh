#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../.github/scripts/ci-lib.sh"

case "${1:-}" in
  standalone)
    cd "$CI_OWNER_DIR/fixtures/gleam"
    for module in geam_httpc_fixture httpc_redirect httpc_binary httpc_tls; do
      python3 ../server.py "$(ci_native_path "$GEAM_BIN")" run --module "$module" \
        --provider-config gleam_httpc=../config/provider.toml
    done
    ;;
  executable)
    GEAM_CONFIG="$(ci_native_path "$CI_OWNER_DIR/fixtures/config/runtime.toml")"
    export GEAM_CONFIG
    cd "$CI_RUN_DIR"
    python3 "$(ci_native_path "$CI_OWNER_DIR/fixtures/server.py")" "$(ci_native_path "$CI_FIXTURE_BINARY")"
    ;;
  *) ci_fail "unknown httpc hook phase: ${1:-}" ;;
esac
