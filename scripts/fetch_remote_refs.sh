#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

REMOTE="${1:-${NORTHROOT_SYNC_REMOTE:-origin}}"

log() {
  printf '[northroot-fetch-refs] %s\n' "$*"
}

fail() {
  printf '[northroot-fetch-refs] error: %s\n' "$*" >&2
  exit 1
}

case "$REMOTE" in
  "" | -* | *[!A-Za-z0-9._-]*)
    fail "invalid remote name: ${REMOTE}"
    ;;
esac

git remote get-url "$REMOTE" >/dev/null 2>&1 \
  || fail "unknown remote: ${REMOTE}"

log "fetching branch refs from ${REMOTE}"
git fetch --prune --no-tags "$REMOTE" "+refs/heads/*:refs/remotes/${REMOTE}/*"
log "ok"
