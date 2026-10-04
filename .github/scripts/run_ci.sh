#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/ci-lib.sh"

main() {
  case "${1:-}" in
    integration)
      [[ $# == 2 || $# == 3 ]] || ci_fail 'usage: run_ci.sh integration NAME [PHASE]'
      run_integration "integrations/$2" "${3:-all}"
      ;;
    integrations)
      [[ $# == 2 ]] || ci_fail 'usage: run_ci.sh integrations PHASE'
      run_integrations "$2"
      ;;
    provider)
      [[ $# == 3 ]] || ci_fail 'usage: run_ci.sh provider DIRECTORY PHASE'
      run_provider "$2" "$3"
      ;;
    *) ci_fail 'usage: run_ci.sh {integration NAME [PHASE]|integrations PHASE|provider DIRECTORY PHASE}' ;;
  esac
}

run_integration() (
  set_owner "$1"
  local phase="$2" script
  case "$phase" in
    source|erlang) ;;
    all|embedding|standalone|executable) ci_require_geam ;;
    *) ci_fail "unknown integration phase: $phase" ;;
  esac
  script="$(jq -er '.script | select(type == "string" and length > 0)' "$CI_OWNER_DIR/ci.json")"
  set_script "$script"
  if [[ "$phase" == all || "$phase" == executable ]]; then create_run_directory; fi
  printf 'Running %s (%s)\n' "$1" "$phase"
  bash "$CI_SCRIPT" "$phase"
)

run_integrations() {
  local cases
  : "${CI_INTEGRATIONS:?Set CI_INTEGRATIONS to the selected integration rows}"
  cases="$(jq -ce 'select(type == "array" and all(.[]; .dir | type == "string"))' <<< "$CI_INTEGRATIONS")"
  jq -r '.[].dir' <<< "$cases" | while IFS= read -r directory; do
    run_integration "$directory" "$1"
  done
}

run_provider() (
  set_owner "$1"
  local phase="$2" metadata row script
  case "$phase" in
    erlang|embedding) ;;
    standalone|executable) ci_require_geam ;;
    *) ci_fail "unknown provider phase: $phase" ;;
  esac
  metadata="$(cd "$CI_ROOT" && cargo metadata --no-deps --format-version 1 --locked)"
  row="$(jq -ce --arg dir "$1" '
    .workspace_members as $members
    | [.packages[]
        | select(.id as $id | $members | index($id))
        | select(.manifest_path | gsub("\\\\"; "/") | endswith("/" + $dir + "/Cargo.toml"))
        | {crate: .name, ci: (if .metadata.geam | has("ci") then .metadata.geam.ci else {} end)}]
    | select(length == 1) | .[0]
    | select(.ci | type == "object")
    | (.ci | if has("script") then .script else "" end) as $script
    | (.ci | if has("script-phases") then ."script-phases" else [] end) as $phases
    | select((.ci | has("integration-case") | not)
        and ($script | type == "string") and ($phases | type == "array"))
    | select(($script | length > 0) == ($phases | length > 0))
    | select(($phases | unique | length) == ($phases | length))
    | select($phases | all(.[]; . == "erlang" or . == "embedding" or . == "standalone" or . == "executable"))
    | select(($phases | index("erlang") == null) or .ci."erlang-oracle" == true)
  ' <<< "$metadata")" || ci_fail "invalid provider CI declaration for $1"
  script="$(jq -r '.ci.script // ""' <<< "$row")"
  if [[ "$phase" == executable ]]; then
    local binary
    binary="$(jq -r '.ci."fixture-bin" // (.crate | gsub("-"; "_") + "_fixture")' <<< "$row")"
    [[ "$binary" =~ ^[A-Za-z0-9_-]+$ ]] || ci_fail 'invalid fixture executable name'
    CI_FIXTURE_BINARY="$(ci_binary_path "$1/fixtures/gleam" "$binary")"
    export CI_FIXTURE_BINARY
    create_run_directory
  fi
  if jq -e --arg phase "$phase" '(.ci."script-phases" // []) | index($phase) != null' <<< "$row" >/dev/null; then
    set_script "$script"
    ci_step "$1 ($phase)" bash "$CI_SCRIPT" "$phase"
  else
    ci_step "$1 ($phase)" run_standard_provider "$phase" "${CI_RUN_DIR:-}" "${CI_FIXTURE_BINARY:-}"
  fi
)

run_standard_provider() {
  case "$1" in
    erlang) cd "$CI_OWNER_DIR/fixtures/gleam"; gleam run ;;
    embedding) cd "$CI_OWNER_DIR/fixtures/embedding"; cargo run --locked ;;
    standalone) cd "$CI_OWNER_DIR/fixtures/gleam"; "$GEAM_BIN" run ;;
    executable) cd "$2"; "$3" ;;
  esac
}

set_owner() {
  ci_relative_path "$1"
  CI_OWNER_DIR="$(cd "$CI_ROOT/$1" && pwd -P)"
  [[ "$CI_OWNER_DIR" == "$CI_ROOT/"* ]] || ci_fail 'CI owner must be inside the repository'
  export CI_OWNER_DIR
}

set_script() {
  ci_relative_path "$1"
  local parent
  parent="$(cd "$CI_OWNER_DIR/$(dirname "$1")" && pwd -P)"
  [[ "$parent" == "$CI_OWNER_DIR" || "$parent" == "$CI_OWNER_DIR/"* ]] || ci_fail 'CI script must belong to its owner'
  CI_SCRIPT="$parent/$(basename "$1")"
  [[ -f "$CI_SCRIPT" && ! -L "$CI_SCRIPT" && "$CI_SCRIPT" == *.sh ]] || ci_fail 'CI script must be a regular Bash file'
}

create_run_directory() {
  local canonical
  CI_RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/geam-ci.XXXXXX")"
  trap 'rm -rf "$CI_RUN_DIR"' EXIT
  canonical="$(cd "$CI_RUN_DIR" && pwd -P)"
  CI_RUN_DIR="$canonical"
  [[ "$CI_RUN_DIR" != "$CI_OWNER_DIR" && "$CI_RUN_DIR" != "$CI_OWNER_DIR/"* ]] || \
    ci_fail 'executable working directory must be outside its CI owner'
  export CI_RUN_DIR
}

main "$@"
