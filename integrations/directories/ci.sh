#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../.github/scripts/ci-lib.sh"
cd "$CI_OWNER_DIR"

main() {
  case "${1:-all}" in
    all)
      ci_step 'directories: source and dependencies' check_source
      ci_step 'directories: original Erlang' run_erlang
      ci_step 'directories: embedding' check_embedding
      ci_step 'directories: standalone' check_standalone
      ci_step 'directories: executable outside its fixture' check_executable
      ;;
    source) ci_step 'directories: source and dependencies' check_source ;;
    erlang) ci_step 'directories: original Erlang' run_erlang ;;
    embedding) ci_step 'directories: embedding' check_embedding ;;
    standalone) ci_step 'directories: standalone' check_standalone ;;
    executable) ci_step 'directories: executable outside its fixture' check_executable ;;
    *) ci_fail "unknown directories phase: $1" ;;
  esac
}

check_source() {
  ci_check_geam_pin fixtures/embedding/Cargo.toml fixtures/embedding/Cargo.lock \
    fixtures/gleam/.cargo/config.toml fixtures/gleam/Cargo.lock
  local fixture
  for fixture in gleam embedding/gleam; do
    (cd "fixtures/$fixture" && gleam deps download && gleam format --check && gleam check)
  done
  (cd fixtures/gleam/build/packages/directories && ci_check_checksum "$PWD/../../../../upstream.sha256")
  (cd fixtures/embedding && cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings)
  (cd fixtures/embedding && RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked)
}

run_erlang() {
  (cd fixtures/gleam && gleam run)
}

check_embedding() {
  cd fixtures/embedding
  if ! "$GEAM_BIN" embedding check; then
    "$GEAM_BIN" embedding sync
    git diff --unified=1 -- src/geam_bindings.rs | sed -n '1,120p'
    exit 1
  fi
  cargo test --locked
  cargo run --locked
}

check_standalone() {
  cd fixtures/gleam
  "$GEAM_BIN" prepare
  "$GEAM_BIN" run
  "$GEAM_BIN" build
}

check_executable() {
  local executable
  executable="$(ci_binary_path integrations/directories/fixtures/gleam geam_directories_fixture)"
  cd "$CI_RUN_DIR"
  "$executable"
}

main "$@"
