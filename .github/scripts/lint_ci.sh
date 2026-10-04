#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

actionlint
git ls-files --cached --others --exclude-standard -z -- '*.sh' |
  while IFS= read -r -d '' script; do
    shellcheck -x -P SCRIPTDIR "$script"
  done
