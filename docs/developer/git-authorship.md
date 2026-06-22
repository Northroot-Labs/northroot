# Git Authorship

Northroot uses commit metadata to distinguish human-only, agent-only, and
human-plus-agent work.

## Human-Only

Use the human contributor as author and committer. Do not add an agent trailer.

```text
Author: Human Name <human@example.com>
```

## Agent-Only

Use the agent identity as author and committer.

```text
Author: Codex <codex@northroot.local>
```

## Human Plus Agent

Use the accountable human or GitHub user as the author, and add the agent as a
co-author trailer:

```text
feat: add node initializer

Co-authored-by: Codex <codex@northroot.local>
```

This keeps GitHub attribution readable while preserving delegation provenance.

## Local Repo Configuration

Agent worktrees should set a repo-local identity instead of falling back to a
machine account:

```bash
git config user.name "Codex"
git config user.email "codex@northroot.local"
```

Human+agent commits should use the human's normal GitHub-linked identity and
include the `Co-authored-by` trailer above.
