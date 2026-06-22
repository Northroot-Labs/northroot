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

## Delegated Agent Work

Agent work is allowed to use short-lived branches for checkpointing and draft
review. The branch name should make automation ownership obvious, such as
`codex/<scope>` for Codex worktrees or `agent/<scope>` for other delegated
agents.

Agents may:

- create and check out their own delegated branches;
- commit frequently on those branches as durable checkpoints;
- push delegated branches;
- open and update draft pull requests;
- keep working on draft pull requests until the change is ready for human final
  review and clearance.

Agents must not merge to protected branches, bypass review, hold long-lived
signing keys, or blur human and agent authorship. Use agent-only metadata when
the agent is the author. Use human+agent metadata only when an accountable human
is the author and the agent is a co-author.

## Local Repo Configuration

Agent worktrees should set a repo-local identity instead of falling back to a
machine account:

```bash
git config user.name "Codex"
git config user.email "codex@northroot.local"
```

Human+agent commits should use the human's normal GitHub-linked identity and
include the `Co-authored-by` trailer above.
