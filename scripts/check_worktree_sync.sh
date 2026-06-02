#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

REMOTE="${NORTHROOT_SYNC_REMOTE:-origin}"
DEFAULT_BRANCH="${NORTHROOT_SYNC_DEFAULT_BRANCH:-main}"
FETCH="${NORTHROOT_SYNC_FETCH:-1}"
REMOTE_BRANCH="${REMOTE}/${DEFAULT_BRANCH}"

log() {
  printf '[northroot-sync-check] %s\n' "$*"
}

fail() {
  printf '[northroot-sync-check] error: %s\n' "$*" >&2
  exit 1
}

if [[ "$FETCH" != "0" ]]; then
  bash scripts/fetch_remote_refs.sh "$REMOTE"
fi

git show-ref --verify --quiet "refs/remotes/${REMOTE}/${DEFAULT_BRANCH}" \
  || fail "missing remote ref ${REMOTE_BRANCH}; fetch or set NORTHROOT_SYNC_DEFAULT_BRANCH"

head_sha="$(git rev-parse --short HEAD)"
remote_sha="$(git rev-parse --short "$REMOTE_BRANCH")"
branch="$(git symbolic-ref --quiet --short HEAD || true)"

if [[ -z "$branch" ]]; then
  branch="<detached>"
fi

log "root: $ROOT"
log "head: ${head_sha} (${branch})"
log "${REMOTE_BRANCH}: ${remote_sha}"

dirty_status="$(git status --porcelain=v1 -uall)"
if [[ -n "$dirty_status" ]]; then
  printf '%s\n' "$dirty_status" >&2
  fail "dirty worktree; commit, stash, or abandon local changes before syncing"
fi

main_worktree=""
current_worktree=""
while IFS= read -r line; do
  case "$line" in
    worktree\ *)
      current_worktree="${line#worktree }"
      ;;
    branch\ refs/heads/"${DEFAULT_BRANCH}")
      main_worktree="$current_worktree"
      ;;
  esac
done < <(git worktree list --porcelain)

if [[ -n "$main_worktree" ]]; then
  log "local ${DEFAULT_BRANCH} worktree: ${main_worktree}"
else
  log "local ${DEFAULT_BRANCH} worktree: <not checked out>"
fi

read -r ahead behind < <(git rev-list --left-right --count "HEAD...${REMOTE_BRANCH}")
log "relative to ${REMOTE_BRANCH}: ahead=${ahead} behind=${behind}"

if [[ "$branch" == "<detached>" ]]; then
  if [[ "$(git rev-parse HEAD)" == "$(git rev-parse "$REMOTE_BRANCH")" ]]; then
    log "detached at ${REMOTE_BRANCH}; create a task branch before editing"
    log "ok"
    exit 0
  fi
  fail "detached HEAD is not at ${REMOTE_BRANCH}; switch to a fresh branch from remote truth"
fi

if [[ "$branch" == "$DEFAULT_BRANCH" ]]; then
  if [[ "$ahead" != "0" ]]; then
    fail "local ${DEFAULT_BRANCH} has commits not present on ${REMOTE_BRANCH}; do not use it as an agent workspace"
  fi
  if [[ "$behind" != "0" ]]; then
    fail "local ${DEFAULT_BRANCH} is behind ${REMOTE_BRANCH}; run scripts/safe_sync_main.sh"
  fi
  log "ok"
  exit 0
fi

if git merge-base --is-ancestor HEAD "$REMOTE_BRANCH"; then
  fail "branch ${branch} is already contained in ${REMOTE_BRANCH}; remove it or start a fresh task branch"
fi

if [[ "$behind" != "0" ]]; then
  fail "branch ${branch} does not include latest ${REMOTE_BRANCH}; rebase/merge intentionally or start fresh"
fi

log "ok"
