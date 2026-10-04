#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../.github/scripts/ci-lib.sh"

certificate="$(ci_native_path "$CI_OWNER_DIR/fixtures/certs/cert.pem")"
key="$(ci_native_path "$CI_OWNER_DIR/fixtures/certs/key.pem")"

case "${1:-}" in
  erlang)
    cd "$CI_OWNER_DIR/fixtures/gleam"
    python3 ../run.py gleam run -- "$certificate" "$key"
    ;;
  embedding)
    cd "$CI_OWNER_DIR/fixtures/embedding"
    python3 ../run.py cargo run --locked
    ;;
  standalone)
    cd "$CI_OWNER_DIR/fixtures/gleam"
    python3 ../run.py "$(ci_native_path "$GEAM_BIN")" run -- "$certificate" "$key"
    ;;
  executable)
    cd "$CI_RUN_DIR"
    python3 "$(ci_native_path "$CI_OWNER_DIR/fixtures/run.py")" \
      "$(ci_native_path "$CI_FIXTURE_BINARY")" "$certificate" "$key"
    ;;
  *) ci_fail "unknown glisten hook phase: ${1:-}" ;;
esac
