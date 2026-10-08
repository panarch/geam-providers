#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != erlang ]]; then
  printf 'Unsupported gramps CI phase: %s\n' "${1:-}" >&2
  exit 1
fi
cd "${CI_OWNER_DIR:?}/fixtures/gleam"
gleam run --module reference
