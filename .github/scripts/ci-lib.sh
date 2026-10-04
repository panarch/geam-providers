#!/usr/bin/env bash
# Shared process, path and fixture checks. Package commands belong to their owner.

CI_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
export CI_ROOT

ci_fail() {
  printf 'CI: %s\n' "$*" >&2
  exit 1
}

ci_step() (
  set -euo pipefail
  local title="$1"
  shift
  if [[ "${GITHUB_ACTIONS:-}" == true ]]; then
    printf '::group::%s\n' "$title"
    trap 'printf "::endgroup::\n"' EXIT
  else
    printf '\n== %s ==\n' "$title"
  fi
  "$@"
)

ci_is_windows() {
  case "${RUNNER_OS:-$(uname -s)}" in
    Windows|MINGW*|MSYS*) return 0 ;;
    *) return 1 ;;
  esac
}

ci_jq() {
  # Native jq.exe converts LF to CRLF unless binary output is requested.
  if ci_is_windows; then jq --binary "$@"; else jq "$@"; fi
}

ci_native_path() {
  if ci_is_windows; then cygpath -w "$1"; else printf '%s\n' "$1"; fi
}

ci_shell_path() {
  if ci_is_windows; then cygpath -u "$1"; else printf '%s\n' "$1"; fi
}

ci_relative_path() {
  case "$1" in
    ''|/*|*\\*|*:*|..|../*|*/../*|*/..|./*|*/./*|*/.|*//*|*$'\n'*|*$'\r'*|*$'\t'*)
      ci_fail "expected a relative repository path: $1" ;;
  esac
}

ci_binary_path() {
  local suffix=''
  if ci_is_windows; then suffix='.exe'; fi
  printf '%s/%s/build/geam/target/debug/%s%s\n' "$CI_ROOT" "$1" "$2" "$suffix"
}

ci_check_geam_pin() {
  local file pattern
  : "${GEAM_REV:?Set GEAM_REV to the pinned repository Geam commit}"
  for file in "$@"; do
    case "$file" in
      */Cargo.lock) pattern="$GEAM_REV#$GEAM_REV" ;;
      *) pattern="rev = \"$GEAM_REV\"" ;;
    esac
    grep -Fq "$pattern" "$file" || ci_fail "Geam revision differs in $file"
  done
}

ci_check_checksum() {
  if command -v sha256sum >/dev/null; then
    sha256sum --check "$1"
  else
    shasum -a 256 -c "$1"
  fi
}

ci_require_geam() {
  : "${GEAM_BIN:?Set GEAM_BIN to the absolute path of the pinned Geam executable}"
  GEAM_BIN="$(ci_shell_path "$GEAM_BIN")"
  [[ "$GEAM_BIN" == /* && -x "$GEAM_BIN" ]] || ci_fail "GEAM_BIN must be an executable absolute path"
  export GEAM_BIN
}
