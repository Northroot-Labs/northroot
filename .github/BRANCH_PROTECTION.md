# Branch Protection Settings

This document describes the required GitHub branch protection settings for `main` to enforce the Git Signing & Merge Policy.

## Manual Configuration Required

These settings must be configured manually in the GitHub repository settings:
**Settings → Branches → Branch protection rules → Add rule (for `main`)**

## Required Settings

### 1. Protect matching branches
- **Branch name pattern**: `main`

### 2. Require a pull request before merging
- ✅ **Require pull request reviews before merging**
  - Required number of approving reviews: `1`
  - ✅ **Require review from CODEOWNERS** (for Tier B paths)
  - ✅ **Dismiss stale pull request approvals when new commits are pushed**
  - ✅ **Require review from Code Owners** (enforces CODEOWNERS file)

### 3. Require status checks to pass before merging
- ✅ **Require status checks to pass before merging**
  - ✅ **Require branches to be up to date before merging**
  - **Status checks that are required:**
    - `Format Check` (from ci.yml)
    - `Clippy` (from ci.yml)
    - `Tests` (from ci.yml)
    - `Golden Tests` (from ci.yml)
    - `Verify Signature Policy` (from signature-policy.yml)

### 4. Require signed commits
- ✅ **Require signed commits**

### 5. Restrict pushes that create matching branches
- ✅ **Restrict who can push to matching branches**
  - Only allow specific actors (or leave empty to allow only PR merges)

### 6. Additional Settings (Recommended)
- ✅ **Do not allow bypassing the above settings**
- ✅ **Allow force pushes**: ❌ (disabled)
- ✅ **Allow deletions**: ❌ (disabled)

## Verification

After configuring, verify that:
1. Direct pushes to `main` are blocked
2. PRs require at least 1 approval
3. PRs touching Tier B paths require CODEOWNERS approval
4. All required status checks must pass
5. Only signed commits can be merged

## Related Documentation

- [Signing Policy](../../docs/security/signing-policy.md) - Full policy details
- [CODEOWNERS](CODEOWNERS) - Tier B path ownership

