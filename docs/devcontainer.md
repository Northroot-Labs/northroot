# Devcontainer Guidelines (Cursor-compatible)

## Goals
Run as non-root, keep keys off the image, and make tools/tests/web browsing work without extra privileges.

## Runtime Settings
- Map UID/GID to your host user to avoid permission issues.
- Forward host ssh-agent (no key copies); this also reuses host-side passphrase caching. Load your key once on the host (`ssh-add -t 1h ~/.ssh/id_ed25519_signing`) and reuse in the container:
  - Docker: `-v $SSH_AUTH_SOCK:/ssh-agent:ro -e SSH_AUTH_SOCK=/ssh-agent`
  - Devcontainer snippet:
    ```json
    {
      "runArgs": [
        "-v", "${env:SSH_AUTH_SOCK}:/ssh-agent:ro",
        "-e", "SSH_AUTH_SOCK=/ssh-agent"
      ],
      "remoteEnv": { "SSH_AUTH_SOCK": "/ssh-agent" }
    }
    ```
- Optional secret mounts (read-only): `-v $HOME/.config/myapp/dev-secrets:/run/secrets:ro`.
- Avoid mounting the Docker socket; use BuildKit/CI for builds that need registry creds.

## Git Signing Quick Start
- `git config --global gpg.format ssh`
- `git config --global user.signingkey <your-signing-pubkey>` (pick the right key, not `head -1`)
- `git config --global commit.gpgsign true`

## Passphrase Caching (TTL)
- On host: load your signing key with a TTL to avoid repeated prompts:
  - `ssh-add -t 1h ~/.ssh/id_ed25519_signing` (1 hour timeout)
  - `ssh-add -t 4h ~/.ssh/id_ed25519_signing` (4 hour timeout)
  - Adjust duration as needed; the container inherits this caching via the forwarded agent.
- To check loaded keys: `ssh-add -l` (works in container too).

## Preflight Check (suggested)
- Ensure `SSH_AUTH_SOCK` is set on host.
- `ssh-add -l` succeeds in container (agent reachable and key loaded).
- Workspace is writable by the non-root user.

## Cursor Compatibility
- Non-root user with writable HOME and workspace.
- Agent socket mount present so tools (git, SSH) work without key files.
- Tests/tools/web browsing run normally; secrets provided via agent/secret mounts or local env files outside the repo.

