# Git Signing & Merge Policy

## Overview

Northroot enforces a two-tier signing policy to provide audit-grade provenance while maintaining low-friction iteration:

- **Tier A (Low Risk)**: CI/GitHub signing allowed
- **Tier B (High Risk)**: Human cryptographic attestation required

## Policy Summary

### Draft Commits (Unprotected Branches)

- Signing is **optional** on feature/agent branches
- Default configuration should be unsigned to maximize throughput
- Developers may sign individual commits when desired

### Accepted Commits (Protected Branches)

- Every commit on `main` **must** be signed
- Signature must be attributable to either:
  - **CI_SIGNER** (GitHub Verified) for Tier A changes
  - **HUMAN_SIGNER** (approved developer key) for Tier B changes

## Risk Tiers

### Tier A: Low Risk (CI/GitHub Signing Allowed)

Changes that do not materially impact security, money movement, proof validity, or invariants.

**Examples:**
- Documentation
- Comments
- Formatting/lint-only changes
- Non-prod tooling
- Refactors with no behavior change (bounded by tests)

**Policy:**
- Squash merge allowed with CI_SIGNER
- Required: PR + required checks + minimum 1 approval

### Tier B: High Risk (Human Sign-Off Required)

Changes that can alter correctness of proofs, receipts, invariants, or security posture.

**Tier B Paths:**
- `schemas/**`
- `crates/northroot-canonical/**`
- `crates/northroot-journal/src/verification.rs`
- `.github/workflows/**`
- `.github/signature-policy.yml` (self-protected: defines the policy)
- `Cargo.lock`
- `deny.toml`

**Policy:**
- PR **must** receive required approvals from CODEOWNERS
- Accepted commit **must** be signed by HUMAN_SIGNER
- CI/GitHub signer alone is **insufficient** for Tier B

## Human Attestation Flow (Tier B)

For PRs touching Tier B paths, after CODEOWNERS approval:

1. **Fetch the PR branch and note the HEAD SHA:**
   ```bash
   git fetch origin pull/$PR_NUMBER/head:$BRANCH_NAME
   git checkout $BRANCH_NAME
   HEAD_SHA=$(git rev-parse HEAD)
   ```

2. **Create a signed tag:**
   ```bash
   git tag -s "approved/pr-$PR_NUMBER@$HEAD_SHA" -m "Approving PR #$PR_NUMBER"
   ```

3. **Push the tag:**
   ```bash
   git push origin "approved/pr-$PR_NUMBER@$HEAD_SHA"
   ```

4. **CI will verify the tag signature and unblock merge**

## Human-Signed Checkpoints (Recommended)

Separately from Tier B PR attestations, Northroot uses **human-signed tags** as "frozen checkpoints" you can point auditors and downstream systems at.

### Release Checkpoints (`v*` tags)

- Create a signed tag like `v1.0.0` on the exact commit you want to freeze.
- Pushing this tag triggers the release workflow to build and publish binaries.

Example:
```bash
VERSION="1.0.0"
git tag -s "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"
```

### Internal Checkpoints (optional)

For non-release checkpoints (e.g. a “frozen” state for staging/prod rollout), use a separate tag namespace such as:
- `checkpoint/stage/<date>@<shortsha>`
- `checkpoint/prod/<date>@<shortsha>`

Example:
```bash
TAG="checkpoint/stage/2025-12-26@$(git rev-parse --short HEAD)"
git tag -s "$TAG" -m "Checkpoint: stage"
git push origin "$TAG"
```

These tags are explicitly human-signed, low-friction, and don’t require direct pushes to protected branches.

### Getting Your SSH Key Fingerprint

To add yourself as an allowed human signer, you need your SSH key fingerprint:

```bash
# For your default SSH key:
ssh-keygen -lf ~/.ssh/id_ed25519.pub

# Or for a specific key:
ssh-keygen -lf ~/.ssh/your_signing_key.pub

# If using ssh-agent:
ssh-add -l
```

The output format should be:
```
SHA256:XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

Add this fingerprint to `.github/signature-policy.yml` under `allowed_human_signers`.

### Configuring Git for SSH Signing

If you haven't set up SSH signing yet:

```bash
# Configure Git to use SSH for signing
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub

# Or specify the key explicitly:
git config --global user.signingkey "key::ssh-ed25519 AAAAC3NzaC1lZDI1NTE5..."
```

## Merge Gates

On `main`, the following are enforced:

1. **Pull request required** (no direct pushes)
2. **Status checks required** (tests, lint, build, signature-policy check)
3. **Minimum approvals:**
   - Tier A: ≥ 1 approval
   - Tier B: CODEOWNERS approval required
4. **Signed commits required** (GitHub setting)
5. **Restricted push**: Only via PR merge

## Agent Delegation Policy

Agents **may:**
- Create branches under `agent/*`
- Push draft commits to those branches
- Open/update PRs

Agents **must not:**
- Merge to protected branches
- Hold or access long-lived signing keys
- Modify branch protection or required checks

Agent tokens **must** be scoped to:
- Read repo + write branches + PRs
- **No** admin, **no** direct push to main, **no** workflow permission escalation

## Audit Trail

For each accepted change, we retain:
- PR link, merge timestamp, merge commit SHA
- Signer identity (CI or human)
- Approvals metadata (who approved, required reviewers satisfied)
- Required checks results

## Local Developer Defaults

**Default:** Do not require signing on every commit locally.

Sign manually when needed:
```bash
git commit -S -m "your message"
```

For Tier B approval tags, use the process described above.

## Troubleshooting

### Tag verification fails

- Ensure your SSH key is added to `signature-policy.yml`
- Verify the tag name format: `approved/pr-$NUMBER@$SHA`
- Check that the tag points to the PR's HEAD commit
- Ensure your SSH key is loaded in your agent: `ssh-add -l`

### CI check fails

- Check the workflow logs for specific error messages
- Verify the tag exists and is pushed: `git ls-remote --tags origin | grep approved`
- Ensure your fingerprint matches exactly (case-sensitive)

## Related Documentation

- [Threat Model](threat-model.md) - Security analysis
- [CONTRIBUTING.md](../../CONTRIBUTING.md) - General contribution guidelines
- [GOVERNANCE.md](../../GOVERNANCE.md) - Project principles

