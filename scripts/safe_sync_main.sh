#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

REMOTE="${NORTHROOT_SYNC_REMOTE:-origin}"
DEFAULT_BRANCH="${NORTHROOT_SYNC_DEFAULT_BRANCH:-main}"
REMOTE_BRANCH="${REMOTE}/${DEFAULT_BRANCH}"

log() {
  printf '[northroot-sync-main] %s\n' "$*"
}

fail() {
  printf '[northroot-sync-main] error: %s\n' "$*" >&2
  exit 1
}

bash scripts/fetch_remote_refs.sh "$REMOTE"

git show-ref --verify --quiet "refs/remotes/${REMOTE}/${DEFAULT_BRANCH}" \
  || fail "missing remote ref ${REMOTE_BRANCH}"

git show-ref --verify --quiet "refs/heads/${DEFAULT_BRANCH}" \
  || fail "missing local branch ${DEFAULT_BRANCH}"

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
  log "syncing checked-out ${DEFAULT_BRANCH} worktree: ${main_worktree}"

  dirty_status="$(git -C "$main_worktree" status --porcelain=v1 -uall)"
  if [[ -n "$dirty_status" ]]; then
    printf '%s\n' "$dirty_status" >&2
    fail "local ${DEFAULT_BRANCH} worktree is dirty; refusing to sync"
  fi

  current_branch="$(git -C "$main_worktree" symbolic-ref --quiet --short HEAD || true)"
  [[ "$current_branch" == "$DEFAULT_BRANCH" ]] \
    || fail "${main_worktree} is not on ${DEFAULT_BRANCH}"

  git -C "$main_worktree" merge-base --is-ancestor "$DEFAULT_BRANCH" "$REMOTE_BRANCH" \
    || fail "local ${DEFAULT_BRANCH} cannot fast-forward to ${REMOTE_BRANCH}"

  git -C "$main_worktree" merge --ff-only "$REMOTE_BRANCH"
  log "ok"
  exit 0
fi

log "local ${DEFAULT_BRANCH} is not checked out; updating branch ref only"

git merge-base --is-ancestor "$DEFAULT_BRANCH" "$REMOTE_BRANCH" \
  || fail "local ${DEFAULT_BRANCH} cannot fast-forward to ${REMOTE_BRANCH}"

old_sha="$(git rev-parse "$DEFAULT_BRANCH")"
new_sha="$(git rev-parse "$REMOTE_BRANCH")"

if [[ "$old_sha" == "$new_sha" ]]; then
  log "${DEFAULT_BRANCH} already matches ${REMOTE_BRANCH}"
  log "ok"
  exit 0
fi

git update-ref "refs/heads/${DEFAULT_BRANCH}" "$new_sha" "$old_sha"
log "fast-forwarded ${DEFAULT_BRANCH} to ${REMOTE_BRANCH}"
log "ok"
