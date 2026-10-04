#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../.github/scripts/ci-lib.sh"
cd "$CI_OWNER_DIR"

main() {
  case "${1:-all}" in
    all)
      ci_step 'clip: source and dependencies' check_source
      ci_step 'clip: original Erlang' run_erlang
      ci_step 'clip: embedding' check_embedding
      ci_step 'clip: standalone' check_standalone
      ci_step 'clip: executables outside their fixtures' check_executables
      ;;
    source) ci_step 'clip: source and dependencies' check_source ;;
    erlang) ci_step 'clip: original Erlang' run_erlang ;;
    embedding) ci_step 'clip: embedding' check_embedding ;;
    standalone) ci_step 'clip: standalone' check_standalone ;;
    executable) ci_step 'clip: executables outside their fixtures' check_executables ;;
    *) ci_fail "unknown clip phase: $1" ;;
  esac
}

check_source() {
  ci_check_geam_pin fixtures/embedding/Cargo.toml fixtures/embedding/Cargo.lock \
    fixtures/native/Cargo.toml fixtures/native/Cargo.lock \
    fixtures/gleam/.cargo/config.toml fixtures/gleam/Cargo.lock \
    fixtures/cli/.cargo/config.toml fixtures/cli/Cargo.lock
  local fixture
  for fixture in gleam cli embedding/gleam; do
    (cd "fixtures/$fixture" && gleam deps download && gleam format --check && gleam check)
    (cd "fixtures/$fixture/build/packages/clip" && ci_check_checksum "$CI_ROOT/integrations/clip/fixtures/upstream.sha256")
  done
  (cd fixtures/native && CARGO_TARGET_DIR=../embedding/target cargo fmt --all --check)
  (cd fixtures/native && CARGO_TARGET_DIR=../embedding/target cargo clippy --all-targets --locked -- -D warnings)
  (cd fixtures/native && CARGO_TARGET_DIR=../embedding/target RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked)
  (cd fixtures/embedding && cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings)
  (cd fixtures/embedding && RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked)
}

run_erlang() {
  (cd fixtures/gleam && gleam run)
  (cd fixtures/cli && gleam run -- --help)
  python3 fixtures/cli/check_cli.py --contracts --erlang-project "$(ci_native_path "$PWD/fixtures/gleam")"
  python3 fixtures/cli/check_cli.py --erlang-project "$(ci_native_path "$PWD/fixtures/cli")"
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
  local fixture
  for fixture in gleam cli; do (cd "fixtures/$fixture" && "$GEAM_BIN" prepare); done
  (cd fixtures/gleam && "$GEAM_BIN" run)
  (cd fixtures/cli && "$GEAM_BIN" run -m argv_contracts -- '' 'space value' -- -dash 한글)
  (cd fixtures/cli && "$GEAM_BIN" run -- search -p Rust ../../README.md)
  for fixture in gleam cli; do (cd "fixtures/$fixture" && "$GEAM_BIN" build); done
  python3 fixtures/cli/check_cli.py --geam-project "$(ci_native_path "$PWD/fixtures/cli")" \
    --geam-bin "$(ci_native_path "$GEAM_BIN")"
}

check_executables() {
  local contracts executable harness
  contracts="$(ci_binary_path integrations/clip/fixtures/gleam geam_clip_fixture)"
  executable="$(ci_binary_path integrations/clip/fixtures/cli geam_clip_search)"
  harness="$(ci_native_path "$PWD/fixtures/cli/check_cli.py")"
  cd "$CI_RUN_DIR"
  python3 "$harness" --contracts --executable "$(ci_native_path "$contracts")"
  python3 "$harness" --executable "$(ci_native_path "$executable")"
}

main "$@"
