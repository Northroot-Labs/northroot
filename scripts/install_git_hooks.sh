#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

HOOKS_DIR="$(git rev-parse --git-path hooks)"
mkdir -p "$HOOKS_DIR"

install_hook() {
  local name="$1"
  local src=".githooks/${name}"
  local dst="${HOOKS_DIR}/${name}"

  if [ ! -f "$src" ]; then
    echo "missing hook template: ${src}" >&2
    return 1
  fi

  cp "$src" "$dst"
  chmod +x "$dst"
  echo "installed ${dst}"
}

install_hook pre-commit
