# Secrets and Environment Handling

Goals: keep secrets out of images and Git, use standard tooling, and stay compatible with devcontainers/Cursor.

## Developer Workflow (devcontainer/Cursor)
- Run container as non-root; map your UID/GID to avoid permission issues.
- Forward SSH agent, don’t copy keys:
  - Docker: `-v $SSH_AUTH_SOCK:/ssh-agent:ro -e SSH_AUTH_SOCK=/ssh-agent`
  - VS Code/Cursor devcontainer snippet:
    ```json
    {
      "runArgs": [
        "-v", "${env:SSH_AUTH_SOCK}:/ssh-agent:ro",
        "-e", "SSH_AUTH_SOCK=/ssh-agent"
      ],
      "remoteEnv": { "SSH_AUTH_SOCK": "/ssh-agent" }
    }
    ```
- Optional secret mounts for local runs: `-v $HOME/.config/myapp/dev-secrets:/run/secrets:ro`.
- Preflight (suggested): check `SSH_AUTH_SOCK` exists and `ssh-add -l` succeeds; fail fast with guidance.
- Git signing: `git config --global gpg.format ssh` and set a dedicated signing key (do not rely on `head -1`).

## Options for K8s Secrets
- External Secrets Operator (ESO): syncs secrets from a backend (AWS/GCP/Azure SM or Vault) into namespaces; per-env KMS keys and paths; supports rotation without baking values into Git.
- SOPS + age: store only encrypted files in Git; per-env age key; decrypt in CI with short-lived cloud creds; write decrypted files to tmpfs before `kubectl apply`.
- CSI Secrets Store: mount secrets as volumes; good for rotation-sensitive data; combine with ESO where supported.

## Environment Separation
- Namespaces per env (dev/stage/prod). No cross-namespace secret reads; least-privilege RBAC per service account.
- Per-env backend scope and keys (e.g., `project-dev/*` with KMS-dev; `project-prod/*` with KMS-prod).
- Distinct ingress/DNS per env; never reuse prod pull secrets in lower envs.

## Rotation and Audit
- Prefer backend-driven rotation; set ESO/CSI sync intervals for low-TTL secrets.
- For manual rotation, use dual-credential pattern: provision new, roll out, revoke old.
- Ensure KMS/secret manager audit logs are enabled and reviewed.

## CI/CD Guidance
- Require `DOCKER_BUILDKIT=1` and `--secret` mounts for builds; exclude sample secret files in `.dockerignore`.
- CI decrypt/apply flow (SOPS path): obtain short-lived cloud creds, decrypt into tmpfs, apply manifests, shred tmpfs.
- Avoid echoing secrets in logs; use masked CI variables.

## Cursor Compatibility Checklist
- Non-root user inside container; HOME writable.
- Agent socket mount present; no Docker socket mount needed for normal dev.
- Tools/tests/web browsing run normally; secrets come from agent/secret mounts or env files kept outside the repo.

