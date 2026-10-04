#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../.github/scripts/ci-lib.sh"

case "${1:-}" in
  erlang)
    cd "$CI_OWNER_DIR/fixtures/gleam"
    gleam run
    gleam run -- --flag '' key=value 한글
    ;;
  standalone)
    cd "$CI_OWNER_DIR/fixtures/gleam"
    "$GEAM_BIN" run
    "$GEAM_BIN" run --
    "$GEAM_BIN" run -- --flag '' key=value 한글
    ;;
  executable)
    cd "$CI_RUN_DIR"
    "$CI_FIXTURE_BINARY"
    "$CI_FIXTURE_BINARY" --flag '' key=value 한글
    ;;
  *) ci_fail "unknown argv hook phase: ${1:-}" ;;
esac
